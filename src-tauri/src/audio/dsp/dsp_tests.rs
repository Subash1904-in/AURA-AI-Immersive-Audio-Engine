#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use arc_swap::ArcSwap;
    use rustfft::{num_complex::Complex, FftPlanner};
    use std::sync::Arc;

    use crate::audio::dsp::chain::DspChain;
    use crate::audio::dsp::params::{CompressorParams, DspParams, FilterType};

    fn generate_sine_wave(freq_hz: f32, sample_rate: f32, num_samples: usize) -> Vec<f32> {
        let mut buffer = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate;
            let val = (2.0 * std::f32::consts::PI * freq_hz * t).sin();
            buffer.push(val);
        }
        buffer
    }

    fn compute_magnitude_at_freq(
        mono_samples: &[f32],
        target_freq_hz: f32,
        sample_rate: f32,
    ) -> f32 {
        let n = mono_samples.len();
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(n);

        let mut buffer: Vec<Complex<f32>> =
            mono_samples.iter().map(|&s| Complex::new(s, 0.0)).collect();

        fft.process(&mut buffer);

        let target_bin = (target_freq_hz * n as f32 / sample_rate).round() as usize;
        let bin = target_bin.clamp(0, n / 2);
        (buffer[bin].norm() * 2.0) / n as f32
    }

    #[test]
    fn test_click_pop_discontinuity_on_stage_toggles() {
        let sample_rate = 44100.0;
        let channels = 2;
        let num_frames = 2048;
        let input_signal = generate_sine_wave(440.0, sample_rate, num_samples_stereo(num_frames));

        let initial_params = DspParams::default();
        let params_bus = Arc::new(ArcSwap::from_pointee(initial_params));
        let mut dsp = DspChain::new(sample_rate, channels, params_bus.clone());

        let stage_names = [
            "eq",
            "bass",
            "compressor",
            "loudness",
            "spatial",
            "reverb",
            "limiter",
        ];

        for &stage in &stage_names {
            let mut test_buffer = input_signal.clone();

            // Process first half with stage disabled
            let half = test_buffer.len() / 2;
            dsp.process_interleaved(&mut test_buffer[..half]);

            // Toggle stage mid-buffer
            let mut updated_params = params_bus.load_full().as_ref().clone();
            match stage {
                "eq" => updated_params.eq_enabled = true,
                "bass" => updated_params.bass_enabled = true,
                "compressor" => updated_params.compressor_enabled = true,
                "loudness" => updated_params.loudness_enabled = true,
                "spatial" => updated_params.spatial_enabled = true,
                "reverb" => updated_params.reverb_enabled = true,
                "limiter" => updated_params.limiter_enabled = true,
                _ => {}
            }
            params_bus.store(Arc::new(updated_params));

            // Process second half with stage enabled
            dsp.process_interleaved(&mut test_buffer[half..]);

            // Check sample-to-sample discontinuity across toggle boundary
            let mut max_delta = 0.0f32;
            for i in 1..test_buffer.len() {
                let delta = (test_buffer[i] - test_buffer[i - 1]).abs();
                max_delta = max_delta.max(delta);
            }

            // Assert mid-buffer toggle does not exceed click/pop threshold (0.15)
            assert!(
                max_delta <= 0.15,
                "Click/pop discontinuity detected on stage toggle '{}': max delta = {}",
                stage,
                max_delta
            );
        }
    }

    #[test]
    fn test_compressor_peak_and_rms_reduction() {
        let sample_rate = 44100.0;
        let channels = 2;
        let num_samples = num_samples_stereo(4096);

        // Signal exceeding compressor threshold (-12 dB = 0.251 amplitude) with 0.9 amplitude
        let loud_signal = generate_sine_wave(1000.0, sample_rate, num_samples)
            .into_iter()
            .map(|s| s * 0.9)
            .collect::<Vec<f32>>();

        let bypassed_params = DspParams {
            compressor_enabled: false,
            ..Default::default()
        };
        let params_bus = Arc::new(ArcSwap::from_pointee(bypassed_params));

        let mut dsp_bypassed = DspChain::new(sample_rate, channels, params_bus.clone());
        let mut bypassed_output = loud_signal.clone();
        dsp_bypassed.process_interleaved(&mut bypassed_output);

        let enabled_params = DspParams {
            compressor_enabled: true,
            compressor: CompressorParams {
                threshold_db: -12.0,
                ratio: 4.0,
                attack_ms: 1.0,
                release_ms: 50.0,
                ..Default::default()
            },
            ..Default::default()
        };
        params_bus.store(Arc::new(enabled_params));

        let mut dsp_enabled = DspChain::new(sample_rate, channels, params_bus.clone());
        let mut compressed_output = loud_signal.clone();
        dsp_enabled.process_interleaved(&mut compressed_output);

        let bypassed_peak = bypassed_output
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        let compressed_peak = compressed_output[num_samples / 2..]
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);

        let bypassed_rms =
            (bypassed_output.iter().map(|s| s * s).sum::<f32>() / num_samples as f32).sqrt();
        let compressed_rms = (compressed_output[num_samples / 2..]
            .iter()
            .map(|s| s * s)
            .sum::<f32>()
            / (num_samples / 2) as f32)
            .sqrt();

        // Assert compressor reduced peak and RMS levels
        assert!(
            compressed_peak < bypassed_peak * 0.85,
            "Compressor failed to reduce peak: compressed peak = {}, bypassed peak = {}",
            compressed_peak,
            bypassed_peak
        );

        assert!(
            compressed_rms < bypassed_rms * 0.85,
            "Compressor failed to reduce RMS: compressed RMS = {}, bypassed RMS = {}",
            compressed_rms,
            bypassed_rms
        );
    }

    #[test]
    fn test_eq_boost_and_cut_measured_via_fft() {
        let sample_rate = 44100.0;
        let channels = 2;
        let fft_size = 2048;
        let num_frames = 4096;
        let total_samples = num_samples_stereo(num_frames);

        // Pick exact integer bin frequency (bin 46) to eliminate spectral leakage
        let target_freq = (46.0 * sample_rate) / (fft_size as f32); // 990.52734 Hz
        let input_signal = generate_sine_wave(target_freq, sample_rate, total_samples);

        // Bypassed EQ
        let bypassed_params = DspParams {
            eq_enabled: false,
            ..Default::default()
        };
        let params_bus = Arc::new(ArcSwap::from_pointee(bypassed_params));

        let mut dsp = DspChain::new(sample_rate, channels, params_bus.clone());
        let mut bypassed_out = input_signal.clone();
        dsp.process_interleaved(&mut bypassed_out);

        // Extract mono channel 0 from steady-state (second half)
        let mono_bypassed: Vec<f32> = bypassed_out[total_samples / 2..]
            .iter()
            .step_by(channels)
            .copied()
            .collect();
        let bypassed_mag = compute_magnitude_at_freq(&mono_bypassed, target_freq, sample_rate);

        // Boosted EQ (+6 dB at target_freq)
        let mut boost_params = DspParams {
            eq_enabled: true,
            ..Default::default()
        };
        boost_params.eq.bands[2].frequency = target_freq;
        boost_params.eq.bands[2].gain_db = 6.0;
        boost_params.eq.bands[2].filter_type = FilterType::Peaking;
        params_bus.store(Arc::new(boost_params));

        let mut dsp_boosted = DspChain::new(sample_rate, channels, params_bus.clone());
        let mut boost_out = input_signal.clone();
        dsp_boosted.process_interleaved(&mut boost_out);

        // Extract mono channel 0 from steady-state (second half)
        let mono_boosted: Vec<f32> = boost_out[total_samples / 2..]
            .iter()
            .step_by(channels)
            .copied()
            .collect();
        let boost_mag = compute_magnitude_at_freq(&mono_boosted, target_freq, sample_rate);

        // +6 dB boost corresponds to 10^(6/20) ≈ 1.995x magnitude increase in steady state
        assert!(
            boost_mag > bypassed_mag * 1.25,
            "EQ +6dB boost failed FFT verification: boosted mag = {}, bypassed mag = {}",
            boost_mag,
            bypassed_mag
        );
    }

    fn num_samples_stereo(frames: usize) -> usize {
        frames * 2
    }
}
