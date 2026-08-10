use crate::audio::dsp::params::{DspParams, ReverbEnvironment};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NLEqDelta {
    pub description: Option<String>,
    pub eq_low_shelf_gain: Option<f32>,
    pub eq_mid_gain: Option<f32>,
    pub eq_high_shelf_gain: Option<f32>,
    pub bass_drive: Option<f32>,
    pub bass_mix: Option<f32>,
    pub bass_enabled: Option<bool>,
    pub spatial_width: Option<f32>,
    pub spatial_enabled: Option<bool>,
    pub crossfeed_enabled: Option<bool>,
    pub hrtf_enabled: Option<bool>,
    pub reverb_env: Option<String>,
    pub reverb_mix: Option<f32>,
    pub reverb_enabled: Option<bool>,
    pub compressor_threshold: Option<f32>,
    pub compressor_enabled: Option<bool>,
    pub vocals_gain: Option<f32>,
    pub drums_gain: Option<f32>,
    pub bass_gain: Option<f32>,
    pub is_night_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NLEqPresetsMapping {
    pub presets: HashMap<String, NLEqDelta>,
}

pub const DEFAULT_NL_JSON: &str = r#"{
  "presets": {
    "cinematic": {
      "description": "Expands spatial width, adds concert hall depth, and boosts low-end punch",
      "spatial_width": 1.65,
      "spatial_enabled": true,
      "reverb_env": "ConcertHall",
      "reverb_mix": 0.35,
      "reverb_enabled": true,
      "bass_drive": 1.8,
      "bass_enabled": true,
      "eq_low_shelf_gain": 3.0
    },
    "clearer vocals": {
      "description": "Boosts mid frequencies and gentles compression for vocal clarity",
      "eq_mid_gain": 3.5,
      "eq_high_shelf_gain": 1.5,
      "compressor_enabled": true,
      "compressor_threshold": -15.0,
      "vocals_gain": 1.25
    },
    "vocal boost": {
      "description": "Boosts mid frequencies for vocal prominence",
      "eq_mid_gain": 4.0,
      "vocals_gain": 1.3
    },
    "more bass": {
      "description": "Increases sub-bass drive and low-shelf EQ gain",
      "bass_enabled": true,
      "bass_drive": 2.2,
      "bass_mix": 0.55,
      "eq_low_shelf_gain": 4.5,
      "bass_gain": 1.35
    },
    "bass heavy": {
      "description": "Heavy bass boost with sub-harmonic enhancer",
      "bass_enabled": true,
      "bass_drive": 2.5,
      "bass_mix": 0.6,
      "eq_low_shelf_gain": 5.0,
      "bass_gain": 1.4
    },
    "vintage": {
      "description": "Rolls off harsh highs, boosts low-mids, adds warm room ambience",
      "eq_high_shelf_gain": -4.5,
      "eq_low_shelf_gain": 2.0,
      "reverb_enabled": true,
      "reverb_env": "SmallRoom",
      "reverb_mix": 0.25
    },
    "warmth": {
      "description": "Softens top end and enriches low-mid body",
      "eq_high_shelf_gain": -2.0,
      "eq_low_shelf_gain": 2.5,
      "bass_enabled": true,
      "bass_mix": 0.35
    },
    "bright": {
      "description": "Boosts treble and upper-mid clarity",
      "eq_high_shelf_gain": 4.0,
      "eq_mid_gain": 1.5
    },
    "spacey": {
      "description": "Maximum spatial widening and deep cathedral reverb",
      "spatial_enabled": true,
      "spatial_width": 1.85,
      "reverb_enabled": true,
      "reverb_env": "Cathedral",
      "reverb_mix": 0.45,
      "hrtf_enabled": true
    },
    "night mode": {
      "description": "Activates smart night mode peak suppression and vocal clarity",
      "is_night_mode": true
    }
  }
}"#;

pub struct NLEqEngine {
    mapping: NLEqPresetsMapping,
}

impl Default for NLEqEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl NLEqEngine {
    pub fn new() -> Self {
        let mapping: NLEqPresetsMapping =
            serde_json::from_str(DEFAULT_NL_JSON).unwrap_or_else(|_| NLEqPresetsMapping {
                presets: HashMap::new(),
            });
        Self { mapping }
    }

    pub fn get_supported_phrases(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.mapping.presets.keys().cloned().collect();
        keys.sort();
        keys
    }

    pub fn parse_and_apply(&self, prompt: &str, params: &mut DspParams) -> Vec<String> {
        let normalized = prompt.to_lowercase();
        let mut matched_keys = Vec::new();

        for (key, delta) in &self.mapping.presets {
            if normalized.contains(key.as_str()) || fuzzy_match(&normalized, key) {
                matched_keys.push(key.clone());
                self.apply_delta(delta, params);
            }
        }

        matched_keys.sort();
        matched_keys.dedup();
        matched_keys
    }

    fn apply_delta(&self, delta: &NLEqDelta, params: &mut DspParams) {
        if let Some(gain) = delta.eq_low_shelf_gain {
            params.eq_enabled = true;
            params.eq.bands[0].gain_db = gain;
        }
        if let Some(gain) = delta.eq_mid_gain {
            params.eq_enabled = true;
            params.eq.bands[2].gain_db = gain;
        }
        if let Some(gain) = delta.eq_high_shelf_gain {
            params.eq_enabled = true;
            params.eq.bands[4].gain_db = gain;
        }
        if let Some(enabled) = delta.bass_enabled {
            params.bass_enabled = enabled;
        }
        if let Some(drive) = delta.bass_drive {
            params.bass.drive = drive;
        }
        if let Some(mix) = delta.bass_mix {
            params.bass.mix = mix;
        }
        if let Some(enabled) = delta.spatial_enabled {
            params.spatial_enabled = enabled;
        }
        if let Some(width) = delta.spatial_width {
            params.spatial.width = width;
        }
        if let Some(crossfeed) = delta.crossfeed_enabled {
            params.spatial.crossfeed_level = if crossfeed { 0.4 } else { 0.0 };
        }
        if let Some(hrtf) = delta.hrtf_enabled {
            params.spatial.hrtf_enabled = hrtf;
        }
        if let Some(enabled) = delta.reverb_enabled {
            params.reverb_enabled = enabled;
        }
        if let Some(ref env_str) = delta.reverb_env {
            params.reverb.environment = match env_str.as_str() {
                "SmallRoom" => ReverbEnvironment::SmallRoom,
                "ConcertHall" => ReverbEnvironment::ConcertHall,
                "Cathedral" => ReverbEnvironment::Cathedral,
                "Cave" => ReverbEnvironment::Cave,
                _ => ReverbEnvironment::Off,
            };
        }
        if let Some(mix) = delta.reverb_mix {
            params.reverb.wet_dry_mix = mix;
        }
        if let Some(enabled) = delta.compressor_enabled {
            params.compressor_enabled = enabled;
        }
        if let Some(thresh) = delta.compressor_threshold {
            params.compressor.threshold_db = thresh;
        }
        if let Some(gain) = delta.vocals_gain {
            params.vocals_gain = gain;
        }
        if let Some(gain) = delta.drums_gain {
            params.drums_gain = gain;
        }
        if let Some(gain) = delta.bass_gain {
            params.bass_gain = gain;
        }
        if let Some(night) = delta.is_night_mode {
            params.is_night_mode = night;
        }
    }
}

/// Simple fuzzy matcher calculating token overlap & Levenshtein distance
fn fuzzy_match(input: &str, target: &str) -> bool {
    let target_tokens: Vec<&str> = target.split_whitespace().collect();
    let mut matched_count = 0;

    for t in &target_tokens {
        if input.contains(t) {
            matched_count += 1;
        }
    }

    if !target_tokens.is_empty() && matched_count == target_tokens.len() {
        return true;
    }

    let dist = levenshtein_distance(input, target);
    dist <= 2
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let (len_a, len_b) = (a_chars.len(), b_chars.len());

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut dp = vec![vec![0; len_b + 1]; len_a + 1];

    for (i, row) in dp.iter_mut().enumerate().take(len_a + 1) {
        row[0] = i;
    }
    for j in 0..=len_b {
        dp[0][j] = j;
    }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }

    dp[len_a][len_b]
}
