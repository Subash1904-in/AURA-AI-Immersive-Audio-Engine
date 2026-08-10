#[cfg(test)]
mod tests {
    use crate::audio::dsp::params::DspParams;
    use crate::ipc::persistence::AppConfig;
    use std::fs;

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_save_and_load_config() {
        let temp_dir = std::env::temp_dir().join("aura_test_persistence");
        let _ = fs::create_dir_all(&temp_dir);
        let config_file = temp_dir.join("config.json");

        let mut params = DspParams::default();
        params.is_night_mode = true;
        params.vocals_gain = 1.25;
        params.eq.bands[0].gain_db = 4.5;
        params.spatial.width = 1.6;

        let original_config = AppConfig {
            dsp_params: params,
            last_track_path: Some("/test/track.mp3".to_string()),
            night_mode: true,
        };

        // Serialize and save to temp file
        let json = serde_json::to_string_pretty(&original_config).expect("Serialize config failed");
        fs::write(&config_file, json).expect("Write temp config failed");

        // Reload and deserialize
        let reloaded_json = fs::read_to_string(&config_file).expect("Read temp config failed");
        let reloaded_config: AppConfig =
            serde_json::from_str(&reloaded_json).expect("Parse reloaded config failed");

        assert_eq!(reloaded_config.night_mode, original_config.night_mode);
        assert_eq!(
            reloaded_config.last_track_path,
            original_config.last_track_path
        );
        assert_eq!(
            reloaded_config.dsp_params.is_night_mode,
            original_config.dsp_params.is_night_mode
        );
        assert_eq!(
            reloaded_config.dsp_params.vocals_gain,
            original_config.dsp_params.vocals_gain
        );
        assert_eq!(
            reloaded_config.dsp_params.eq.bands[0].gain_db,
            original_config.dsp_params.eq.bands[0].gain_db
        );
        assert_eq!(
            reloaded_config.dsp_params.spatial.width,
            original_config.dsp_params.spatial.width
        );

        let _ = fs::remove_dir_all(temp_dir);
    }
}
