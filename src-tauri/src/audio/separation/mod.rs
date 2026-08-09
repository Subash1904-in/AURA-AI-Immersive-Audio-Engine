pub mod cache;
pub mod model;

#[cfg(test)]
mod separation_tests;

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use tauri::Emitter;

use crate::audio::player::AudioPlayer;
use cache::{
    enforce_cache_limit, get_track_cache_dir, load_metadata, save_metadata, CacheMetadata,
};
use model::{get_file_hash, SeparationModel};

#[derive(Debug, Clone, Serialize)]
pub struct SeparationProgressPayload {
    pub job_id: String,
    pub hash: String,
    pub progress: f32,
    pub cache_hit: bool,
    pub status: String, // "processing", "completed", "error"
    pub message: String,
}

/// Main entry point to separate a track in a background thread.
/// Returns the track hash as the job ID.
pub fn separate_track(
    path: String,
    player: Arc<AudioPlayer>,
    app: Option<tauri::AppHandle>,
) -> Result<String, String> {
    let input_path = PathBuf::from(&path);
    if !input_path.exists() {
        return Err(format!("File does not exist: {}", path));
    }

    let hash = get_file_hash(&input_path);
    let job_id = hash.clone();
    let cache_dir = get_track_cache_dir(&hash);

    // Update DspParams to not ready by default, until we verify cache or separate
    {
        let mut params = player.get_dsp_params();
        params.stems_ready = false;
        player.set_dsp_params(params);
    }

    // Check if cache exists
    if let Some(mut meta) = load_metadata(&hash) {
        let vocals = cache_dir.join("vocals.wav");
        let drums = cache_dir.join("drums.wav");
        let bass = cache_dir.join("bass.wav");
        let other = cache_dir.join("other.wav");

        if vocals.exists() && drums.exists() && bass.exists() && other.exists() {
            // Cache Hit!
            eprintln!("[AURA Separation] Cache hit for track: {}", hash);

            // Mark stems as ready
            let mut params = player.get_dsp_params();
            params.stems_ready = true;
            player.set_dsp_params(params);

            // Emit completion event
            if let Some(ref a) = app {
                let payload = SeparationProgressPayload {
                    job_id: job_id.clone(),
                    hash: hash.clone(),
                    progress: 1.0,
                    cache_hit: true,
                    status: "completed".to_string(),
                    message: "Loaded from cache".to_string(),
                };
                let _ = a.emit("separation-progress", payload);
            }
            return Ok(job_id);
        }
    }

    // Cache Miss: Spawning background thread for separation
    let app_clone = app.clone();
    let hash_clone = hash.clone();
    let job_id_clone = job_id.clone();
    let path_clone = path.clone();
    let cache_dir_clone = cache_dir.clone();

    // Emit initial status
    if let Some(ref a) = app {
        let payload = SeparationProgressPayload {
            job_id: job_id.clone(),
            hash: hash.clone(),
            progress: 0.0,
            cache_hit: false,
            status: "processing".to_string(),
            message: "Starting separation...".to_string(),
        };
        let _ = a.emit("separation-progress", payload);
    }

    thread::spawn(move || {
        let model = SeparationModel::new(None);

        let job_id_cb = job_id_clone.clone();
        let hash_cb = hash_clone.clone();
        let app_cb = app_clone.clone();

        let progress_cb = move |progress: f32| {
            if let Some(ref a) = app_cb {
                let payload = SeparationProgressPayload {
                    job_id: job_id_cb.clone(),
                    hash: hash_cb.clone(),
                    progress,
                    cache_hit: false,
                    status: "processing".to_string(),
                    message: format!("Processing stems: {:.0}%", progress * 100.0),
                };
                let _ = a.emit("separation-progress", payload);
            }
        };

        match model.separate(&path_clone, &cache_dir_clone, progress_cb) {
            Ok(_) => {
                // Calculate folder size to save metadata
                let mut total_size_bytes = 0u64;
                if let Ok(read_dir) = std::fs::read_dir(&cache_dir_clone) {
                    for entry in read_dir.flatten() {
                        if let Ok(meta) = entry.metadata() {
                            total_size_bytes += meta.len();
                        }
                    }
                }

                // Get details from input to cache
                let sample_rate = 44100; // default/fallback
                let channels = 2; // default/fallback

                let meta = CacheMetadata {
                    hash: hash_clone.clone(),
                    original_path: path_clone,
                    last_accessed: 0, // save_metadata updates this to now
                    sample_rate,
                    channels,
                    duration_ms: 0, // updated if needed, optional
                    total_size_bytes,
                };

                let _ = save_metadata(&hash_clone, meta);

                // Enforce 5GB limit (5 * 1024 * 1024 * 1024 = 5368709120)
                let max_size = 5 * 1024 * 1024 * 1024;
                let _ = enforce_cache_limit(max_size);

                // Mark stems as ready
                let mut params = player.get_dsp_params();
                params.stems_ready = true;
                player.set_dsp_params(params);

                // Emit completion event
                if let Some(ref a) = app_clone {
                    let payload = SeparationProgressPayload {
                        job_id: job_id_clone.clone(),
                        hash: hash_clone.clone(),
                        progress: 1.0,
                        cache_hit: false,
                        status: "completed".to_string(),
                        message: "Separation complete".to_string(),
                    };
                    let _ = a.emit("separation-progress", payload);
                }
                eprintln!(
                    "[AURA Separation] Successfully separated and cached track: {}",
                    hash_clone
                );
            }
            Err(e) => {
                eprintln!("[AURA Separation] Separation failed: {}", e);
                if let Some(ref a) = app_clone {
                    let payload = SeparationProgressPayload {
                        job_id: job_id_clone.clone(),
                        hash: hash_clone.clone(),
                        progress: 0.0,
                        cache_hit: false,
                        status: "error".to_string(),
                        message: format!("Error: {}", e),
                    };
                    let _ = a.emit("separation-progress", payload);
                }
            }
        }
    });

    Ok(job_id)
}
