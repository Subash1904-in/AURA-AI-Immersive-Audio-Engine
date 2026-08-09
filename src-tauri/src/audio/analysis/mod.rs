pub mod bpm;
pub mod classifier;
pub mod presets;
pub mod spectral;

#[cfg(test)]
mod analysis_tests;

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

pub struct AnalysisEngine {
    state_bus: Arc<ArcSwap<AnalysisStateInfo>>,
    params_bus: Arc<ArcSwap<DspParams>>,
    sample_sender: Sender<Vec<f32>>,
    is_running: Arc<AtomicBool>,
}

impl AnalysisEngine {
    pub fn new(sample_rate: f32, params_bus: Arc<ArcSwap<DspParams>>) -> Self {
        let state_bus = Arc::new(ArcSwap::from_pointee(AnalysisStateInfo::default()));
        let (tx, rx) = channel::<Vec<f32>>();
        let is_running = Arc::new(AtomicBool::new(true));

        let state_bus_worker = state_bus.clone();
        let params_bus_worker = params_bus.clone();
        let is_running_worker = is_running.clone();

        thread::spawn(move || {
            let mut bpm_detector = BpmDetector::new(sample_rate);
            let spectral_extractor = SpectralExtractor::new(sample_rate, 2048);
            let classifier = Classifier::new(None);

            let mut last_genre = "Unknown".to_string();

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
                }
            }
        });

        Self {
            state_bus,
            params_bus,
            sample_sender: tx,
            is_running,
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
}

impl Drop for AnalysisEngine {
    fn drop(&mut self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}
