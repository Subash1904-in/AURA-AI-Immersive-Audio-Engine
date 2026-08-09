pub mod convolver;
pub mod ir_generator;

use crate::audio::dsp::params::{ReverbEnvironment, ReverbParams};
use convolver::PartitionedConvolver;
use ir_generator::IrGenerator;

/// Convolution reverb processor with dynamic environment switching.
///
/// Uses a partitioned FFT convolver for efficient processing and supports
/// smooth parameter & IR crossfading between environments to eliminate audio clicks.
pub struct ConvolutionReverb {
    sample_rate: f32,
    channels: usize,
    // Per-channel convolvers for current environment
    convolvers: Vec<PartitionedConvolver>,
    // Crossfade state for smooth environment transitions
    crossfade_convolvers: Option<Vec<PartitionedConvolver>>,
    crossfade_pos: usize,
    crossfade_len: usize,
    old_env: ReverbEnvironment,
    current_env: ReverbEnvironment,
    wet_dry_mix: f32,
}

impl ConvolutionReverb {
    pub fn new(sample_rate: f32, channels: usize, params: &ReverbParams) -> Self {
        let ir = IrGenerator::generate(params.environment, sample_rate);
        let partition_size = 512;
        let convolvers = (0..channels)
            .map(|_| PartitionedConvolver::new(&ir, partition_size))
            .collect();

        // Crossfade over 1024 samples (~23ms at 44.1 kHz)
        let crossfade_len = 1024;

        Self {
            sample_rate,
            channels,
            convolvers,
            crossfade_convolvers: None,
            crossfade_pos: 0,
            crossfade_len,
            old_env: params.environment,
            current_env: params.environment,
            wet_dry_mix: params.wet_dry_mix.clamp(0.0, 1.0),
        }
    }

    pub fn update_params(&mut self, params: &ReverbParams) {
        self.wet_dry_mix = params.wet_dry_mix.clamp(0.0, 1.0);

        // If environment changed, initiate crossfade
        if params.environment != self.current_env {
            let ir = IrGenerator::generate(params.environment, self.sample_rate);
            let partition_size = 512;

            // Move current convolvers to crossfade, create new ones for target environment
            let old_convolvers = std::mem::replace(
                &mut self.convolvers,
                (0..self.channels)
                    .map(|_| PartitionedConvolver::new(&ir, partition_size))
                    .collect(),
            );
            self.crossfade_convolvers = Some(old_convolvers);
            self.crossfade_pos = 0;
            self.old_env = self.current_env;
            self.current_env = params.environment;
        }
    }

    /// Process interleaved stereo samples through convolution reverb.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if self.current_env == ReverbEnvironment::Off
            && self.old_env == ReverbEnvironment::Off
            && self.crossfade_convolvers.is_none()
        {
            return;
        }

        let num_frames = samples.len() / self.channels;

        let old_target_mix = if self.old_env == ReverbEnvironment::Off {
            0.0
        } else {
            self.wet_dry_mix
        };
        let new_target_mix = if self.current_env == ReverbEnvironment::Off {
            0.0
        } else {
            self.wet_dry_mix
        };

        for frame in 0..num_frames {
            let fade = if self.crossfade_convolvers.is_some() {
                (self.crossfade_pos as f32 / self.crossfade_len as f32).min(1.0)
            } else {
                1.0
            };

            let current_mix = old_target_mix * (1.0 - fade) + new_target_mix * fade;

            for ch in 0..self.channels {
                let idx = frame * self.channels + ch;
                let dry = samples[idx];

                // Process through current convolver
                let wet_new = self.convolvers[ch].process_sample(dry);

                // If crossfading, also process through old convolver
                let wet = if let Some(ref mut old_convolvers) = self.crossfade_convolvers {
                    let wet_old = old_convolvers[ch].process_sample(dry);
                    wet_old * (1.0 - fade) + wet_new * fade
                } else {
                    wet_new
                };

                // Apply blended wet/dry mix
                samples[idx] = dry * (1.0 - current_mix) + wet * current_mix;
            }

            // Advance crossfade
            if self.crossfade_convolvers.is_some() {
                self.crossfade_pos += 1;
                if self.crossfade_pos >= self.crossfade_len {
                    self.crossfade_convolvers = None;
                    self.crossfade_pos = 0;
                    self.old_env = self.current_env;
                }
            }
        }
    }
}
