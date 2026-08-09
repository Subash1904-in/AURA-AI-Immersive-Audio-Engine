# Third-Party Assets & Licenses

This document tracks all third-party assets, algorithms, and data sources used in AURA.

## Synthetic HRIR (Head-Related Impulse Response)

**Status**: Generated at runtime (no external binary assets required)

### Algorithm
AURA generates synthetic HRIR pairs for 5 virtual speaker positions (0°, ±90°, ±135°) using a physically-inspired model:

- **ITD (Interaural Time Difference)**: Modeled as a fractional sample delay proportional to `sin(azimuth)`, with a maximum of ~0.65 ms at 90° (based on average adult head radius of ~8.75 cm and speed of sound 343 m/s).
- **ILD (Interaural Level Difference)**: Frequency-dependent attenuation on the shadowed ear, increasing with frequency to simulate the head shadow effect. At 90° azimuth, the far ear receives approximately −6 dB attenuation at high frequencies.
- **Pinnae filtering**: Subtle early reflections in the near-ear HRIR simulate outer ear (pinna) interactions.
- **Head shadow low-pass**: The far-ear HRIR is low-pass filtered with a cutoff that varies with azimuth (3–8 kHz range) to model high-frequency shadowing.

### HRIR Length
128 samples per ear at any sample rate (typically 44.1 kHz or 48 kHz).

### Recommended Real-World Replacement
For production-quality binaural audio, replace synthetic HRIRs with measured datasets:

- **MIT KEMAR HRTF Dataset**
  - URL: https://sound.media.mit.edu/resources/KEMAR.html
  - License: Public domain / MIT open access
  - Description: Measured using a KEMAR mannequin with Realistic pinnae at 710 positions in the horizontal and median planes.
  - Format: 128-tap stereo WAV files per position

- **CIPIC HRTF Database**
  - URL: https://www.ece.ucdavis.edu/cipic/spatial-sound/hrtf-data/
  - License: Academic / free for non-commercial use

---

## Synthetic Impulse Responses (Reverb)

**Status**: Generated at runtime (no external binary assets required)

### Algorithm
AURA generates impulse responses for 4 reverb environments using a deterministic algorithm:

1. **Exponentially decaying noise**: White noise with amplitude envelope `exp(-6.908 * t / RT60)`, ensuring −60 dB at the specified reverberation time.
2. **Early reflections**: 6 sparse impulses in the first 50 ms (at 0, 5, 12, 21, 35, 48 ms) simulating first-order room reflections.
3. **Frequency shaping**: 1-pole low-pass filter (air absorption simulation) followed by 1-pole high-pass filter (room mode removal).
4. **Normalization**: Peak-normalized to 1.0.
5. **Deterministic PRNG**: xorshift32 with per-environment seed for reproducible output across platforms and CI runs.

### Environment Parameters

| Environment  | RT60 (s) | LP Cutoff (Hz) | HP Cutoff (Hz) | Seed | Approx. Length (44.1 kHz) |
|-------------|----------|----------------|----------------|------|---------------------------|
| Small Room   | 0.3      | 2000           | 200            | 42   | ~13,230 samples           |
| Concert Hall | 1.8      | 4000           | 100            | 137  | ~79,380 samples           |
| Cathedral    | 4.0      | 3000           | 80             | 293  | ~176,400 samples          |
| Cave         | 6.0      | 2500           | 150            | 571  | ~264,600 samples          |

---

## AI Analysis & Heuristic Classifier

### Architecture
AURA uses a dual-engine classification strategy:
1. **ONNX Model Runner**: Uses the `ort` (ONNX Runtime) Rust crate with `load-dynamic` feature.
2. **Heuristic Classifier (Graceful Fallback)**: Built-in deterministic decision tree evaluating extracted spectral features when the ONNX model is missing or fails to load.

### Extracted Features
- **BPM / Beat Onsets**: Energy flux autocorrelation over a 3-second sliding window.
- **Spectral Centroid**: Weighted average frequency measuring brightness ($f_{centroid} = \frac{\sum f \cdot |X(f)|}{\sum |X(f)|}$).
- **Spectral Flatness**: Ratio of geometric mean to arithmetic mean ($\frac{\exp(\frac{1}{N}\sum \ln|X|)}{\frac{1}{N}\sum |X|}$).
- **Band Energy Ratios**: Sub-bass (<250 Hz), Mid (250–4000 Hz), High (>4000 Hz).
- **Zero-Crossing Rate (ZCR)**: Percussiveness / noise density.

### Heuristic Classifier Rules & Presets

| Genre     | Feature Condition                                              | Target DSP Preset Settings                                              |
|-----------|----------------------------------------------------------------|-------------------------------------------------------------------------|
| **Rock**  | `energy_mid > 0.38` & `centroid > 2400Hz` & `zcr > 0.07`       | Width 1.4, Bass Drive 2.0, EQ +3dB @ 3.5kHz                             |
| **EDM**   | `energy_sub_bass > 0.40` & `BPM ≥ 115`                          | Width 1.5, Bass Drive 3.0, Mix 0.50, Compressor Ratio 6:1               |
| **Classical** | `flatness < 0.18` & `energy_high > 0.22` & `energy < 0.20` | Width 1.0, ConcertHall Reverb (Wet 0.35), EQ Flat                        |
| **Podcast**| `energy_mid > 0.60` & `energy_high < 0.20`                    | Width 0.0 (Mono), Target -16 LUFS, EQ Low-cut (-4dB @ 100Hz), Mid +2.5dB |
| **Lofi**  | `energy_sub_bass > 0.30` & `centroid < 1800Hz` & `BPM < 105`  | Width 1.2, Bass Drive 1.8, SmallRoom Reverb (Wet 0.25), EQ High-cut (-3.5dB @ 8kHz) |
| **Pop**   | Default Fallback                                               | Balanced Default DSP parameters                                         |

### Recommended Open ONNX Models
- **MusicNN / Essentia ONNX Genre Classifier**
  - URL: https://essentia.upf.edu/models/
  - License: CC BY 4.0 / Apache 2.0
  - Description: Pretrained genre and mood (valence/arousal) classification models ONNX export.

---

## FFT Library

- **RustFFT** (version 6.2)
  - URL: https://crates.io/crates/rustfft
  - License: MIT / Apache-2.0
  - Usage: Partitioned FFT convolution and spectral analysis.
