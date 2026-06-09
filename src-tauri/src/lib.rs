use std::sync::Mutex;
use std::path::PathBuf;
use tauri::{Emitter, Manager, State};

use vtl_core::config::AppConfig;
use vtl_core::history::HistoryItem;

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

// ── Demo texts ─────────────────────────────────────────────────────────────────

const DEMO_TEXTS: &[&str] = &[
    "Hello! Voice-typeless is working. Say anything and it will be transcribed instantly.",
    "This is a demonstration of the voice-to-text feature. Try pressing the hotkey to start.",
    "Voice-typeless supports multiple languages including English, Chinese, Japanese, and more.",
    "You can use push-to-talk mode by holding your hotkey, or free speech mode to speak naturally.",
    "All your voice recordings are processed locally — no data leaves your computer.",
];

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
fn start_recording(app: tauri::AppHandle, mode: String) -> Result<(), String> {
    println!("start_recording: mode={mode}");
    app.emit("recording-started", serde_json::json!({"timestamp": 0}))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn stop_recording(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    app.emit("recording-stopped", serde_json::json!({"duration_ms": 500}))
        .map_err(|e| e.to_string())?;
    // Emit recognition-result after a short delay so status returns to idle
    // (In real implementation, this would come from the ASR engine)
    let app2 = app.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        let _ = app2.emit("recognition-result", serde_json::json!({
            "text": "",
            "language": "en",
            "confidence": 0.0
        }));
    });
    Ok(serde_json::json!({
        "text": "", "language": "en", "confidence": 0.0, "duration_ms": 500
    }))
}

#[tauri::command]
fn cancel_recording(app: tauri::AppHandle) -> Result<(), String> {
    app.emit("recording-cancelled", serde_json::json!(null))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_devices() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![
        serde_json::json!({"id": "default", "name": "Default Microphone"}),
        serde_json::json!({"id": "realtek", "name": "Realtek Audio"}),
        serde_json::json!({"id": "usb",     "name": "USB Microphone"}),
    ])
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
fn set_config(state: State<'_, Mutex<AppState>>, config: serde_json::Value) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let mut merged = serde_json::to_value(&s.config).map_err(|e| e.to_string())?;
    let mut patch = config;
    convert_keys_camel_to_snake(&mut patch);
    deep_merge(&mut merged, patch);
    let new_config: AppConfig = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    s.config = new_config;
    vtl_core::config::save(&s.config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn run_demo(
    app: tauri::AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    app.emit("recording-started", serde_json::json!({"timestamp": ts}))
        .map_err(|e| e.to_string())?;

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    app.emit("recording-stopped", serde_json::json!({"duration_ms": 2000}))
        .map_err(|e| e.to_string())?;

    tokio::time::sleep(std::time::Duration::from_millis(600)).await;

    let idx = (ts as usize) % DEMO_TEXTS.len();
    let text = DEMO_TEXTS[idx].to_string();

    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let item = HistoryItem {
            id: ts.to_string(),
            text: text.clone(),
            language: "en".to_string(),
            timestamp: ts / 1000,
        };
        s.history.insert(0, item);
        if s.history.len() > 50 {
            s.history.truncate(50);
        }
        let path = s.history_path.clone();
        save_history(&path, &s.history).map_err(|e| e.to_string())?;
    }

    app.emit(
        "recognition-result",
        serde_json::json!({"text": text, "language": "en", "confidence": 0.95}),
    )
    .map_err(|e| e.to_string())?;

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

// ── run() ─────────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let config = vtl_core::config::load().unwrap_or_default();
            let dir = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("VoiceTypeless");
            std::fs::create_dir_all(&dir).ok();
            let history_path = dir.join("history.json");
            let history = load_history(&history_path);
            app.manage(Mutex::new(AppState {
                config,
                history,
                history_path,
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
            run_demo,
            get_autostart_enabled,
            set_autostart_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Voice-typeless");
}

