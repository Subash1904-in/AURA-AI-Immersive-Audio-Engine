#![allow(clippy::field_reassign_with_default)]
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use aura_lib::audio::dsp::params::{ReverbEnvironment, ReverbParams};
use aura_lib::audio::reverb::ConvolutionReverb;

fn generate_white_noise_stereo(num_frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    let mut seed: u32 = 98765;
    for _ in 0..(num_frames * 2) {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let float_val = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
        buffer.push(float_val);
    }
    buffer
}

fn bench_convolution_reverb(c: &mut Criterion) {
    let sample_rate = 48000.0;
    let channels = 2;
    let block_size = 1024;

    let environments = [
        ("SmallRoom", ReverbEnvironment::SmallRoom),
        ("ConcertHall", ReverbEnvironment::ConcertHall),
        ("Cathedral", ReverbEnvironment::Cathedral),
        ("Cave", ReverbEnvironment::Cave),
    ];

    let mut group = c.benchmark_group("convolution_reverb_block_processing");

    for (name, env) in &environments {
        let input_samples = generate_white_noise_stereo(block_size);
        let params = ReverbParams {
            environment: *env,
            wet_dry_mix: 0.5,
        };

        group.bench_with_input(BenchmarkId::new("reverb_stereo", name), name, |b, _| {
            let mut reverb = ConvolutionReverb::new(sample_rate, channels, &params);
            let mut test_buffer = input_samples.clone();
            b.iter(|| {
                test_buffer.copy_from_slice(&input_samples);
                reverb.process_interleaved(black_box(&mut test_buffer));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_convolution_reverb);
criterion_main!(benches);
