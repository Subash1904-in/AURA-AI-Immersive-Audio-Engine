use super::params::{EqBand, EqParams, FilterType};

#[derive(Debug, Clone, Copy)]
pub struct BiquadCoeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl Default for BiquadCoeffs {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

impl BiquadCoeffs {
    pub fn calculate(band: &EqBand, sample_rate: f32) -> Self {
        if !band.enabled || band.gain_db.abs() < 1e-4 {
            return Self::default();
        }

        let f0 = band.frequency.clamp(10.0, sample_rate * 0.49);
        let q = band.q.max(0.1);
        let a = 10.0f32.powf(band.gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * f0 / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match band.filter_type {
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let sqrt_a = a.sqrt();
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let sqrt_a = a.sqrt();
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadState {
    #[inline]
    fn process(&mut self, input: f32, coeffs: &BiquadCoeffs) -> f32 {
        let output = coeffs.b0 * input + coeffs.b1 * self.x1 + coeffs.b2 * self.x2
            - coeffs.a1 * self.y1
            - coeffs.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

#[derive(Debug, Clone)]
pub struct ParametricEq {
    sample_rate: f32,
    channels: usize,
    coeffs: Vec<BiquadCoeffs>,
    states: Vec<Vec<BiquadState>>, // [band][channel]
}

impl ParametricEq {
    pub fn new(sample_rate: f32, channels: usize, params: &EqParams) -> Self {
        let num_bands = params.bands.len();
        let coeffs = params
            .bands
            .iter()
            .map(|b| BiquadCoeffs::calculate(b, sample_rate))
            .collect();
        let states = vec![vec![BiquadState::default(); channels]; num_bands];

        Self {
            sample_rate,
            channels,
            coeffs,
            states,
        }
    }

    pub fn update_params(&mut self, params: &EqParams) {
        if self.coeffs.len() != params.bands.len() {
            self.coeffs = params
                .bands
                .iter()
                .map(|b| BiquadCoeffs::calculate(b, self.sample_rate))
                .collect();
            self.states = vec![vec![BiquadState::default(); self.channels]; params.bands.len()];
        } else {
            for (i, band) in params.bands.iter().enumerate() {
                self.coeffs[i] = BiquadCoeffs::calculate(band, self.sample_rate);
            }
        }
    }

    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        let ch = channel % self.channels;
        let mut val = sample;
        for band_idx in 0..self.coeffs.len() {
            val = self.states[band_idx][ch].process(val, &self.coeffs[band_idx]);
        }
        val
    }

    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        for (i, sample) in samples.iter_mut().enumerate() {
            let channel = i % self.channels;
            *sample = self.process_sample(*sample, channel);
        }
    }
}
