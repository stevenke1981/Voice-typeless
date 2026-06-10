use std::str::FromStr;
use std::sync::Mutex;
use log::info;
use std::path::PathBuf;
use tauri::{Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use vtl_core::audio::{AudioError, AudioPlayer, AudioChunk, AudioRecorder, DeviceEnumerator, Enumerator, Player, Recorder, RecorderConfig};
use vtl_core::config::AppConfig;
use vtl_core::engine::{self as engine_mod, Engine as _, ModelType, DeviceType, SenseVoiceEngine};
use vtl_core::history::HistoryItem;
use vtl_core::paste::write_clipboard;
use enigo::Keyboard as _;

// ── Config helpers ─────────────────────────────────────────────────────────────

fn camel_to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            for lower in c.to_lowercase() {
                result.push(lower);
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn convert_keys_camel_to_snake(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                let snake = camel_to_snake(&key);
                if snake != key {
                    if let Some(v) = map.remove(&key) {
                        map.insert(snake, v);
                    }
                }
            }
            for v in map.values_mut() {
                convert_keys_camel_to_snake(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                convert_keys_camel_to_snake(v);
            }
        }
        _ => {}
    }
}

fn deep_merge(a: &mut serde_json::Value, b: serde_json::Value) {
    match (a, b) {
        (serde_json::Value::Object(ref mut a_map), serde_json::Value::Object(b_map)) => {
            for (k, v) in b_map {
                if a_map.contains_key(&k) && v.is_object() {
                    deep_merge(&mut a_map[&k], v);
                } else {
                    a_map.insert(k, v);
                }
            }
        }
        (a, b) => *a = b,
    }
}

// ── Model info ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ModelInfo {
    id: String,
    name: String,
    #[serde(rename = "type")]
    model_type: String,
    size_bytes: u64,
    languages: Vec<String>,
    is_active: bool,
    is_downloaded: bool,
    device: Option<String>,
}

fn available_models(active_id: &str) -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "sensevoice-small".into(),
            name: "SenseVoice Small".into(),
            model_type: "sensevoice".into(),
            size_bytes: 38_000_000,
            languages: vec!["zh".into(), "en".into(), "ja".into(), "ko".into()],
            is_active: active_id == "sensevoice-small",
            is_downloaded: true,
            device: Some("directml".into()),
        },
        ModelInfo {
            id: "whisper-tiny".into(),
            name: "Whisper Tiny".into(),
            model_type: "whisper-tiny".into(),
            size_bytes: 75_000_000,
            languages: vec!["en".into(), "zh".into()],
            is_active: active_id == "whisper-tiny",
            is_downloaded: true,
            device: Some("cpu".into()),
        },
        ModelInfo {
            id: "custom-onnx".into(),
            name: "Custom ONNX".into(),
            model_type: "custom-onnx".into(),
            size_bytes: 0,
            languages: vec![],
            is_active: active_id == "custom-onnx",
            is_downloaded: false,
            device: None,
        },
    ]
}

// ── App state ──────────────────────────────────────────────────────────────────

struct AppState {
    config: AppConfig,
    history: Vec<HistoryItem>,
    history_path: PathBuf,
    recorder: Recorder,
    player: Player,
    engine: Option<SenseVoiceEngine>,
}

// ── I/O helpers ────────────────────────────────────────────────────────────────

fn load_history(path: &PathBuf) -> Vec<HistoryItem> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_history(path: &PathBuf, items: &[HistoryItem]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(items).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
fn start_recording(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
    mode: String,
) -> Result<(), String> {
    println!("start_recording: mode={mode}");
    let s = state.lock().map_err(|e| e.to_string())?;
    let cfg = RecorderConfig {
        device_id: s.config.audio.device_id.clone(),
        sample_rate: s.config.audio.sample_rate,
        channels: s.config.audio.channels as u32,
        buffer_size: 0,
    };
    // Ignore AlreadyRecording — treat as a no-op restart
    match s.recorder.start(cfg) {
        Err(AudioError::AlreadyRecording) => {}
        Err(e) => {
            app.emit("recognition-error", serde_json::json!({
                "message": format!("Recording failed: {}", e),
            })).ok();
            return Err(e.to_string());
        }
        Ok(_) => {}
    }
    // Play start tone (no-op if sounds disabled via set_enabled)
    let _ = s.player.play_start();
    drop(s);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    app.emit("recording-started", serde_json::json!({"timestamp": ts}))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn stop_recording(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<serde_json::Value, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    // Gracefully handle "not recording" edge case
    let chunk = match s.recorder.stop() {
        Ok(()) => {
            let _ = s.player.play_stop();
            s.recorder.drain().unwrap_or(AudioChunk {
                samples: vec![],
                sample_rate: 16000,
                captured_at: std::time::SystemTime::now(),
            })
        }
        Err(AudioError::NotRecording) => {
            let _ = s.player.play_stop();
            app.emit("recording-stopped", serde_json::json!({"duration_ms": 0}))
                .ok();
            return Ok(serde_json::json!({
                "text": "", "language": "en", "confidence": 0.0, "duration_ms": 0
            }));
        }
        Err(e) => return Err(e.to_string()),
    };
    // Run ASR if samples are available and engine is loaded
    let recognition = (!chunk.samples.is_empty())
        .then(|| {
            s.engine.as_mut().and_then(|engine| {
                engine.recognize(&chunk.samples, chunk.sample_rate).ok()
            })
        })
        .flatten();
    drop(s);

    let duration_ms = if chunk.sample_rate > 0 {
        (chunk.samples.len() as u64 * 1000) / chunk.sample_rate as u64
    } else {
        0
    };
    app.emit("recording-stopped", serde_json::json!({"duration_ms": duration_ms}))
        .map_err(|e| e.to_string())?;

    let (text, language, confidence) = match recognition {
        Some(r) => (r.text, r.language, r.confidence),
        None => (String::new(), "en".into(), 0.0),
    };
    app.emit("recognition-result", serde_json::json!({
        "text": text,
        "language": language,
        "confidence": confidence,
        "duration_ms": duration_ms,
    })).map_err(|e| e.to_string())?;

    // Auto-paste recognized text into the active application.
    // This is a background best-effort operation — failures are logged but
    // never propagated, so the front-end still receives the result payload.
    if !text.is_empty() {
        if let Err(e) = do_paste(&text) {
            println!("auto-paste: {}", e);
        }
    }

    Ok(serde_json::json!({
        "text": text, "language": language, "confidence": confidence, "duration_ms": duration_ms
    }))
}

#[tauri::command]
fn cancel_recording(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    s.recorder.cancel();
    drop(s);
    app.emit("recording-cancelled", serde_json::json!(null))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_devices() -> Result<Vec<serde_json::Value>, String> {
    let enumerator = Enumerator;
    let devices = enumerator.list_input_devices().map_err(|e| e.to_string())?;
    let mut result = vec![serde_json::json!({
        "id": "default",
        "name": "Default Microphone",
        "is_default": true,
        "channels": 1,
        "sample_rates": [16000],
    })];
    for dev in devices {
        result.push(serde_json::json!({
            "id": dev.id,
            "name": dev.name,
            "is_default": dev.is_default,
            "channels": dev.channels,
            "sample_rates": dev.sample_rates,
        }));
    }
    Ok(result)
}

#[tauri::command]
fn get_model_list(state: State<'_, Mutex<AppState>>) -> Result<Vec<ModelInfo>, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(available_models(&s.config.model.active_model_id))
}

#[tauri::command]
fn set_active_model(
    state: State<'_, Mutex<AppState>>,
    model_id: String,
) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.config.model.active_model_id = model_id;
    vtl_core::config::save(&s.config).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_history(state: State<'_, Mutex<AppState>>, limit: u32) -> Result<Vec<HistoryItem>, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    let items: Vec<HistoryItem> = s.history.iter().take(limit as usize).cloned().collect();
    Ok(items)
}

#[tauri::command]
fn delete_history_item(state: State<'_, Mutex<AppState>>, id: String) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.history.retain(|item| item.id != id);
    let path = s.history_path.clone();
    save_history(&path, &s.history)
}

#[tauri::command]
fn clear_history(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.history.clear();
    let path = s.history_path.clone();
    save_history(&path, &s.history)
}

#[tauri::command]
fn export_history_text(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    if s.history.is_empty() {
        return Ok(String::new());
    }
    let lines: Vec<String> = s
        .history
        .iter()
        .enumerate()
        .map(|(i, item)| format!("{}. [{}] {}", i + 1, item.language.to_uppercase(), item.text))
        .collect();
    Ok(lines.join("\n\n"))
}

#[tauri::command]
fn get_stats(state: State<'_, Mutex<AppState>>) -> Result<serde_json::Value, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    let total_items = s.history.len();
    let total_chars: usize = s.history.iter().map(|item| item.text.chars().count()).sum();
    let mut languages: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for item in &s.history {
        *languages.entry(item.language.clone()).or_insert(0) += 1;
    }
    Ok(serde_json::json!({
        "total_items": total_items,
        "total_chars": total_chars,
        "languages": languages,
    }))
}

#[tauri::command]
fn get_config(state: State<'_, Mutex<AppState>>) -> Result<AppConfig, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.config.clone())
}

#[tauri::command]
fn set_device(state: State<'_, Mutex<AppState>>, device: String) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.config.model.device = device;
    vtl_core::config::save(&s.config).map_err(|e| e.to_string())
}

#[tauri::command]
fn run_demo(app: tauri::AppHandle) -> Result<(), String> {
    app.emit("recognition-result", serde_json::json!({
        "text": "這是一個語音辨識示範結果。",
        "language": "zh",
        "confidence": 0.95,
        "duration_ms": 1500,
    })).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_config(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
    config: serde_json::Value,
) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let old_hotkey_cfg = s.config.hotkey.clone();
    let mut merged = serde_json::to_value(&s.config).map_err(|e| e.to_string())?;
    let mut patch = config;
    convert_keys_camel_to_snake(&mut patch);
    deep_merge(&mut merged, patch);
    let new_config: AppConfig = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    s.config = new_config;
    vtl_core::config::save(&s.config).map_err(|e| e.to_string())?;

    // Re-register hotkeys if bindings changed
    let new_hotkey = &s.config.hotkey;
    if old_hotkey_cfg.push_to_talk != new_hotkey.push_to_talk
        || old_hotkey_cfg.free_speech != new_hotkey.free_speech
        || old_hotkey_cfg.cancel != new_hotkey.cancel
    {
        for old in [old_hotkey_cfg.push_to_talk.as_str(), old_hotkey_cfg.free_speech.as_str(), old_hotkey_cfg.cancel.as_str()] {
            if !old.is_empty() {
                let _ = app.global_shortcut().unregister(old);
            }
        }
        for new in [new_hotkey.push_to_talk.as_str(), new_hotkey.free_speech.as_str(), new_hotkey.cancel.as_str()] {
            if !new.is_empty() {
                app.global_shortcut()
                    .register(new)
                    .map_err(|e| format!("failed to register hotkey '{}': {}", new, e))?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn get_autostart_enabled() -> Result<bool, String> {
    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        match hkcu.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run") {
            Ok(run_key) => {
                let value: Result<String, _> = run_key.get_value("VoiceTypeless");
                Ok(value.is_ok())
            }
            Err(_) => Ok(false),
        }
    }
    #[cfg(not(windows))]
    {
        Ok(false)
    }
}

#[tauri::command]
fn set_autostart_enabled(enable: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        use winreg::{
            enums::{HKEY_CURRENT_USER, KEY_WRITE},
            RegKey,
        };
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let run_key = hkcu
            .open_subkey_with_flags(
                "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
                KEY_WRITE,
            )
            .map_err(|e| e.to_string())?;
        if enable {
            let exe_path = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .to_string();
            run_key
                .set_value("VoiceTypeless", &exe_path)
                .map_err(|e| e.to_string())?;
        } else {
            // Ignore error if key doesn't exist
            let _ = run_key.delete_value("VoiceTypeless");
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = enable;
        Ok(())
    }
}

/// Core paste helper: write text to clipboard then simulate paste keystroke (Ctrl+V / Cmd+V).
///
/// Uses Control on Windows/Linux, Meta (Cmd) on macOS.
fn do_paste(text: &str) -> Result<(), String> {
    write_clipboard(text).map_err(|e| format!("clipboard write failed: {}", e))?;

    // Brief wait for clipboard to settle before pasting
    std::thread::sleep(std::time::Duration::from_millis(50));

    let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
        .map_err(|e| format!("enigo init failed: {}", e))?;

    let modifier = if cfg!(target_os = "macos") {
        enigo::Key::Meta
    } else {
        enigo::Key::Control
    };

    // Modifier down
    enigo
        .key(modifier, enigo::Direction::Press)
        .map_err(|e| format!("modifier press failed: {}", e))?;
    // V down + up (click)
    enigo
        .key(enigo::Key::Unicode('v'), enigo::Direction::Click)
        .map_err(|e| format!("v click failed: {}", e))?;
    // Modifier up
    enigo
        .key(modifier, enigo::Direction::Release)
        .map_err(|e| format!("modifier release failed: {}", e))?;

    Ok(())
}

#[tauri::command]
fn paste_text(text: String) -> Result<(), String> {
    do_paste(&text)
}

// ── Engine loading ──────────────────────────────────────────────────────────────

fn load_engine(config: &AppConfig) -> Option<SenseVoiceEngine> {
    let models_dir_str = if config.model.models_dir.is_empty() {
        let fallback = PathBuf::from("models");
        if fallback.is_dir() {
            info!("Using fallback models directory: {:?}", fallback);
            fallback.to_string_lossy().to_string()
        } else {
            info!("Model configuration incomplete: models_dir not set and ./models/ not found");
            return None;
        }
    } else {
        config.model.models_dir.clone()
    };

    if config.model.active_model_id.is_empty() {
        info!("Model configuration incomplete: active_model_id not set");
        return None;
    }

    let model_path = format!(
        "{}/{}/model.int8.onnx",
        models_dir_str, config.model.active_model_id
    );
    let tokens_path = format!(
        "{}/{}/tokens.txt",
        models_dir_str, config.model.active_model_id
    );
    // If model files don't exist, don't bother attempting to load
    if !std::path::Path::new(&model_path).exists() {
        println!("engine: model file not found at '{}'", model_path);
        return None;
    }
    let device = DeviceType::from_str(&config.model.device).unwrap_or(DeviceType::Auto);
    let language = if config.text.language.is_empty() {
        "auto".to_string()
    } else {
        config.text.language.clone()
    };

    // Determine ModelType from active_model_id prefix
    let model_type = match config.model.active_model_id.as_str() {
        id if id.starts_with("sensevoice") => ModelType::SenseVoice,
        id if id.starts_with("whisper") => ModelType::WhisperTiny,
        _ => ModelType::SenseVoice,
    };

    let engine_cfg = engine_mod::ModelConfig {
        model_type,
        model_path,
        tokens_path,
        device,
        language,
        num_threads: 0, // auto
    };

    let mut engine = SenseVoiceEngine::new();
    match engine.load_model(engine_cfg) {
        Ok(()) => Some(engine),
        Err(e) => {
            // Model loading failure is non-fatal; recognition falls back to empty
            println!("engine: load failed: {}", e);
            None
        }
    }
}

// ── run() ─────────────────────────────────────────────────────────────────────

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
            let history = load_history(&history_path);
            let recorder = Recorder::new();
            let player = Player::new();
            if !config.audio.enable_sounds {
                player.set_enabled(false);
            }
            // Register global hotkeys from config
            for hotkey_str in [&config.hotkey.push_to_talk, &config.hotkey.free_speech, &config.hotkey.cancel] {
                if !hotkey_str.is_empty() {
                    let _ = app.global_shortcut().register(hotkey_str.as_str());
                }
            }

            // Attempt to load the ASR engine
            let engine = load_engine(&config);
            match &engine {
                Some(_) => println!("engine: {} model loaded", config.model.active_model_id),
                None => println!("engine: model '{}' not available; recognition disabled", config.model.active_model_id),
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

            let quit = MenuItemBuilder::with_id("quit", "Quit Voice-typeless").build(app)?;
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
            start_recording,
            stop_recording,
            cancel_recording,
            paste_text,
            set_device,
            run_demo,
            get_devices,
            get_model_list,
            set_active_model,
            get_history,
            delete_history_item,
            clear_history,
            export_history_text,
            get_stats,
            get_config,
            set_config,
            get_autostart_enabled,
            set_autostart_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Voice-typeless");
}

