#![allow(clippy::needless_range_loop, clippy::manual_repeat_n)]
use rustfft::{num_complex::Complex, FftPlanner};

/// Partitioned FFT-based convolution engine.
///
/// Partitions the impulse response into blocks and performs overlap-add
/// FFT convolution for each input block. This bounds latency to the
/// partition size and uses O(N·log(N)) per partition instead of O(N·M)
/// naive time-domain convolution.
pub struct PartitionedConvolver {
    partition_size: usize,
    fft_size: usize,
    num_partitions: usize,
    is_first_block: bool,
    // Pre-computed FFT of each IR partition
    ir_spectra: Vec<Vec<Complex<f32>>>,
    // Input delay line (frequency domain) — circular buffer of past input spectra
    input_spectra: Vec<Vec<Complex<f32>>>,
    input_write_pos: usize,
    // Time-domain input accumulation buffer
    input_buffer: Vec<f32>,
    buffer_pos: usize,
    // Overlap-add tail from previous block
    overlap: Vec<f32>,
    // Output buffer for current block
    output_buffer: Vec<f32>,
    output_pos: usize,
}

impl PartitionedConvolver {
    /// Create a new partitioned convolver for the given IR.
    ///
    /// `partition_size` should be a power of 2 (e.g., 512).
    pub fn new(ir: &[f32], partition_size: usize) -> Self {
        let partition_size = partition_size.next_power_of_two();
        let fft_size = partition_size * 2;

        // Partition the IR into blocks
        let num_partitions = ir.len().div_ceil(partition_size);

        let mut planner = FftPlanner::new();
        let fft_fwd = planner.plan_fft_forward(fft_size);

        let mut ir_spectra = Vec::with_capacity(num_partitions);
        for p in 0..num_partitions {
            let start = p * partition_size;
            let end = (start + partition_size).min(ir.len());

            let mut padded: Vec<Complex<f32>> = Vec::with_capacity(fft_size);
            for i in 0..partition_size {
                let val = if start + i < end { ir[start + i] } else { 0.0 };
                padded.push(Complex::new(val, 0.0));
            }
            // Zero-pad to fft_size
            padded.resize(fft_size, Complex::new(0.0, 0.0));
            fft_fwd.process(&mut padded);
            ir_spectra.push(padded);
        }

        let input_spectra = vec![vec![Complex::new(0.0, 0.0); fft_size]; num_partitions];

        Self {
            partition_size,
            fft_size,
            num_partitions,
            is_first_block: true,
            ir_spectra,
            input_spectra,
            input_write_pos: 0,
            input_buffer: vec![0.0; partition_size],
            buffer_pos: 0,
            overlap: vec![0.0; partition_size],
            output_buffer: vec![0.0; partition_size],
            output_pos: partition_size, // Start with output exhausted to trigger first fill
        }
    }

    /// Process a single mono sample through the convolver.
    /// Returns the convolved output sample.
    pub fn process_sample(&mut self, input: f32) -> f32 {
        self.input_buffer[self.buffer_pos] = input;
        self.buffer_pos += 1;

        let output = if self.output_pos < self.partition_size {
            let val = self.output_buffer[self.output_pos];
            self.output_pos += 1;
            val
        } else {
            0.0
        };

        if self.buffer_pos >= self.partition_size {
            self.process_block();
            self.buffer_pos = 0;
            self.output_pos = 0;
        }

        output
    }

    fn process_block(&mut self) {
        let mut planner = FftPlanner::new();
        let fft_fwd = planner.plan_fft_forward(self.fft_size);
        let fft_inv = planner.plan_fft_inverse(self.fft_size);

        // Forward FFT of current input block (zero-padded)
        let mut input_freq: Vec<Complex<f32>> = self
            .input_buffer
            .iter()
            .map(|&s| Complex::new(s, 0.0))
            .chain(std::iter::repeat(Complex::new(0.0, 0.0)).take(self.partition_size))
            .collect();
        fft_fwd.process(&mut input_freq);

        // Store in circular buffer
        self.input_spectra[self.input_write_pos] = input_freq;

        // Accumulate frequency-domain products across all partitions
        let mut accum = vec![Complex::new(0.0f32, 0.0); self.fft_size];

        for p in 0..self.num_partitions {
            // Index into input_spectra circular buffer
            let input_idx = (self.input_write_pos + self.num_partitions - p) % self.num_partitions;

            for k in 0..self.fft_size {
                accum[k] += self.input_spectra[input_idx][k] * self.ir_spectra[p][k];
            }
        }

        // Inverse FFT
        fft_inv.process(&mut accum);
        let scale = 1.0 / self.fft_size as f32;

        // Overlap-add: first half is output, second half is new overlap
        for i in 0..self.partition_size {
            self.output_buffer[i] = accum[i].re * scale + self.overlap[i];
        }
        for i in 0..self.partition_size {
            self.overlap[i] = accum[self.partition_size + i].re * scale;
        }

        // Fade in the first output block to prevent transient click on new convolver
        if self.is_first_block {
            for i in 0..self.partition_size {
                let fade = i as f32 / self.partition_size as f32;
                self.output_buffer[i] *= fade;
            }
            self.is_first_block = false;
        }

        // Advance write position
        self.input_write_pos = (self.input_write_pos + 1) % self.num_partitions;
    }

    /// Get the partition size (latency in samples).
    pub fn partition_size(&self) -> usize {
        self.partition_size
    }

    /// Get the number of IR partitions.
    pub fn num_partitions(&self) -> usize {
        self.num_partitions
    }
}
