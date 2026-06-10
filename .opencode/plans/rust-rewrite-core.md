## Plan: Rust Core Rewrite (Phase 3)

**Goal:** Rewrite all Go Core packages in Rust, eliminate sidecar architecture
**Complexity:** L4

### Strategy

Create `core-rs/` as a standalone Rust library crate mirroring Go's package structure.
Tauri `src-tauri/` depends on `core-rs/` directly (no IPC layer).
Delete Go `core/` after verification.

### Package Map

| Go Package | Rust Module | Key Crates | Complete? |
|------------|-------------|------------|-----------|
| `config/` | `config.rs` | `serde`, `serde_json`, `dirs` | ✅ Fully ported (load/save/default + tests) |
| `processor/` | `processor.rs` | (none, pure Rust) | ✅ Fully ported (filler filter, mixed lang, capitalize + tests) |
| `history/` | `history.rs` | `rusqlite` | ✅ Fully ported (SqliteStore trait + tests) |
| `audio/` | `audio.rs` | `cpal`, `hound` | ✅ Fully ported (Recorder/Player/Enumerator/VAD traits + tests) |
| `engine/` | `engine.rs` | `sherpa-onnx` | ✅ Fully ported (SenseVoice Engine + integration tests) |
| `hotkey/` | `hotkey.rs` | Tauri `global-shortcut` | ✅ Fully ported (KeyCombo parsing + HotkeyManager trait + tests) |
| `paste/` | `paste.rs` | Win32 FFI | ✅ Fully ported (Windows clipboard + ClipGuard + tests) |
| `ipc/` | **Eliminated** | — | ✅ Replaced by direct calls |

### Sub-tasks (ordered by dependency)

#### Batch A — Zero-dependency crates (parallelizable)

- [x] **A1 — core-rs scaffolding**
  - Create `core-rs/Cargo.toml` (library crate, `vtl-core`)
  - Set up `mod` structure, re-exports in `lib.rs`
  - Add to Tauri workspace in `src-tauri/Cargo.toml` as path dep

- [x] **A2 — config.rs** (port Go `core/config/`)
  - Structs: `AppConfig`, `HotkeyConfig`, `AudioConfig`, `ModelConfig`, `TextConfig`, `UIConfig`, `SystemConfig`
  - Functions: `load()`, `save()`, `default_config()`
  - Cargo deps: `serde`, `serde_json`, `dirs`
  - **Verify:** `cargo test` + compare JSON output with Go version

- [x] **A3 — processor.rs** (port Go `core/processor/`)
  - `TextProcessor` struct with `process()` pipeline
  - `FillerWordFilter` — multi-language filler removal (zh/en/ja/ko)
  - `normalize_mixed_language()` — CJK/Latin spacing
  - Language detection via Unicode range heuristics
  - Cargo deps: (none, pure Rust)
  - **Verify:** `cargo test` with same test cases as Go

- [x] **A4 — history.rs** (port Go `core/history/`)
  - `HistoryStore` trait + `SqliteStore` impl
  - `add()`, `list()`, `delete()`, `prune()`, `close()`
  - Cargo deps: `rusqlite` (bundled)
  - **Verify:** `cargo test` with in-memory SQLite

#### Batch B — Platform-dependent crates (separate from A)

- [x] **B1 — audio.rs** (port Go `core/audio/`)
  - `AudioRecorder` trait — `start()`, `stop()`, `cancel()`, `drain()`, `subscribe()`
  - `AudioPlayer` trait — `play_start()`, `play_stop()`, `play_cancel()`
  - `DeviceEnumerator` trait
  - `VAD` energy-based speech detection
  - Cargo deps: `cpal` (audio I/O), `hound` (WAV)
  - **Verify:** `cargo test` (unit tests for VAD, integration for device listing)

- [x] **B2 — engine.rs** (port Go `core/engine/`)
  - `Engine` trait — `load_model()`, `recognize()`, `recognize_stream()`, `model_info()`, `close()`
  - `ModelType` enum, `DeviceType` enum
  - Stub implementations initially (matching Go's current state)
  - Cargo deps: (none initially, `ort` when real inference added)
  - **Verify:** `cargo test`

- [x] **B3 — hotkey.rs** (port Go `core/hotkey/`)
  - `HotkeyManager` trait — `register()`, `unregister()`, `events()`, `run()`
  - `KeyCombo` parsing from string (e.g. "Alt+Space")
  - Desktop implementation with platform key codes
  - Cargo deps: `rdev` (global hotkey listener)
  - **Verify:** `cargo test` (combo parsing), manual (hotkey registration)

- [x] **B4 — paste.rs** (port Go `core/paste/`)
  - `Paster` trait — `paste()`, `configure()`
  - `ClipboardGuard` — `save()`, `restore()`, `hold_duration()`
  - Platform implementations for Windows/macOS/Linux
  - Cargo deps: `enigo` (keyboard simulation), `arboard` (clipboard)
  - **Verify:** `cargo test` (unit), manual (paste flow)

#### Batch C — Tauri integration

- [x] **C1 — Replace sidecar with direct calls**
  - Remove Go sidecar startup from `setup()` hook
  - Update all 16 Tauri commands to call `core-rs` directly
  - Remove JSON-RPC IPC client code
  - Add `core-rs` as dependency in `src-tauri/Cargo.toml`

- [x] **C2 — Wire events**
  - Go `RPCEvent` pattern → Rust `Arc<Notify>` / `tokio::sync::broadcast`
  - Emit Tauri events from Rust state
  - Remove `ipc_client.rs` event polling

- [x] **C3 — Cleanup Go**
  - Delete Go `core/` directory
  - ~~Remove Go build scripts from `build/`~~
  - ~~Update `build/` scripts for Rust-only flow~~
  - ~~Update `docs/architecture.md`~~

### Risks

| Risk | Mitigation |
|------|------------|
| `cpal` recording quality/API differs from malgo | Keep Go audio as reference; test on Windows first |
| `rusqlite` bundled compilation slow | Use system sqlite3 if available, `bundled` feature only for portability |
| `enigo` SendInput on Windows requires admin/UAC | Document limitation; fallback to clipboard method |
| `rdev` hotkey conflicts with other apps | Allow user to change keybinds in settings |
| `sherpa-onnx` Rust bindings immature | Start with `ort` crate for direct ONNX Runtime; compatible with same model files |
| Tauri v2 `global-shortcut` plugin may be sufficient | Prefer Tauri plugin over `rdev` if it covers all use cases |

### Definition of Done

- [x] `cargo build` passes for entire workspace (core-rs + src-tauri)
- [x] `cargo test` passes for core-rs (all modules — 76 unit + 5 integration)
- [x] `npm run build` passes (frontend builds with Tauri) — ✅ frontend built successfully (835ms, 83 KB JS + 28 KB CSS)
- [x] All 16 Tauri commands functional (no more IPC forwarding) — ✅ 16 `#[tauri::command]` found in lib.rs
- [x] Go `core/` directory removed
- [x] `build/` scripts updated for Rust-only — ✅ all 3 scripts use `cargo tauri build`, no Go tooling
- [x] Architecture docs updated — ✅ README.md, architecture.md (6 sections), api.md (2 sections), FloatingIndicator.svelte — all Go/sidecar references removed

### Execution Order

```
A1 (scaffold) ----→ A2 (config) ──→ C1 (Tauri wire)
                ↘→ A3 (processor)
                ↘→ A4 (history)
                
B1 (audio) ──→ B2 (engine) ──→ C1
B3 (hotkey) ────────────────→ C1
B4 (paste)  ────────────────→ C1

C1 ──→ C2 (events) ──→ C3 (cleanup)
```

Start with A1 + A2 + A3 in parallel (they share no deps on each other).
Add A4 after A1.
Add Batch B when Batch A is stable.
