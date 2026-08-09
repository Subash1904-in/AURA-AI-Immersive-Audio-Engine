use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};

use super::decoder::AudioDecoder;
use super::dsp::chain::DspChain;
use super::dsp::params::DspParams;
use super::output::AudioOutput;

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
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (tx, rx) = channel::<PlayerCommand>();
        let params_bus = Arc::new(ArcSwap::from_pointee(DspParams::default()));
        let params_bus_worker = params_bus.clone();

        thread::spawn(move || {
            let mut current_decoder: Option<AudioDecoder> = None;
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
                if let (Some(decoder), Some(output)) =
                    (current_decoder.as_mut(), current_output.as_mut())
                {
                    if !is_eof {
                        // Check if ring buffer has space for decoding next frame
                        match decoder.next_samples() {
                            Ok(Some(mut samples)) => {
                                did_work = true;

                                // Run real-time 5-stage DSP chain
                                if let Some(ref mut dsp) = current_dsp {
                                    dsp.process_interleaved(&mut samples);
                                }

                                use ringbuf::traits::Producer;
                                let mut offset = 0;
                                while offset < samples.len() {
                                    let pushed = output.producer.push_slice(&samples[offset..]);
                                    if pushed == 0 {
                                        // Buffer full, sleep briefly to let output consumer drain
                                        thread::sleep(Duration::from_millis(5));
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
}
