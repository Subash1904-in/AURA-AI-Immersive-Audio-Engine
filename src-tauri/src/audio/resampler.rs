/// Resamples and remaps interleaved multi-channel PCM audio from
/// (src_sample_rate, src_channels) to (target_sample_rate, target_channels).
pub struct AudioResampler {
    src_rate: f32,
    target_rate: f32,
    src_channels: usize,
    target_channels: usize,
    phase: f64,
}

impl AudioResampler {
    pub fn new(
        src_rate: u32,
        target_rate: u32,
        src_channels: usize,
        target_channels: usize,
    ) -> Self {
        Self {
            src_rate: src_rate as f32,
            target_rate: target_rate as f32,
            src_channels: src_channels.max(1),
            target_channels: target_channels.max(1),
            phase: 0.0,
        }
    }

    /// Process input interleaved buffer into resampled & channel-matched output buffer.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        if input.is_empty() {
            return Vec::new();
        }

        let num_src_frames = input.len() / self.src_channels;
        if num_src_frames == 0 {
            return Vec::new();
        }

        // 1. Channel Remapping / Mixing
        let channel_matched_input: Vec<f32> = if self.src_channels == self.target_channels {
            input.to_vec()
        } else {
            let mut out = Vec::with_capacity(num_src_frames * self.target_channels);
            for frame_idx in 0..num_src_frames {
                let frame_start = frame_idx * self.src_channels;
                if self.src_channels == 1 && self.target_channels == 2 {
                    let mono = input[frame_start];
                    out.push(mono);
                    out.push(mono);
                } else if self.src_channels == 2 && self.target_channels == 1 {
                    let left = input[frame_start];
                    let right = input[frame_start + 1];
                    out.push(0.5 * (left + right));
                } else {
                    for c in 0..self.target_channels {
                        let sample = if c < self.src_channels {
                            input[frame_start + c]
                        } else {
                            input[frame_start]
                        };
                        out.push(sample);
                    }
                }
            }
            out
        };

        // 2. Sample Rate Resampling (if rates differ)
        let rate_diff = (self.src_rate - self.target_rate).abs();
        let resampled: Vec<f32> = if rate_diff < 1.0 {
            channel_matched_input
        } else {
            let ratio = self.src_rate as f64 / self.target_rate as f64;
            let channels = self.target_channels;
            let total_frames = channel_matched_input.len() / channels;

            let estimated_out_frames = ((total_frames as f64) / ratio).ceil() as usize;
            let mut out = Vec::with_capacity(estimated_out_frames * channels);

            let mut current_phase = self.phase;

            while current_phase < (total_frames as f64) {
                let idx0 = current_phase.floor() as usize;
                let idx1 = (idx0 + 1).min(total_frames - 1);
                let frac = (current_phase - idx0 as f64) as f32;

                let frame0_offset = idx0 * channels;
                let frame1_offset = idx1 * channels;

                for c in 0..channels {
                    let sample0 = channel_matched_input[frame0_offset + c];
                    let sample1 = channel_matched_input[frame1_offset + c];
                    let interpolated = sample0 + frac * (sample1 - sample0);
                    out.push(interpolated);
                }

                current_phase += ratio;
            }

            self.phase = current_phase - (total_frames as f64);
            out
        };

        // 3. Soft Clipping Protection to eliminate harsh DAC overdrive/clipping
        resampled
            .into_iter()
            .map(|s| {
                if s > 0.95 {
                    0.95 + (1.0 - 0.95) * ((s - 0.95) / (1.0 - 0.95)).tanh()
                } else if s < -0.95 {
                    -0.95 + (-1.0 - (-0.95)) * ((s - (-0.95)) / (-1.0 - (-0.95))).tanh()
                } else {
                    s
                }
            })
            .collect()
    }
}
