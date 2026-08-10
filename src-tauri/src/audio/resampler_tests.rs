#[cfg(test)]
mod tests {
    use crate::audio::resampler::AudioResampler;

    #[test]
    fn test_mono_to_stereo_upmix() {
        let mut resampler = AudioResampler::new(44100, 44100, 1, 2);
        let mono_input = vec![0.5f32, -0.5f32, 0.8f32];
        let stereo_output = resampler.process(&mono_input);

        assert_eq!(stereo_output.len(), 6);
        assert_eq!(stereo_output[0], 0.5);
        assert_eq!(stereo_output[1], 0.5);
        assert_eq!(stereo_output[2], -0.5);
        assert_eq!(stereo_output[3], -0.5);
    }

    #[test]
    fn test_stereo_to_mono_downmix() {
        let mut resampler = AudioResampler::new(44100, 44100, 2, 1);
        let stereo_input = vec![0.4f32, 0.6f32, -0.2f32, -0.4f32];
        let mono_output = resampler.process(&stereo_input);

        assert_eq!(mono_output.len(), 2);
        assert!((mono_output[0] - 0.5).abs() < 1e-4);
        assert!((mono_output[1] - (-0.3)).abs() < 1e-4);
    }

    #[test]
    fn test_sample_rate_resampling_ratio() {
        let mut resampler = AudioResampler::new(44100, 48000, 2, 2);
        let num_src_frames = 44100;
        let mut input = vec![0.0f32; num_src_frames * 2];
        for i in 0..num_src_frames {
            let t = i as f32 / 44100.0;
            let val = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            input[i * 2] = val;
            input[i * 2 + 1] = val;
        }

        let output = resampler.process(&input);
        let expected_out_frames = 48000;
        let actual_out_frames = output.len() / 2;

        // Verify frame count ratio within 1% boundary
        assert!(
            (actual_out_frames as i32 - expected_out_frames as i32).abs() < 50,
            "Resampled frame count {} should match target 48000",
            actual_out_frames
        );
    }

    #[test]
    fn test_soft_clipping_guard() {
        let mut resampler = AudioResampler::new(44100, 44100, 1, 1);
        let loud_input = vec![1.5f32, -1.8f32, 2.0f32];
        let output = resampler.process(&loud_input);

        for sample in output {
            assert!(
                sample <= 1.0 && sample >= -1.0,
                "Sample {} exceeds range [-1.0, 1.0]",
                sample
            );
        }
    }
}
