#[cfg(test)]
mod tests {
    use crate::audio::dsp::nl_eq::NLEqEngine;
    use crate::audio::dsp::params::{DspParams, ReverbEnvironment};

    #[test]
    fn test_cinematic_phrase() {
        let engine = NLEqEngine::new();
        let mut params = DspParams::default();
        let matched = engine.parse_and_apply("Make it sound more cinematic please", &mut params);

        assert!(matched.contains(&"cinematic".to_string()));
        assert!(params.spatial_enabled);
        assert_eq!(params.spatial.width, 1.65);
        assert!(params.reverb_enabled);
        assert_eq!(params.reverb.environment, ReverbEnvironment::ConcertHall);
        assert_eq!(params.reverb.wet_dry_mix, 0.35);
        assert!(params.bass_enabled);
        assert_eq!(params.bass.drive, 1.8);
        assert_eq!(params.eq.bands[0].gain_db, 3.0);
    }

    #[test]
    fn test_clearer_vocals_phrase() {
        let engine = NLEqEngine::new();
        let mut params = DspParams::default();
        let matched = engine.parse_and_apply("I need clearer vocals for this podcast", &mut params);

        assert!(matched.contains(&"clearer vocals".to_string()));
        assert!(params.eq_enabled);
        assert_eq!(params.eq.bands[2].gain_db, 3.5);
        assert!(params.compressor_enabled);
        assert_eq!(params.compressor.threshold_db, -15.0);
        assert_eq!(params.vocals_gain, 1.25);
    }

    #[test]
    fn test_more_bass_and_spacey_combination() {
        let engine = NLEqEngine::new();
        let mut params = DspParams::default();
        let matched = engine.parse_and_apply("give me more bass and make it spacey", &mut params);

        assert!(matched.contains(&"more bass".to_string()));
        assert!(matched.contains(&"spacey".to_string()));
        assert!(params.bass_enabled);
        assert_eq!(params.bass.drive, 2.2);
        assert!(params.spatial_enabled);
        assert_eq!(params.reverb.environment, ReverbEnvironment::Cathedral);
    }

    #[test]
    fn test_night_mode_phrase() {
        let engine = NLEqEngine::new();
        let mut params = DspParams::default();
        let matched = engine.parse_and_apply("turn on night mode", &mut params);

        assert!(matched.contains(&"night mode".to_string()));
        assert!(params.is_night_mode);
    }
}
