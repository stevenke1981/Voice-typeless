# Build Scripts

| Script | Platform | Description |
|--------|----------|-------------|
| `build-win.ps1` | Windows 10/11 | Full build (Rust core + Tauri) |
| `build-win7.ps1` | Windows 7 | Slim build, CPU-only inference (Rust core + Tauri) |
| `build-mac.sh` | macOS | macOS universal binary (Rust core + Tauri) |

All builds use `core-rs/` (Rust, crate `vtl-core`) as a workspace dependency of `src-tauri/`.
No Go sidecar process — Tauri commands call `vtl-core` directly.
