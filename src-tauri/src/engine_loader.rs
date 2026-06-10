use std::path::PathBuf;
use std::str::FromStr;

use log::info;

use vtl_core::config::AppConfig;
use vtl_core::engine::{self as engine_mod, DeviceType, ModelType};

pub(crate) fn load_engine(config: &AppConfig) -> Option<Box<dyn engine_mod::Engine>> {
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
