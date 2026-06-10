pub mod commands;
pub mod config_helpers;
pub mod engine_loader;
pub mod history_io;
pub mod model_downloader;
pub mod model_info;
pub mod state;

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use vtl_core::audio::{AudioPlayer, Player, Recorder};

use crate::state::AppState;

// ═══════════════════════════════════════════════════════════════════════════
// Portable mode helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Returns the directory containing the current executable.
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Returns `true` when a `portable.txt` file exists next to the executable.
///
/// In portable mode all data (config, history, models) is stored **locally**
/// next to the EXE instead of `%APPDATA%` so the whole app can run from a
/// USB drive or any folder without leaving traces on the host.
fn is_portable() -> bool {
    exe_dir().join("portable.txt").exists()
}

/// Returns the data directory for history / misc runtime files.
fn data_dir(portable: bool) -> PathBuf {
    if portable {
        exe_dir()
    } else {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("VoiceTypeless")
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Application entry-point
// ═══════════════════════════════════════════════════════════════════════════

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    // Read config to match accelerator → action
                    let action = app
                        .state::<Mutex<AppState>>()
                        .lock()
                        .ok()
                        .map(|s| {
                            let cfg = &s.config.hotkey;

                            // Parse each config hotkey into a HotKey struct and compare
                            // the mods + key fields directly.  This avoids all string-
                            // format pitfalls:
                            //   - keyboard_types::Code::Display outputs "KeyV" not "V"
                            //   - into_string() uses fixed modifier order (shift→control→alt→super)
                            //   - config aliases ("Ctrl" vs "Control", "Win" vs "Super")
                            if let Ok(hk) = Shortcut::from_str(&cfg.push_to_talk) {
                                if shortcut.mods == hk.mods && shortcut.key == hk.key {
                                    return "ptt".to_string();
                                }
                            }
                            if let Ok(hk) = Shortcut::from_str(&cfg.free_speech) {
                                if shortcut.mods == hk.mods && shortcut.key == hk.key {
                                    return "free_speech".to_string();
                                }
                            }
                            if let Ok(hk) = Shortcut::from_str(&cfg.cancel) {
                                if shortcut.mods == hk.mods && shortcut.key == hk.key {
                                    return "cancel".to_string();
                                }
                            }
                            String::new()
                        })
                        .unwrap_or_default();

                    if action.is_empty() {
                        return;
                    }

                    println!("hotkey event: {} state={:?}", action, event.state);

                    // Emit a debug event so the frontend can show key press status
                    app.emit(
                        "debug-hotkey-event",
                        serde_json::json!({
                            "action": action,
                            "accelerator": shortcut.to_string(),
                            "state": format!("{:?}", event.state),
                        }),
                    )
                    .ok();

                    match (action.as_str(), event.state) {
                        ("ptt", ShortcutState::Pressed) => {
                            app.emit("hotkey-ptt-pressed", ()).ok();
                        }
                        ("ptt", ShortcutState::Released) => {
                            app.emit("hotkey-ptt-released", ()).ok();
                        }
                        ("free_speech", ShortcutState::Pressed) => {
                            app.emit("hotkey-free-speech", ()).ok();
                        }
                        ("cancel", ShortcutState::Pressed) => {
                            app.emit("hotkey-cancel", ()).ok();
                        }
                        _ => {}
                    }
                })
                .build(),
        )
        .setup(|app| {
            let portable = is_portable();

            // ── Portable mode: redirect config & data paths ──────────────
            if portable {
                let cfg_path = exe_dir().join("config.json");
                // Tell vtl_core::config::load/save which file to use
                std::env::set_var("VTYPELESS_CONFIG_PATH", cfg_path.to_string_lossy().as_ref());
                println!("portable mode: config → {:?}", cfg_path);
            }

            let d = data_dir(portable);
            if !portable {
                // Only create the %APPDATA% subdirectory in installed mode;
                // in portable mode the exe directory already exists.
                std::fs::create_dir_all(&d).ok();
            }
            let history_path = d.join("history.json");
            let history = history_io::load_history(&history_path);

            let config = vtl_core::config::load().unwrap_or_default();
            let recorder = Recorder::new();
            let player = Player::new();
            if !config.audio.enable_sounds {
                player.set_enabled(false);
            }
            let app_handle = app.handle();

            // ── Register global hotkeys from config ────────────────────────
            let hotkey_labels = ["push_to_talk", "free_speech", "cancel"];
            let hotkey_values = [
                &config.hotkey.push_to_talk,
                &config.hotkey.free_speech,
                &config.hotkey.cancel,
            ];
            let mut reg_results = Vec::new();
            for (label, hotkey_str) in hotkey_labels.iter().zip(hotkey_values.iter()) {
                let (ok, error) = if hotkey_str.is_empty() {
                    (false, String::from("empty hotkey string"))
                } else {
                    match app.global_shortcut().register(hotkey_str.as_str()) {
                        Ok(_) => {
                            println!("hotkey registered: {} ({})", hotkey_str, label);
                            (true, String::new())
                        }
                        Err(e) => {
                            let msg = format!(
                                "hotkey registration FAILED '{}' ({}): {} — other programs may be using this key",
                                hotkey_str, label, e
                            );
                            println!("{}", msg);
                            (false, e.to_string())
                        }
                    }
                };
                reg_results.push(serde_json::json!({
                    "action": label,
                    "hotkey": hotkey_str,
                    "ok": ok,
                    "error": error,
                }));
            }
            // Keep a copy so the frontend can poll after mount (Tauri setup
            // may emit events before the webview's IPC listeners are registered).
            let reg_results_permanent: Vec<serde_json::Value> = reg_results;
            app_handle
                .emit(
                    "hotkey-registration",
                    serde_json::json!({"results": &reg_results_permanent}),
                )
                .ok();

            // ── Attempt to load the ASR engine with progress events ────────
            app_handle
                .emit(
                    "model-loading",
                    serde_json::json!({
                        "progress": 0.3,
                        "stage": "load",
                    }),
                )
                .ok();

            let engine = engine_loader::load_engine(&config);
            match &engine {
                Some(_) => {
                    println!("engine: {} model loaded", config.model.active_model_id);
                    app_handle
                        .emit(
                            "model-ready",
                            serde_json::json!({
                                "modelId": config.model.active_model_id,
                                "device": config.model.device,
                            }),
                        )
                        .ok();
                }
                None => {
                    println!(
                        "engine: model '{}' not available; starting auto-download",
                        config.model.active_model_id
                    );
                    // Emit "model required" so the frontend can show download UI
                    app_handle
                        .emit(
                            "model-required",
                            serde_json::json!({
                                "modelId": config.model.active_model_id,
                                "message": format!(
                                    "Model '{}' not found locally. Downloading…",
                                    config.model.active_model_id
                                ),
                            }),
                        )
                        .ok();

                    // ── Background model download ──────────────────────────
                    // In portable mode models go to ./models/ next to EXE;
                    // in installed mode they go to {config}/VoiceTypeless/models/
                    let models_base = data_dir(portable).join("models");
                    let model_id = config.model.active_model_id.clone();
                    let emit = app_handle.clone();
                    std::thread::spawn(move || {
                        let _ = emit.emit(
                            "model-loading",
                            serde_json::json!({
                                "progress": 0.0,
                                "stage": "download",
                            }),
                        );

                        let result = model_downloader::download_model(
                            &models_base,
                            &model_id,
                            |p| {
                                let fraction = if p.total_bytes > 0 {
                                    p.bytes_written as f64 / p.total_bytes as f64
                                } else {
                                    0.0
                                };
                                let _ = emit.emit(
                                    "model-loading",
                                    serde_json::json!({
                                        "progress": (fraction * 100.0).round() / 100.0,
                                        "stage": "download",
                                    }),
                                );
                            },
                        );

                        match result {
                            Ok(path) => {
                                // Persist the download path in config.json
                                let models_dir =
                                    path.parent().unwrap_or(&path).to_string_lossy().to_string();
                                if let Ok(mut cfg) = vtl_core::config::load() {
                                    cfg.model.models_dir.clone_from(&models_dir);
                                    let _ = vtl_core::config::save(&cfg);
                                }
                                println!("engine: model downloaded to {models_dir}");
                                let _ = emit.emit("model-downloaded", serde_json::json!({
                                    "modelId": model_id,
                                    "path": models_dir,
                                }));
                            }
                            Err(e) => {
                                println!("engine: download failed: {e}");
                                let _ = emit.emit(
                                    "model-error",
                                    serde_json::json!({
                                        "message": format!("Model download failed: {e}"),
                                    }),
                                );
                            }
                        }
                    });
                }
            }

            app.manage(Mutex::new(AppState {
                config,
                history,
                history_path,
                recorder,
                player,
                engine,
                hotkey_registration: reg_results_permanent,
            }));

            use tauri::{
                menu::{MenuBuilder, MenuItemBuilder},
                tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
            };

            let quit =
                MenuItemBuilder::with_id("quit", "Quit Voice-typeless").build(app)?;
            let show = MenuItemBuilder::with_id("show", "Show / Hide").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Voice-typeless")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                window.hide().ok();
                            } else {
                                window.show().ok();
                                window.set_focus().ok();
                            }
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                window.hide().ok();
                            } else {
                                window.show().ok();
                                window.set_focus().ok();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_recording,
            commands::stop_recording,
            commands::cancel_recording,
            commands::paste_text,
            commands::set_device,
            commands::run_demo,
            commands::get_devices,
            commands::get_model_list,
            commands::set_active_model,
            commands::get_history,
            commands::delete_history_item,
            commands::clear_history,
            commands::export_history_text,
            commands::get_stats,
            commands::get_config,
            commands::set_config,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
            commands::get_engine_status,
            commands::retry_engine,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Voice-typeless");
}
