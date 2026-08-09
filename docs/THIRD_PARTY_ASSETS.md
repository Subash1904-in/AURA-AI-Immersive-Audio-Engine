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

### Recommended Real-World Replacements
For production-quality reverb, replace synthetic IRs with recorded impulse responses:

- **OpenAIR (Open Acoustic Impulse Response Library)**
  - URL: https://www.openair.hosted.york.ac.uk/
  - License: CC BY 4.0 (most IRs)
  - Description: High-quality measured IRs from real acoustic spaces worldwide.

- **EchoThief**
  - URL: http://www.echothief.com/
  - License: Free for any use
  - Description: Collection of impulse responses captured in unusual acoustic spaces.

- **Voxengo Free Reverb Impulse Responses**
  - URL: https://www.voxengo.com/free/impulse-response-library/
  - License: Free for any use

---

## FFT Library

- **RustFFT** (version 6.2)
  - URL: https://crates.io/crates/rustfft
  - License: MIT / Apache-2.0
  - Usage: Partitioned FFT convolution for HRTF rendering and convolution reverb.

---

## Convolution Engine

AURA uses a partitioned overlap-add FFT convolution engine:

- **Partition size**: 512 samples (default), power-of-2
- **FFT size**: 2× partition size (1024)
- **Algorithm**: Standard overlap-add with frequency-domain accumulation across IR partitions
- **Latency**: Equal to partition size (~11.6 ms at 44.1 kHz with 512-sample partitions)
- **CPU cost**: O(P × N × log(N)) per block, where P = number of IR partitions, N = FFT size
