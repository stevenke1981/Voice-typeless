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

pub(crate) fn available_models(active_id: &str) -> Vec<ModelInfo> {
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
