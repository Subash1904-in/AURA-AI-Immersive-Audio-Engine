use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FilterType {
    LowShelf,
    Peaking,
    HighShelf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    pub enabled: bool,
    pub filter_type: FilterType,
    pub frequency: f32,
    pub gain_db: f32,
    pub q: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqParams {
    pub bands: Vec<EqBand>,
}

impl Default for EqParams {
    fn default() -> Self {
        Self {
            bands: vec![
                EqBand {
                    enabled: true,
                    filter_type: FilterType::LowShelf,
                    frequency: 80.0,
                    gain_db: 0.0,
                    q: 0.707,
                },
                EqBand {
                    enabled: true,
                    filter_type: FilterType::Peaking,
                    frequency: 300.0,
                    gain_db: 0.0,
                    q: 1.0,
                },
                EqBand {
                    enabled: true,
                    filter_type: FilterType::Peaking,
                    frequency: 1000.0,
                    gain_db: 0.0,
                    q: 1.0,
                },
                EqBand {
                    enabled: true,
                    filter_type: FilterType::Peaking,
                    frequency: 3500.0,
                    gain_db: 0.0,
                    q: 1.0,
                },
                EqBand {
                    enabled: true,
                    filter_type: FilterType::HighShelf,
                    frequency: 10000.0,
                    gain_db: 0.0,
                    q: 0.707,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BassEnhancerParams {
    pub cutoff_hz: f32,
    pub drive: f32,
    pub mix: f32,
}

impl Default for BassEnhancerParams {
    fn default() -> Self {
        Self {
            cutoff_hz: 120.0,
            drive: 1.5,
            mix: 0.35,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressorParams {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub knee_width_db: f32,
}

impl Default for CompressorParams {
    fn default() -> Self {
        Self {
            threshold_db: -12.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_width_db: 6.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoudnessParams {
    pub target_lufs: f32,
    pub max_gain_db: f32,
}

impl Default for LoudnessParams {
    fn default() -> Self {
        Self {
            target_lufs: -14.0,
            max_gain_db: 12.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimiterParams {
    pub ceiling_db: f32,
    pub release_ms: f32,
}

impl Default for LimiterParams {
    fn default() -> Self {
        Self {
            ceiling_db: -0.1,
            release_ms: 50.0,
        }
    }
}

// --- Phase 2: Spatial Audio & Reverb Parameters ---

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ReverbEnvironment {
    Off,
    SmallRoom,
    ConcertHall,
    Cathedral,
    Cave,
}

impl Default for ReverbEnvironment {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialParams {
    /// Stereo width factor: 0.0 = mono, 1.0 = original, 2.0 = wide
    pub width: f32,
    /// Crossfeed level for headphone listening: 0.0–1.0
    pub crossfeed_level: f32,
    /// Enable HRTF binaural rendering
    pub hrtf_enabled: bool,
}

impl Default for SpatialParams {
    fn default() -> Self {
        Self {
            width: 1.0,
            crossfeed_level: 0.3,
            hrtf_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverbParams {
    /// Selected reverb environment
    pub environment: ReverbEnvironment,
    /// Wet/dry mix: 0.0 = fully dry, 1.0 = fully wet
    pub wet_dry_mix: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            environment: ReverbEnvironment::Off,
            wet_dry_mix: 0.3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DspParams {
    pub eq_enabled: bool,
    pub eq: EqParams,

    pub bass_enabled: bool,
    pub bass: BassEnhancerParams,

    pub compressor_enabled: bool,
    pub compressor: CompressorParams,

    pub loudness_enabled: bool,
    pub loudness: LoudnessParams,

    pub spatial_enabled: bool,
    pub spatial: SpatialParams,

    pub reverb_enabled: bool,
    pub reverb: ReverbParams,

    pub limiter_enabled: bool,
    pub limiter: LimiterParams,
}
