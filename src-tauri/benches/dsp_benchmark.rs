use arc_swap::ArcSwap;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;

use aura_lib::audio::dsp::chain::DspChain;
use aura_lib::audio::dsp::params::{DspParams, FilterType};

fn generate_white_noise_stereo(num_frames: usize) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    let mut seed: u32 = 54321;
    for _ in 0..(num_frames * 2) {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let float_val = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
        buffer.push(float_val);
    }
    buffer
}

fn bench_dsp_chain(c: &mut Criterion) {
    let sample_rate = 48000.0;
    let channels = 2;

    let mut full_params = DspParams::default();
    full_params.eq_enabled = true;
    full_params.eq.bands[0].gain_db = 3.0;
    full_params.eq.bands[2].gain_db = -4.0;
    full_params.bass_enabled = true;
    full_params.compressor_enabled = true;
    full_params.loudness_enabled = true;
    full_params.limiter_enabled = true;

    let params_bus = Arc::new(ArcSwap::from_pointee(full_params));

    let mut group = c.benchmark_group("dsp_chain_block_processing");

    for block_size in [512, 1024, 2048].iter() {
        let input_samples = generate_white_noise_stereo(*block_size);

        group.bench_with_input(
            BenchmarkId::new("full_chain_stereo", block_size),
            block_size,
            |b, _| {
                let mut dsp = DspChain::new(sample_rate, channels, params_bus.clone());
                let mut test_buffer = input_samples.clone();
                b.iter(|| {
                    test_buffer.copy_from_slice(&input_samples);
                    dsp.process_interleaved(black_box(&mut test_buffer));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_dsp_chain);
criterion_main!(benches);
