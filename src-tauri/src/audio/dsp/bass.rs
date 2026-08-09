use super::params::BassEnhancerParams;

#[derive(Debug, Clone, Default)]
struct FilterState {
    x1: f32,
    y1: f32,
}

#[derive(Debug, Clone)]
pub struct BassEnhancer {
    sample_rate: f32,
    channels: usize,
    cutoff_hz: f32,
    drive: f32,
    mix: f32,
    // 1-pole low pass filter states per channel
    lp_states: Vec<FilterState>,
    // 1-pole high pass filter states per channel for harmonics
    hp_states: Vec<FilterState>,
}

impl BassEnhancer {
    pub fn new(sample_rate: f32, channels: usize, params: &BassEnhancerParams) -> Self {
        Self {
            sample_rate,
            channels,
            cutoff_hz: params.cutoff_hz,
            drive: params.drive,
            mix: params.mix,
            lp_states: vec![FilterState::default(); channels],
            hp_states: vec![FilterState::default(); channels],
        }
    }

    pub fn update_params(&mut self, params: &BassEnhancerParams) {
        self.cutoff_hz = params.cutoff_hz;
        self.drive = params.drive;
        self.mix = params.mix;
    }

    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        let ch = channel % self.channels;

        // 1-pole Low pass cutoff alpha
        let fc = self.cutoff_hz.clamp(20.0, 300.0);
        let dt = 1.0 / self.sample_rate;
        let rc_lp = 1.0 / (2.0 * std::f32::consts::PI * fc);
        let alpha_lp = dt / (rc_lp + dt);

        // Low-pass to isolate bass frequencies
        let lp_in = sample;
        let lp_out = self.lp_states[ch].y1 + alpha_lp * (lp_in - self.lp_states[ch].y1);
        self.lp_states[ch].y1 = lp_out;

        // Non-linear wave-shaping for harmonic generation (2nd + 3rd order harmonics)
        let driven = lp_out * self.drive;
        let odd_harmonics = driven.tanh();
        let even_harmonics = driven * driven.abs();
        let harmonics_raw = 0.6 * odd_harmonics + 0.4 * even_harmonics - lp_out;

        // High-pass filter to pass only generated upper harmonics
        let rc_hp = 1.0 / (2.0 * std::f32::consts::PI * (fc * 0.8));
        let alpha_hp = rc_hp / (rc_hp + dt);
        let hp_in = harmonics_raw;
        let hp_out = alpha_hp * (self.hp_states[ch].y1 + hp_in - self.hp_states[ch].x1);
        self.hp_states[ch].x1 = hp_in;
        self.hp_states[ch].y1 = hp_out;

        // Mix generated harmonics back into the output signal
        sample + self.mix * hp_out
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        for (i, sample) in samples.iter_mut().enumerate() {
            let channel = i % self.channels;
            *sample = self.process_sample(*sample, channel);
        }
    }
}
