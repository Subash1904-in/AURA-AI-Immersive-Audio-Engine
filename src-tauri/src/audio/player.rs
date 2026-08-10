use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};

use super::analysis::AnalysisEngine;
use super::decoder::AudioDecoder;
use super::dsp::chain::DspChain;
use super::dsp::params::{AnalysisStateInfo, DspParams};
use super::output::AudioOutput;
use super::separation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub file_path: String,
    pub title: String,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStateInfo {
    pub is_playing: bool,
    pub current_position_ms: u64,
    pub duration_ms: u64,
    pub track: Option<TrackInfo>,
}

enum PlayerCommand {
    Load {
        path: String,
        reply: Sender<Result<TrackInfo, String>>,
    },
    Play,
    Pause,
    Seek {
        ms: u64,
        reply: Sender<Result<(), String>>,
    },
    GetState {
        reply: Sender<PlaybackStateInfo>,
    },
}

pub struct AudioPlayer {
    sender: Sender<PlayerCommand>,
    pub params_bus: Arc<ArcSwap<DspParams>>,
    pub analysis_engine: Arc<AnalysisEngine>,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (tx, rx) = channel::<PlayerCommand>();
        let initial_params = if let Ok(config) = crate::ipc::persistence::load_config() {
            config.dsp_params
        } else {
            DspParams::default()
        };
        let params_bus = Arc::new(ArcSwap::from_pointee(initial_params));
        let params_bus_worker = params_bus.clone();

        let analysis_engine = Arc::new(AnalysisEngine::new(44100.0, params_bus.clone()));
        let analysis_engine_worker = analysis_engine.clone();

        thread::spawn(move || {
            let mut current_decoder: Option<AudioDecoder> = None;
            let mut stem_decoders: Option<Vec<AudioDecoder>> = None;
            let mut current_track_hash: Option<String> = None;
            let mut current_output: Option<AudioOutput> = None;
            let mut current_dsp: Option<DspChain> = None;
            let mut current_track: Option<TrackInfo> = None;
            let mut base_ms: u64 = 0;
            let mut is_eof = false;

            loop {
                // Process any pending commands
                while let Ok(cmd) = rx.try_recv() {
                    match cmd {
                        PlayerCommand::Load { path, reply } => {
                            // Stop current output
                            if let Some(out) = current_output.take() {
                                out.is_playing.store(false, Ordering::SeqCst);
                            }

                            stem_decoders = None;
                            current_track_hash = None;

                            let res = (|| -> Result<TrackInfo, String> {
                                let path_buf = PathBuf::from(&path);
                                let title = path_buf
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "Unknown Track".to_string());

                                let decoder = AudioDecoder::open(&path)?;
                                let sample_rate = decoder.sample_rate();
                                let channels = decoder.channels();
                                let duration_ms = decoder.duration_ms();

                                // Calculate track hash
                                let hash = separation::model::get_file_hash(&path_buf);
                                current_track_hash = Some(hash);

                                // 2 seconds buffer capacity
                                let buffer_capacity = (sample_rate as usize) * channels * 2;
                                let output = AudioOutput::new(buffer_capacity)?;
                                let dsp = DspChain::new(
                                    sample_rate as f32,
                                    channels,
                                    params_bus_worker.clone(),
                                );

                                let track = TrackInfo {
                                    file_path: path.clone(),
                                    title,
                                    duration_ms,
                                    sample_rate,
                                    channels,
                                };

                                current_decoder = Some(decoder);
                                current_output = Some(output);
                                current_dsp = Some(dsp);
                                current_track = Some(track.clone());
                                base_ms = 0;
                                is_eof = false;

                                Ok(track)
                            })();

                            let _ = reply.send(res);
                        }
                        PlayerCommand::Play => {
                            if let Some(ref output) = current_output {
                                output.is_playing.store(true, Ordering::SeqCst);
                            }
                        }
                        PlayerCommand::Pause => {
                            if let Some(ref output) = current_output {
                                output.is_playing.store(false, Ordering::SeqCst);
                            }
                        }
                        PlayerCommand::Seek { ms, reply } => {
                            let res = (|| -> Result<(), String> {
                                if let (Some(decoder), Some(output)) =
                                    (current_decoder.as_mut(), current_output.as_mut())
                                {
                                    let was_playing = output.is_playing.load(Ordering::SeqCst);
                                    output.is_playing.store(false, Ordering::SeqCst);

                                    // Seek decoder
                                    let actual_ms = decoder.seek(ms)?;
                                    base_ms = actual_ms;
                                    output.played_samples.store(0, Ordering::SeqCst);
                                    is_eof = false;

                                    // Seek stem decoders if active
                                    if let Some(ref mut stems) = stem_decoders {
                                        for dec in stems.iter_mut() {
                                            let _ = dec.seek(ms);
                                        }
                                    }

                                    // Re-create output to ensure ring buffer is completely flushed
                                    let capacity =
                                        (output.sample_rate as usize) * output.channels * 2;
                                    if let Ok(new_output) = AudioOutput::new(capacity) {
                                        new_output.is_playing.store(was_playing, Ordering::SeqCst);
                                        *output = new_output;
                                    } else {
                                        output.is_playing.store(was_playing, Ordering::SeqCst);
                                    }

                                    Ok(())
                                } else {
                                    Err("No audio file loaded".to_string())
                                }
                            })();
                            let _ = reply.send(res);
                        }
                        PlayerCommand::GetState { reply } => {
                            let (is_playing, current_pos, duration) =
                                if let (Some(output), Some(track)) =
                                    (current_output.as_ref(), current_track.as_ref())
                                {
                                    let playing = output.is_playing.load(Ordering::SeqCst);
                                    let played_samples =
                                        output.played_samples.load(Ordering::SeqCst);
                                    let total_channels = output.channels.max(1) as u64;
                                    let sample_rate = output.sample_rate as u64;

                                    let elapsed_ms = if sample_rate > 0 {
                                        (played_samples * 1000) / (sample_rate * total_channels)
                                    } else {
                                        0
                                    };

                                    let pos = (base_ms + elapsed_ms).min(track.duration_ms);
                                    (playing, pos, track.duration_ms)
                                } else {
                                    (false, 0, 0)
                                };

                            let state_info = PlaybackStateInfo {
                                is_playing,
                                current_position_ms: current_pos,
                                duration_ms: duration,
                                track: current_track.clone(),
                            };

                            let _ = reply.send(state_info);
                        }
                    }
                }

                // If output is active and not EOF, decode and push samples to ring buffer
                let mut did_work = false;
                if let Some(output) = current_output.as_mut() {
                    let dsp_params = params_bus_worker.load();
                    let use_stems = dsp_params.stems_active && dsp_params.stems_ready;

                    if use_stems {
                        if stem_decoders.is_none() {
                            if let Some(ref hash) = current_track_hash {
                                let dir = separation::cache::get_track_cache_dir(hash);
                                let vocals_path = dir.join("vocals.wav");
                                let drums_path = dir.join("drums.wav");
                                let bass_path = dir.join("bass.wav");
                                let other_path = dir.join("other.wav");

                                if vocals_path.exists()
                                    && drums_path.exists()
                                    && bass_path.exists()
                                    && other_path.exists()
                                {
                                    let v_dec = AudioDecoder::open(&vocals_path);
                                    let d_dec = AudioDecoder::open(&drums_path);
                                    let b_dec = AudioDecoder::open(&bass_path);
                                    let o_dec = AudioDecoder::open(&other_path);

                                    if let (Ok(mut v), Ok(mut d), Ok(mut b), Ok(mut o)) =
                                        (v_dec, d_dec, b_dec, o_dec)
                                    {
                                        // Seek to current playhead
                                        let played_samples =
                                            output.played_samples.load(Ordering::SeqCst);
                                        let total_channels = output.channels.max(1) as u64;
                                        let sample_rate = output.sample_rate as u64;
                                        let elapsed_ms = if sample_rate > 0 {
                                            (played_samples * 1000) / (sample_rate * total_channels)
                                        } else {
                                            0
                                        };
                                        let current_pos_ms = base_ms + elapsed_ms;

                                        let _ = v.seek(current_pos_ms);
                                        let _ = d.seek(current_pos_ms);
                                        let _ = b.seek(current_pos_ms);
                                        let _ = o.seek(current_pos_ms);

                                        stem_decoders = Some(vec![v, d, b, o]);
                                        eprintln!("[AURA Player] Switched to stem decoding.");
                                    }
                                }
                            }
                        }
                    } else {
                        // Not using stems. Clear if loaded, and seek current_decoder back to playhead
                        if stem_decoders.is_some() {
                            stem_decoders = None;
                            if let Some(ref mut dec) = current_decoder {
                                let played_samples = output.played_samples.load(Ordering::SeqCst);
                                let total_channels = output.channels.max(1) as u64;
                                let sample_rate = output.sample_rate as u64;
                                let elapsed_ms = if sample_rate > 0 {
                                    (played_samples * 1000) / (sample_rate * total_channels)
                                } else {
                                    0
                                };
                                let current_pos_ms = base_ms + elapsed_ms;
                                let _ = dec.seek(current_pos_ms);
                                eprintln!(
                                    "[AURA Player] Switched back to original track decoding."
                                );
                            }
                        }
                    }

                    use ringbuf::traits::{Observer, Producer};
                    let vacant = output.producer.vacant_len();

                    if vacant >= 2048 && !is_eof {
                        let next_samples_res = if let Some(ref mut stems) = stem_decoders {
                            let vocals_opt = stems[0].next_samples();
                            let drums_opt = stems[1].next_samples();
                            let bass_opt = stems[2].next_samples();
                            let other_opt = stems[3].next_samples();

                            if let (
                                Ok(Some(mut vocals)),
                                Ok(Some(drums)),
                                Ok(Some(bass)),
                                Ok(Some(other)),
                            ) = (vocals_opt, drums_opt, bass_opt, other_opt)
                            {
                                let len = vocals.len();
                                let v_gain = if dsp_params.vocals_mute {
                                    0.0
                                } else {
                                    dsp_params.vocals_gain
                                };
                                let d_gain = if dsp_params.drums_mute {
                                    0.0
                                } else {
                                    dsp_params.drums_gain
                                };
                                let b_gain = if dsp_params.bass_mute {
                                    0.0
                                } else {
                                    dsp_params.bass_gain
                                };
                                let o_gain = if dsp_params.other_mute {
                                    0.0
                                } else {
                                    dsp_params.other_gain
                                };

                                #[allow(clippy::needless_range_loop)]
                                for i in 0..len {
                                    let v_s = vocals[i] * v_gain;
                                    let d_s = drums.get(i).copied().unwrap_or(0.0) * d_gain;
                                    let b_s = bass.get(i).copied().unwrap_or(0.0) * b_gain;
                                    let o_s = other.get(i).copied().unwrap_or(0.0) * o_gain;
                                    vocals[i] = v_s + d_s + b_s + o_s;
                                }
                                Ok(Some(vocals))
                            } else {
                                Ok(None)
                            }
                        } else {
                            current_decoder
                                .as_mut()
                                .map_or(Ok(None), |dec| dec.next_samples())
                        };

                        match next_samples_res {
                            Ok(Some(mut samples)) => {
                                did_work = true;
                                analysis_engine_worker.push_samples(&samples);

                                if let Some(ref mut dsp) = current_dsp {
                                    dsp.process_interleaved(&mut samples);
                                }

                                let mut offset = 0;
                                let mut wait_count = 0;
                                while offset < samples.len() {
                                    let pushed = output.producer.push_slice(&samples[offset..]);
                                    if pushed == 0 {
                                        if !output.is_playing.load(Ordering::SeqCst)
                                            || wait_count > 10
                                        {
                                            break;
                                        }
                                        thread::sleep(Duration::from_millis(5));
                                        wait_count += 1;
                                    } else {
                                        offset += pushed;
                                    }
                                }
                            }
                            Ok(None) => {
                                is_eof = true;
                            }
                            Err(e) => {
                                eprintln!("Decode error during stream pumping: {}", e);
                                is_eof = true;
                            }
                        }
                    }
                }

                if !did_work {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });

        Self {
            sender: tx,
            params_bus,
            analysis_engine,
        }
    }

    pub fn load_file(&self, path: String) -> Result<TrackInfo, String> {
        let (tx, rx) = channel();
        self.sender
            .send(PlayerCommand::Load { path, reply: tx })
            .map_err(|_| "Audio worker thread down".to_string())?;
        rx.recv()
            .map_err(|_| "No response from audio thread".to_string())?
    }

    pub fn play(&self) -> Result<(), String> {
        self.sender
            .send(PlayerCommand::Play)
            .map_err(|_| "Audio worker thread down".to_string())
    }

    pub fn pause(&self) -> Result<(), String> {
        self.sender
            .send(PlayerCommand::Pause)
            .map_err(|_| "Audio worker thread down".to_string())
    }

    pub fn seek(&self, ms: u64) -> Result<(), String> {
        let (tx, rx) = channel();
        self.sender
            .send(PlayerCommand::Seek { ms, reply: tx })
            .map_err(|_| "Audio worker thread down".to_string())?;
        rx.recv()
            .map_err(|_| "No response from audio thread".to_string())?
    }

    pub fn get_position(&self) -> Result<PlaybackStateInfo, String> {
        let (tx, rx) = channel();
        self.sender
            .send(PlayerCommand::GetState { reply: tx })
            .map_err(|_| "Audio worker thread down".to_string())?;
        rx.recv()
            .map_err(|_| "No response from audio thread".to_string())
    }

    pub fn get_dsp_params(&self) -> DspParams {
        self.params_bus.load_full().as_ref().clone()
    }

    pub fn set_dsp_params(&self, params: DspParams) {
        self.params_bus.store(Arc::new(params));
    }

    pub fn toggle_dsp_stage(&self, stage: &str, enabled: bool) {
        let mut params = self.get_dsp_params();
        match stage {
            "eq" => params.eq_enabled = enabled,
            "bass" => params.bass_enabled = enabled,
            "compressor" => params.compressor_enabled = enabled,
            "loudness" => params.loudness_enabled = enabled,
            "spatial" => params.spatial_enabled = enabled,
            "reverb" => params.reverb_enabled = enabled,
            "limiter" => params.limiter_enabled = enabled,
            _ => {}
        }
        self.set_dsp_params(params);
    }

    pub fn set_eq_band(&self, index: usize, freq: f32, gain_db: f32, q: f32) {
        let mut params = self.get_dsp_params();
        if index < params.eq.bands.len() {
            params.eq.bands[index].frequency = freq;
            params.eq.bands[index].gain_db = gain_db;
            params.eq.bands[index].q = q;
            self.set_dsp_params(params);
        }
    }

    pub fn set_reverb_environment(&self, env: &str) {
        use super::dsp::params::ReverbEnvironment;
        let mut params = self.get_dsp_params();
        params.reverb.environment = match env {
            "SmallRoom" => ReverbEnvironment::SmallRoom,
            "ConcertHall" => ReverbEnvironment::ConcertHall,
            "Cathedral" => ReverbEnvironment::Cathedral,
            "Cave" => ReverbEnvironment::Cave,
            _ => ReverbEnvironment::Off,
        };
        // Auto-enable reverb when selecting a non-Off environment
        params.reverb_enabled = params.reverb.environment != ReverbEnvironment::Off;
        self.set_dsp_params(params);
    }

    pub fn set_spatial_width(&self, width: f32) {
        let mut params = self.get_dsp_params();
        params.spatial.width = width.clamp(0.0, 2.0);
        self.set_dsp_params(params);
    }

    pub fn set_reverb_mix(&self, mix: f32) {
        let mut params = self.get_dsp_params();
        params.reverb.wet_dry_mix = mix.clamp(0.0, 1.0);
        self.set_dsp_params(params);
    }

    pub fn toggle_crossfeed(&self, enabled: bool) {
        let mut params = self.get_dsp_params();
        if !enabled {
            params.spatial.crossfeed_level = 0.0;
        } else if params.spatial.crossfeed_level < 0.01 {
            params.spatial.crossfeed_level = 0.3; // Restore default level
        }
        self.set_dsp_params(params);
    }

    pub fn toggle_hrtf(&self, enabled: bool) {
        let mut params = self.get_dsp_params();
        params.spatial.hrtf_enabled = enabled;
        self.set_dsp_params(params);
    }

    // --- Phase 3 AI Analysis Methods ---

    pub fn toggle_auto_mode(&self, enabled: bool) {
        self.analysis_engine.set_auto_mode(enabled);
    }

    pub fn toggle_beat_modulation(&self, enabled: bool) {
        self.analysis_engine.set_beat_modulation(enabled);
    }

    pub fn get_analysis_state(&self) -> AnalysisStateInfo {
        self.analysis_engine.get_state()
    }

    // --- Phase 4 Source Separation Methods ---

    pub fn set_stem_gain(&self, stem: &str, gain: f32) {
        let mut params = self.get_dsp_params();
        let g = gain.clamp(0.0, 1.0);
        match stem {
            "vocals" => params.vocals_gain = g,
            "drums" => params.drums_gain = g,
            "bass" => params.bass_gain = g,
            "other" => params.other_gain = g,
            _ => {}
        }
        self.set_dsp_params(params);
    }

    pub fn set_stem_mute(&self, stem: &str, mute: bool) {
        let mut params = self.get_dsp_params();
        match stem {
            "vocals" => params.vocals_mute = mute,
            "drums" => params.drums_mute = mute,
            "bass" => params.bass_mute = mute,
            "other" => params.other_mute = mute,
            _ => {}
        }
        self.set_dsp_params(params);
    }

    pub fn set_stems_active(&self, active: bool) {
        let mut params = self.get_dsp_params();
        params.stems_active = active;
        self.set_dsp_params(params);
    }

    // --- Phase 6: NL EQ, Night Mode, and Persistence Methods ---

    pub fn apply_nl_prompt(&self, prompt: &str) -> (DspParams, Vec<String>) {
        let mut params = self.get_dsp_params();
        let engine = crate::audio::dsp::nl_eq::NLEqEngine::new();
        let matched = engine.parse_and_apply(prompt, &mut params);
        self.set_dsp_params(params.clone());
        (params, matched)
    }

    pub fn toggle_night_mode(&self, enabled: bool) -> DspParams {
        let mut params = self.get_dsp_params();
        crate::audio::dsp::night_mode::apply_night_mode(&mut params, enabled);
        self.set_dsp_params(params.clone());
        params
    }

    pub fn save_settings(&self) -> Result<(), String> {
        let params = self.get_dsp_params();
        let night_mode = params.is_night_mode;
        let config = crate::ipc::persistence::AppConfig {
            dsp_params: params,
            last_track_path: None,
            night_mode,
        };
        crate::ipc::persistence::save_config(&config)
    }

    pub fn load_settings(&self) -> Result<DspParams, String> {
        let config = crate::ipc::persistence::load_config()?;
        self.set_dsp_params(config.dsp_params.clone());
        Ok(config.dsp_params)
    }

    pub fn reset_settings(&self) -> Result<DspParams, String> {
        let config = crate::ipc::persistence::reset_config()?;
        self.set_dsp_params(config.dsp_params.clone());
        Ok(config.dsp_params)
    }
}
