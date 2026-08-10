#[cfg(test)]
mod tests {
    use crate::audio::dsp::chain::DspChain;
    use crate::audio::dsp::night_mode::apply_night_mode;
    use crate::audio::dsp::params::DspParams;

    #[test]
    fn test_night_mode_peak_reduction() {
        let sample_rate = 44100.0;
        let mut baseline_params = DspParams::default();
        let mut night_params = DspParams::default();
        apply_night_mode(&mut night_params, true);

        assert!(night_params.is_night_mode);
        assert_eq!(night_params.active_preset, "Night Mode");

        // Synthesize 1.0s stereo signal with a massive transient peak spike
        let num_samples = 44100;
        let mut test_buffer = vec![0.0f32; num_samples * 2];
        for i in 0..num_samples {
            let t = i as f32 / sample_rate;
            let val = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
            test_buffer[i * 2] = val;
            test_buffer[i * 2 + 1] = val;
        }
        // Insert a 2.5 amplitude spike at sample 10000
        for i in 10000..10100 {
            test_buffer[i * 2] = 2.5;
            test_buffer[i * 2 + 1] = 2.5;
        }

        let mut baseline_chain = DspChain::new(sample_rate);
        let mut night_chain = DspChain::new(sample_rate);

        let mut baseline_buf = test_buffer.clone();
        let mut night_buf = test_buffer;

        baseline_chain.process(&mut baseline_buf, &baseline_params);
        night_chain.process(&mut night_buf, &night_params);

        let baseline_max = baseline_buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let night_max = night_buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        // Assert night mode peak is strictly constrained below ceiling (< 0.98)
        assert!(
            night_max < 0.98,
            "Night mode peak peak should be constrained below ceiling, got {}",
            night_max
        );
        assert!(
            night_max < baseline_max,
            "Night mode peak ({}) should be smaller than baseline peak ({})",
            night_max,
            baseline_max
        );
    }
}
