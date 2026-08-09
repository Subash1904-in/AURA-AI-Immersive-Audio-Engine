/// Bauer-style crossfeed for headphone listening.
///
/// Mixes a low-passed, attenuated version of the opposite channel into each ear,
/// reducing harsh hard-panned separation without collapsing the stereo image.
#[derive(Debug, Clone)]
pub struct Crossfeed {
    sample_rate: f32,
    level: f32,
    // 1-pole low-pass filter states for left and right channels
    lp_state_l: f32,
    lp_state_r: f32,
    // Previous opposite-channel samples for 1-sample delay
    prev_l: f32,
    prev_r: f32,
}

impl Crossfeed {
    pub fn new(sample_rate: f32, level: f32) -> Self {
        Self {
            sample_rate,
            level: level.clamp(0.0, 1.0),
            lp_state_l: 0.0,
            lp_state_r: 0.0,
            prev_l: 0.0,
            prev_r: 0.0,
        }
    }

    pub fn update_level(&mut self, level: f32) {
        self.level = level.clamp(0.0, 1.0);
    }

    /// Process interleaved stereo samples in-place.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if self.level < 1e-6 {
            return;
        }

        // Crossfeed cutoff ~700 Hz — only low frequencies bleed across
        let cutoff_hz = 700.0;
        let dt = 1.0 / self.sample_rate;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
        let alpha = dt / (rc + dt);

        // Attenuation for crossfeed signal (~-4.5 dB)
        let attenuation = 0.6 * self.level;

        let num_frames = samples.len() / 2;
        for i in 0..num_frames {
            let l = samples[i * 2];
            let r = samples[i * 2 + 1];

            // Low-pass filter the opposite channel
            self.lp_state_l = self.lp_state_l + alpha * (r - self.lp_state_l);
            self.lp_state_r = self.lp_state_r + alpha * (l - self.lp_state_r);

            // Mix delayed, filtered opposite channel with attenuation
            let new_l = l + attenuation * self.prev_r;
            let new_r = r + attenuation * self.prev_l;

            // Store current filtered values for 1-sample delay
            self.prev_l = self.lp_state_r;
            self.prev_r = self.lp_state_l;

            samples[i * 2] = new_l;
            samples[i * 2 + 1] = new_r;
        }
    }
}
