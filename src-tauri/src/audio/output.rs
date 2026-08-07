use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::{Consumer, Split};
use ringbuf::HeapRb;

pub struct AudioOutput {
    _stream: cpal::Stream,
    pub producer: ringbuf::wrap::caching::CachingProducer<Arc<HeapRb<f32>>, true, false>,
    pub sample_rate: u32,
    pub channels: usize,
    pub is_playing: Arc<AtomicBool>,
    pub played_samples: Arc<AtomicU64>,
}

impl AudioOutput {
    pub fn new(capacity: usize) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No default audio output device found".to_string())?;

        let config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get default output config: {}", e))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;

        let rb = HeapRb::<f32>::new(capacity);
        let (producer, mut consumer) = rb.split();

        let is_playing = Arc::new(AtomicBool::new(false));
        let played_samples = Arc::new(AtomicU64::new(0));

        let is_playing_cb = Arc::clone(&is_playing);
        let played_samples_cb = Arc::clone(&played_samples);

        let err_fn = |err| eprintln!("Audio output stream error: {}", err);

        let sample_format = config.sample_format();
        let config_proto: cpal::StreamConfig = config.into();

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device
                .build_output_stream(
                    &config_proto,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        // CRITICAL: ZERO HEAP ALLOCATIONS / ZERO LOCKS IN AUDIO CALLBACK
                        // This block executes directly inside the OS high-priority audio render thread.
                        // No dynamic memory allocations (vec!, box, string), no locks, no syscalls.
                        if is_playing_cb.load(Ordering::Relaxed) {
                            let read_count = consumer.pop_slice(data);
                            if read_count < data.len() {
                                // Zero-fill remaining buffer on underflow
                                data[read_count..].fill(0.0);
                            }
                            played_samples_cb.fetch_add(read_count as u64, Ordering::Relaxed);
                        } else {
                            data.fill(0.0);
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Failed to build F32 audio stream: {}", e))?,
            _ => {
                return Err(format!(
                    "Unsupported sample format: {:?}. Expected F32.",
                    sample_format
                ))
            }
        };

        stream
            .play()
            .map_err(|e| format!("Failed to start audio stream: {}", e))?;

        Ok(Self {
            _stream: stream,
            producer,
            sample_rate,
            channels,
            is_playing,
            played_samples,
        })
    }
}
