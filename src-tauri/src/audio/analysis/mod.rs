pub mod bpm;
pub mod classifier;
pub mod presets;
pub mod spectral;

#[cfg(test)]
mod analysis_tests;

#[cfg(test)]
mod visualizer_tests;

use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;

use crate::audio::dsp::params::{AnalysisStateInfo, DspParams};
use bpm::BpmDetector;
use classifier::Classifier;
use presets::PresetEngine;
use spectral::SpectralExtractor;

#[derive(Debug, Clone, serde::Serialize)]
pub struct VisualizerPayload {
    pub magnitudes: Vec<f32>,
    pub is_beat: bool,
    pub beat_boost: f32,
    pub rms_energy: f32,
}

pub struct AnalysisEngine {
    state_bus: Arc<ArcSwap<AnalysisStateInfo>>,
    params_bus: Arc<ArcSwap<DspParams>>,
    sample_sender: Sender<Vec<f32>>,
    is_running: Arc<AtomicBool>,
    visualizer_callback:
        Arc<ArcSwap<Option<Arc<dyn Fn(VisualizerPayload) + Send + Sync + 'static>>>>,
}

impl AnalysisEngine {
    pub fn new(sample_rate: f32, params_bus: Arc<ArcSwap<DspParams>>) -> Self {
        let state_bus = Arc::new(ArcSwap::from_pointee(AnalysisStateInfo::default()));
        let (tx, rx) = channel::<Vec<f32>>();
        let is_running = Arc::new(AtomicBool::new(true));
        let visualizer_callback = Arc::new(ArcSwap::from_pointee(None));

        let state_bus_worker = state_bus.clone();
        let params_bus_worker = params_bus.clone();
        let is_running_worker = is_running.clone();
        let visualizer_callback_worker = visualizer_callback.clone();

        thread::spawn(move || {
            let mut bpm_detector = BpmDetector::new(sample_rate);
            let spectral_extractor = SpectralExtractor::new(sample_rate, 2048);
            let classifier = Classifier::new(None);

            let mut last_genre = "Unknown".to_string();
            let mut last_emit = std::time::Instant::now();

            while is_running_worker.load(Ordering::Relaxed) {
                if let Ok(samples) = rx.recv() {
                    if samples.is_empty() {
                        continue;
                    }

                    // 1. Run BPM and beat onset detector
                    let (bpm, is_beat, beat_boost) = bpm_detector.process_block(&samples);

                    // 2. Extract spectral features
                    let features = spectral_extractor.extract(&samples);

                    // 3. Classify genre and mood
                    let classification = classifier.classify(&features, bpm);

                    // 4. Update analysis state bus
                    let current_params = params_bus_worker.load();
                    let active_preset = if current_params.is_auto_mode {
                        classification.genre.clone()
                    } else {
                        "Manual".to_string()
                    };

                    let new_state = AnalysisStateInfo {
                        bpm,
                        genre: classification.genre.clone(),
                        mood_valence: classification.mood_valence,
                        mood_energy: classification.mood_energy,
                        is_beat,
                        active_preset: active_preset.clone(),
                        is_auto_mode: current_params.is_auto_mode,
                        is_onnx_loaded: classification.is_onnx_loaded,
                    };
                    state_bus_worker.store(Arc::new(new_state));

                    // 5. If Auto mode is enabled and genre changed, apply adaptive preset
                    if current_params.is_auto_mode {
                        if classification.genre != last_genre {
                            let mut target_params =
                                PresetEngine::get_preset_params(&classification.genre);
                            target_params.is_auto_mode = true;
                            target_params.beat_modulation_enabled =
                                current_params.beat_modulation_enabled;
                            params_bus_worker.store(Arc::new(target_params));
                            last_genre = classification.genre;
                        }

                        // Also update beat_boost envelope in params_bus for real-time DSP modulation
                        if current_params.beat_modulation_enabled {
                            let mut updated = params_bus_worker.load_full().as_ref().clone();
                            updated.beat_boost = beat_boost;
                            params_bus_worker.store(Arc::new(updated));
                        }
                    }

                    // 6. Visualizer emission (throttled to ~40Hz)
                    if current_params.visualizer_enabled {
                        let now = std::time::Instant::now();
                        let elapsed = now.duration_since(last_emit);
                        if elapsed.as_millis() >= 25 {
                            // ~40Hz cap
                            if let Some(ref cb) = **visualizer_callback_worker.load() {
                                let downsampled = downsample_magnitudes(&features.magnitudes, 64);
                                cb(VisualizerPayload {
                                    magnitudes: downsampled,
                                    is_beat,
                                    beat_boost,
                                    rms_energy: features.rms_energy,
                                });
                                last_emit = now;
                            }
                        }
                    }
                }
            }
        });

        Self {
            state_bus,
            params_bus,
            sample_sender: tx,
            is_running,
            visualizer_callback,
        }
    }

    /// Push an audio buffer to the background analysis thread.
    pub fn push_samples(&self, samples: &[f32]) {
        let _ = self.sample_sender.send(samples.to_vec());
    }

    /// Get current analysis state info.
    pub fn get_state(&self) -> AnalysisStateInfo {
        self.state_bus.load_full().as_ref().clone()
    }

    /// Toggle Auto (AI-driven) mode.
    pub fn set_auto_mode(&self, enabled: bool) {
        let mut params = self.params_bus.load_full().as_ref().clone();
        params.is_auto_mode = enabled;

        if enabled {
            let state = self.get_state();
            let genre = if state.genre != "Unknown" {
                &state.genre
            } else {
                "Pop"
            };
            let mut preset_params = PresetEngine::get_preset_params(genre);
            preset_params.is_auto_mode = true;
            preset_params.beat_modulation_enabled = params.beat_modulation_enabled;
            self.params_bus.store(Arc::new(preset_params));
        } else {
            params.active_preset = "Manual".to_string();
            self.params_bus.store(Arc::new(params));
        }
    }

    /// Toggle beat-driven modulation.
    pub fn set_beat_modulation(&self, enabled: bool) {
        let mut params = self.params_bus.load_full().as_ref().clone();
        params.beat_modulation_enabled = enabled;
        if !enabled {
            params.beat_boost = 0.0;
        }
        self.params_bus.store(Arc::new(params));
    }

    /// Set a callback for throttled real-time visualizer data.
    pub fn set_visualizer_callback(
        &self,
        callback: Option<Arc<dyn Fn(VisualizerPayload) + Send + Sync + 'static>>,
    ) {
        self.visualizer_callback.store(Arc::new(callback));
    }
}

fn downsample_magnitudes(magnitudes: &[f32], target_bins: usize) -> Vec<f32> {
    if magnitudes.is_empty() {
        return vec![0.0; target_bins];
    }
    let mut result = Vec::with_capacity(target_bins);
    let n = magnitudes.len();
    for i in 0..target_bins {
        let start_frac = i as f32 / target_bins as f32;
        let end_frac = (i + 1) as f32 / target_bins as f32;

        let start_idx = ((start_frac * start_frac) * n as f32) as usize;
        let end_idx = (((end_frac * end_frac) * n as f32) as usize).clamp(start_idx + 1, n);

        let mut sum = 0.0;
        let mut count = 0;
        for j in start_idx..end_idx {
            sum += magnitudes[j];
            count += 1;
        }
        let avg = if count > 0 { sum / count as f32 } else { 0.0 };
        result.push(avg);
    }
    result
}

impl Drop for AnalysisEngine {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}
