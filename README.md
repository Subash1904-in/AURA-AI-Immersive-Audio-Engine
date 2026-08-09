# AURA AI Immersive Audio Engine

AURA is an AI-powered immersive audio desktop application built with Tauri v2, React, TypeScript, and Rust.

## Building & Testing

**No local native build toolchain is required.** You do not need Visual Studio Build Tools (Windows), Xcode Command Line Tools (macOS), or a local C/C++ linker installed on your development machine.

All cross-platform compilation, linting, testing, and packaging are automated via GitHub Actions:

### Continuous Integration (`ci.yml`)
- **Trigger**: Every `push` to any branch or `pull_request`.
- **Matrix**: `windows-latest`, `macos-latest`, `ubuntu-latest`.
- **Verification Steps**:
  1. Installs Rust stable and Node.js LTS.
  2. Installs Linux prerequisites (`webkit2gtk`, `appindicator`, `rsvg`, `patchelf`).
  3. Verifies Rust code formatting (`cargo fmt --check`).
  4. Runs Rust linter (`cargo clippy --all-targets -- -D warnings`).
  5. Executes Rust test suite (`cargo test --workspace`).
  6. Builds Vite frontend bundle (`npm run build`).
  7. Compiles debug Tauri application (`npx tauri build --debug --no-bundle`).

### Release Installer Generation (`build.yml`)
- **Trigger**: Manual trigger from the GitHub Actions tab (`workflow_dispatch`) or pushing a version tag matching `v*.*.*` (e.g. `v0.1.0`).
- **Matrix**: `windows-latest`, `macos-latest`, `ubuntu-latest`.
- **Output**:
  - Automatically compiles release bundles for Windows (`.msi`, `.exe`), macOS (`.dmg`), and Linux (`.AppImage`, `.deb`).
  - Attaches generated installers as downloadable workflow run artifacts.
  - On version tag pushes, attaches installers to a draft GitHub Release.

## Recommended Local Development Setup

- **IDE**: [VS Code](https://code.visualstudio.com/) + [Tauri Extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
- **Frontend Development**: Run `npm run dev` for fast UI feedback with Vite HMR.
