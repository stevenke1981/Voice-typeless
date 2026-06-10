use std::path::PathBuf;
use std::str::FromStr;

use log::info;

use vtl_core::config::AppConfig;
use vtl_core::engine::{self as engine_mod, DeviceType, ModelType};

/// Resolve the models directory by trying a sequence of candidate paths.
///
/// Priority order:
///   1. Explicit `models_dir` from config (already set)
///   2. CWD-relative  `models/`          (developer from project root)
///   3. CWD-relative  `../models/`       (cargo tauri dev from src-tauri/)
///   4. Exec-rel      `../models/`       (target/debug/ → project root)
///   5. Config-dir    `{config}/VoiceTypeless/models/`  (installed app)
fn resolve_models_dir(config: &AppConfig) -> Option<PathBuf> {
    // 1. Explicit path from config
    if !config.model.models_dir.is_empty() {
        let p = PathBuf::from(&config.model.models_dir);
        if p.is_dir() {
            return Some(p);
        }
    }

    // 2–3. CWD-relative candidates
    for rel in &["models", "../models"] {
        let candidate = PathBuf::from(rel);
        if candidate.is_dir() {
            return Some(candidate);
        }
    }

    // 4. Executable-relative candidates
    //
    // Priority within this group:
    //   a) `./models/`        — portable mode (EXE + models/ side by side)
    //   b) `../models/`       — debug build (target/debug/ → project root)
    //   c) `../../models/`    — deeper nesting
    //   d) `../../../models/` — release build (target/release/ → project root)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            for rel in &["./models", "../models", "../../models", "../../../models"] {
                let candidate = exe_dir.join(rel);
                if candidate.is_dir() {
                    return Some(candidate);
                }
            }
        }
    }

    // 5. Config directory fallback
    if let Some(cfg_dir) = dirs::config_dir() {
        let candidate = cfg_dir.join("VoiceTypeless").join("models");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }

    // ══ debug: show what we tried ═══════════════════════════════════════════
    println!("engine: model directory not found — tried:");
    if let Ok(exe) = std::env::current_exe() {
        println!("  exe location: {:?}", exe);
        if let Some(exe_dir) = exe.parent() {
            for rel in &["./models", "../models", "../../models", "../../../models"] {
                println!("  {} → {:?}", rel, exe_dir.join(rel));
            }
        }
    } else {
        println!("  (could not get exe path)");
    }
    println!("  CWD models/  → {:?}", std::env::current_dir().map(|d| d.join("models")));
    println!("  CWD ../models/ → {:?}", std::env::current_dir().map(|d| d.join("../models")));
    // ────────────────────────────────────────────────────────────────────────

    None
}

pub(crate) fn load_engine(config: &AppConfig) -> Option<Box<dyn engine_mod::Engine>> {
    let models_dir = resolve_models_dir(config)?;
    let models_dir_str = models_dir.to_string_lossy().to_string();
    info!("Using models directory: {:?}", models_dir);

    if config.model.active_model_id.is_empty() {
        info!("Model configuration incomplete: active_model_id not set");
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
        id if id.starts_with("whisper-cpp") => ModelType::WhisperCpp,
        id if id.starts_with("whisper") => ModelType::WhisperTiny,
        _ => ModelType::SenseVoice,
    };

    // Build model file path (returns None if files don't exist)
    let (model_path, tokens_path) =
        build_model_paths(&models_dir_str, &config.model.active_model_id, &model_type)?;

    let engine_cfg = engine_mod::ModelConfig {
        model_type,
        model_path,
        tokens_path,
        device,
        language,
        num_threads: 0, // auto
    };

    let mut engine = match engine_mod::new_engine(model_type) {
        Ok(e) => e,
        Err(e) => {
            println!(
                "engine: could not create engine for {}: {}",
                model_type, e
            );
            return None;
        }
    };

    match engine.load_model(engine_cfg) {
        Ok(()) => Some(engine),
        Err(e) => {
            println!("engine: load failed: {}", e);
            None
        }
    }
}

/// Build model file path and tokens path for a given model type.
/// Returns `None` if the required model file does not exist on disk.
pub(crate) fn build_model_paths(
    models_dir: &str,
    model_id: &str,
    model_type: &ModelType,
) -> Option<(String, String)> {
    match model_type {
        ModelType::SenseVoice | ModelType::WhisperTiny => {
            let model_path = format!("{}/{}/model.int8.onnx", models_dir, model_id);
            if !std::path::Path::new(&model_path).exists() {
                println!("engine: model file not found at '{}'", model_path);
                return None;
            }
            let tokens_path = format!("{}/{}/tokens.txt", models_dir, model_id);
            Some((model_path, tokens_path))
        }
        ModelType::WhisperCpp => {
            // Whisper.cpp models are single GGML / GGUF files
            let ggml_path = format!("{}/{}/ggml-model.bin", models_dir, model_id);
            let gguf_path = format!("{}/{}/ggml-model.gguf", models_dir, model_id);
            let model_path = if std::path::Path::new(&ggml_path).exists() {
                ggml_path
            } else if std::path::Path::new(&gguf_path).exists() {
                gguf_path
            } else {
                println!(
                    "engine: whisper-cpp model not found at '{}' or '{}'",
                    ggml_path, gguf_path
                );
                return None;
            };
            Some((model_path, String::new()))
        }
        _ => {
            println!(
                "engine: unsupported model type '{}' in path builder",
                model_type
            );
            None
        }
    }
}
