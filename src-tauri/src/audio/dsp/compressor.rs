use super::params::CompressorParams;

#[derive(Debug, Clone)]
pub struct Compressor {
    sample_rate: f32,
    channels: usize,
    threshold_db: f32,
    ratio: f32,
    attack_ms: f32,
    release_ms: f32,
    knee_width_db: f32,
    envelope_db: f32,
    gain_reduction_db: f32,
}

impl Compressor {
    pub fn new(sample_rate: f32, channels: usize, params: &CompressorParams) -> Self {
        Self {
            sample_rate,
            channels,
            threshold_db: params.threshold_db,
            ratio: params.ratio.max(1.0),
            attack_ms: params.attack_ms.max(0.1),
            release_ms: params.release_ms.max(1.0),
            knee_width_db: params.knee_width_db.max(0.0),
            envelope_db: -120.0,
            gain_reduction_db: 0.0,
        }
    }

    pub fn update_params(&mut self, params: &CompressorParams) {
        self.threshold_db = params.threshold_db;
        self.ratio = params.ratio.max(1.0);
        self.attack_ms = params.attack_ms.max(0.1);
        self.release_ms = params.release_ms.max(1.0);
        self.knee_width_db = params.knee_width_db.max(0.0);
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if samples.is_empty() {
            return;
        }

        let attack_sec = self.attack_ms / 1000.0;
        let release_sec = self.release_ms / 1000.0;

        let attack_coeff = (-2.1972245 / (self.sample_rate * attack_sec)).exp();
        let release_coeff = (-2.1972245 / (self.sample_rate * release_sec)).exp();

        let num_frames = samples.len() / self.channels;

        for frame_idx in 0..num_frames {
            let offset = frame_idx * self.channels;

            // Find peak amplitude across channels in frame
            let mut max_abs = 1e-6f32;
            for ch in 0..self.channels {
                max_abs = max_abs.max(samples[offset + ch].abs());
            }

            let input_db = 20.0 * max_abs.log10();
            let threshold = self.threshold_db;
            let ratio = self.ratio;
            let knee = self.knee_width_db;

            // Soft-knee gain computation
            let delta = input_db - threshold;
            let target_gr_db = if 2.0 * delta < -knee {
                0.0
            } else if (2.0 * delta).abs() <= knee {
                let slope = (1.0 / ratio) - 1.0;
                let k = delta + (knee / 2.0);
                slope * (k * k) / (2.0 * knee)
            } else {
                ((1.0 / ratio) - 1.0) * delta
            };

            // Smooth gain reduction (attack vs release)
            if target_gr_db < self.gain_reduction_db {
                self.gain_reduction_db =
                    attack_coeff * self.gain_reduction_db + (1.0 - attack_coeff) * target_gr_db;
            } else {
                self.gain_reduction_db =
                    release_coeff * self.gain_reduction_db + (1.0 - release_coeff) * target_gr_db;
            }

            let linear_gain = 10.0f32.powf(self.gain_reduction_db / 20.0);

            for ch in 0..self.channels {
                samples[offset + ch] *= linear_gain;
            }
        }
    }
}
