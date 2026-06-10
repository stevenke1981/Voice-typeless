pub mod config_helpers;
pub mod model_info;
pub mod state;
pub mod history_io;
pub mod engine_loader;
pub mod commands;

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use vtl_core::audio::{AudioPlayer, Player, Recorder};

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    let accel = shortcut.to_string();

                    // Read config to match accelerator → action
                    let action = app
                        .state::<Mutex<AppState>>()
                        .lock()
                        .ok()
                        .map(|s| {
                            let cfg = &s.config.hotkey;
                            let lower = accel.to_lowercase();
                            if lower == cfg.push_to_talk.to_lowercase() {
                                "ptt"
                            } else if lower == cfg.free_speech.to_lowercase() {
                                "free_speech"
                            } else if lower == cfg.cancel.to_lowercase() {
                                "cancel"
                            } else {
                                ""
                            }
                        })
                        .unwrap_or_default();

                    if action.is_empty() {
                        return;
                    }

                    match (action, event.state) {
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
            let config = vtl_core::config::load().unwrap_or_default();
            let dir = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("VoiceTypeless");
            std::fs::create_dir_all(&dir).ok();
            let history_path = dir.join("history.json");
            let history = history_io::load_history(&history_path);
            let recorder = Recorder::new();
            let player = Player::new();
            if !config.audio.enable_sounds {
                player.set_enabled(false);
            }
            // Register global hotkeys from config
            for hotkey_str in [
                &config.hotkey.push_to_talk,
                &config.hotkey.free_speech,
                &config.hotkey.cancel,
            ] {
                if !hotkey_str.is_empty() {
                    let _ = app.global_shortcut().register(hotkey_str.as_str());
                }
            }

            // Attempt to load the ASR engine with progress events
            let app_handle = app.handle();
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
                        "engine: model '{}' not available; recognition disabled",
                        config.model.active_model_id
                    );
                    app_handle
                        .emit(
                            "model-error",
                            serde_json::json!({
                                "message": format!(
                                    "Model '{}' could not be loaded. Check models/ directory.",
                                    config.model.active_model_id
                                ),
                            }),
                        )
                        .ok();
                }
            }

            app.manage(Mutex::new(AppState {
                config,
                history,
                history_path,
                recorder,
                player,
                engine,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Voice-typeless");
}
