use super::params::LoudnessParams;

// ITU-R BS.1770 K-weighting biquad coefficients for 44.1/48kHz approximation
#[derive(Debug, Clone)]
struct KWeightingFilter {
    // Stage 1: High shelf
    b0_hs: f32,
    b1_hs: f32,
    b2_hs: f32,
    a1_hs: f32,
    a2_hs: f32,
    x1_hs: f32,
    x2_hs: f32,
    y1_hs: f32,
    y2_hs: f32,

    // Stage 2: High pass
    b0_hp: f32,
    b1_hp: f32,
    b2_hp: f32,
    a1_hp: f32,
    a2_hp: f32,
    x1_hp: f32,
    x2_hp: f32,
    y1_hp: f32,
    y2_hp: f32,
}

impl KWeightingFilter {
    fn new(sample_rate: f32) -> Self {
        // High shelf filter coefficients (BS.1770 approximation)
        let f0_hs = 1500.0;
        let gain_db_hs = 4.0;
        let w0_hs = 2.0 * std::f32::consts::PI * f0_hs / sample_rate;
        let a_hs = 10.0f32.powf(gain_db_hs / 40.0);
        let alpha_hs = w0_hs.sin() / (2.0 * 0.707);
        let cos_w0_hs = w0_hs.cos();
        let sqrt_a_hs = a_hs.sqrt();

        let b0_hs = a_hs * ((a_hs + 1.0) + (a_hs - 1.0) * cos_w0_hs + 2.0 * sqrt_a_hs * alpha_hs);
        let b1_hs = -2.0 * a_hs * ((a_hs - 1.0) + (a_hs + 1.0) * cos_w0_hs);
        let b2_hs = a_hs * ((a_hs + 1.0) + (a_hs - 1.0) * cos_w0_hs - 2.0 * sqrt_a_hs * alpha_hs);
        let a0_hs = (a_hs + 1.0) - (a_hs - 1.0) * cos_w0_hs + 2.0 * sqrt_a_hs * alpha_hs;
        let a1_hs = 2.0 * ((a_hs - 1.0) - (a_hs + 1.0) * cos_w0_hs);
        let a2_hs = (a_hs + 1.0) - (a_hs - 1.0) * cos_w0_hs - 2.0 * sqrt_a_hs * alpha_hs;

        // High pass filter (38 Hz cutoff)
        let f0_hp = 38.0;
        let w0_hp = 2.0 * std::f32::consts::PI * f0_hp / sample_rate;
        let cos_w0_hp = w0_hp.cos();
        let alpha_hp = w0_hp.sin() / (2.0 * 0.5);

        let b0_hp = (1.0 + cos_w0_hp) / 2.0;
        let b1_hp = -(1.0 + cos_w0_hp);
        let b2_hp = (1.0 + cos_w0_hp) / 2.0;
        let a0_hp = 1.0 + alpha_hp;
        let a1_hp = -2.0 * cos_w0_hp;
        let a2_hp = 1.0 - alpha_hp;

        Self {
            b0_hs: b0_hs / a0_hs,
            b1_hs: b1_hs / a0_hs,
            b2_hs: b2_hs / a0_hs,
            a1_hs: a1_hs / a0_hs,
            a2_hs: a2_hs / a0_hs,
            x1_hs: 0.0,
            x2_hs: 0.0,
            y1_hs: 0.0,
            y2_hs: 0.0,

            b0_hp: b0_hp / a0_hp,
            b1_hp: b1_hp / a0_hp,
            b2_hp: b2_hp / a0_hp,
            a1_hp: a1_hp / a0_hp,
            a2_hp: a2_hp / a0_hp,
            x1_hp: 0.0,
            x2_hp: 0.0,
            y1_hp: 0.0,
            y2_hp: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        // High shelf
        let y_hs = self.b0_hs * input + self.b1_hs * self.x1_hs + self.b2_hs * self.x2_hs
            - self.a1_hs * self.y1_hs
            - self.a2_hs * self.y2_hs;
        self.x2_hs = self.x1_hs;
        self.x1_hs = input;
        self.y2_hs = self.y1_hs;
        self.y1_hs = y_hs;

        // High pass
        let y_hp = self.b0_hp * y_hs + self.b1_hp * self.x1_hp + self.b2_hp * self.x2_hp
            - self.a1_hp * self.y1_hp
            - self.a2_hp * self.y2_hp;
        self.x2_hp = self.x1_hp;
        self.x1_hp = y_hs;
        self.y2_hp = self.y1_hp;
        self.y1_hp = y_hp;

        y_hp
    }
}

#[derive(Debug, Clone)]
pub struct LoudnessNormalizer {
    sample_rate: f32,
    channels: usize,
    target_lufs: f32,
    max_gain_db: f32,
    filters: Vec<KWeightingFilter>,
    mean_square_energy: f32,
    smoothed_gain: f32,
}

impl LoudnessNormalizer {
    pub fn new(sample_rate: f32, channels: usize, params: &LoudnessParams) -> Self {
        let filters = vec![KWeightingFilter::new(sample_rate); channels];
        Self {
            sample_rate,
            channels,
            target_lufs: params.target_lufs,
            max_gain_db: params.max_gain_db,
            filters,
            mean_square_energy: 1e-5, // Initial non-zero baseline
            smoothed_gain: 1.0,
        }
    }

    pub fn update_params(&mut self, params: &LoudnessParams) {
        self.target_lufs = params.target_lufs;
        self.max_gain_db = params.max_gain_db;
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if samples.is_empty() {
            return;
        }

        let num_frames = samples.len() / self.channels;
        // Exponential moving average alpha for 400ms window
        let alpha = 1.0 - (-1.0 / (self.sample_rate * 0.4)).exp();
        // Exponential smoothing for gain updates (100ms)
        let gain_alpha = 1.0 - (-1.0 / (self.sample_rate * 0.1)).exp();

        for frame_idx in 0..num_frames {
            let offset = frame_idx * self.channels;
            let mut frame_power = 0.0;

            for ch in 0..self.channels {
                let filtered = self.filters[ch].process(samples[offset + ch]);
                frame_power += filtered * filtered;
            }
            frame_power /= self.channels as f32;

            self.mean_square_energy = (1.0 - alpha) * self.mean_square_energy + alpha * frame_power;
            let current_lufs = -0.691 + 10.0 * (self.mean_square_energy + 1e-10).log10();

            let target_gain_db =
                (self.target_lufs - current_lufs).clamp(-self.max_gain_db, self.max_gain_db);
            let target_gain = 10.0f32.powf(target_gain_db / 20.0);

            self.smoothed_gain = (1.0 - gain_alpha) * self.smoothed_gain + gain_alpha * target_gain;

            for ch in 0..self.channels {
                samples[offset + ch] *= self.smoothed_gain;
            }
        }
    }
}
