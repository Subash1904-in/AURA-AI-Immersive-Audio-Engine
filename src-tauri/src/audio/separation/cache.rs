use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub hash: String,
    pub original_path: String,
    pub last_accessed: u64,
    pub sample_rate: u32,
    pub channels: usize,
    pub duration_ms: u64,
    pub total_size_bytes: u64,
}

pub fn get_cache_root() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".aura").join("cache")
}

pub fn get_track_cache_dir(hash: &str) -> PathBuf {
    get_cache_root().join(hash)
}

/// A lightweight WAV file writer to output 16-bit PCM WAV stems.
pub struct WavWriter {
    file: File,
    data_size: u32,
    sample_rate: u32,
    channels: u16,
}

impl WavWriter {
    pub fn create<P: AsRef<Path>>(
        path: P,
        sample_rate: u32,
        channels: u16,
    ) -> std::io::Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        // Write 44-byte WAV header placeholder
        let header = [0u8; 44];
        file.write_all(&header)?;
        Ok(Self {
            file,
            data_size: 0,
            sample_rate,
            channels,
        })
    }

    pub fn write_samples(&mut self, samples: &[f32]) -> std::io::Result<()> {
        for &s in samples {
            let clamped = s.clamp(-1.0, 1.0);
            let val = (clamped * 32767.0) as i16;
            self.file.write_all(&val.to_le_bytes())?;
            self.data_size += 2;
        }
        Ok(())
    }

    pub fn finalize(&mut self) -> std::io::Result<()> {
        self.file.seek(SeekFrom::Start(0))?;

        let num_channels = self.channels;
        let sample_rate = self.sample_rate;
        let bits_per_sample = 16u16;
        let byte_rate = sample_rate * num_channels as u32 * (bits_per_sample as u32 / 8);
        let block_align = num_channels * (bits_per_sample / 8);
        let chunk_size = 36 + self.data_size;

        let mut header = [0u8; 44];
        header[0..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&chunk_size.to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16u32.to_le_bytes());
        header[20..22].copy_from_slice(&1u16.to_le_bytes());
        header[22..24].copy_from_slice(&num_channels.to_le_bytes());
        header[24..28].copy_from_slice(&sample_rate.to_le_bytes());
        header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        header[32..34].copy_from_slice(&block_align.to_le_bytes());
        header[34..36].copy_from_slice(&bits_per_sample.to_le_bytes());
        header[36..40].copy_from_slice(b"data");
        header[40..44].copy_from_slice(&self.data_size.to_le_bytes());

        self.file.write_all(&header)?;
        Ok(())
    }
}

/// Load metadata for a cached track. Updates `last_accessed`.
pub fn load_metadata(hash: &str) -> Option<CacheMetadata> {
    let dir = get_track_cache_dir(hash);
    let meta_path = dir.join("meta.json");
    if !meta_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&meta_path).ok()?;
    let mut meta: CacheMetadata = serde_json::from_str(&content).ok()?;

    // Update last accessed timestamp
    meta.last_accessed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Ok(updated_content) = serde_json::to_string(&meta) {
        let _ = fs::write(&meta_path, updated_content);
    }

    Some(meta)
}

/// Save metadata for a cached track.
pub fn save_metadata(hash: &str, mut meta: CacheMetadata) -> std::io::Result<()> {
    let dir = get_track_cache_dir(hash);
    fs::create_dir_all(&dir)?;

    meta.last_accessed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let meta_path = dir.join("meta.json");
    let content = serde_json::to_string_pretty(&meta)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(meta_path, content)?;

    Ok(())
}

/// Check cache size and evict oldest (LRU) entries if over limit.
pub fn enforce_cache_limit(max_size_bytes: u64) -> std::io::Result<()> {
    let root = get_cache_root();
    if !root.exists() {
        return Ok(());
    }

    let mut entries = Vec::new();
    let mut total_size = 0u64;

    if let Ok(read_dir) = fs::read_dir(&root) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let meta_path = path.join("meta.json");
                if meta_path.exists() {
                    if let Ok(content) = fs::read_to_string(&meta_path) {
                        if let Ok(meta) = serde_json::from_str::<CacheMetadata>(&content) {
                            // Calculate directory size
                            let mut dir_size = 0u64;
                            if let Ok(files) = fs::read_dir(&path) {
                                for file_entry in files.flatten() {
                                    if let Ok(metadata) = file_entry.metadata() {
                                        dir_size += metadata.len();
                                    }
                                }
                            }
                            entries.push((path.clone(), meta.last_accessed, dir_size));
                            total_size += dir_size;
                        }
                    }
                }
            }
        }
    }

    // Sort entries by last_accessed ascending (oldest first)
    entries.sort_by_key(|&(_, last_accessed, _)| last_accessed);

    for (path, _, dir_size) in entries {
        if total_size <= max_size_bytes {
            break;
        }
        if let Err(e) = fs::remove_dir_all(&path) {
            eprintln!(
                "[AURA Cache] Failed to evict cache entry at {:?}: {}",
                path, e
            );
        } else {
            total_size = total_size.saturating_sub(dir_size);
            eprintln!("[AURA Cache] Evicted cached track at {:?}", path);
        }
    }

    Ok(())
}
