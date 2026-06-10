use std::sync::Mutex;

use tauri::{Emitter, State};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use vtl_core::audio::{AudioChunk, AudioError, AudioPlayer, AudioRecorder, DeviceEnumerator, Enumerator, RecorderConfig};
use vtl_core::config::AppConfig;

use vtl_core::history::HistoryItem;
use vtl_core::paste::write_clipboard;
use enigo::Keyboard as _;

use crate::config_helpers::*;
use crate::history_io::save_history;
use crate::model_info::*;
use crate::state::AppState;

#[tauri::command]
pub fn start_recording(
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
            app.emit(
                "recognition-error",
                serde_json::json!({
                    "message": format!("Recording failed: {}", e),
                }),
            )
            .ok();
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
pub fn stop_recording(
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
            s.engine
                .as_mut()
                .and_then(|engine| engine.recognize(&chunk.samples, chunk.sample_rate).ok())
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
    app.emit(
        "recognition-result",
        serde_json::json!({
            "text": text,
            "language": language,
            "confidence": confidence,
            "duration_ms": duration_ms,
        }),
    )
    .map_err(|e| e.to_string())?;

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
pub fn cancel_recording(
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
pub fn get_devices() -> Result<Vec<serde_json::Value>, String> {
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
pub fn get_model_list(state: State<'_, Mutex<AppState>>) -> Result<Vec<ModelInfo>, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(available_models(&s.config.model.active_model_id))
}

#[tauri::command]
pub fn set_active_model(
    state: State<'_, Mutex<AppState>>,
    model_id: String,
) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.config.model.active_model_id = model_id;
    vtl_core::config::save(&s.config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_history(
    state: State<'_, Mutex<AppState>>,
    limit: u32,
) -> Result<Vec<HistoryItem>, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    let items: Vec<HistoryItem> = s.history.iter().take(limit as usize).cloned().collect();
    Ok(items)
}

#[tauri::command]
pub fn delete_history_item(
    state: State<'_, Mutex<AppState>>,
    id: String,
) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.history.retain(|item| item.id != id);
    let path = s.history_path.clone();
    save_history(&path, &s.history)
}

#[tauri::command]
pub fn clear_history(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.history.clear();
    let path = s.history_path.clone();
    save_history(&path, &s.history)
}

#[tauri::command]
pub fn export_history_text(state: State<'_, Mutex<AppState>>) -> Result<String, String> {
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
pub fn get_stats(state: State<'_, Mutex<AppState>>) -> Result<serde_json::Value, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    let total_items = s.history.len();
    let total_chars: usize = s.history.iter().map(|item| item.text.chars().count()).sum();
    let mut languages: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
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
pub fn get_config(state: State<'_, Mutex<AppState>>) -> Result<AppConfig, String> {
    let s = state.lock().map_err(|e| e.to_string())?;
    Ok(s.config.clone())
}

#[tauri::command]
pub fn set_device(
    state: State<'_, Mutex<AppState>>,
    device: String,
) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.config.model.device = device;
    vtl_core::config::save(&s.config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn run_demo(app: tauri::AppHandle) -> Result<(), String> {
    app.emit(
        "recognition-result",
        serde_json::json!({
            "text": "這是一個語音辨識示範結果。",
            "language": "zh",
            "confidence": 0.95,
            "duration_ms": 1500,
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_config(
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
        for old in [
            old_hotkey_cfg.push_to_talk.as_str(),
            old_hotkey_cfg.free_speech.as_str(),
            old_hotkey_cfg.cancel.as_str(),
        ] {
            if !old.is_empty() {
                let _ = app.global_shortcut().unregister(old);
            }
        }
        for new in [
            new_hotkey.push_to_talk.as_str(),
            new_hotkey.free_speech.as_str(),
            new_hotkey.cancel.as_str(),
        ] {
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
pub fn get_autostart_enabled() -> Result<bool, String> {
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
pub fn set_autostart_enabled(enable: bool) -> Result<(), String> {
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
pub fn paste_text(text: String) -> Result<(), String> {
    do_paste(&text)
}
