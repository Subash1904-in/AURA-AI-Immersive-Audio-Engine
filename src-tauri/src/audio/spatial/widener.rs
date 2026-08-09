/// Mid-side stereo widener.
///
/// Decodes stereo L/R into mid/side representation, scales the side
/// channel by a width factor, and encodes back to L/R.
#[derive(Debug, Clone)]
pub struct StereoWidener {
    width: f32,
}

impl StereoWidener {
    pub fn new(width: f32) -> Self {
        Self {
            width: width.clamp(0.0, 2.0),
        }
    }

    pub fn update_width(&mut self, width: f32) {
        self.width = width.clamp(0.0, 2.0);
    }

    /// Process interleaved stereo samples in-place.
    /// Assumes `samples.len()` is even (L, R, L, R, ...).
    pub fn process_interleaved(&self, samples: &mut [f32]) {
        let num_frames = samples.len() / 2;
        for i in 0..num_frames {
            let l = samples[i * 2];
            let r = samples[i * 2 + 1];

            let mid = (l + r) * 0.5;
            let side = (l - r) * 0.5;

            let widened_side = side * self.width;

            samples[i * 2] = mid + widened_side;
            samples[i * 2 + 1] = mid - widened_side;
        }
    }
}
