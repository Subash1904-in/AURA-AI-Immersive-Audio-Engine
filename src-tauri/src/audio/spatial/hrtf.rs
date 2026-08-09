#![allow(clippy::needless_range_loop, clippy::manual_repeat_n)]
use rustfft::{num_complex::Complex, FftPlanner};

/// HRTF binaural renderer using synthetic Head-Related Impulse Responses.
///
/// Generates frequency-dependent ITD (interaural time difference) and
/// ILD (interaural level difference) HRIR pairs for 5 virtual speaker
/// positions, then applies partitioned FFT convolution.
#[derive(Debug, Clone)]
pub struct HrtfRenderer {
    hrir_len: usize,
    // Pre-computed frequency-domain HRIR pairs for left and right ears
    // Indexed by virtual speaker position
    hrir_spectra_l: Vec<Vec<Complex<f32>>>,
    hrir_spectra_r: Vec<Vec<Complex<f32>>>,
    // Convolution state buffers
    input_buffer: Vec<f32>,
    output_l: Vec<f32>,
    output_r: Vec<f32>,
    overlap_l: Vec<f32>,
    overlap_r: Vec<f32>,
    fft_size: usize,
    buffer_pos: usize,
}

/// Virtual speaker azimuth positions in degrees
const SPEAKER_AZIMUTHS: [f32; 5] = [0.0, 90.0, -90.0, 135.0, -135.0];

impl HrtfRenderer {
    pub fn new(sample_rate: f32) -> Self {
        let hrir_len: usize = 128;
        let fft_size = (hrir_len * 2).next_power_of_two();

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);

        let mut hrir_spectra_l = Vec::new();
        let mut hrir_spectra_r = Vec::new();

        for &azimuth in &SPEAKER_AZIMUTHS {
            let (hrir_l, hrir_r) = generate_synthetic_hrir(azimuth, hrir_len, sample_rate);

            // Zero-pad and FFT the left HRIR
            let mut spectrum_l: Vec<Complex<f32>> = hrir_l
                .iter()
                .map(|&s| Complex::new(s, 0.0))
                .chain(std::iter::repeat(Complex::new(0.0, 0.0)).take(fft_size - hrir_len))
                .collect();
            fft.process(&mut spectrum_l);
            hrir_spectra_l.push(spectrum_l);

            // Zero-pad and FFT the right HRIR
            let mut spectrum_r: Vec<Complex<f32>> = hrir_r
                .iter()
                .map(|&s| Complex::new(s, 0.0))
                .chain(std::iter::repeat(Complex::new(0.0, 0.0)).take(fft_size - hrir_len))
                .collect();
            fft.process(&mut spectrum_r);
            hrir_spectra_r.push(spectrum_r);
        }

        Self {
            hrir_len,
            hrir_spectra_l,
            hrir_spectra_r,
            input_buffer: vec![0.0; fft_size],
            output_l: vec![0.0; fft_size],
            output_r: vec![0.0; fft_size],
            overlap_l: vec![0.0; hrir_len],
            overlap_r: vec![0.0; hrir_len],
            fft_size,
            buffer_pos: 0,
        }
    }

    /// Process interleaved stereo samples through HRTF convolution.
    /// Uses the front speaker (azimuth 0°) as default for simplicity.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        let speaker_idx = 0; // Front speaker
        let num_frames = samples.len() / 2;
        let block_size = self.hrir_len;

        let mut frame = 0;
        while frame < num_frames {
            let remaining = num_frames - frame;
            let to_process = remaining.min(block_size - self.buffer_pos);

            // Accumulate mono mix into input buffer
            for i in 0..to_process {
                let l = samples[(frame + i) * 2];
                let r = samples[(frame + i) * 2 + 1];
                self.input_buffer[self.buffer_pos + i] = (l + r) * 0.5;
            }
            self.buffer_pos += to_process;

            if self.buffer_pos >= block_size {
                // Process one block through FFT convolution
                self.convolve_block(speaker_idx);
                self.buffer_pos = 0;

                // Write convolved output back to interleaved buffer
                let write_start = frame + to_process - block_size;
                for i in 0..block_size {
                    if write_start + i < num_frames {
                        samples[(write_start + i) * 2] = self.output_l[i];
                        samples[(write_start + i) * 2 + 1] = self.output_r[i];
                    }
                }
            }

            frame += to_process;
        }
    }

    fn convolve_block(&mut self, speaker_idx: usize) {
        let mut planner = FftPlanner::new();
        let fft_fwd = planner.plan_fft_forward(self.fft_size);
        let fft_inv = planner.plan_fft_inverse(self.fft_size);

        // Zero-pad input to fft_size
        for i in self.hrir_len..self.fft_size {
            self.input_buffer[i] = 0.0;
        }

        // Forward FFT of input
        let mut input_spectrum: Vec<Complex<f32>> = self
            .input_buffer
            .iter()
            .map(|&s| Complex::new(s, 0.0))
            .collect();
        fft_fwd.process(&mut input_spectrum);

        // Multiply with HRIR spectra for left ear
        let mut result_l: Vec<Complex<f32>> = input_spectrum
            .iter()
            .zip(self.hrir_spectra_l[speaker_idx].iter())
            .map(|(a, b)| a * b)
            .collect();
        fft_inv.process(&mut result_l);

        // Multiply with HRIR spectra for right ear
        let mut result_r: Vec<Complex<f32>> = input_spectrum
            .iter()
            .zip(self.hrir_spectra_r[speaker_idx].iter())
            .map(|(a, b)| a * b)
            .collect();
        fft_inv.process(&mut result_r);

        let scale = 1.0 / self.fft_size as f32;

        // Overlap-add for left channel
        for i in 0..self.hrir_len {
            self.output_l[i] = result_l[i].re * scale + self.overlap_l[i];
        }
        for i in 0..self.hrir_len {
            self.overlap_l[i] = if self.hrir_len + i < self.fft_size {
                result_l[self.hrir_len + i].re * scale
            } else {
                0.0
            };
        }

        // Overlap-add for right channel
        for i in 0..self.hrir_len {
            self.output_r[i] = result_r[i].re * scale + self.overlap_r[i];
        }
        for i in 0..self.hrir_len {
            self.overlap_r[i] = if self.hrir_len + i < self.fft_size {
                result_r[self.hrir_len + i].re * scale
            } else {
                0.0
            };
        }
    }
}

/// Generate a synthetic HRIR pair (left ear, right ear) for a given azimuth.
///
/// Uses a frequency-dependent ITD/ILD model:
/// - ITD (interaural time difference): modeled as a fractional sample delay
///   proportional to sin(azimuth), with max ~0.65 ms at 90°
/// - ILD (interaural level difference): frequency-dependent attenuation on the
///   shadowed ear, increasing with frequency (head shadow effect)
pub fn generate_synthetic_hrir(
    azimuth_deg: f32,
    length: usize,
    sample_rate: f32,
) -> (Vec<f32>, Vec<f32>) {
    let azimuth_rad = azimuth_deg.to_radians();
    let sin_az = azimuth_rad.sin();

    // Max ITD ~0.65 ms (head radius ~8.75 cm, speed of sound 343 m/s)
    let max_itd_samples = 0.00065 * sample_rate;
    let itd_samples = max_itd_samples * sin_az.abs();

    // Generate a minimum-phase impulse with head-shadow frequency shaping
    let mut hrir_near = vec![0.0f32; length];
    let mut hrir_far = vec![0.0f32; length];

    // Near ear: mostly direct impulse with slight high-frequency emphasis
    hrir_near[0] = 1.0;
    // Apply a gentle low-pass decay to simulate pinnae filtering
    for i in 1..length {
        let t = i as f32 / sample_rate;
        let decay = (-t * 8000.0).exp();
        // Add subtle reflections from pinnae
        let pinnae = if i < 8 {
            0.1 * (-0.3 * i as f32).exp() * ((i as f32 * 2.5).sin())
        } else {
            0.0
        };
        hrir_near[i] = pinnae * decay;
    }

    // Far ear: delayed, attenuated, low-pass filtered (head shadow)
    let delay_int = itd_samples.floor() as usize;
    let delay_frac = itd_samples - delay_int as f32;

    // ILD: ~6 dB attenuation at high frequencies for 90°, scales with azimuth
    let ild_factor = 1.0 - 0.5 * sin_az.abs();

    if delay_int < length {
        // Fractional delay via linear interpolation
        hrir_far[delay_int] = ild_factor * (1.0 - delay_frac);
        if delay_int + 1 < length {
            hrir_far[delay_int + 1] = ild_factor * delay_frac;
        }
    }

    // Apply stronger low-pass to far ear (head shadow removes high frequencies)
    let shadow_cutoff = 3000.0 + 5000.0 * (1.0 - sin_az.abs());
    let rc = 1.0 / (2.0 * std::f32::consts::PI * shadow_cutoff);
    let dt = 1.0 / sample_rate;
    let alpha = dt / (rc + dt);

    let mut filtered_far = vec![0.0f32; length];
    let mut state = 0.0f32;
    for i in 0..length {
        state = state + alpha * (hrir_far[i] - state);
        filtered_far[i] = state;
    }

    // Assign left/right based on azimuth sign
    // Positive azimuth = source on left, so left ear is near
    if sin_az >= 0.0 {
        (hrir_near, filtered_far)
    } else {
        (filtered_far, hrir_near)
    }
}
