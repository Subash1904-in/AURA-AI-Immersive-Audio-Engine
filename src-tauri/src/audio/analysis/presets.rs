use crate::audio::dsp::params::{
    CompressorParams, DspParams, EqParams, FilterType, LimiterParams, LoudnessParams,
    ReverbEnvironment, ReverbParams, SpatialParams,
};

/// Data-driven preset engine mapping AI analysis (Genre & Mood) to target DSP parameters.
pub struct PresetEngine;

impl PresetEngine {
    /// Get target `DspParams` for a given genre preset name.
    pub fn get_preset_params(genre: &str) -> DspParams {
        match genre {
            "Rock" => DspParams {
                eq_enabled: true,
                eq: EqParams {
                    bands: vec![
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::LowShelf,
                            frequency: 80.0,
                            gain_db: 1.5,
                            q: 0.707,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 300.0,
                            gain_db: -1.0,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 1000.0,
                            gain_db: 0.0,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 3500.0,
                            gain_db: 3.0,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::HighShelf,
                            frequency: 10000.0,
                            gain_db: 2.0,
                            q: 0.707,
                        },
                    ],
                },
                bass_enabled: true,
                bass: crate::audio::dsp::params::BassEnhancerParams {
                    cutoff_hz: 140.0,
                    drive: 2.0,
                    mix: 0.40,
                },
                compressor_enabled: true,
                compressor: CompressorParams {
                    threshold_db: -14.0,
                    ratio: 4.0,
                    attack_ms: 15.0,
                    release_ms: 120.0,
                    knee_width_db: 6.0,
                },
                loudness_enabled: true,
                loudness: LoudnessParams {
                    target_lufs: -14.0,
                    max_gain_db: 12.0,
                },
                spatial_enabled: true,
                spatial: SpatialParams {
                    width: 1.4,
                    crossfeed_level: 0.25,
                    hrtf_enabled: false,
                },
                reverb_enabled: false,
                reverb: ReverbParams::default(),
                limiter_enabled: true,
                limiter: LimiterParams::default(),
                is_auto_mode: true,
                active_preset: "Rock".to_string(),
                beat_modulation_enabled: true,
                beat_boost: 0.0,
                ..DspParams::default()
            },

            "EDM" => DspParams {
                eq_enabled: true,
                eq: EqParams {
                    bands: vec![
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::LowShelf,
                            frequency: 80.0,
                            gain_db: 3.5,
                            q: 0.707,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 300.0,
                            gain_db: -1.5,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 1000.0,
                            gain_db: 0.0,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 3500.0,
                            gain_db: 1.5,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::HighShelf,
                            frequency: 10000.0,
                            gain_db: 2.5,
                            q: 0.707,
                        },
                    ],
                },
                bass_enabled: true,
                bass: crate::audio::dsp::params::BassEnhancerParams {
                    cutoff_hz: 120.0,
                    drive: 3.0,
                    mix: 0.50,
                },
                compressor_enabled: true,
                compressor: CompressorParams {
                    threshold_db: -16.0,
                    ratio: 6.0,
                    attack_ms: 5.0,
                    release_ms: 80.0,
                    knee_width_db: 4.0,
                },
                loudness_enabled: true,
                loudness: LoudnessParams {
                    target_lufs: -13.0,
                    max_gain_db: 12.0,
                },
                spatial_enabled: true,
                spatial: SpatialParams {
                    width: 1.5,
                    crossfeed_level: 0.20,
                    hrtf_enabled: false,
                },
                reverb_enabled: false,
                reverb: ReverbParams::default(),
                limiter_enabled: true,
                limiter: LimiterParams::default(),
                is_auto_mode: true,
                active_preset: "EDM".to_string(),
                beat_modulation_enabled: true,
                beat_boost: 0.0,
                ..DspParams::default()
            },

            "Classical" => DspParams {
                eq_enabled: true,
                eq: EqParams::default(),
                bass_enabled: false,
                bass: crate::audio::dsp::params::BassEnhancerParams::default(),
                compressor_enabled: false,
                compressor: CompressorParams::default(),
                loudness_enabled: true,
                loudness: LoudnessParams {
                    target_lufs: -18.0,
                    max_gain_db: 12.0,
                },
                spatial_enabled: true,
                spatial: SpatialParams {
                    width: 1.0,
                    crossfeed_level: 0.35,
                    hrtf_enabled: false,
                },
                reverb_enabled: true,
                reverb: ReverbParams {
                    environment: ReverbEnvironment::ConcertHall,
                    wet_dry_mix: 0.35,
                },
                limiter_enabled: true,
                limiter: LimiterParams::default(),
                is_auto_mode: true,
                active_preset: "Classical".to_string(),
                beat_modulation_enabled: false,
                beat_boost: 0.0,
                ..DspParams::default()
            },

            "Podcast" => DspParams {
                eq_enabled: true,
                eq: EqParams {
                    bands: vec![
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::LowShelf,
                            frequency: 100.0,
                            gain_db: -4.0,
                            q: 0.707,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 300.0,
                            gain_db: 0.0,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 1000.0,
                            gain_db: 1.0,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 2500.0,
                            gain_db: 2.5,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::HighShelf,
                            frequency: 10000.0,
                            gain_db: -1.0,
                            q: 0.707,
                        },
                    ],
                },
                bass_enabled: false,
                bass: crate::audio::dsp::params::BassEnhancerParams::default(),
                compressor_enabled: true,
                compressor: CompressorParams {
                    threshold_db: -14.0,
                    ratio: 3.5,
                    attack_ms: 10.0,
                    release_ms: 150.0,
                    knee_width_db: 6.0,
                },
                loudness_enabled: true,
                loudness: LoudnessParams {
                    target_lufs: -16.0,
                    max_gain_db: 12.0,
                },
                spatial_enabled: true,
                spatial: SpatialParams {
                    width: 0.0, // Mono focus for speech clarity
                    crossfeed_level: 0.0,
                    hrtf_enabled: false,
                },
                reverb_enabled: false,
                reverb: ReverbParams::default(),
                limiter_enabled: true,
                limiter: LimiterParams::default(),
                is_auto_mode: true,
                active_preset: "Podcast".to_string(),
                beat_modulation_enabled: false,
                beat_boost: 0.0,
                ..DspParams::default()
            },

            "Lofi" => DspParams {
                eq_enabled: true,
                eq: EqParams {
                    bands: vec![
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::LowShelf,
                            frequency: 100.0,
                            gain_db: 3.0,
                            q: 0.707,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 300.0,
                            gain_db: 1.0,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 1000.0,
                            gain_db: 0.0,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::Peaking,
                            frequency: 3500.0,
                            gain_db: -1.5,
                            q: 1.0,
                        },
                        crate::audio::dsp::params::EqBand {
                            enabled: true,
                            filter_type: FilterType::HighShelf,
                            frequency: 8000.0,
                            gain_db: -3.5,
                            q: 0.707,
                        },
                    ],
                },
                bass_enabled: true,
                bass: crate::audio::dsp::params::BassEnhancerParams {
                    cutoff_hz: 130.0,
                    drive: 1.8,
                    mix: 0.40,
                },
                compressor_enabled: true,
                compressor: CompressorParams {
                    threshold_db: -12.0,
                    ratio: 3.0,
                    attack_ms: 20.0,
                    release_ms: 180.0,
                    knee_width_db: 8.0,
                },
                loudness_enabled: true,
                loudness: LoudnessParams {
                    target_lufs: -14.0,
                    max_gain_db: 12.0,
                },
                spatial_enabled: true,
                spatial: SpatialParams {
                    width: 1.2,
                    crossfeed_level: 0.30,
                    hrtf_enabled: false,
                },
                reverb_enabled: true,
                reverb: ReverbParams {
                    environment: ReverbEnvironment::SmallRoom,
                    wet_dry_mix: 0.25,
                },
                limiter_enabled: true,
                limiter: LimiterParams::default(),
                is_auto_mode: true,
                active_preset: "Lofi".to_string(),
                beat_modulation_enabled: true,
                beat_boost: 0.0,
                ..DspParams::default()
            },

            _ => DspParams {
                is_auto_mode: true,
                active_preset: "Pop".to_string(),
                ..DspParams::default()
            },
        }
    }
}
