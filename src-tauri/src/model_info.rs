use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) model_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) languages: Vec<String>,
    pub(crate) is_active: bool,
    pub(crate) is_downloaded: bool,
    pub(crate) device: Option<String>,
}

/// Checks whether model files exist on disk for a given model directory + id.
fn model_on_disk(config_models_dir: &str, model_id: &str) -> bool {
    // Check config-specified directory first
    if !config_models_dir.is_empty() {
        let p = Path::new(config_models_dir).join(model_id).join("model.int8.onnx");
        if p.exists() {
            return true;
        }
    }
    // Check a few known fallback locations
    for base in &[".", "..", "../.."] {
        let p = Path::new(base).join("models").join(model_id).join("model.int8.onnx");
        if let Ok(canon) = p.canonicalize() {
            if canon.exists() {
                return true;
            }
        }
    }
    false
}

pub(crate) fn available_models(active_id: &str, config_models_dir: &str) -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "sensevoice-small".into(),
            name: "SenseVoice Small".into(),
            model_type: "sensevoice".into(),
            size_bytes: 38_000_000,
            languages: vec!["zh".into(), "en".into(), "ja".into(), "ko".into()],
            is_active: active_id == "sensevoice-small",
            is_downloaded: model_on_disk(config_models_dir, "sensevoice-small"),
            device: Some("directml".into()),
        },
        ModelInfo {
            id: "whisper-tiny".into(),
            name: "Whisper Tiny".into(),
            model_type: "whisper-tiny".into(),
            size_bytes: 75_000_000,
            languages: vec!["en".into(), "zh".into()],
            is_active: active_id == "whisper-tiny",
            is_downloaded: model_on_disk(config_models_dir, "whisper-tiny"),
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
