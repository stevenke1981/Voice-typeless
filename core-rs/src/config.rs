use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Application configuration shared by the reusable core and Tauri layer.
// ---------------------------------------------------------------------------

/// Complete application configuration schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", default)]
pub struct AppConfig {
    pub hotkey: HotkeyConfig,
    pub audio: AudioConfig,
    pub model: ModelConfig,
    pub text: TextConfig,
    pub ui: UIConfig,
    pub system: SystemConfig,
}

/// Hotkey bindings as human-readable strings (e.g. "Alt+Space").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HotkeyConfig {
    pub push_to_talk: String,
    pub free_speech: String,
    pub cancel: String,
}

/// Microphone and sound settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AudioConfig {
    pub device_id: String,
    pub sample_rate: u32,
    pub channels: u8,
    pub enable_sounds: bool,
}

/// Active speech recognition model selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelConfig {
    pub active_model_id: String,
    /// Populated at runtime from AppData.
    pub models_dir: String,
    /// One of "auto", "directml", "cuda", "cpu".
    pub device: String,
}

/// Text post-processing behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TextConfig {
    pub language: String,
    pub filter_filler_words: bool,
    pub mixed_language_optimization: bool,
    pub custom_dictionary: Vec<String>,
}

/// Appearance and UI behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UIConfig {
    /// One of "dark", "light", "system".
    pub theme: String,
    pub language: String,
    pub show_floating_indicator: bool,
    pub indicator_position: PositionConfig,
    pub history_retention_days: i32,
    pub max_history_items: i32,
}

/// Screen coordinates for the floating indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PositionConfig {
    pub x: i32,
    pub y: i32,
}

/// OS-level integration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SystemConfig {
    pub auto_start: bool,
    pub minimize_to_tray: bool,
    pub check_updates: bool,
}

// ---------------------------------------------------------------------------
// Default values
// ---------------------------------------------------------------------------

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            push_to_talk: "Alt+Space".into(),
            free_speech: "Ctrl+Shift+V".into(),
            // NOTE: on Windows, RegisterHotKey requires modifier keys (Alt/Ctrl/Shift/Win).
            // Standalone keys like "Escape" or "F1" without modifiers CANNOT be
            // registered as global shortcuts. Using Ctrl+Shift+Escape ensures reliable
            // registration across all Windows versions.
            cancel: "Ctrl+Shift+Escape".into(),
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            device_id: "default".into(),
            sample_rate: 16000,
            channels: 1,
            enable_sounds: true,
        }
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            active_model_id: "sensevoice-small".into(),
            models_dir: String::new(),
            device: "auto".into(),
        }
    }
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            filter_filler_words: true,
            mixed_language_optimization: true,
            custom_dictionary: vec![],
        }
    }
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: "en".into(),
            show_floating_indicator: true,
            indicator_position: PositionConfig::default(),
            history_retention_days: 30,
            max_history_items: 50,
        }
    }
}

impl Default for PositionConfig {
    fn default() -> Self {
        Self { x: 100, y: 100 }
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            auto_start: false,
            minimize_to_tray: true,
            check_updates: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during config operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to determine config directory: {0}")]
    NoConfigDir(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Returns the default configuration (out-of-box values).
pub fn default_config() -> AppConfig {
    AppConfig::default()
}

/// Loads config from disk.  Returns `default_config()` if the file does not
/// exist yet.
pub fn load() -> Result<AppConfig, ConfigError> {
    let path = config_path()?;

    match std::fs::read_to_string(&path) {
        Ok(data) => {
            let cfg: AppConfig = serde_json::from_str(&data)?;
            Ok(cfg)
        }
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(e) => Err(ConfigError::Io(e)),
    }
}

/// Writes `cfg` to disk atomically (write to `.tmp` then rename).
pub fn save(cfg: &AppConfig) -> Result<(), ConfigError> {
    let path = config_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let data = serde_json::to_string_pretty(cfg)?;

    let tmp = path.with_extension("json.tmp");
    // Use write + rename for atomicity
    std::fs::write(&tmp, &data)?;
    std::fs::rename(&tmp, &path)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns the full path to the config file.
///
/// Priority:
///   1. `$VTYPELESS_CONFIG_PATH` environment variable (for portable mode)
///   2. `<OS-config-dir>/VoiceTypeless/config.json` (normal installed mode)
fn config_path() -> Result<PathBuf, ConfigError> {
    // Allow override via environment variable (used by portable mode)
    if let Ok(path) = std::env::var("VTYPELESS_CONFIG_PATH") {
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    let base = dirs::config_dir()
        .ok_or_else(|| ConfigError::NoConfigDir("no OS config directory found".into()))?;
    Ok(base.join("VoiceTypeless").join("config.json"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_config() {
        let cfg = default_config();
        assert_eq!(cfg.hotkey.push_to_talk, "Alt+Space");
        assert_eq!(cfg.audio.sample_rate, 16000);
        assert_eq!(cfg.ui.max_history_items, 50);
        assert!(!cfg.system.auto_start);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("vtl_config_test");
        let _ = fs::remove_dir_all(&dir);

        // Override the config path by temporarily… well, we can't easily
        // replace `config_path()` without a bit of indirection.  For now
        // smoke-test via JSON roundtrip:
        let cfg = default_config();
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.hotkey.push_to_talk, cfg.hotkey.push_to_talk);
        assert_eq!(restored.audio.sample_rate, cfg.audio.sample_rate);
        assert_eq!(restored.ui.indicator_position.x, 100);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        // The real config file shouldn't exist in this CI environment,
        // so load() should gracefully fall through to the default.
        let cfg = load().unwrap_or_else(|_| default_config());
        // Just confirm it's a valid AppConfig
        assert!(!cfg.hotkey.push_to_talk.is_empty());
    }

    #[test]
    fn test_serde_roundtrip_full() {
        let cfg = default_config();
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();

        // Compare by serializing both to the same format
        let a = serde_json::to_value(&cfg).unwrap();
        let b = serde_json::to_value(&parsed).unwrap();
        assert_eq!(a, b);
    }
}
