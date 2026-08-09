use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::{Time, TimeBase};

pub struct AudioDecoder {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: usize,
    total_frames: Option<u64>,
    time_base: Option<TimeBase>,
    duration_ms: u64,
}

impl AudioDecoder {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path.as_ref())
            .map_err(|e| format!("Failed to open file {:?}: {}", path.as_ref(), e))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.as_ref().extension() {
            hint.with_extension(&ext.to_string_lossy());
        }

        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let metadata_opts: MetadataOptions = Default::default();
        let decoder_opts: DecoderOptions = Default::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| format!("Unsupported format or corrupt file: {}", e))?;

        let format_reader = probed.format;

        let track = format_reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| "No valid audio track found in file".to_string())?;

        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
        let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

        let total_frames = track.codec_params.n_frames;
        let time_base = track.codec_params.time_base;

        let duration_ms = if let Some(n_frames) = total_frames {
            if sample_rate > 0 {
                (n_frames * 1000) / sample_rate as u64
            } else {
                0
            }
        } else if let Some(tb) = time_base {
            if let Some(n_frames) = total_frames {
                let time = tb.calc_time(n_frames);
                (time.seconds * 1000) + (time.frac * 1000.0) as u64
            } else {
                0
            }
        } else {
            0
        };

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &decoder_opts)
            .map_err(|e| format!("Failed to create audio decoder: {}", e))?;

        Ok(Self {
            format_reader,
            decoder,
            track_id,
            sample_rate,
            channels,
            total_frames,
            time_base,
            duration_ms,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    pub fn total_frames(&self) -> Option<u64> {
        self.total_frames
    }

    /// Read next audio packet and decode into interleaved f32 samples.
    /// Returns Ok(None) at end of stream.
    pub fn next_samples(&mut self) -> Result<Option<Vec<f32>>, String> {
        loop {
            let packet = match self.format_reader.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(ref err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None);
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(err) => return Err(format!("Error reading packet: {}", err)),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(audio_buf) => {
                    let capacity = audio_buf.capacity();
                    let spec = *audio_buf.spec();
                    let mut sample_buf = SampleBuffer::<f32>::new(capacity as u64, spec);
                    sample_buf.copy_interleaved_ref(audio_buf);
                    return Ok(Some(sample_buf.samples().to_vec()));
                }
                Err(SymphoniaError::DecodeError(_)) => {
                    // Ignore non-fatal decode error in frame and continue
                    continue;
                }
                Err(err) => return Err(format!("Decoding error: {}", err)),
            }
        }
    }

    /// Seek to specified target timestamp in milliseconds.
    pub fn seek(&mut self, target_ms: u64) -> Result<u64, String> {
        let sec = target_ms / 1000;
        let frac = (target_ms % 1000) as f64 / 1000.0;

        let seek_to = SeekTo::Time {
            time: Time::new(sec, frac),
            track_id: Some(self.track_id),
        };

        let seeked_to = self
            .format_reader
            .seek(SeekMode::Accurate, seek_to)
            .map_err(|e| format!("Seek failed: {}", e))?;

        self.decoder.reset();

        let actual_ms = if let Some(tb) = self.time_base {
            let time = tb.calc_time(seeked_to.actual_ts);
            (time.seconds * 1000) + (time.frac * 1000.0) as u64
        } else if self.sample_rate > 0 {
            (seeked_to.actual_ts * 1000) / self.sample_rate as u64
        } else {
            target_ms
        };

        Ok(actual_ms)
    }
}
