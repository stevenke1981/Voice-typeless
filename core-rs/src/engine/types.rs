use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;

// ── ModelType ──

/// Supported speech recognition model types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelType {
    #[serde(rename = "sensevoice")]
    SenseVoice,
    #[serde(rename = "whisper-tiny")]
    WhisperTiny,
    #[serde(rename = "whisper-cpp")]
    WhisperCpp,
    #[serde(rename = "custom-onnx")]
    CustomOnnx,
}

impl ModelType {
    /// Returns the string representation matching the Go engine constants.
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelType::SenseVoice => "sensevoice",
            ModelType::WhisperTiny => "whisper-tiny",
            ModelType::WhisperCpp => "whisper-cpp",
            ModelType::CustomOnnx => "custom-onnx",
        }
    }
}

impl fmt::Display for ModelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ModelType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sensevoice" => Ok(ModelType::SenseVoice),
            "whisper-tiny" => Ok(ModelType::WhisperTiny),
            "whisper-cpp" => Ok(ModelType::WhisperCpp),
            "custom-onnx" => Ok(ModelType::CustomOnnx),
            _ => Err(format!("unknown model type: {s}")),
        }
    }
}

// ── DeviceType ──

/// Hardware device preference for model inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "directml")]
    DirectML,
    #[serde(rename = "cuda")]
    Cuda,
    #[serde(rename = "cpu")]
    Cpu,
}

impl DeviceType {
    /// Returns the string representation matching the Go device constants.
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceType::Auto => "auto",
            DeviceType::DirectML => "directml",
            DeviceType::Cuda => "cuda",
            DeviceType::Cpu => "cpu",
        }
    }

    /// Convert to sherpa-onnx provider string.
    pub fn to_provider_str(&self) -> &'static str {
        match self {
            DeviceType::Auto => "cpu", // probe falls back to cpu
            DeviceType::DirectML => "directml",
            DeviceType::Cuda => "cuda",
            DeviceType::Cpu => "cpu",
        }
    }
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DeviceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(DeviceType::Auto),
            "directml" => Ok(DeviceType::DirectML),
            "cuda" => Ok(DeviceType::Cuda),
            "cpu" => Ok(DeviceType::Cpu),
            _ => Err(format!("unknown device type: {s}")),
        }
    }
}

// ── Structs ──

/// Configuration for loading a speech recognition model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model_type: ModelType,
    pub model_path: String,
    pub tokens_path: String,
    pub device: DeviceType,
    pub language: String,
    pub num_threads: i32,
}

/// A single transcribed segment with timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub text: String,
    pub start: Duration,
    pub end: Duration,
}

/// The result of a recognition request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognitionResult {
    pub text: String,
    pub language: String,
    pub confidence: f64,
    pub duration: Duration,
    pub segments: Vec<Segment>,
}

/// Static metadata about a loaded model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub model_type: ModelType,
    pub name: String,
    pub description: String,
    pub size_bytes: u64,
    pub languages: Vec<String>,
    pub device: DeviceType,
}

// ── Error ──

/// Errors that can occur during engine operations.
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("engine: model not loaded; call load_model first")]
    ModelNotLoaded,
    #[error("engine: unknown model type \"{0}\"")]
    UnknownModelType(String),
    #[error("engine: model load error: {0}")]
    ModelLoadError(String),
    #[error("engine: decode error: {0}")]
    DecodeError(String),
    #[error("engine: whisper.cpp error: {0}")]
    WhisperError(String),
    #[error("engine: whisper model not found: {0}")]
    WhisperModelNotFound(String),
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_type_as_str() {
        assert_eq!(ModelType::SenseVoice.as_str(), "sensevoice");
        assert_eq!(ModelType::WhisperTiny.as_str(), "whisper-tiny");
        assert_eq!(ModelType::WhisperCpp.as_str(), "whisper-cpp");
        assert_eq!(ModelType::CustomOnnx.as_str(), "custom-onnx");
    }

    #[test]
    fn test_device_type_as_str() {
        assert_eq!(DeviceType::Auto.as_str(), "auto");
        assert_eq!(DeviceType::DirectML.as_str(), "directml");
        assert_eq!(DeviceType::Cuda.as_str(), "cuda");
        assert_eq!(DeviceType::Cpu.as_str(), "cpu");
    }

    #[test]
    fn test_model_type_display() {
        assert_eq!(format!("{}", ModelType::SenseVoice), "sensevoice");
        assert_eq!(format!("{}", ModelType::WhisperTiny), "whisper-tiny");
        assert_eq!(format!("{}", ModelType::WhisperCpp), "whisper-cpp");
        assert_eq!(format!("{}", ModelType::CustomOnnx), "custom-onnx");
    }

    #[test]
    fn test_device_type_display() {
        assert_eq!(format!("{}", DeviceType::Auto), "auto");
        assert_eq!(format!("{}", DeviceType::DirectML), "directml");
        assert_eq!(format!("{}", DeviceType::Cuda), "cuda");
        assert_eq!(format!("{}", DeviceType::Cpu), "cpu");
    }

    #[test]
    fn test_model_type_from_str() {
        assert_eq!(
            "sensevoice".parse::<ModelType>().unwrap(),
            ModelType::SenseVoice
        );
        assert_eq!(
            "whisper-tiny".parse::<ModelType>().unwrap(),
            ModelType::WhisperTiny
        );
        assert_eq!(
            "whisper-cpp".parse::<ModelType>().unwrap(),
            ModelType::WhisperCpp
        );
        assert_eq!(
            "custom-onnx".parse::<ModelType>().unwrap(),
            ModelType::CustomOnnx
        );
        assert!("unknown".parse::<ModelType>().is_err());
    }

    #[test]
    fn test_device_type_from_str() {
        assert_eq!("auto".parse::<DeviceType>().unwrap(), DeviceType::Auto);
        assert_eq!(
            "directml".parse::<DeviceType>().unwrap(),
            DeviceType::DirectML
        );
        assert_eq!("cuda".parse::<DeviceType>().unwrap(), DeviceType::Cuda);
        assert_eq!("cpu".parse::<DeviceType>().unwrap(), DeviceType::Cpu);
        assert!("unknown".parse::<DeviceType>().is_err());
    }

    #[test]
    fn test_device_type_to_provider_str() {
        assert_eq!(DeviceType::Auto.to_provider_str(), "cpu");
        assert_eq!(DeviceType::DirectML.to_provider_str(), "directml");
        assert_eq!(DeviceType::Cuda.to_provider_str(), "cuda");
        assert_eq!(DeviceType::Cpu.to_provider_str(), "cpu");
    }
}
