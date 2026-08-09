# Manual Verification Guide — Spatial Audio & Reverb

This document describes optional manual listening tests for AURA's Phase 2 spatial audio and reverb features.

> **Note**: These tests are NOT required for phase completion. Automated CI tests verify correctness. This guide is for subjective quality evaluation.

---

## Prerequisites

1. Download a debug build from the CI `Build Tauri debug binary` step, or from a `build.yml` workflow artifact if available.
2. Use **headphones** (essential for spatial audio and crossfeed evaluation).
3. Prepare test audio files:
   - A well-mixed stereo track (e.g., any pop/rock song)
   - A hard-panned stereo track (e.g., early Beatles, some jazz recordings)
   - A mono recording (e.g., spoken word, podcast)

---

## Test Procedures

### 1. Stereo Width
1. Load a stereo track and start playback.
2. Enable the **Spatial Audio** stage.
3. Sweep the **Stereo Width** slider from 0.0 (mono) to 2.0 (wide).
4. **Expected**: At 0.0, sound collapses to center. At 1.0, original mix is preserved. At 2.0, stereo image widens noticeably. No clicks or pops during adjustment.

### 2. Crossfeed
1. Load a hard-panned stereo track (e.g., instrument fully in one ear).
2. Enable **Spatial Audio** and increase crossfeed to ~50%.
3. **Expected**: The hard-panned instrument should feel less "stuck" to one ear. The mix should feel more natural and speaker-like without losing stereo separation entirely.

### 3. HRTF Binaural
1. Load any stereo track.
2. Enable **Spatial Audio** and toggle on **HRTF Binaural**.
3. **Expected**: The sound stage should feel externalized — as if coming from speakers in front of you, rather than inside your head. Some tonal coloration is normal (this is the HRTF filtering).

### 4. Reverb Environments
1. Load a dry recording (spoken word works well).
2. Enable **Convolution Reverb** and select each environment in order:
   - **Small Room**: Subtle, short reverb tail. Should add "presence" without muddying speech.
   - **Concert Hall**: Longer, richer tail. Should sound spacious.
   - **Cathedral**: Long, diffuse reverb. Should feel very large and echoey.
   - **Cave**: Longest tail, darker tone. Should feel cavernous.
3. **Expected**: Each switch should be smooth (crossfaded). No clicks, pops, or abrupt cuts. The wet/dry mix slider should smoothly blend between dry and reverberant signal.

### 5. Full Chain
1. Enable all DSP stages (EQ, Bass, Compressor, Loudness, Spatial, Reverb, Limiter).
2. Adjust various parameters while playing.
3. **Expected**: No audible artifacts, clicks, or CPU dropouts. Audio should remain clean and real-time.

---

## What to Listen For (Quality Checklist)

| Feature         | Good Sign                                    | Bad Sign                                     |
|----------------|----------------------------------------------|----------------------------------------------|
| Stereo Width    | Smooth mono↔wide transition                  | Clicks, phase artifacts, channel imbalance   |
| Crossfeed       | Natural speaker-like imaging on headphones   | Mono collapse, hollow/phasey sound           |
| HRTF            | Externalized sound stage                     | Extreme coloration, metallic artifacts       |
| Reverb          | Natural, smooth environment transitions      | Clicks on switch, metallic ringing, mud      |
| Wet/Dry Mix     | Clean blend from dry to fully wet            | Abrupt volume changes, phase cancellation    |
| Full Chain      | Clean, real-time processing                  | Dropouts, stuttering, latency spikes         |
