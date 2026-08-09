#[cfg(test)]
#[allow(
    clippy::field_reassign_with_default,
    clippy::needless_range_loop,
    clippy::manual_repeat_n
)]
mod tests {
    use arc_swap::ArcSwap;
    use std::sync::Arc;

    use crate::audio::dsp::chain::DspChain;
    use crate::audio::dsp::params::{DspParams, ReverbEnvironment, ReverbParams, SpatialParams};
    use crate::audio::reverb::ConvolutionReverb;
    use crate::audio::spatial::widener::StereoWidener;

    fn generate_sine_stereo(freq_hz: f32, sample_rate: f32, num_frames: usize) -> Vec<f32> {
        let mut buffer = Vec::with_capacity(num_frames * 2);
        for i in 0..num_frames {
            let t = i as f32 / sample_rate;
            let val = (2.0 * std::f32::consts::PI * freq_hz * t).sin();
            buffer.push(val); // L
            buffer.push(val); // R (identical mono-in-stereo)
        }
        buffer
    }

    fn generate_white_noise_stereo(num_frames: usize) -> Vec<f32> {
        let mut buffer = Vec::with_capacity(num_frames * 2);
        let mut seed: u32 = 12345;
        for _ in 0..(num_frames * 2) {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let val = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            buffer.push(val);
        }
        buffer
    }

    fn rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    // --- Test 1: Environment crossfade discontinuity ---

    #[test]
    fn test_environment_crossfade_no_discontinuity() {
        let sample_rate = 44100.0;
        let channels = 2;
        let num_frames = 4096;

        let transitions = [
            (ReverbEnvironment::Off, ReverbEnvironment::SmallRoom),
            (ReverbEnvironment::SmallRoom, ReverbEnvironment::ConcertHall),
            (ReverbEnvironment::ConcertHall, ReverbEnvironment::Cathedral),
            (ReverbEnvironment::Cathedral, ReverbEnvironment::Cave),
        ];

        for (from_env, to_env) in &transitions {
            let input_signal = generate_sine_stereo(440.0, sample_rate, num_frames);
            let mut params = ReverbParams {
                environment: *from_env,
                wet_dry_mix: 0.5,
            };

            let mut reverb = ConvolutionReverb::new(sample_rate, channels, &params);

            let mut test_buffer = input_signal.clone();
            let half = test_buffer.len() / 2;

            // Process first half with old environment
            reverb.process_interleaved(&mut test_buffer[..half]);

            // Switch environment mid-buffer
            params.environment = *to_env;
            reverb.update_params(&params);

            // Process second half with new environment (crossfading)
            reverb.process_interleaved(&mut test_buffer[half..]);

            // Check channel-aligned sample-to-sample derivative across transition boundary
            // Interleaved stereo: half is Left[half_frame], half - 2 is Left[half_frame - 1]
            let left_boundary_delta = (test_buffer[half] - test_buffer[half - 2]).abs();
            let left_prev_delta = (test_buffer[half - 2] - test_buffer[half - 4]).abs();

            let right_boundary_delta = (test_buffer[half + 1] - test_buffer[half - 1]).abs();
            let right_prev_delta = (test_buffer[half - 1] - test_buffer[half - 3]).abs();

            let left_allowed = (left_prev_delta * 2.0).max(0.15);
            let right_allowed = (right_prev_delta * 2.0).max(0.15);

            assert!(
                left_boundary_delta <= left_allowed,
                "Left channel crossfade discontinuity on transition {:?} -> {:?}: boundary delta = {}, allowed = {}",
                from_env,
                to_env,
                left_boundary_delta,
                left_allowed
            );

            assert!(
                right_boundary_delta <= right_allowed,
                "Right channel crossfade discontinuity on transition {:?} -> {:?}: boundary delta = {}, allowed = {}",
                from_env,
                to_env,
                right_boundary_delta,
                right_allowed
            );
        }
    }

    // --- Test 2: HRTF cross-correlation ---

    #[test]
    fn test_hrtf_produces_binaural_output() {
        let sample_rate = 44100.0;
        let channels = 2;
        let num_frames = 4096;

        // Create a mono-in-stereo test tone
        let input_signal = generate_sine_stereo(1000.0, sample_rate, num_frames);

        // Process with HRTF enabled
        let mut params = DspParams::default();
        params.spatial_enabled = true;
        params.spatial = SpatialParams {
            width: 1.0,
            crossfeed_level: 0.0, // Disable crossfeed to isolate HRTF
            hrtf_enabled: true,
        };

        let params_bus = Arc::new(ArcSwap::from_pointee(params));
        let mut dsp = DspChain::new(sample_rate, channels, params_bus);

        let mut output = input_signal.clone();
        dsp.process_interleaved(&mut output);

        // Extract L and R channels from second half (steady state)
        let start = num_frames; // Second half, interleaved offset
        let left: Vec<f32> = output[start..].iter().step_by(2).copied().collect();
        let right: Vec<f32> = output[start..].iter().skip(1).step_by(2).copied().collect();

        // HRTF should produce non-trivial output
        let left_rms = rms(&left);
        let right_rms = rms(&right);

        assert!(
            left_rms > 0.001,
            "HRTF left channel output too quiet: RMS = {}",
            left_rms
        );
        assert!(
            right_rms > 0.001,
            "HRTF right channel output too quiet: RMS = {}",
            right_rms
        );

        // For front azimuth (0°), L and R should be similar (correlated)
        // Compute normalized cross-correlation at lag 0
        let n = left.len().min(right.len());
        let dot: f32 = left
            .iter()
            .zip(right.iter())
            .take(n)
            .map(|(l, r)| l * r)
            .sum();
        let norm_l: f32 = left.iter().take(n).map(|s| s * s).sum::<f32>().sqrt();
        let norm_r: f32 = right.iter().take(n).map(|s| s * s).sum::<f32>().sqrt();

        let cross_corr = if norm_l > 1e-6 && norm_r > 1e-6 {
            dot / (norm_l * norm_r)
        } else {
            0.0
        };

        assert!(
            cross_corr >= 0.5,
            "HRTF front azimuth L/R cross-correlation too low: {}",
            cross_corr
        );
    }

    // --- Test 3: Reverb wet/dry energy scaling ---

    #[test]
    fn test_reverb_wet_dry_energy_scaling() {
        let sample_rate = 44100.0;
        let channels = 2;
        let num_frames = 8192;

        let input_signal = generate_white_noise_stereo(num_frames);

        let mut rms_values = Vec::new();

        for mix in [0.0f32, 0.5, 1.0] {
            let params = ReverbParams {
                environment: ReverbEnvironment::SmallRoom,
                wet_dry_mix: mix,
            };

            let mut reverb = ConvolutionReverb::new(sample_rate, channels, &params);
            let mut output = input_signal.clone();
            reverb.process_interleaved(&mut output);

            // Measure RMS from second half (past initial convolution latency)
            let rms_val = rms(&output[num_frames..]);
            rms_values.push((mix, rms_val));
        }

        // At mix=0.0 (fully dry) and mix=1.0 (fully wet), output should be non-zero
        assert!(
            rms_values[0].1 > 0.01,
            "Dry-only RMS too low: {}",
            rms_values[0].1
        );
        assert!(
            rms_values[2].1 > 0.001,
            "Wet-only RMS too low: {}",
            rms_values[2].1
        );
    }

    // --- Test 4: Stereo widener correctness ---

    #[test]
    fn test_stereo_widener_mono_and_wide() {
        // Width=0 should produce mono (L==R)
        let mut mono_buf = vec![0.8f32, -0.2, 0.5, 0.1, -0.3, 0.6];
        let widener = StereoWidener::new(0.0);
        widener.process_interleaved(&mut mono_buf);

        for frame in mono_buf.chunks(2) {
            assert!(
                (frame[0] - frame[1]).abs() < 1e-6,
                "Width=0 did not produce mono: L={}, R={}",
                frame[0],
                frame[1]
            );
        }

        // Width=2 should increase difference between L and R
        let input = vec![0.8f32, -0.2, 0.5, 0.1, -0.3, 0.6];
        let mut wide_buf = input.clone();
        let widener_wide = StereoWidener::new(2.0);
        widener_wide.process_interleaved(&mut wide_buf);

        // Check that the side component is amplified
        let orig_side_energy: f32 = input.chunks(2).map(|c| ((c[0] - c[1]) * 0.5).powi(2)).sum();
        let wide_side_energy: f32 = wide_buf
            .chunks(2)
            .map(|c| ((c[0] - c[1]) * 0.5).powi(2))
            .sum();

        assert!(
            wide_side_energy > orig_side_energy * 1.5,
            "Width=2 did not widen: orig side energy = {}, wide side energy = {}",
            orig_side_energy,
            wide_side_energy
        );
    }
}
