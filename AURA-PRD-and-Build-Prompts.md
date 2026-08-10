# AURA — AI Immersive Audio Engine
**PRD + Phase-by-Phase AI Agent Build Prompts**
Working name: **AURA** (rename freely — code should not hardcode brand strings)

**Constraint: no local Visual Studio/MSVC, Xcode, or other native build toolchains.** All compiling, testing, and desktop-app bundling happens on GitHub Actions. Local machine is used only for editing files and commands that don't require a linker (`git`, `cargo fmt`, file edits).

---

## 1. Product Summary

Desktop-first app (Tauri: Rust + React/TS) that ingests any audio file/stream and applies real-time AI-driven enhancement: adaptive EQ, bass extension, spatial/binaural rendering, source separation, mood/genre-aware presets, and a beat-synced visualizer.

**Non-goal for v1:** streaming service integration, mobile apps, cloud account system, DRM-protected sources. Local files only until Phase 6+.

---

## 2. Goals & Success Metrics

| Goal | Metric |
|---|---|
| Real-time enhancement feels better than raw playback | A/B blind test: >70% prefer enhanced |
| Low latency | End-to-end DSP chain <20ms on mid-tier laptop |
| Stable under load | No dropouts/glitches over 60min continuous playback |
| AI enhancement useful, not gimmicky | User can disable any single stage independently |

---

## 3. Scope by Version

| Version | Includes |
|---|---|
| MVP (Ph 0–1) | Playback, decode, manual DSP chain (EQ, bass, compressor, loudness norm) |
| V1 (Ph 2–3) | Spatial audio, room sim, genre/mood/beat detection, adaptive modes |
| V2 (Ph 4–5) | Source separation, visualizer |
| V3 (Ph 6) | Natural-language EQ control, night mode, UI polish |
| Backlog (not prompted below) | Head tracking, remix generation, karaoke, collaborative rooms, wearable-synced bass |

---

## 4. Architecture

```
UI (React/TS, WebGL visualizer)
        │  Tauri IPC (events/commands)
Playback Controller (Rust)
        │
Decoder (symphonia) ──► Ring Buffer ──► DSP Chain ──► Output (cpal)
                                          │
                          AI Analysis (ONNX Runtime, async, non-blocking)
                          genre / mood / BPM / beat / source-separation
```

DSP chain order: Decoder → FFT/analysis tap → EQ → Bass Enhancer → Compressor → Spatial/HRTF → Reverb (room sim) → Limiter → Output.

AI inference runs on a separate thread/worker; results feed DSP parameters via a lock-free parameter bus (never blocks the audio callback).

---

## 5. Tech Stack Decision

| Layer | Choice | Why | Alternative considered |
|---|---|---|---|
| Shell | Tauri | Native perf + web UI, smaller than Electron | Electron (heavier), Flutter (weaker desktop audio ecosystem) |
| Audio IO | `cpal` | Cross-platform, low-latency, Rust-native | PortAudio (C bindings overhead) |
| Decode | `symphonia` | Pure Rust, no FFmpeg binary dependency | FFmpeg (licensing/packaging overhead) |
| DSP | Custom Rust (biquad filters, `rustfft`) | Full control over latency | JUCE/C++ (adds cross-language build complexity) |
| AI inference | ONNX Runtime (`ort` crate) | Ship pre-trained models, no Python runtime needed | Python microservice (latency, packaging pain) |
| Model training/export | Python (offline only, not shipped) | Standard ML tooling | — |
| Visualizer | WebGL via Three.js in frontend | GPU-accelerated, easy shader iteration | Native OpenGL/Vulkan (more code, no benefit at this scale) |
| Source separation | HTDemucs exported to ONNX | Best quality/speed tradeoff, exportable | Spleeter (lower quality) |
| Build/CI | GitHub Actions (windows-latest/macos-latest/ubuntu-latest) | Provides MSVC/Xcode/linux toolchains without local install | Local native toolchain (unavailable) |

**Rejected for v1:** Flutter (audio plugin ecosystem too thin for sub-20ms DSP), pure web app (Web Audio API can't do custom low-level DSP at needed latency).

---

## 6. Non-Functional Requirements

- Audio thread never allocates or blocks (no locks, no syscalls in callback).
- All AI/ML inference off the audio thread.
- Graceful degradation: if a model fails to load, that feature disables silently, playback continues unaffected.
- Cross-platform: Windows, macOS, Linux from one codebase.
- All correctness verification is automatable and runs in CI — no step should require a human to build natively.

---

## 7. Risks

| Risk | Mitigation |
|---|---|
| Real-time DSP + AI inference contention | Strict thread separation; AI writes to param bus, audio thread only reads |
| ONNX model size bloats installer | Lazy-download models on first use, cache locally |
| HRTF quality varies by headphone | Ship a generic HRTF set (e.g., MIT KEMAR), allow user calibration later |
| Source separation artifacts | Expose "separation strength" blend control, default conservative |
| No local build to sanity-check before pushing | Keep local checks to `cargo fmt`/`clippy`(if linker-free)/`git`; treat every push as the real test via CI; keep CI turnaround fast (cache deps, small test fixtures) |

---

## 8. Repo Structure

```
aura/
├── .github/
│   └── workflows/
│       ├── ci.yml         # lint/test/compile-check, matrix win/mac/linux
│       └── build.yml      # full Tauri bundle → installers, on demand or tag
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── audio/          # decode, DSP chain, output
│   │   ├── analysis/       # AI inference workers
│   │   ├── ipc/            # Tauri commands/events
│   │   └── main.rs
│   └── models/              # bundled/downloaded ONNX models
├── src/                     # React/TS frontend
│   ├── components/
│   ├── visualizer/          # WebGL/Three.js
│   └── state/
└── docs/
```

---

## 9. CI/CD Strategy (read before running any phase prompt below)

No local Visual Studio, Xcode, or MSVC assumed. All building and verification happens on GitHub Actions.

| Workflow | File | Trigger | Purpose |
|---|---|---|---|
| CI | `.github/workflows/ci.yml` | push, PR | lint, unit test, compile-check on windows-latest |
| Desktop Build | `.github/workflows/build.yml` | `workflow_dispatch`, tag `v*` | full Tauri bundle (.msi/.exe, .dmg, .AppImage/.deb) via `tauri-apps/tauri-action`, uploaded as artifacts / draft release |

**Agent loop for every phase:**
1. Write code + automated tests locally. Only run linker-free local commands (`cargo fmt`, `cargo check` may still fail without a linker on Windows — don't rely on it; treat it as best-effort).
2. Commit, push to a branch, open/update a PR.
3. Poll status: `gh run watch` or `gh pr checks --watch`.
4. Phase is only "done" when `ci.yml` is green on all 3 OS legs.
5. Anywhere the original spec said "manual listening test": replace with an automated offline check — render DSP output to a buffer/WAV in a test, then assert on measurable properties (RMS, band energy via FFT, cross-correlation, peak levels) instead of listening. Keep manual listening as an optional, non-blocking spot check once the user downloads an installer from a `build.yml` run.
6. Anywhere the original spec said "measure CPU %" or "60fps": these vary by hardware and can't be hard-gated on shared CI runners. Convert to a `criterion` benchmark (or equivalent) whose results are logged as a CI artifact and tracked for regressions (e.g., flag >20% slowdown vs. previous run), not a pass/fail threshold.

---

# 10. Phase-by-Phase Prompts for AI Coding Agent

Phase 0 (scaffold + playback) is already built locally. Run **Phase 0.5 next** to retrofit CI before continuing.

---

### Phase 0.5 — CI/CD Setup (retrofit, run before Phase 1)

```
Phase 0 (playback pipeline) is already implemented locally. Before continuing, set up CI/CD so all
further building and verification happens on GitHub Actions — assume no local Visual Studio Build
Tools, Xcode Command Line Tools, or GNU linker is available on this machine.

Deliverables:
1. `.github/workflows/ci.yml`:
   - Matrix: windows-latest, macos-latest, ubuntu-latest.
   - Steps: checkout; install Rust stable (`dtolnay/rust-toolchain@stable`); install Node LTS;
     cache cargo registry/target and npm cache; on ubuntu-latest install Tauri's Linux prerequisites
     (webkit2gtk, appindicator, rsvg, patchelf — use the current official Tauri prerequisites list
     for the Tauri version pinned in Cargo.toml); run `cargo fmt --check`; `cargo clippy --all-targets
     -- -D warnings`; `cargo test --workspace`; `npm ci`; `npm run build`; then a debug Tauri compile
     step (`cargo tauri build --debug --no-bundle`, or `cargo build` inside src-tauri if that flag
     isn't available) to confirm the app links successfully on all 3 OS.
   - Trigger: push to any branch, and pull_request.
2. `.github/workflows/build.yml`:
   - Trigger: `workflow_dispatch` and tags matching `v*.*.*`.
   - Matrix windows-latest/macos-latest/ubuntu-latest, use `tauri-apps/tauri-action@v0` to produce
     installers (.msi/.exe, .dmg, .AppImage/.deb) as workflow artifacts; on a tag push, also create a
     draft GitHub Release with the artifacts attached.
3. Add a "Building & Testing" section to root `README.md`: explain no local native build is required;
   pushing a branch triggers `ci.yml`; running `build.yml` manually from the Actions tab (or pushing a
   `v*` tag) produces downloadable installers from the run's artifacts or the draft release.
4. Confirm `.gitignore` excludes `target/`, `src-tauri/target/`, `node_modules/`, `dist/`.
5. Do NOT attempt `cargo tauri build` or a full `cargo build` locally to "test" this — assume it will
   fail locally due to missing linker/toolchain. Validate entirely by pushing and watching the Actions
   run (`gh run watch`).

Acceptance criteria:
- Pushing to any branch triggers `ci.yml`, green on all 3 OS matrix legs.
- Manually triggering `build.yml` from the Actions tab (or a `v0.0.1-test` tag) produces downloadable
  Windows/macOS/Linux installer artifacts.
- README documents the workflow clearly enough that local native builds are never required.
```

---

### Phase 1 — Core Manual DSP Chain

```
Extend AURA with a manual (non-AI) real-time DSP chain, inserted between decode and output.
Verification is via `ci.yml` (push and confirm green on all 3 OS) — no local build required.

Deliverables:
1. `audio::dsp` module with independently bypassable stages, in this fixed order:
   - Parametric EQ: minimum 5-band biquad (peaking/shelf), coefficients computed from freq/gain/Q.
   - Bass enhancer: harmonic generation for frequencies below a cutoff (not just gain boost).
   - Compressor: threshold/ratio/attack/release, soft-knee.
   - Loudness normalizer: target LUFS via a running loudness estimate (ITU-R BS.1770 style).
   - Limiter: brick-wall, prevents clipping post-chain.
2. Parameter bus: a lock-free structure (e.g., `arc-swap` or triple-buffer) so the UI thread can
   update DSP params without blocking the audio thread.
3. Tauri commands to get/set each stage's parameters and enable/disable each stage independently.
4. React UI: sliders/toggles per stage (basic, no design polish yet).
5. Automated tests (run in CI, not locally) using synthetic signals (sine sweep, impulse, white
   noise), rendering DSP output to an in-memory buffer and asserting on measurable properties:
   - Toggling any stage mid-buffer does not produce a sample-to-sample delta above a defined
     discontinuity threshold (click/pop check).
   - Compressor reduces peak level on a signal exceeding threshold, verified via RMS/peak comparison.
   - EQ boost/cut at a target frequency is measurable via FFT on rendered output vs. bypassed output.
6. Add a `criterion` benchmark for the full chain's per-block processing time; log as a CI artifact
   (informational — not a hard pass/fail gate, since runner hardware varies).

Do NOT implement spatial audio, reverb, or AI-driven parameter selection yet — all params are
user-set. Do NOT rely on manual listening or local CPU profiling to validate this phase.

Acceptance criteria:
- `ci.yml` green on all 3 OS with the new tests included.
- Automated click/pop test passes for every stage toggle combination.
- Automated compressor and EQ measurement tests pass with documented tolerance thresholds.
- Benchmark artifact is produced and viewable in the CI run (no fixed CPU% requirement).
```

---

### Phase 2 — Spatial Audio & Room Simulation

```
Add spatial audio and environmental simulation stages to AURA's DSP chain, after the Phase 1 chain
and before the limiter. Verification via `ci.yml` only.

Deliverables:
1. `audio::spatial` module:
   - Stereo widening (mid-side processing).
   - Crossfeed for headphone listening (reduce hard-panned harshness).
   - Binaural/HRTF rendering: integrate a public-domain HRTF dataset (e.g., MIT KEMAR); convolve
     input with left/right HRIR pairs for a virtual speaker layout (front, side, rear).
2. `audio::reverb` module: convolution reverb using pre-captured impulse responses (bundle 3–4
   free/CC0 IRs: small room, concert hall, cathedral, cave). Wet/dry mix control.
3. Efficient convolution: partitioned/FFT-based convolution (`rustfft`) to bound latency/CPU — no
   naive time-domain convolution for IRs longer than ~50ms.
4. Tauri commands + UI: environment selector (Room/Hall/Cathedral/Cave/Off), spatial width slider,
   crossfeed toggle.
5. Bundle HRIR/IR assets under `src-tauri/models/` or `assets/`; document license in
   `docs/THIRD_PARTY_ASSETS.md`.
6. Automated tests (CI-run):
   - Environment crossfade: assert no sample-delta spike above threshold when switching environments
     mid-buffer (offline render test).
   - HRTF repositioning: cross-correlate offline-rendered L/R output for a hard-panned test tone
     against an expected HRIR-convolved reference; assert similarity above a defined threshold.
   - Reverb wet/dry mix: assert output energy scales as expected across mix settings.
7. `criterion` benchmark for convolution cost per IR length, logged as CI artifact (informational).

Acceptance criteria:
- `ci.yml` green on all 3 OS.
- Crossfade discontinuity test passes for every environment transition pair.
- HRTF cross-correlation test passes against the reference threshold.
- Benchmark artifact produced for all bundled IRs.
- Note in `docs/manual_verify.md`: optional real-headphone listening check once an installer is
  downloaded from a `build.yml` run — not required for this phase to be considered complete.
```

---

### Phase 3 — AI Analysis: Genre, Mood, BPM, Beat, Adaptive Modes

```
Add an AI analysis pipeline that runs off the audio thread and drives adaptive DSP presets.
Verification via `ci.yml` only.

Deliverables:
1. `analysis` module in a dedicated worker thread (never the audio callback thread):
   - BPM/beat detection (onset detection + tempo estimation; DSP-based, no ML required).
   - Genre classification via a pre-trained ONNX model through the `ort` crate. Source/convert a
     small open genre-classification model; document source + license in
     `docs/THIRD_PARTY_ASSETS.md`. Check the model into the repo (or fetch it in a CI step) only if
     small enough to keep CI fast — otherwise document the download step.
   - Mood classification (valence/energy) via ONNX, or derived heuristically from spectral features +
     tempo if no suitable open model exists — document which approach was used and why.
2. Parameter bus extension: analysis results write suggested DSP presets to the same lock-free bus
   from Phase 1/2. Audio thread only ever reads.
3. Adaptive Listening Modes as data-driven presets (JSON/config, not hardcoded branches): genre →
   preset mapping (Rock: wider stage, EDM: deeper bass, Classical: hall reverb, Podcast: vocal-focused
   EQ + spatial off, Lo-fi: warm EQ curve).
4. Beat-driven modulation: on detected energy jump, briefly boost bass enhancer + stereo width via an
   envelope follower, decaying back — not per-sample ML inference.
5. UI: "Auto" (AI-driven) vs "Manual" toggle; display detected genre/mood/BPM.
6. Graceful fallback: if the ONNX model fails to load, log a warning, disable AI-driven auto mode,
   keep beat detection and manual controls fully functional.
7. Automated tests (CI-run):
   - BPM detection accuracy against a synthetic click track of known BPM (generated in the test, no
     external fixture needed), asserting result within a defined tolerance.
   - Model-missing fallback: test harness masks the model path, asserts the app doesn't panic and
     playback/manual controls still work.
   - Auto→Manual toggle: assert no sample-delta discontinuity spike on switch (same technique as
     Phase 1/2).
   - If a genre/mood accuracy fixture dataset is available, assert classification accuracy above a
     threshold; otherwise, a smoke test asserting the model runs end-to-end without error is
     sufficient for this phase (note the gap explicitly in the PR description).

Acceptance criteria:
- `ci.yml` green on all 3 OS, including the new analysis tests.
- BPM detection test passes within tolerance.
- Missing-model fallback test passes (no crash, feature disables cleanly).
- Auto↔Manual switch discontinuity test passes.
```

---

### Phase 4 — AI Source Separation

```
Add AI source separation to AURA (vocals/drums/bass/other stem control). Verification via `ci.yml`
only; keep any test audio fixtures short (<5s) to keep CI fast and cheap.

Deliverables:
1. Integrate an HTDemucs (or equivalent) model exported to ONNX, run via `ort`. Run as a background
   job when a track loads (not real-time): separate to 4 stems, cache to disk
   (`~/.aura/cache/<track-hash>/`).
2. Once cached, mixing is real-time: DSP chain from Phases 1-3 applies per-stem gain before summing
   (vocals/drums/bass/other independent gain + mute).
3. Tauri commands: `separate_track(path)` (returns job id, progress events), `set_stem_gain`,
   `set_stem_mute`.
4. UI: 4 stem faders + mute, progress indicator, cache-hit indicator.
5. Cache management: configurable size cap (default 5GB), evict oldest on overflow.
6. If separation fails or is still processing, playback continues on the original mix — never blocks
   playback start.
7. Automated tests (CI-run, headless, no GUI needed):
   - Integration test invoking Tauri commands directly: start playback, call `separate_track`, assert
     playback is uninterrupted while the job runs.
   - Cache hit: run separation once, assert second call for the same file loads from cache without
     re-invoking inference (mock/spy on the inference call count).
   - Mute isolation: render mixed output with a stem muted vs. not muted; assert that stem's energy
     contribution is ~0 in the muted case via RMS comparison of the difference signal.
   - Cache eviction: unit test with mocked file sizes asserting the cap is enforced and oldest entries
     evict first.

Acceptance criteria:
- `ci.yml` green on all 3 OS.
- All 4 automated tests above pass.
- CI run time stays reasonable — flag in the PR if the ONNX model/test fixtures meaningfully slow
  down the pipeline, and note the runtime added.
```

---

### Phase 5 — Visualizer

```
Add a beat-synced WebGL visualizer to AURA's frontend. Visual/pixel-level correctness (frame rate,
visual sync "feel") is NOT CI-gated — most GitHub-hosted runners have no GPU and no display. CI only
gates: code compiles, data pipeline is correct, and frontend unit tests pass.

Deliverables:
1. Rust side: emit real-time FFT magnitude data + beat/energy signal (from Phase 3) to the frontend
   via Tauri events, throttled to ~30-60Hz (do not flood IPC per-audio-sample).
2. Frontend `visualizer/` module using Three.js: at minimum 2 modes — (a) circular spectrum analyzer,
   (b) particle field reactive to beat/energy.
3. Visualizer reacts to FFT bands (frequency-based color/shape) and the beat-drop envelope
   (Phase 3) for pulse/burst effects.
4. UI: visualizer mode selector, fullscreen toggle, on/off (disabling stops FFT event emission).
5. Automated tests (CI-run):
   - Backend: assert FFT/beat events are emitted at the expected throttled rate for a given input,
     and assert zero events are emitted for a time window after the visualizer is disabled.
   - Frontend: unit/component tests (Vitest + React Testing Library) verifying the visualizer
     component renders without error given mock event data, and mode switching updates state
     correctly. Do not attempt to assert actual rendered pixel output or frame timing in CI.
6. `npm run build` must succeed on all 3 OS in `ci.yml` (frontend build is portable, so this check is
   meaningful even though runtime visuals aren't).

Acceptance criteria:
- `ci.yml` green on all 3 OS (build + the automated tests above).
- Event-throttling and disable-stops-events tests pass.
- Add `docs/manual_verify.md` entry: once an installer is downloaded from a `build.yml` run, spot-check
  visual sync and frame rate manually — explicitly optional, not required for phase completion.
```

---

### Phase 6 — Natural-Language EQ, Night Mode, UI Polish

```
Finalize AURA: natural-language control layer, night mode, and UI polish pass. Verification via
`ci.yml`; UI visual polish is spot-checked manually post-download (optional), not CI-gated.

Deliverables:
1. Natural-language EQ control: a local, offline mapping layer (rule-based or lightweight keyword/
   fuzzy matcher — no cloud LLM calls) mapping phrases like "more cinematic", "clearer vocals", "more
   bass", "vintage" to concrete parameter deltas across the Phase 1-2 DSP stages. Implement as an
   editable JSON mapping (phrase → {stage: param: delta}), not a black box.
2. Smart Night Mode: preset toggle that (a) reduces high-frequency harshness via EQ, (b) applies
   gentle vocal boost, (c) tightens compressor/limiter to prevent sudden loud peaks, (d) keeps bass
   audible at lower overall level. Implemented as a documented preset over existing Phase 1 params.
3. UI polish pass: dark OLED theme, glassmorphism panels, large album art, animated background
   reactive to Phase 5 visualizer data at low intensity, smooth transitions.
4. Settings persistence: save manual EQ/spatial/mode preferences to a local config file, restore on
   launch.
5. Full regression pass: re-run Phase 0-5 automated test suites to confirm no regressions from UI/
   config changes.
6. Automated tests (CI-run):
   - NL mapping: unit tests asserting each supported phrase produces the expected parameter delta
     dict (pure logic, no audio needed).
   - Night mode: render a test signal with synthetic peaks through the night-mode preset vs. baseline,
     assert peak level is reduced.
   - Settings persistence: write config, reload in-process (simulating restart), assert values match.

Acceptance criteria:
- `ci.yml` green on OS, full test suite (Phases 1-6) passing with zero regressions.
- NL mapping, night mode, and persistence tests all pass.
- `build.yml` run (manual trigger or tag) produces installers for manual UI/visual spot-check —
  optional, non-blocking for calling this phase complete.
```

---

## 11. Backlog (not scoped in prompts above)

Head tracking, AI remix generation, karaoke mode, 8D audio auto-generation, Dolby Atmos upmix, collaborative listening rooms, wearable heartbeat-synced bass, AI-generated ambient layers. Revisit after V3 ships and core is stable.
