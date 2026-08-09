use super::params::LimiterParams;

#[derive(Debug, Clone)]
pub struct BrickwallLimiter {
    sample_rate: f32,
    channels: usize,
    ceiling: f32,
    release_ms: f32,
    envelope: f32,
}

impl BrickwallLimiter {
    pub fn new(sample_rate: f32, channels: usize, params: &LimiterParams) -> Self {
        let ceiling = 10.0f32.powf(params.ceiling_db.min(0.0) / 20.0);
        Self {
            sample_rate,
            channels,
            ceiling,
            release_ms: params.release_ms.max(1.0),
            envelope: 0.0,
        }
    }

    pub fn update_params(&mut self, params: &LimiterParams) {
        self.ceiling = 10.0f32.powf(params.ceiling_db.min(0.0) / 20.0);
        self.release_ms = params.release_ms.max(1.0);
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if samples.is_empty() {
            return;
        }

        let release_sec = self.release_ms / 1000.0;
        let release_coeff = (-1.0 / (self.sample_rate * release_sec)).exp();
        let num_frames = samples.len() / self.channels;

        for frame_idx in 0..num_frames {
            let offset = frame_idx * self.channels;

            let mut peak = 0.0f32;
            for ch in 0..self.channels {
                peak = peak.max(samples[offset + ch].abs());
            }

            if peak > self.envelope {
                self.envelope = peak;
            } else {
                self.envelope = release_coeff * self.envelope + (1.0 - release_coeff) * peak;
            }

            let gain = if self.envelope > self.ceiling && self.envelope > 1e-6 {
                self.ceiling / self.envelope
            } else {
                1.0
            };

            for ch in 0..self.channels {
                let limited = samples[offset + ch] * gain;
                samples[offset + ch] = limited.clamp(-self.ceiling, self.ceiling);
            }
        }
    }
}
