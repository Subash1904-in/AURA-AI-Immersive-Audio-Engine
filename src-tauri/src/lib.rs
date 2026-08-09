pub mod audio;
pub mod ipc;

use audio::player::AudioPlayer;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let player = Arc::new(AudioPlayer::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(player)
        .invoke_handler(tauri::generate_handler![
            ipc::commands::load_file,
            ipc::commands::play,
            ipc::commands::pause,
            ipc::commands::seek,
            ipc::commands::get_position,
            ipc::commands::get_dsp_params,
            ipc::commands::set_dsp_params,
            ipc::commands::toggle_dsp_stage,
            ipc::commands::set_eq_band,
            ipc::commands::set_reverb_environment,
            ipc::commands::set_spatial_width,
            ipc::commands::set_reverb_mix,
            ipc::commands::toggle_crossfeed,
            ipc::commands::toggle_hrtf,
            ipc::commands::toggle_auto_mode,
            ipc::commands::toggle_beat_modulation,
            ipc::commands::get_analysis_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AURA application");
}
