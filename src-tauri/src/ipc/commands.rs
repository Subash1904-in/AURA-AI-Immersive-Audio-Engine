use crate::audio::dsp::params::{AnalysisStateInfo, DspParams};
use crate::audio::player::{AudioPlayer, PlaybackStateInfo, TrackInfo};
use std::sync::Arc;

#[tauri::command]
pub fn load_file(
    path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<TrackInfo, String> {
    let track = state.load_file(path.clone())?;
    let _ = crate::audio::separation::separate_track(path, state.inner().clone(), Some(app));
    Ok(track)
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

// --- Phase 2: Spatial Audio & Reverb Commands ---

#[tauri::command]
pub fn set_reverb_environment(
    env: String,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.set_reverb_environment(&env);
    Ok(())
}

#[tauri::command]
pub fn set_spatial_width(
    width: f32,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.set_spatial_width(width);
    Ok(())
}

#[tauri::command]
pub fn set_reverb_mix(mix: f32, state: tauri::State<'_, Arc<AudioPlayer>>) -> Result<(), String> {
    state.set_reverb_mix(mix);
    Ok(())
}

#[tauri::command]
pub fn toggle_crossfeed(
    enabled: bool,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.toggle_crossfeed(enabled);
    Ok(())
}

#[tauri::command]
pub fn toggle_hrtf(enabled: bool, state: tauri::State<'_, Arc<AudioPlayer>>) -> Result<(), String> {
    state.toggle_hrtf(enabled);
    Ok(())
}

// --- Phase 3: AI Analysis & Adaptive Presets Commands ---

#[tauri::command]
pub fn toggle_auto_mode(
    enabled: bool,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.toggle_auto_mode(enabled);
    Ok(())
}

#[tauri::command]
pub fn toggle_beat_modulation(
    enabled: bool,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.toggle_beat_modulation(enabled);
    Ok(())
}

#[tauri::command]
pub fn get_analysis_state(
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<AnalysisStateInfo, String> {
    Ok(state.get_analysis_state())
}

// --- Phase 4: Source Separation Commands ---

#[tauri::command]
pub fn separate_track(
    path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<String, String> {
    crate::audio::separation::separate_track(path, state.inner().clone(), Some(app))
}

#[tauri::command]
pub fn set_stem_gain(
    stem: String,
    gain: f32,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.set_stem_gain(&stem, gain);
    Ok(())
}

#[tauri::command]
pub fn set_stem_mute(
    stem: String,
    mute: bool,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.set_stem_mute(&stem, mute);
    Ok(())
}

#[tauri::command]
pub fn set_stems_active(
    active: bool,
    state: tauri::State<'_, Arc<AudioPlayer>>,
) -> Result<(), String> {
    state.set_stems_active(active);
    Ok(())
}
