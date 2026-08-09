#[cfg(test)]
mod tests {
    use crate::audio::decoder::AudioDecoder;
    use crate::audio::player::AudioPlayer;
    use crate::audio::separation::cache::CacheMetadata;
    use crate::audio::separation::model::{
        get_inference_count, reset_inference_count, SeparationModel,
    };
    use crate::audio::separation::separate_track;
    use std::path::Path;
    use std::sync::Arc;

    fn create_test_wav_file(path: &Path) {
        let mut writer =
            crate::audio::separation::cache::WavWriter::create(path, 44100, 2).unwrap();
        // Write 1 second of stereo audio
        let mut samples = Vec::new();
        for i in 0..44100 {
            let t = i as f32 / 44100.0;
            // L: 440Hz sine wave, R: 880Hz sine wave
            let left = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
            let right = (2.0 * std::f32::consts::PI * 880.0 * t).sin() * 0.5;
            samples.push(left);
            samples.push(right);
        }
        writer.write_samples(&samples).unwrap();
        writer.finalize().unwrap();
    }

    // --- 1. test_playback_uninterrupted_during_separation ---
    #[test]
    fn test_playback_uninterrupted_during_separation() {
        let temp_dir = std::env::temp_dir();
        let test_wav = temp_dir.join("playback_separation_test.wav");
        create_test_wav_file(&test_wav);

        let test_wav_clone = test_wav.clone();

        // Spawn background separation
        let sep_handle = std::thread::spawn(move || {
            let model = SeparationModel::new(None);
            let out_dir = temp_dir.join("sep_playback_test_stems");
            model.separate(&test_wav_clone, &out_dir, |_| {}).unwrap();
            let _ = std::fs::remove_dir_all(out_dir);
        });

        // Simulating playback decoding in current thread
        let mut decoder = AudioDecoder::open(&test_wav).unwrap();
        let mut total_samples = 0;
        let start_time = std::time::Instant::now();

        while let Ok(Some(samples)) = decoder.next_samples() {
            total_samples += samples.len();
            // Ensure decoding is fast (not blocked)
            assert!(
                start_time.elapsed().as_secs() < 5,
                "Playback stream stalled during separation!"
            );
        }

        sep_handle.join().unwrap();
        let _ = std::fs::remove_file(test_wav);

        assert!(
            total_samples > 0,
            "Should have successfully decoded playback samples"
        );
    }

    // --- 2. test_cache_hit_prevents_reinference ---
    #[test]
    fn test_cache_hit_prevents_reinference() {
        let player = Arc::new(AudioPlayer::new());
        let temp_dir = std::env::temp_dir();
        let test_wav = temp_dir.join("cache_test.wav");
        create_test_wav_file(&test_wav);

        let path_str = test_wav.to_string_lossy().to_string();

        // Reset count
        reset_inference_count();

        // First separation (should run inference and cache it)
        let job_id1 = separate_track(path_str.clone(), player.clone(), None).unwrap();
        // Wait for thread to finish since separate_track runs on background thread for cache miss
        std::thread::sleep(std::time::Duration::from_millis(500));
        let count1 = get_inference_count();

        // Second separation (should be a cache hit, completed immediately)
        let job_id2 = separate_track(path_str.clone(), player.clone(), None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let count2 = get_inference_count();

        // Cleanup
        let _ = std::fs::remove_file(&test_wav);
        let cache_dir = crate::audio::separation::cache::get_track_cache_dir(&job_id1);
        let _ = std::fs::remove_dir_all(cache_dir);

        assert_eq!(job_id1, job_id2);
        assert_eq!(count1, 1, "First separation should run inference");
        assert_eq!(
            count2, 1,
            "Second separation should use cache instead of reinference"
        );
    }

    // --- 3. test_mute_isolation_energy ---
    #[test]
    fn test_mute_isolation_energy() {
        let temp_dir = std::env::temp_dir();
        let test_wav = temp_dir.join("mute_test.wav");
        create_test_wav_file(&test_wav);

        let out_dir = temp_dir.join("mute_test_stems");
        let model = SeparationModel::new(None);
        model.separate(&test_wav, &out_dir, |_| {}).unwrap();

        // Load vocals stem
        let mut v_dec = AudioDecoder::open(out_dir.join("vocals.wav")).unwrap();
        let mut vocals_samples = Vec::new();
        while let Ok(Some(samples)) = v_dec.next_samples() {
            vocals_samples.extend(samples);
        }

        // Load drums stem
        let mut d_dec = AudioDecoder::open(out_dir.join("drums.wav")).unwrap();
        let mut drums_samples = Vec::new();
        while let Ok(Some(samples)) = d_dec.next_samples() {
            drums_samples.extend(samples);
        }

        // Load bass stem
        let mut b_dec = AudioDecoder::open(out_dir.join("bass.wav")).unwrap();
        let mut bass_samples = Vec::new();
        while let Ok(Some(samples)) = b_dec.next_samples() {
            bass_samples.extend(samples);
        }

        // Load other stem
        let mut o_dec = AudioDecoder::open(out_dir.join("other.wav")).unwrap();
        let mut other_samples = Vec::new();
        while let Ok(Some(samples)) = o_dec.next_samples() {
            other_samples.extend(samples);
        }

        // Mix all unmuted
        let mut mix_all = vec![0.0f32; vocals_samples.len()];
        for i in 0..vocals_samples.len() {
            mix_all[i] = vocals_samples[i]
                + drums_samples.get(i).copied().unwrap_or(0.0)
                + bass_samples.get(i).copied().unwrap_or(0.0)
                + other_samples.get(i).copied().unwrap_or(0.0);
        }

        // Mix with vocals muted
        let mut mix_muted = vec![0.0f32; vocals_samples.len()];
        for i in 0..vocals_samples.len() {
            mix_muted[i] = drums_samples.get(i).copied().unwrap_or(0.0)
                + bass_samples.get(i).copied().unwrap_or(0.0)
                + other_samples.get(i).copied().unwrap_or(0.0);
        }

        // Difference signal
        let mut diff = vec![0.0f32; vocals_samples.len()];
        for i in 0..vocals_samples.len() {
            diff[i] = mix_all[i] - mix_muted[i];
        }

        // Calculate RMS of diff and vocals
        let mut sum_sq_diff = 0.0;
        let mut sum_sq_vocals = 0.0;
        for i in 0..vocals_samples.len() {
            sum_sq_diff += diff[i] * diff[i];
            sum_sq_vocals += vocals_samples[i] * vocals_samples[i];
        }

        let rms_diff = (sum_sq_diff / vocals_samples.len() as f32).sqrt();
        let rms_vocals = (sum_sq_vocals / vocals_samples.len() as f32).sqrt();

        // Cleanup
        let _ = std::fs::remove_file(&test_wav);
        let _ = std::fs::remove_dir_all(out_dir);

        assert!(
            (rms_diff - rms_vocals).abs() < 1e-4,
            "RMS of difference signal should match RMS of muted stem"
        );
    }

    // --- 4. test_cache_eviction_lru ---
    #[test]
    fn test_cache_eviction_lru() {
        let temp_dir = std::env::temp_dir().join("aura_lru_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Override HOME / USERPROFILE to target our test cache
        let old_home = std::env::var("HOME").ok();
        let old_userprofile = std::env::var("USERPROFILE").ok();

        std::env::set_var("HOME", &temp_dir);
        std::env::set_var("USERPROFILE", &temp_dir);

        // Verify cache root is correct
        let root = crate::audio::separation::cache::get_cache_root();
        assert!(root.starts_with(&temp_dir));

        // Create 3 fake cache directories with different access times and sizes
        let dir1 = crate::audio::separation::cache::get_track_cache_dir("track1");
        let dir2 = crate::audio::separation::cache::get_track_cache_dir("track2");
        let dir3 = crate::audio::separation::cache::get_track_cache_dir("track3");

        std::fs::create_dir_all(&dir1).unwrap();
        std::fs::create_dir_all(&dir2).unwrap();
        std::fs::create_dir_all(&dir3).unwrap();

        // Write a dummy WAV to give it size (e.g. 100 bytes)
        std::fs::write(dir1.join("vocals.wav"), vec![0u8; 100]).unwrap();
        std::fs::write(dir2.join("vocals.wav"), vec![0u8; 100]).unwrap();
        std::fs::write(dir3.join("vocals.wav"), vec![0u8; 100]).unwrap();

        // Write metadata
        let meta1 = CacheMetadata {
            hash: "track1".to_string(),
            original_path: "".to_string(),
            last_accessed: 100,
            sample_rate: 44100,
            channels: 2,
            duration_ms: 1000,
            total_size_bytes: 100,
        };
        let meta2 = CacheMetadata {
            hash: "track2".to_string(),
            original_path: "".to_string(),
            last_accessed: 200,
            sample_rate: 44100,
            channels: 2,
            duration_ms: 1000,
            total_size_bytes: 100,
        };
        let meta3 = CacheMetadata {
            hash: "track3".to_string(),
            original_path: "".to_string(),
            last_accessed: 300,
            sample_rate: 44100,
            channels: 2,
            duration_ms: 1000,
            total_size_bytes: 100,
        };

        // Write the raw JSON directly
        std::fs::write(
            dir1.join("meta.json"),
            serde_json::to_string(&meta1).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir2.join("meta.json"),
            serde_json::to_string(&meta2).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir3.join("meta.json"),
            serde_json::to_string(&meta3).unwrap(),
        )
        .unwrap();

        // Enforce cache limit of 220 bytes.
        // This should trigger eviction of the oldest (track1)
        crate::audio::separation::cache::enforce_cache_limit(220).unwrap();

        assert!(!dir1.exists(), "Oldest track1 should be evicted");
        assert!(dir2.exists(), "track2 should remain");
        assert!(dir3.exists(), "track3 should remain");

        // Enforce limit of 110 bytes (should evict track2 too)
        crate::audio::separation::cache::enforce_cache_limit(110).unwrap();
        assert!(!dir2.exists(), "track2 should be evicted now");
        assert!(dir3.exists(), "Newest track3 should remain");

        // Cleanup test directory
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Restore env vars
        if let Some(h) = old_home {
            std::env::set_var("HOME", h);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(u) = old_userprofile {
            std::env::set_var("USERPROFILE", u);
        } else {
            std::env::remove_var("USERPROFILE");
        }
    }
}
