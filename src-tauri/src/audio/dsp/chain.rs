use arc_swap::ArcSwap;
use std::sync::Arc;

use super::bass::BassEnhancer;
use super::compressor::Compressor;
use super::eq::ParametricEq;
use super::limiter::BrickwallLimiter;
use super::loudness::LoudnessNormalizer;
use super::params::DspParams;

pub struct DspChain {
    sample_rate: f32,
    channels: usize,
    params_bus: Arc<ArcSwap<DspParams>>,

    eq: ParametricEq,
    bass: BassEnhancer,
    compressor: Compressor,
    loudness: LoudnessNormalizer,
    limiter: BrickwallLimiter,

    cached_params: DspParams,
}

impl DspChain {
    pub fn new(sample_rate: f32, channels: usize, params_bus: Arc<ArcSwap<DspParams>>) -> Self {
        let initial_params = params_bus.load_full();
        let eq = ParametricEq::new(sample_rate, channels, &initial_params.eq);
        let bass = BassEnhancer::new(sample_rate, channels, &initial_params.bass);
        let compressor = Compressor::new(sample_rate, channels, &initial_params.compressor);
        let loudness = LoudnessNormalizer::new(sample_rate, channels, &initial_params.loudness);
        let limiter = BrickwallLimiter::new(sample_rate, channels, &initial_params.limiter);

        Self {
            sample_rate,
            channels,
            params_bus,
            eq,
            bass,
            compressor,
            loudness,
            limiter,
            cached_params: (*initial_params).clone(),
        }
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if samples.is_empty() {
            return;
        }

        // Lock-free read of current DSP parameters
        let latest_params = self.params_bus.load();

        // Check if any stage parameters changed
        if latest_params.eq_enabled {
            self.eq.update_params(&latest_params.eq);
            self.eq.process_interleaved(samples);
        }

        if latest_params.bass_enabled {
            self.bass.update_params(&latest_params.bass);
            self.bass.process_interleaved(samples);
        }

        if latest_params.compressor_enabled {
            self.compressor.update_params(&latest_params.compressor);
            self.compressor.process_interleaved(samples);
        }

        if latest_params.loudness_enabled {
            self.loudness.update_params(&latest_params.loudness);
            self.loudness.process_interleaved(samples);
        }

        if latest_params.limiter_enabled {
            self.limiter.update_params(&latest_params.limiter);
            self.limiter.process_interleaved(samples);
        }
    }
}
