use crate::audio::dsp::params::DspParams;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub dsp_params: DspParams,
    pub last_track_path: Option<String>,
    pub night_mode: bool,
}

pub fn get_config_dir() -> PathBuf {
    if let Some(home) = dirs_home() {
        home.join(".aura")
    } else {
        PathBuf::from(".aura")
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let dir = get_config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    let path = get_config_path();
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    let mut file =
        File::create(path).map_err(|e| format!("Failed to create config file: {}", e))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    Ok(())
}

pub fn load_config() -> Result<AppConfig, String> {
    let path = get_config_path();
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let mut file = File::open(path).map_err(|e| format!("Failed to open config file: {}", e))?;
    let mut json = String::new();
    file.read_to_string(&mut json)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let config: AppConfig =
        serde_json::from_str(&json).map_err(|e| format!("Failed to parse config file: {}", e))?;
    Ok(config)
}

pub fn reset_config() -> Result<AppConfig, String> {
    let path = get_config_path();
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    let default_config = AppConfig::default();
    let _ = save_config(&default_config);
    Ok(default_config)
}
