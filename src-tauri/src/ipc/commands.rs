use std::sync::Arc;
use crate::audio::player::{AudioPlayer, PlaybackStateInfo, TrackInfo};

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
