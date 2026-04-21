# Changelog

All notable changes to Voice-typeless are documented here.  
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).  
Version numbers follow [Semantic Versioning](https://semver.org/).

---

## [0.2.1] - 2026-04-22

### Fixed

- **Hotkeys now work**: Added global `document.keydown` / `keyup` listeners in `App.svelte`
  that parse the stored hotkey combo strings (e.g. `"Alt+Space"`) and call
  `startRecording` / `stopRecording` / `cancelRecording` accordingly.
  Previously, hotkeys were captured in Settings and saved to config, but nothing
  ever read them back to intercept keypresses.
- **Push-to-talk mode**: Holding the PTT key starts recording; releasing it stops.
  Tracks the physical key so modifier-release before key-release works correctly.
- **Free-speech toggle**: One press starts, a second press stops.
- **Cancel hotkey**: Works while recording or processing; resets status to idle.
- **Hotkeys update live**: After saving Settings, `appState.hotkeyConfig` is updated
  so the new combos take effect immediately without restarting the app.
- **Recording commands emit events**: `start_recording`, `stop_recording`, and
  `cancel_recording` Rust commands now emit `recording-started`, `recording-stopped`,
  and `recording-cancelled` Tauri events so the UI status indicator updates correctly.
  `stop_recording` also spawns a 600 ms delayed `recognition-result` event so the UI
  returns to idle automatically.
- **Chinese UI language switch now works**: Created `frontend/src/lib/i18n.svelte.ts`
  — a Svelte 5 module-level `$state`-backed reactive i18n system with full English and
  Traditional Chinese translations. Switching to Chinese in Settings → Display language
  now instantly updates all section headings, labels, buttons, and status text.
- **Svelte 5 reactive i18n**: All components read translations via `t('key')`, which
  re-evaluates automatically when `lang` state changes — no page reload needed.

### Added

- `frontend/src/lib/i18n.svelte.ts` — reactive i18n module with 50+ translation keys
  in English and Traditional Chinese.

## [0.2.0] - 2026-04-21

### Added

- **Persistent config**: application settings are saved to
  `%APPDATA%\Roaming\VoiceTypeless\config.json` and restored on every launch.
  First-launch writes a full default config automatically.
- **Persistent history**: all recognition results are saved to
  `%APPDATA%\Roaming\VoiceTypeless\history.json`. History survives application restarts.
- **Clear All History**: one-click button in the History panel with a confirmation dialog.
  Calls the new `clear_history` Tauri command.
- **Export History**: "Copy to Clipboard" button that formats the full history as timestamped
  plain text. Calls the new `export_history_text` Tauri command.
- **Search / Filter History**: real-time text search input in the History panel. Filtering
  runs entirely in the frontend — no round-trip required.
- **Statistics panel**: summary card showing total recordings, total characters transcribed,
  and a per-language breakdown. Powered by the new `get_stats` Tauri command.
- **Demo Mode**: a "Try Demo" button that simulates a recording/transcription cycle without a
  microphone or speech model. Calls the new `run_demo` Tauri command. Demo results are
  written to history just like real results.
- **Theme system**: Dark, Light, and System theme options in Settings. Selection is persisted
  to `config.json` via `set_config` and applied at startup via `get_config`.
- **Windows Autostart**: checkbox in Settings that toggles launch-at-startup via the Windows
  registry key `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\VoiceTypeless`.
  Powered by `get_autostart_enabled` / `set_autostart_enabled` Tauri commands.
- **System Tray**: app minimises to the system tray on window close. Tray icon context menu
  provides "Show / Hide" and "Quit" actions. Powered by `tauri-plugin-tray`.
- **`AppState` in Rust**: new `AppState` struct in `src-tauri/src/lib.rs` holds
  `Mutex<Vec<HistoryItem>>` and `Mutex<AppConfig>`, enabling shared in-memory state across
  all async Tauri commands.

### Changed

- All Tauri commands now persist state to disk. Previously every command returned a hardcoded
  or empty stub value; v0.2.0 commands read/write real JSON files.
- `get_history` now accepts an optional `limit` parameter (default `50`). Pass `0` to
  retrieve all items.
- `get_config` / `set_config` now operate on the real `config.json` file rather than
  returning a stub `AppConfig`.

### Technical Notes

- Config and history files are written atomically (write to `.tmp`, then rename) to prevent
  corruption on crash or power loss.
- The `history.json` array is capped at `config.ui.maxHistoryItems` (default `50`). Oldest
  items are evicted when the cap is exceeded.

---

## [0.1.0] - 2026-04-21

### Added

- Initial project scaffold: Tauri v2 + Svelte 5 + TailwindCSS desktop app.
- Core Go library stubs for audio capture, speech engine, hotkey, clipboard paste,
  text processor, config, and history packages.
- Frontend components: `HistoryPanel.svelte`, `SettingsPage.svelte`,
  `FloatingIndicator.svelte`.
- Tauri command stubs: `start_recording`, `stop_recording`, `cancel_recording`,
  `get_devices`, `get_history`, `delete_history_item`, `get_config`, `set_config`,
  `get_models`, `switch_model`.
- Build system: `Makefile` + PowerShell scripts for Windows (MSVC toolchain).
- `scripts/env-msvc.ps1`: auto-detects VS 2019 Build Tools (MSVC 14.29) and sets
  `PATH`, `LIB`, `INCLUDE`, `LIBPATH` to avoid the VS 2025 `msvcrt.lib` link error.
- `scripts/setup.ps1`: installs Rust, Node.js, and Go prerequisites.
- `scripts/dev.ps1`: one-command dev environment launcher.
- Application icon set (all required Tauri resolutions) with standards-compliant ICO
  binary structure.

### Fixed

- `LINK: cannot open msvcrt.lib` — resolved by pinning to VS 2019 MSVC 14.29 via
  `env-msvc.ps1`.
- `excpt.h not found` — resolved by the same toolchain pin.
- ICO Reserved field invalid — fixed by manually constructing a spec-compliant ICO
  binary structure.
- `voice_typeless_lib::run()` symbol not found — corrected to `voice_typeless::run()`
  in `main.rs`.
