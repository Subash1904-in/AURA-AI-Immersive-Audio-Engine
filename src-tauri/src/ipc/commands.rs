use crate::audio::dsp::params::DspParams;
use crate::audio::player::{AudioPlayer, PlaybackStateInfo, TrackInfo};
use std::sync::Arc;

#[tauri::command]
pub fn load_file(
    path: String,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<TrackInfo, String> {
    state.load_file(path)
}

#[tauri::command]
pub fn play(state: tauri::State<'_, Arc<AudioPlayer>>) -> Result<(), String> {
    state.play()
}

#[tauri::command]
pub fn pause(state: tauri::State<'_, Arc<AudioPlayer>>) -> Result<(), String> {
    state.pause()
}

#[tauri::command]
pub fn seek(ms: u64, state: tauri::State<'_, Arc<AudioPlayer>>) -> Result<(), String> {
    state.seek(ms)
}

#[tauri::command]
pub fn get_position(
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<PlaybackStateInfo, String> {
    state.get_position()
}

#[tauri::command]
pub fn get_dsp_params(state: tauri::State<'_, Arc<AudioPlayer>>) -> Result<DspParams, String> {
    Ok(state.get_dsp_params())
}

#[tauri::command]
pub fn set_dsp_params(
    params: DspParams,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.set_dsp_params(params);
    Ok(())
}

#[tauri::command]
pub fn toggle_dsp_stage(
    stage: String,
    enabled: bool,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.toggle_dsp_stage(&stage, enabled);
    Ok(())
}

#[tauri::command]
pub fn set_eq_band(
    index: usize,
    freq: f32,
    gain_db: f32,
    q: f32,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.set_eq_band(index, freq, gain_db, q);
    Ok(())
}
