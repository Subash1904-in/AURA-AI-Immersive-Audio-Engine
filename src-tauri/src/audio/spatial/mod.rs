pub mod crossfeed;
pub mod hrtf;
pub mod widener;

#[cfg(test)]
mod spatial_tests;

use crate::audio::dsp::params::SpatialParams;
use crossfeed::Crossfeed;
use hrtf::HrtfRenderer;
use widener::StereoWidener;

/// Composite spatial audio processor combining stereo widening,
/// crossfeed, and HRTF binaural rendering.
pub struct SpatialProcessor {
    widener: StereoWidener,
    crossfeed: Crossfeed,
    hrtf: HrtfRenderer,
    hrtf_enabled: bool,
}

impl SpatialProcessor {
    pub fn new(sample_rate: f32, params: &SpatialParams) -> Self {
        Self {
            widener: StereoWidener::new(params.width),
            crossfeed: Crossfeed::new(sample_rate, params.crossfeed_level),
            hrtf: HrtfRenderer::new(sample_rate),
            hrtf_enabled: params.hrtf_enabled,
        }
    }

    pub fn update_params(&mut self, params: &SpatialParams) {
        self.widener.update_width(params.width);
        self.crossfeed.update_level(params.crossfeed_level);
        self.hrtf_enabled = params.hrtf_enabled;
    }

    /// Process interleaved stereo samples through spatial stages.
    /// Order: Widener → Crossfeed → HRTF (if enabled)
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        self.widener.process_interleaved(samples);
        self.crossfeed.process_interleaved(samples);
        if self.hrtf_enabled {
            self.hrtf.process_interleaved(samples);
        }
    }
}
