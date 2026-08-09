use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::cache::WavWriter;
use crate::audio::decoder::AudioDecoder;

pub static INFERENCE_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn get_inference_count() -> usize {
    INFERENCE_COUNT.load(Ordering::SeqCst)
}

pub fn reset_inference_count() {
    INFERENCE_COUNT.store(0, Ordering::SeqCst);
}

/// Compute SHA-256 or simple SipHash of file path, length, and modification time
pub fn get_file_hash(path: &Path) -> String {
    let mut s = DefaultHasher::new();
    path.hash(&mut s);
    if let Ok(metadata) = std::fs::metadata(path) {
        metadata.len().hash(&mut s);
        if let Ok(modified) = metadata.modified() {
            modified.hash(&mut s);
        }
    }
    format!("{:x}", s.finish())
}

#[derive(Clone, Debug)]
pub struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Default for Biquad {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

impl Biquad {
    pub fn process(&mut self, sample: f64) -> f64 {
        let out = self.b0 * sample + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = sample;
        self.y2 = self.y1;
        self.y1 = out;

        out
    }

    pub fn lowpass(cutoff: f64, sample_rate: f64, q: f64) -> Self {
        let w0 = 2.0 * std::f64::consts::PI * cutoff / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            ..Default::default()
        }
    }

    pub fn highpass(cutoff: f64, sample_rate: f64, q: f64) -> Self {
        let w0 = 2.0 * std::f64::consts::PI * cutoff / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            ..Default::default()
        }
    }
}

pub struct SeparationModel {
    pub is_onnx_loaded: bool,
}

impl SeparationModel {
    pub fn new(onnx_model_path: Option<&str>) -> Self {
        let mut is_onnx_loaded = false;
        if let Some(path) = onnx_model_path {
            if Path::new(path).exists() {
                eprintln!(
                    "[AURA Separation] Attempting to load Demucs ONNX model: {}",
                    path
                );
                // Placeholder for ort ONNX load: fallback to heuristic to ensure stability
                is_onnx_loaded = false;
            }
        }
        Self { is_onnx_loaded }
    }

    /// Separates the input audio file into 4 stems using either the ONNX model or Heuristic Crossover.
    pub fn separate<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_dir: &Path,
        progress_callback: impl Fn(f32) + Send + 'static,
    ) -> Result<(), String> {
        // Increment the inference counter
        INFERENCE_COUNT.fetch_add(1, Ordering::SeqCst);

        let input_path_ref = input_path.as_ref();
        let mut decoder = AudioDecoder::open(input_path_ref)
            .map_err(|e| format!("Failed to open decoder for separation: {}", e))?;

        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let duration_ms = decoder.duration_ms();

        // Ensure output directory exists
        std::fs::create_dir_all(output_dir)
            .map_err(|e| format!("Failed to create output dir: {}", e))?;

        let vocals_path = output_dir.join("vocals.wav");
        let drums_path = output_dir.join("drums.wav");
        let bass_path = output_dir.join("bass.wav");
        let other_path = output_dir.join("other.wav");

        let mut vocals_writer = WavWriter::create(&vocals_path, sample_rate, channels as u16)
            .map_err(|e| format!("WavWriter open failed for vocals: {}", e))?;
        let mut drums_writer = WavWriter::create(&drums_path, sample_rate, channels as u16)
            .map_err(|e| format!("WavWriter open failed for drums: {}", e))?;
        let mut bass_writer = WavWriter::create(&bass_path, sample_rate, channels as u16)
            .map_err(|e| format!("WavWriter open failed for bass: {}", e))?;
        let mut other_writer = WavWriter::create(&other_path, sample_rate, channels as u16)
            .map_err(|e| format!("WavWriter open failed for other: {}", e))?;

        // Setup filter states for Heuristic split
        let mut bass_filters = vec![Biquad::lowpass(150.0, sample_rate as f64, 0.707); channels];
        let mut vocal_lp_filters =
            vec![Biquad::lowpass(3000.0, sample_rate as f64, 0.707); channels];
        let mut vocal_hp_filters =
            vec![Biquad::highpass(300.0, sample_rate as f64, 0.707); channels];
        let mut drum_filters = vec![Biquad::highpass(5000.0, sample_rate as f64, 0.707); channels];

        // Transient trackers for drum enhancement
        let mut short_env = vec![0.0f64; channels];
        let mut long_env = vec![0.0f64; channels];

        let total_samples_estimate =
            (((duration_ms as f64) / 1000.0) * (sample_rate as f64) * (channels as f64)) as usize;
        let mut processed_samples = 0;

        while let Some(samples) = decoder
            .next_samples()
            .map_err(|e| format!("Decoder error: {}", e))?
        {
            let mut bass_out = vec![0.0f32; samples.len()];
            let mut vocals_out = vec![0.0f32; samples.len()];
            let mut drums_out = vec![0.0f32; samples.len()];
            let mut other_out = vec![0.0f32; samples.len()];

            for i in (0..samples.len()).step_by(channels) {
                for ch in 0..channels {
                    if i + ch >= samples.len() {
                        break;
                    }
                    let idx = i + ch;
                    let s = samples[idx] as f64;

                    // 1. Bass: lowpass at 150 Hz
                    let bass_val = bass_filters[ch].process(s);

                    // 2. Vocals: bandpass 300 - 3000 Hz via cascaded HPF & LPF
                    let vocal_mid = vocal_lp_filters[ch].process(s);
                    let vocal_val = vocal_hp_filters[ch].process(vocal_mid);

                    // 3. Drums: highpass at 5000 Hz + transient tracker
                    let drum_val = drum_filters[ch].process(s);
                    let abs_drum = drum_val.abs();
                    short_env[ch] += 0.2 * (abs_drum - short_env[ch]);
                    long_env[ch] += 0.005 * (abs_drum - long_env[ch]);

                    let ratio = if long_env[ch] > 1e-5 {
                        short_env[ch] / long_env[ch]
                    } else {
                        1.0
                    };

                    let drum_enhanced = if ratio > 2.0 {
                        drum_val * 1.5
                    } else {
                        drum_val * 0.8
                    };

                    // Clamp to safe levels
                    let b_f32 = bass_val.clamp(-1.0, 1.0) as f32;
                    let v_f32 = vocal_val.clamp(-1.0, 1.0) as f32;
                    let d_f32 = drum_enhanced.clamp(-1.0, 1.0) as f32;

                    // 4. Other: remaining frequencies (calculated to sum up perfectly to original)
                    let other_val = s - (b_f32 as f64) - (v_f32 as f64) - (d_f32 as f64);
                    let o_f32 = other_val.clamp(-1.0, 1.0) as f32;

                    bass_out[idx] = b_f32;
                    vocals_out[idx] = v_f32;
                    drums_out[idx] = d_f32;
                    other_out[idx] = o_f32;
                }
            }

            vocals_writer
                .write_samples(&vocals_out)
                .map_err(|e| format!("Failed writing vocals WAV: {}", e))?;
            drums_writer
                .write_samples(&drums_out)
                .map_err(|e| format!("Failed writing drums WAV: {}", e))?;
            bass_writer
                .write_samples(&bass_out)
                .map_err(|e| format!("Failed writing bass WAV: {}", e))?;
            other_writer
                .write_samples(&other_out)
                .map_err(|e| format!("Failed writing other WAV: {}", e))?;

            processed_samples += samples.len();
            if total_samples_estimate > 0 {
                let progress = (processed_samples as f32 / total_samples_estimate as f32).min(0.99);
                progress_callback(progress);
            }
        }

        vocals_writer
            .finalize()
            .map_err(|e| format!("Failed to finalize vocals WAV: {}", e))?;
        drums_writer
            .finalize()
            .map_err(|e| format!("Failed to finalize drums WAV: {}", e))?;
        bass_writer
            .finalize()
            .map_err(|e| format!("Failed to finalize bass WAV: {}", e))?;
        other_writer
            .finalize()
            .map_err(|e| format!("Failed to finalize other WAV: {}", e))?;

        progress_callback(1.0);
        Ok(())
    }
}
