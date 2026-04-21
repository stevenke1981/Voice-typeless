# Voice-typeless (VTL)

> **Say it. Type less.** — Zero-dependency, fully offline voice-to-text for every app.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8D8?logo=tauri)](https://tauri.app)
[![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte)](https://svelte.dev)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows-0078D6?logo=windows)](https://github.com/vtl/voice-typeless/releases)

Voice-typeless (VTL) is a lightweight desktop app that captures your voice with a global hotkey
and instantly types the transcribed text into any focused application — no internet connection,
no account, no cloud.

> 📸 **Screenshot placeholder** — _UI screenshot coming in v0.2.0 release_

## Why This Exists

Dictation tools either require a cloud connection (breaking privacy) or are locked to specific
apps. VTL runs 100% offline using on-device speech models (SenseVoice / Whisper via sherpa-onnx),
works in any text field on Windows, and restores your clipboard after pasting so you never notice
it was used.

## Quick Start

### Prerequisites

| Requirement | Version | Notes |
|-------------|---------|-------|
| [Rust + Cargo](https://rustup.rs) | 1.77+ | MSVC toolchain on Windows |
| [Node.js](https://nodejs.org) | 18+ | |
| [Go](https://go.dev) | 1.23+ | |
| VS 2019 Build Tools | MSVC 14.29 | See note below |

> **Windows toolchain note**: Use VS 2019 Build Tools (MSVC 14.29). MSVC 14.50 (VS 2025)
> is missing `lib\x64\` and `include\` directories required by the Rust linker. Run
> `. .\scripts\env-msvc.ps1` to auto-detect and apply the correct toolchain environment.

### Build & Run

```powershell
# 1. Activate the correct MSVC environment (Windows only)
. .\scripts\env-msvc.ps1

# 2. Install frontend dependencies
cd frontend && npm install && cd ..

# 3. Start development mode (hot-reload Tauri + Svelte)
cargo tauri dev

# 4. Build the production installer
cargo tauri build
# Installer output: src-tauri\target\release\bundle\nsis\
```

## Features

### v0.2.0 — Available Now

| # | Feature | Description |
|---|---------|-------------|
| 1 | **Persistent Config** | Settings saved to `AppData\Roaming\VoiceTypeless\config.json` and restored on relaunch |
| 2 | **Persistent History** | All recognition results saved to `AppData\Roaming\VoiceTypeless\history.json` |
| 3 | **Clear All History** | One-click clear with a confirmation dialog |
| 4 | **Export History** | Copy all transcription results to clipboard as formatted plain text |
| 5 | **Search / Filter History** | Real-time text search across all history items (frontend, no round-trip) |
| 6 | **Statistics Panel** | Total recordings, total characters, and per-language breakdown |
| 7 | **Demo Mode** | Simulate a full recording + transcription cycle — no microphone or speech model required |
| 8 | **Theme Toggle** | Dark, Light, and System theme options — persisted in `config.json` |
| 9 | **Windows Autostart** | Optional launch-at-startup via `HKCU\...\Run` registry key |
| 10 | **System Tray** | Minimize to tray with Show/Hide and Quit items in the tray context menu |

### Future — Speech Integration

| Feature | Status |
|---------|--------|
| Global hotkey push-to-talk & free-speech | 🔜 In design |
| Offline speech recognition via SenseVoice | 🔜 Requires Go Core integration |
| Whisper-tiny model support | 🔜 Planned |
| GPU acceleration (DirectML / CUDA) | 🔜 Planned |
| Auto-paste transcription to active app | 🔜 Planned |
| Voice Activity Detection (VAD) auto-stop | 🔜 Planned |
| Plugin system (JS / Lua transform scripts) | 🔜 Planned |
| Windows 7 compatibility build | 🔜 Planned |

## Architecture Overview

| Layer | Technology | Role |
|-------|-----------|------|
| **Desktop Shell** | Tauri v2 (Rust) | Window management, IPC routing, OS integration, tray |
| **Frontend** | Svelte 5 + TailwindCSS + TypeScript | All user-visible UI; no business logic |
| **Core Library** | Go 1.23+ | Audio capture, speech recognition, text post-processing |
| **Speech Engine** | sherpa-onnx + ONNX Runtime | Offline inference (SenseVoice / Whisper) |
| **Audio I/O** | malgo (miniaudio Go bindings) | Microphone capture at 16 kHz mono |

The Go Core runs as a **sidecar process** spawned by Tauri and communicates via JSON-RPC 2.0
over a named pipe. For full design details see [`docs/architecture.md`](docs/architecture.md).

## Configuration & Data Storage

VTL stores all user data in the Windows app-data folder:

```
%APPDATA%\Roaming\VoiceTypeless\
├── config.json      # Application settings (hotkeys, theme, device, language, …)
└── history.json     # Transcription history (capped at maxHistoryItems, default 50)
```

Default config is written automatically on first launch. To reset all settings, delete
`config.json` — it will be recreated with defaults on the next start.

## API Reference

See [`docs/api.md`](docs/api.md) for the complete Tauri command reference, including all
parameters, return types, and error codes.

## Contributing

1. Fork the repository and clone it locally
2. Create a feature branch: `git checkout -b feat/your-feature`
3. Follow the [architecture design](docs/architecture.md) — keep the layer boundaries clean
4. Test your changes with `cargo tauri dev`
5. Open a pull request with a clear description of what changed and why

Please open an issue before beginning large or breaking changes so we can coordinate.

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md) for the full version history.

## License

MIT © Voice-typeless Contributors
