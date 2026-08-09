#![allow(clippy::needless_range_loop, clippy::manual_repeat_n)]

use rustfft::{num_complex::Complex, FftPlanner};

/// Spectral features extracted from an audio block.
#[derive(Debug, Clone, Default)]
pub struct SpectralFeatures {
    /// Spectral centroid in Hz (brightness / timbre center)
    pub spectral_centroid: f32,
    /// Spectral flatness (0.0 = pure tone, 1.0 = white noise)
    pub spectral_flatness: f32,
    /// Energy ratio in sub-bass / bass (< 250 Hz)
    pub energy_sub_bass: f32,
    /// Energy ratio in midrange (250 Hz – 4000 Hz)
    pub energy_mid: f32,
    /// Energy ratio in high frequencies (> 4000 Hz)
    pub energy_high: f32,
    /// Zero-crossing rate (0.0 – 1.0)
    pub zcr: f32,
    /// Total RMS energy
    pub rms_energy: f32,
}

pub struct SpectralExtractor {
    sample_rate: f32,
    fft_size: usize,
}

impl SpectralExtractor {
    pub fn new(sample_rate: f32, fft_size: usize) -> Self {
        let fft_size = fft_size.next_power_of_two();
        Self {
            sample_rate,
            fft_size,
        }
    }

    /// Extract spectral features from a block of audio samples.
    pub fn extract(&self, samples: &[f32]) -> SpectralFeatures {
        if samples.is_empty() {
            return SpectralFeatures::default();
        }

        // 1. Calculate Zero-Crossing Rate (ZCR)
        let mut zero_crossings = 0;
        for i in 1..samples.len() {
            if (samples[i] >= 0.0 && samples[i - 1] < 0.0)
                || (samples[i] < 0.0 && samples[i - 1] >= 0.0)
            {
                zero_crossings += 1;
            }
        }
        let zcr = zero_crossings as f32 / samples.len() as f32;

        // 2. Calculate RMS Energy
        let rms_energy = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

        // 3. FFT Magnitude Spectrum
        let n = self.fft_size.min(samples.len());
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.fft_size);

        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .take(n)
            .map(|&s| Complex::new(s, 0.0))
            .chain(std::iter::repeat(Complex::new(0.0, 0.0)).take(self.fft_size - n))
            .collect();

        fft.process(&mut buffer);

        let num_bins = self.fft_size / 2;
        let bin_width = self.sample_rate / self.fft_size as f32;

        let mut magnitudes = Vec::with_capacity(num_bins);
        let mut total_mag = 0.0f32;
        let mut weighted_freq_sum = 0.0f32;

        let mut sub_bass_energy = 0.0f32;
        let mut mid_energy = 0.0f32;
        let mut high_energy = 0.0f32;

        for bin in 0..num_bins {
            let mag = buffer[bin].norm();
            magnitudes.push(mag);
            total_mag += mag;

            let freq = bin as f32 * bin_width;
            weighted_freq_sum += freq * mag;

            let bin_energy = mag * mag;
            if freq < 250.0 {
                sub_bass_energy += bin_energy;
            } else if freq < 4000.0 {
                mid_energy += bin_energy;
            } else {
                high_energy += bin_energy;
            }
        }

        // Spectral Centroid
        let spectral_centroid = if total_mag > 1e-6 {
            weighted_freq_sum / total_mag
        } else {
            0.0
        };

        // Spectral Flatness (geometric_mean / arithmetic_mean)
        let spectral_flatness = if total_mag > 1e-6 {
            let log_sum: f32 = magnitudes.iter().map(|&m| (m.max(1e-8)).ln()).sum::<f32>();
            let geom_mean = (log_sum / num_bins as f32).exp();
            let arith_mean = total_mag / num_bins as f32;
            (geom_mean / arith_mean.max(1e-8)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Normalize Band Energies
        let total_band_energy = (sub_bass_energy + mid_energy + high_energy).max(1e-8);
        let energy_sub_bass = sub_bass_energy / total_band_energy;
        let energy_mid = mid_energy / total_band_energy;
        let energy_high = high_energy / total_band_energy;

        SpectralFeatures {
            spectral_centroid,
            spectral_flatness,
            energy_sub_bass,
            energy_mid,
            energy_high,
            zcr,
            rms_energy,
        }
    }
}
