#![allow(clippy::needless_range_loop)]

/// DSP-based BPM and beat onset detector.
///
/// Uses spectral energy flux and autocorrelation over a sliding window
/// to estimate tempo (BPM) and detect instantaneous beat onsets.
#[derive(Debug, Clone)]
pub struct BpmDetector {
    sample_rate: f32,
    // Buffer storing past energy envelope values for tempo estimation
    energy_history: Vec<f32>,
    history_capacity: usize,
    // Moving average of energy for beat onset thresholding
    moving_energy_avg: f32,
    // Current estimated BPM
    current_bpm: f32,
    // Beat envelope follower (0.0 to 1.0)
    beat_envelope: f32,
}

impl BpmDetector {
    pub fn new(sample_rate: f32) -> Self {
        // ~3 seconds of energy history at 100 blocks/sec (block size 441 samples = ~10ms)
        let history_capacity = 300;
        Self {
            sample_rate,
            energy_history: Vec::with_capacity(history_capacity),
            history_capacity,
            moving_energy_avg: 0.0,
            current_bpm: 120.0,
            beat_envelope: 0.0,
        }
    }

    /// Process a block of mono or interleaved audio samples.
    /// Returns `(estimated_bpm, is_beat, beat_boost_envelope)`.
    pub fn process_block(&mut self, samples: &[f32]) -> (f32, bool, f32) {
        if samples.is_empty() {
            return (self.current_bpm, false, self.beat_envelope);
        }

        // Calculate root-mean-square (RMS) energy of current block
        let energy = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();

        // Update moving average (alpha ~ 0.05 for smooth background tracking)
        self.moving_energy_avg = 0.95 * self.moving_energy_avg + 0.05 * energy;

        // Onset threshold: energy jump exceeding 1.35x moving average and minimum absolute energy
        let is_onset = energy > self.moving_energy_avg * 1.35 && energy > 0.01;

        if is_onset {
            self.beat_envelope = 1.0; // Instantaneous attack on beat
        } else {
            self.beat_envelope *= 0.85; // Exponential decay (~150ms tail)
        }

        // Store energy value in history buffer
        if self.energy_history.len() >= self.history_capacity {
            self.energy_history.remove(0);
        }
        self.energy_history.push(energy);

        // Periodically re-estimate BPM when history buffer has sufficient samples
        if self.energy_history.len() >= 100 {
            self.estimate_bpm_from_history();
        }

        (self.current_bpm, is_onset, self.beat_envelope)
    }

    /// Estimate tempo (BPM) from energy history via autocorrelation peak detection.
    fn estimate_bpm_from_history(&mut self) {
        let n = self.energy_history.len();
        if n < 50 {
            return;
        }

        // Search lag range corresponding to 60 BPM to 180 BPM
        // At ~100 blocks/sec (441 samples per block @ 44.1 kHz):
        // 60 BPM = 1 beat/sec = lag 100 blocks
        // 180 BPM = 3 beats/sec = lag 33 blocks
        let min_lag = 25; // ~240 BPM
        let max_lag = 120; // ~50 BPM

        let mut max_corr = 0.0f32;
        let mut best_lag = 50;

        for lag in min_lag..=max_lag.min(n / 2) {
            let mut corr = 0.0f32;
            let mut count = 0;
            for i in 0..(n - lag) {
                corr += self.energy_history[i] * self.energy_history[i + lag];
                count += 1;
            }
            if count > 0 {
                corr /= count as f32;
            }

            if corr > max_corr {
                max_corr = corr;
                best_lag = lag;
            }
        }

        // Convert best lag in blocks back to BPM
        // Block time dt = 441.0 / sample_rate seconds (~0.01s)
        let block_time_sec = 441.0 / self.sample_rate;
        let period_sec = best_lag as f32 * block_time_sec;

        if period_sec > 0.1 {
            let bpm = 60.0 / period_sec;
            // Smoothly update current estimated BPM (alpha ~ 0.2)
            self.current_bpm = 0.8 * self.current_bpm + 0.2 * bpm.clamp(60.0, 180.0);
        }
    }

    /// Direct static helper to estimate BPM from a complete audio buffer (used by unit tests).
    pub fn estimate_bpm_from_buffer(samples: &[f32], sample_rate: f32) -> f32 {
        let block_size = 441; // 10ms at 44.1 kHz
        let mut detector = Self::new(sample_rate);

        for chunk in samples.chunks(block_size) {
            detector.process_block(chunk);
        }

        detector.current_bpm
    }
}
