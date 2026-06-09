use serde::{Deserialize, Serialize};
use std::fmt;
use std::num::NonZeroUsize;
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
    #[serde(rename = "custom-onnx")]
    CustomOnnx,
}

impl ModelType {
    /// Returns the string representation matching the Go engine constants.
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelType::SenseVoice => "sensevoice",
            ModelType::WhisperTiny => "whisper-tiny",
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
}

// ── Engine Trait ──

/// The abstract speech recognition engine interface.
///
/// Mirrors the Go `Engine` interface from `core/engine/engine.go`.
pub trait Engine {
    /// Load a model with the given configuration.
    fn load_model(&mut self, cfg: ModelConfig) -> Result<(), EngineError>;

    /// Transcribe audio samples and return the recognition result.
    fn recognize(&self, audio: &[f32], sample_rate: u32) -> Result<RecognitionResult, EngineError>;

    /// Return metadata about the loaded model.
    fn model_info(&self) -> ModelInfo;

    /// Release engine resources and mark as closed.
    fn close(&mut self) -> Result<(), EngineError>;
}

// ── SenseVoiceEngine ──

/// Stub implementation of the SenseVoice engine.
///
/// Port of Go `senseVoiceEngine` from `core/engine/sensevoice.go`.
/// Real inference via sherpa-onnx is TODO.
#[derive(Debug, Clone)]
pub struct SenseVoiceEngine {
    cfg: ModelConfig,
    ready: bool,
}

impl SenseVoiceEngine {
    /// Create a new uninitialized SenseVoice engine.
    pub fn new() -> Self {
        Self {
            cfg: ModelConfig {
                model_type: ModelType::SenseVoice,
                model_path: String::new(),
                tokens_path: String::new(),
                device: DeviceType::Auto,
                language: String::from("auto"),
                num_threads: 0,
            },
            ready: false,
        }
    }
}

impl Default for SenseVoiceEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine for SenseVoiceEngine {
    fn load_model(&mut self, mut cfg: ModelConfig) -> Result<(), EngineError> {
        if cfg.device == DeviceType::Auto {
            cfg.device = probe_device();
        }
        if cfg.num_threads <= 0 {
            let count = std::thread::available_parallelism()
                .map(NonZeroUsize::get)
                .unwrap_or(4);
            cfg.num_threads = std::cmp::max(1, count / 2) as i32;
        }
        self.cfg = cfg;
        self.ready = true;
        Ok(())
    }

    fn recognize(&self, _audio: &[f32], _sample_rate: u32) -> Result<RecognitionResult, EngineError> {
        if !self.ready {
            return Err(EngineError::ModelNotLoaded);
        }
        Ok(RecognitionResult {
            text: String::new(),
            language: self.cfg.language.clone(),
            confidence: 0.0,
            duration: Duration::ZERO,
            segments: vec![],
        })
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            id: String::from("sensevoice-small"),
            model_type: ModelType::SenseVoice,
            name: String::from("SenseVoice Small"),
            description: String::from("SenseVoice small speech recognition model"),
            size_bytes: 0,
            languages: vec![
                String::from("zh"),
                String::from("en"),
                String::from("ja"),
                String::from("ko"),
            ],
            device: self.cfg.device,
        }
    }

    fn close(&mut self) -> Result<(), EngineError> {
        self.ready = false;
        Ok(())
    }
}

// ── Free Functions ──

/// Probe the system for the best available compute device.
///
/// Port of Go `ProbeDevice()` from `core/engine/device.go`.
/// Currently a stub that always returns `DeviceType::Cpu`.
/// TODO: check DirectML / CUDA availability.
pub fn probe_device() -> DeviceType {
    DeviceType::Cpu
}

/// Create a new engine for the given model type.
///
/// Port of Go `New()` from `core/engine/engine.go`.
pub fn new_engine(model_type: ModelType) -> Result<SenseVoiceEngine, EngineError> {
    match model_type {
        ModelType::SenseVoice => Ok(SenseVoiceEngine::new()),
        _ => Err(EngineError::UnknownModelType(model_type.to_string())),
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_type_as_str() {
        assert_eq!(ModelType::SenseVoice.as_str(), "sensevoice");
        assert_eq!(ModelType::WhisperTiny.as_str(), "whisper-tiny");
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
            "custom-onnx".parse::<ModelType>().unwrap(),
            ModelType::CustomOnnx
        );
        assert!("unknown".parse::<ModelType>().is_err());
    }

    #[test]
    fn test_device_type_from_str() {
        assert_eq!("auto".parse::<DeviceType>().unwrap(), DeviceType::Auto);
        assert_eq!("directml".parse::<DeviceType>().unwrap(), DeviceType::DirectML);
        assert_eq!("cuda".parse::<DeviceType>().unwrap(), DeviceType::Cuda);
        assert_eq!("cpu".parse::<DeviceType>().unwrap(), DeviceType::Cpu);
        assert!("unknown".parse::<DeviceType>().is_err());
    }

    #[test]
    fn test_new_sensevoice_engine() {
        let engine = new_engine(ModelType::SenseVoice).unwrap();
        let info = engine.model_info();
        assert_eq!(info.id, "sensevoice-small");
    }

    #[test]
    fn test_load_model_sets_ready() {
        let mut engine = SenseVoiceEngine::new();
        assert!(!engine.ready);

        let cfg = ModelConfig {
            model_type: ModelType::SenseVoice,
            model_path: String::from("/path/to/model"),
            tokens_path: String::from("/path/to/tokens"),
            device: DeviceType::Cpu,
            language: String::from("auto"),
            num_threads: 2,
        };
        engine.load_model(cfg).unwrap();
        assert!(engine.ready);
    }

    #[test]
    fn test_recognize_before_load_errors() {
        let engine = SenseVoiceEngine::new();
        let result = engine.recognize(&[], 16000);
        assert!(matches!(result, Err(EngineError::ModelNotLoaded)));
    }

    #[test]
    fn test_sensevoice_model_info() {
        let engine = SenseVoiceEngine::new();
        let info = engine.model_info();
        assert_eq!(info.id, "sensevoice-small");
        assert_eq!(info.model_type, ModelType::SenseVoice);
        assert_eq!(info.name, "SenseVoice Small");
        assert!(info.languages.contains(&String::from("zh")));
        assert!(info.languages.contains(&String::from("en")));
        assert!(info.languages.contains(&String::from("ja")));
        assert!(info.languages.contains(&String::from("ko")));
    }

    #[test]
    fn test_close_resets_ready() {
        let mut engine = SenseVoiceEngine::new();
        let cfg = ModelConfig {
            model_type: ModelType::SenseVoice,
            model_path: String::new(),
            tokens_path: String::new(),
            device: DeviceType::Cpu,
            language: String::from("auto"),
            num_threads: 1,
        };
        engine.load_model(cfg).unwrap();
        assert!(engine.ready);
        engine.close().unwrap();
        assert!(!engine.ready);
    }

    #[test]
    fn test_probe_device_returns_cpu_stub() {
        assert_eq!(probe_device(), DeviceType::Cpu);
    }

    #[test]
    fn test_new_engine_unknown_type() {
        assert!(matches!(
            new_engine(ModelType::WhisperTiny),
            Err(EngineError::UnknownModelType(_))
        ));
        assert!(matches!(
            new_engine(ModelType::CustomOnnx),
            Err(EngineError::UnknownModelType(_))
        ));
    }

    #[test]
    fn test_load_model_auto_device() {
        let mut engine = SenseVoiceEngine::new();
        let cfg = ModelConfig {
            model_type: ModelType::SenseVoice,
            model_path: String::new(),
            tokens_path: String::new(),
            device: DeviceType::Auto,
            language: String::from("auto"),
            num_threads: 0,
        };
        engine.load_model(cfg).unwrap();
        assert_eq!(engine.cfg.device, DeviceType::Cpu);
        assert!(engine.cfg.num_threads > 0);
    }
}
