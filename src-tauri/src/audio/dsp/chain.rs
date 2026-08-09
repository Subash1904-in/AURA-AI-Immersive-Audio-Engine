use arc_swap::ArcSwap;
use std::sync::Arc;

use super::bass::BassEnhancer;
use super::compressor::Compressor;
use super::eq::ParametricEq;
use super::limiter::BrickwallLimiter;
use super::loudness::LoudnessNormalizer;
use super::params::DspParams;
use crate::audio::reverb::ConvolutionReverb;
use crate::audio::spatial::SpatialProcessor;

pub struct DspChain {
    params_bus: Arc<ArcSwap<DspParams>>,

    eq: ParametricEq,
    bass: BassEnhancer,
    compressor: Compressor,
    loudness: LoudnessNormalizer,
    spatial: SpatialProcessor,
    reverb: ConvolutionReverb,
    limiter: BrickwallLimiter,
}

impl DspChain {
    pub fn new(sample_rate: f32, channels: usize, params_bus: Arc<ArcSwap<DspParams>>) -> Self {
        let initial_params = params_bus.load_full();
        let eq = ParametricEq::new(sample_rate, channels, &initial_params.eq);
        let bass = BassEnhancer::new(sample_rate, channels, &initial_params.bass);
        let compressor = Compressor::new(sample_rate, channels, &initial_params.compressor);
        let loudness = LoudnessNormalizer::new(sample_rate, channels, &initial_params.loudness);
        let spatial = SpatialProcessor::new(sample_rate, &initial_params.spatial);
        let reverb = ConvolutionReverb::new(sample_rate, channels, &initial_params.reverb);
        let limiter = BrickwallLimiter::new(sample_rate, channels, &initial_params.limiter);

        Self {
            params_bus,
            eq,
            bass,
            compressor,
            loudness,
            spatial,
            reverb,
            limiter,
        }
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if samples.is_empty() {
            return;
        }

        // Lock-free read of current DSP parameters
        let latest_params = self.params_bus.load();

        // Stage 1: Parametric EQ
        if latest_params.eq_enabled {
            self.eq.update_params(&latest_params.eq);
            self.eq.process_interleaved(samples);
        }

        // Stage 2: Bass Enhancer (with optional beat modulation boost)
        if latest_params.bass_enabled {
            let mut bass_params = latest_params.bass.clone();
            if latest_params.beat_modulation_enabled && latest_params.beat_boost > 0.01 {
                bass_params.drive += latest_params.beat_boost * 0.8;
                bass_params.mix = (bass_params.mix + latest_params.beat_boost * 0.15).min(1.0);
            }
            self.bass.update_params(&bass_params);
            self.bass.process_interleaved(samples);
        }

        // Stage 3: Compressor
        if latest_params.compressor_enabled {
            self.compressor.update_params(&latest_params.compressor);
            self.compressor.process_interleaved(samples);
        }

        // Stage 4: Loudness Normalizer
        if latest_params.loudness_enabled {
            self.loudness.update_params(&latest_params.loudness);
            self.loudness.process_interleaved(samples);
        }

        // Stage 5: Spatial Audio (with optional beat modulation width boost)
        if latest_params.spatial_enabled {
            let mut spatial_params = latest_params.spatial.clone();
            if latest_params.beat_modulation_enabled && latest_params.beat_boost > 0.01 {
                spatial_params.width =
                    (spatial_params.width + latest_params.beat_boost * 0.25).min(2.0);
            }
            self.spatial.update_params(&spatial_params);
            self.spatial.process_interleaved(samples);
        }

        // Stage 6: Convolution Reverb
        if latest_params.reverb_enabled {
            self.reverb.update_params(&latest_params.reverb);
            self.reverb.process_interleaved(samples);
        }

        // Stage 7: Brick-Wall Limiter (always last)
        if latest_params.limiter_enabled {
            self.limiter.update_params(&latest_params.limiter);
            self.limiter.process_interleaved(samples);
        }
    }
}
