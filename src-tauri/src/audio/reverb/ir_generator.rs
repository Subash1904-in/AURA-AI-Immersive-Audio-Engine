#![allow(clippy::needless_range_loop)]
use crate::audio::dsp::params::ReverbEnvironment;

/// Synthetic impulse response generator.
///
/// Generates deterministic impulse responses for 4 environments using
/// exponentially decaying filtered noise with configurable RT60.
pub struct IrGenerator;

impl IrGenerator {
    /// Generate a synthetic IR for the given environment at the specified sample rate.
    /// Returns a mono impulse response as a Vec<f32>.
    pub fn generate(env: ReverbEnvironment, sample_rate: f32) -> Vec<f32> {
        match env {
            ReverbEnvironment::Off => vec![1.0], // Pass-through (dry impulse)
            ReverbEnvironment::SmallRoom => Self::generate_ir(sample_rate, 0.3, 2000.0, 200.0, 42),
            ReverbEnvironment::ConcertHall => {
                Self::generate_ir(sample_rate, 1.8, 4000.0, 100.0, 137)
            }
            ReverbEnvironment::Cathedral => Self::generate_ir(sample_rate, 4.0, 3000.0, 80.0, 293),
            ReverbEnvironment::Cave => Self::generate_ir(sample_rate, 6.0, 2500.0, 150.0, 571),
        }
    }

    /// Generate a synthetic IR with the given parameters.
    ///
    /// - `rt60`: reverberation time in seconds (time for -60 dB decay)
    /// - `lp_cutoff`: low-pass filter cutoff in Hz (air absorption)
    /// - `hp_cutoff`: high-pass filter cutoff in Hz (room mode)
    /// - `seed`: deterministic PRNG seed for reproducible output
    fn generate_ir(
        sample_rate: f32,
        rt60: f32,
        lp_cutoff: f32,
        hp_cutoff: f32,
        seed: u32,
    ) -> Vec<f32> {
        let length = (rt60 * sample_rate) as usize;
        let mut ir = Vec::with_capacity(length);

        // Deterministic PRNG (xorshift32)
        let mut rng_state = seed;

        // Decay constant: amplitude should reach -60 dB at rt60
        let decay_rate = -6.9078 / (rt60 * sample_rate); // ln(0.001) / (rt60 * sr)

        // Generate exponentially decaying noise
        for i in 0..length {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 17;
            rng_state ^= rng_state << 5;
            let noise = (rng_state as f32 / u32::MAX as f32) * 2.0 - 1.0;

            let envelope = (decay_rate * i as f32).exp();
            ir.push(noise * envelope);
        }

        // Add early reflections (sparse impulses in first 50ms)
        let early_reflection_len = (0.05 * sample_rate) as usize;
        let reflection_positions: &[(usize, f32)] = &[
            // (delay_in_samples_fraction, amplitude)
            (1, 0.8),                                      // Direct sound
            ((0.005 * sample_rate as f64) as usize, 0.4),  // 5ms
            ((0.012 * sample_rate as f64) as usize, 0.3),  // 12ms
            ((0.021 * sample_rate as f64) as usize, 0.25), // 21ms
            ((0.035 * sample_rate as f64) as usize, 0.15), // 35ms
            ((0.048 * sample_rate as f64) as usize, 0.1),  // 48ms
        ];

        for &(pos, amp) in reflection_positions {
            if pos < early_reflection_len && pos < length {
                ir[pos] += amp;
            }
        }

        // Apply 1-pole low-pass filter (air absorption)
        let dt = 1.0 / sample_rate;
        let rc_lp = 1.0 / (2.0 * std::f32::consts::PI * lp_cutoff);
        let alpha_lp = dt / (rc_lp + dt);
        let mut lp_state = 0.0f32;
        for sample in ir.iter_mut() {
            lp_state = lp_state + alpha_lp * (*sample - lp_state);
            *sample = lp_state;
        }

        // Apply 1-pole high-pass filter (remove DC / room mode)
        let rc_hp = 1.0 / (2.0 * std::f32::consts::PI * hp_cutoff);
        let alpha_hp = rc_hp / (rc_hp + dt);
        let mut hp_state = 0.0f32;
        let mut hp_prev = 0.0f32;
        for sample in ir.iter_mut() {
            let out = alpha_hp * (hp_state + *sample - hp_prev);
            hp_prev = *sample;
            hp_state = out;
            *sample = out;
        }

        // Normalize IR peak to 1.0
        let peak = ir.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        if peak > 1e-6 {
            let inv_peak = 1.0 / peak;
            for sample in ir.iter_mut() {
                *sample *= inv_peak;
            }
        }

        ir
    }
}
