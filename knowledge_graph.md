# Knowledge Graph — Voice-typeless (VTL)
初始建立：2026-04-21  
最後更新：2026-04-21（v2.0 — v0.2.0 功能節點）

## 節點

| 節點 | 類型 | 路徑 | 狀態 |
|------|------|------|------|
| `core/audio` | Go package | `core/audio/` | stub |
| `core/engine` | Go package | `core/engine/` | stub (sensevoice placeholder) |
| `core/hotkey` | Go package | `core/hotkey/` | stub |
| `core/paste` | Go package | `core/paste/` | stub |
| `core/processor` | Go package | `core/processor/` | stub |
| `core/history` | Go package | `core/history/` | stub |
| `core/ipc` | Go package | `core/ipc/` | stub |
| `src-tauri` | Rust/Tauri shell | `src-tauri/` | compiles ✓ |
| `src-tauri/AppState` | Rust struct | `src-tauri/src/lib.rs` | **active** — holds `Mutex<Vec<HistoryItem>>` + `Mutex<AppConfig>` |
| `src-tauri/tray` | Rust module | `src-tauri/src/tray.rs` | **active** — system tray setup + Show/Hide/Quit menu |
| `frontend` | Svelte 5 + Vite | `frontend/` | builds ✓ |
| `frontend/ThemeToggle` | Svelte component | `frontend/src/lib/components/` | **active** — Dark/Light/System selector |
| `frontend/StatisticsPanel` | Svelte component | `frontend/src/lib/components/` | **active** — recordings, characters, language breakdown |
| `frontend/HistorySearch` | Svelte feature | `frontend/src/lib/components/HistoryPanel.svelte` | **active** — real-time client-side filter |
| `frontend/DemoMode` | Svelte feature | `frontend/src/lib/components/` | **active** — UI trigger for `run_demo` command |
| `scripts/env-msvc.ps1` | Build helper | `scripts/` | active |
| `scripts/setup.ps1` | Setup script | `scripts/` | active |
| `scripts/dev.ps1` | Dev launcher | `scripts/` | active |
| `config.json` | Data file | `%APPDATA%\Roaming\VoiceTypeless\config.json` | **active** — persisted app settings |
| `history.json` | Data file | `%APPDATA%\Roaming\VoiceTypeless\history.json` | **active** — persisted transcription history |

## 關係

```
core/audio       → depends_on → malgo (miniaudio Go bindings)
core/engine      → depends_on → core/audio
core/engine      → depends_on → sherpa-onnx-go (TBD — not yet in go.mod)
core/processor   → depends_on → core/engine
core/ipc         → depends_on → core/audio, core/hotkey, core/paste, core/processor
src-tauri        → spawns     → core binary (subprocess IPC)
src-tauri        → depends_on → tauri v2, tauri-plugin-shell
src-tauri        → depends_on → tauri-plugin-tray (system tray)
src-tauri/AppState → manages  → config.json (read/write via get_config/set_config)
src-tauri/AppState → manages  → history.json (read/write via get_history/clear_history/…)
frontend         → communicates_via → tauri IPC commands → src-tauri
frontend/ThemeToggle → invokes → set_config (ui.theme)
frontend/StatisticsPanel → invokes → get_stats
frontend/HistorySearch → filters → in-memory HistoryItem[] (no IPC)
frontend/DemoMode → invokes  → run_demo → appends to history.json
src-tauri/tray   → emits     → window show/hide events → frontend
scripts/dev.ps1  → sources   → scripts/env-msvc.ps1
scripts/setup.ps1 → sources  → scripts/env-msvc.ps1
Makefile(build)  → sources   → scripts/env-msvc.ps1
```

## 已知技術債

| 項目 | 位置 | 說明 |
|------|------|------|
| sherpa-onnx-go | `core/engine/sensevoice.go` | 版本待確認，import 暫時 commented out |
| Tauri recording stubs | `src-tauri/src/lib.rs` | `start_recording`/`stop_recording`/`cancel_recording` 仍返回 stub 值，待接入 Go Core |
| A11y warnings | `frontend/src/lib/components/FloatingIndicator.svelte` | `<div>` 缺少 role/keyboard 處理 |
| Tailwind v4 config | `frontend/src/app.css` | 可能需要 `@config` 指令 |
| Autostart cross-platform | `src-tauri/src/commands.rs` | `set_autostart_enabled` 僅支援 Windows registry；macOS/Linux 尚未實作 |
| history.json cap | `src-tauri/src/lib.rs` | maxHistoryItems 上限邏輯需驗證是否在 set_config 變更時即時生效 |

## 已解決問題（歷次迭代）

| 問題 | 根因 | 解法 |
|------|------|------|
| `LINK: cannot open msvcrt.lib` | VS2025 MSVC 14.50 缺少 `lib\x64\` 目錄 | 使用 VS2019 BuildTools MSVC 14.29 |
| `excpt.h` not found | VS2025 MSVC 缺少 `include/` 目錄 | 同上；透過 `env-msvc.ps1` 自動偵測 |
| ICO Reserved field invalid | `GetHicon()` 產生非標準 ICO 格式 | 手動構建符合規範的 ICO 二進位結構 |
| `voice_typeless_lib::run()` | main.rs 使用了不存在的 crate 名稱 | 改為 `voice_typeless::run()` |
| Tauri commands 全為 stub | v0.1.0 所有命令返回硬編碼值 | v0.2.0 實作 AppState + JSON 持久化 |

## 變更歷史

| 版本 | 日期 | 內容 | 影響範圍 |
|------|------|------|----------|
| v1.0 | 2026-04-21 | 初始建立；記錄所有 build 修復 | 全專案 |
| v2.0 | 2026-04-21 | 新增 v0.2.0 功能節點：AppState、tray、theme、demo mode、config/history 持久化、統計、autostart | src-tauri, frontend, docs |
