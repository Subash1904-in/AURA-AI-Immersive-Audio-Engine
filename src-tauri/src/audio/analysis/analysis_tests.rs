#[cfg(test)]
#[allow(clippy::field_reassign_with_default, clippy::needless_range_loop)]
mod tests {
    use arc_swap::ArcSwap;
    use std::sync::Arc;

    use crate::audio::analysis::bpm::BpmDetector;
    use crate::audio::analysis::classifier::Classifier;
    use crate::audio::analysis::presets::PresetEngine;
    use crate::audio::analysis::spectral::SpectralExtractor;
    use crate::audio::dsp::chain::DspChain;
    use crate::audio::dsp::params::DspParams;

    fn generate_click_track(bpm: f32, sample_rate: f32, total_seconds: f32) -> Vec<f32> {
        let total_samples = (total_seconds * sample_rate) as usize;
        let mut buffer = vec![0.0f32; total_samples];
        let click_interval = (sample_rate * 60.0 / bpm) as usize;

        let mut i = 0;
        while i < total_samples {
            // Short 5ms 1kHz click pulse
            for k in 0..220 {
                if i + k < total_samples {
                    let t = k as f32 / sample_rate;
                    buffer[i + k] = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * 0.9;
                }
            }
            i += click_interval;
        }

        buffer
    }

    fn generate_sine_stereo(freq_hz: f32, sample_rate: f32, num_frames: usize) -> Vec<f32> {
        let mut buffer = Vec::with_capacity(num_frames * 2);
        for i in 0..num_frames {
            let t = i as f32 / sample_rate;
            let val = (2.0 * std::f32::consts::PI * freq_hz * t).sin() * 0.8;
            buffer.push(val);
            buffer.push(val);
        }
        buffer
    }

    // --- Requirement 7.1: BPM detection accuracy against synthetic click track ---

    #[test]
    fn test_bpm_detection_accuracy_synthetic_click_track() {
        let target_bpm = 120.0f32;
        let sample_rate = 44100.0f32;
        let click_track = generate_click_track(target_bpm, sample_rate, 4.0);

        let detected_bpm = BpmDetector::estimate_bpm_from_buffer(&click_track, sample_rate);

        assert!(
            (detected_bpm - target_bpm).abs() <= 3.0,
            "BPM detection failed: expected {} ± 3, got {}",
            target_bpm,
            detected_bpm
        );
    }

    // --- Requirement 7.2: Model-missing fallback test ---

    #[test]
    fn test_model_missing_fallback() {
        // Pass non-existent path to Classifier
        let missing_path = "non_existent_path_to_onnx_model.onnx";
        let classifier = Classifier::new(Some(missing_path));

        // 1. Assert classifier gracefully initializes with is_onnx_loaded = false
        assert!(
            !classifier.is_onnx_loaded(),
            "Expected is_onnx_loaded to be false when model file is missing"
        );

        // 2. Assert classification does not panic and returns valid fallback prediction
        let extractor = SpectralExtractor::new(44100.0, 2048);
        let samples = generate_click_track(120.0, 44100.0, 1.0);
        let features = extractor.extract(&samples);

        let result = classifier.classify(&features, 120.0);

        assert!(
            !result.genre.is_empty(),
            "Genre prediction should not be empty"
        );
        assert!(
            result.mood_valence >= 0.0 && result.mood_valence <= 1.0,
            "Valence should be in 0.0..=1.0 range"
        );
        assert!(
            result.mood_energy >= 0.0 && result.mood_energy <= 1.0,
            "Energy should be in 0.0..=1.0 range"
        );
        assert!(
            !result.is_onnx_loaded,
            "Result should reflect fallback state"
        );
    }

    // --- Requirement 7.3: Auto <-> Manual toggle boundary discontinuity test ---

    #[test]
    fn test_auto_manual_toggle_discontinuity() {
        let sample_rate = 44100.0;
        let channels = 2;
        let num_frames = 4096;

        let input_signal = generate_sine_stereo(440.0, sample_rate, num_frames);
        let manual_params = DspParams::default();

        let params_bus = Arc::new(ArcSwap::from_pointee(manual_params));
        let mut dsp = DspChain::new(sample_rate, channels, params_bus.clone());

        let mut test_buffer = input_signal.clone();
        let half = test_buffer.len() / 2;

        // Process first half in Manual mode
        dsp.process_interleaved(&mut test_buffer[..half]);

        // Switch to Auto mode (Preset "EDM") mid-stream
        let mut auto_preset = PresetEngine::get_preset_params("EDM");
        auto_preset.is_auto_mode = true;
        params_bus.store(Arc::new(auto_preset));

        // Process second half in Auto mode
        dsp.process_interleaved(&mut test_buffer[half..]);

        // Check channel-aligned boundary derivative across switch point
        let left_boundary_delta = (test_buffer[half] - test_buffer[half - 2]).abs();
        let left_prev_delta = (test_buffer[half - 2] - test_buffer[half - 4]).abs();

        let left_allowed = (left_prev_delta * 2.0).max(0.25);

        assert!(
            left_boundary_delta <= left_allowed,
            "Auto/Manual toggle boundary discontinuity spike detected: delta = {}, allowed = {}",
            left_boundary_delta,
            left_allowed
        );
    }

    // --- Requirement 7.4: Genre/Mood Analysis Smoke Test ---

    #[test]
    fn test_genre_mood_analysis_smoke() {
        let sample_rate = 44100.0;
        let extractor = SpectralExtractor::new(sample_rate, 2048);
        let classifier = Classifier::new(None);

        let audio = generate_sine_stereo(1000.0, sample_rate, 2048);
        let features = extractor.extract(&audio);
        let classification = classifier.classify(&features, 128.0);

        assert!(!classification.genre.is_empty());
        assert!(classification.mood_valence >= 0.0 && classification.mood_valence <= 1.0);
        assert!(classification.mood_energy >= 0.0 && classification.mood_energy <= 1.0);
    }
}
