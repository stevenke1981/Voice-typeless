use serde::{Deserialize, Serialize};
use std::fmt;
#[cfg(any(feature = "engine-sensevoice", feature = "engine-whisper-cpp"))]
use std::num::NonZeroUsize;
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;

#[cfg(feature = "engine-sensevoice")]
use sherpa_onnx::{
    OfflineModelConfig, OfflineRecognizer, OfflineRecognizerConfig,
    OfflineSenseVoiceModelConfig,
};
#[cfg(feature = "engine-whisper-cpp")]
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters,
};

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

// ── Engine Trait ──

/// The abstract speech recognition engine interface.
///
/// Mirrors the Go `Engine` interface from `core/engine/engine.go`.
pub trait Engine: Send {
    /// Load a model with the given configuration.
    fn load_model(&mut self, cfg: ModelConfig) -> Result<(), EngineError>;

    /// Transcribe audio samples and return the recognition result.
    fn recognize(&mut self, audio: &[f32], sample_rate: u32) -> Result<RecognitionResult, EngineError>;

    /// Return metadata about the loaded model.
    fn model_info(&self) -> ModelInfo;

    /// Close and release all engine resources.
    fn close(&mut self) -> Result<(), EngineError>;

    /// Returns `true` if a model is currently loaded.
    fn is_loaded(&self) -> bool {
        false
    }
}

// ── SenseVoiceEngine ──

/// SenseVoice engine implementation backed by sherpa-onnx `OfflineRecognizer`.
///
/// Port of Go `senseVoiceEngine` from `core/engine/sensevoice.go`.
#[cfg(feature = "engine-sensevoice")]
pub struct SenseVoiceEngine {
    cfg: ModelConfig,
    recognizer: Option<OfflineRecognizer>,
}

// Manual Debug impl: OfflineRecognizer does not impl Debug.
#[cfg(feature = "engine-sensevoice")]
impl fmt::Debug for SenseVoiceEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SenseVoiceEngine")
            .field("cfg", &self.cfg)
            .field("recognizer_loaded", &self.recognizer.is_some())
            .finish()
    }
}

// Safety: OfflineRecognizer wraps a raw C pointer (ONNX Runtime session).
// It is Send (ownership moves between threads) but not Sync.
#[cfg(feature = "engine-sensevoice")]
unsafe impl Send for SenseVoiceEngine {}

#[cfg(feature = "engine-sensevoice")]
impl SenseVoiceEngine {
    /// Create a new uninitialized SenseVoice engine (no model loaded).
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
            recognizer: None,
        }
    }

    /// Returns `true` if a model is currently loaded.
    pub fn is_loaded(&self) -> bool {
        self.recognizer.is_some()
    }
}

#[cfg(feature = "engine-sensevoice")]
impl Default for SenseVoiceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "engine-sensevoice")]
impl Engine for SenseVoiceEngine {
    fn load_model(&mut self, mut cfg: ModelConfig) -> Result<(), EngineError> {
        // Resolve Auto device
        if cfg.device == DeviceType::Auto {
            cfg.device = probe_device();
        }

        // Resolve thread count: default to half of available parallelism
        if cfg.num_threads <= 0 {
            let count = std::thread::available_parallelism()
                .map(NonZeroUsize::get)
                .unwrap_or(4);
            cfg.num_threads = std::cmp::max(1, count / 2) as i32;
        }

        // Map device to sherpa-onnx provider string
        let provider = cfg.device.to_provider_str();

        // Build sherpa-onnx config using Default builder pattern.
        // Sherpa-onnx v1.13.2 Rust API:
        //   - sense_voice field is a direct OfflineSenseVoiceModelConfig (not Box/Option)
        //   - tokens is on OfflineModelConfig, not on the model-specific config
        //   - provider is Option<String> on OfflineModelConfig
        //   - decoding_method is Option<String> on OfflineRecognizerConfig
        //   - FeatureConfig default is sample_rate=16000, feature_dim=80
        let mut recognizer_config = OfflineRecognizerConfig::default();
        recognizer_config.model_config = OfflineModelConfig {
            sense_voice: OfflineSenseVoiceModelConfig {
                model: Some(cfg.model_path.clone()),
                language: Some(cfg.language.clone()),
                use_itn: true,
            },
            tokens: Some(cfg.tokens_path.clone()),
            num_threads: cfg.num_threads,
            provider: Some(provider.to_string()),
            ..Default::default()
        };
        recognizer_config.decoding_method = Some("greedy_search".into());

        // Store config before attempting recognizer creation
        // so resolved device/threads are persisted even on failure
        self.cfg = cfg;

        // Create the recognizer
        let recognizer = OfflineRecognizer::create(&recognizer_config)
            .ok_or_else(|| {
                let msg = format!(
                    "failed to create OfflineRecognizer (model={}, tokens={}, provider={})",
                    self.cfg.model_path, self.cfg.tokens_path, provider
                );
                EngineError::ModelLoadError(msg)
            })?;

        self.recognizer = Some(recognizer);
        Ok(())
    }

    fn recognize(
        &mut self,
        audio: &[f32],
        sample_rate: u32,
    ) -> Result<RecognitionResult, EngineError> {
        let recognizer = self
            .recognizer
            .as_ref()
            .ok_or(EngineError::ModelNotLoaded)?;

        // sherpa-onnx requires 16 kHz input
        let sr = sample_rate as i32;

        // Create an offline stream, feed audio, decode
        let stream = recognizer.create_stream();
        stream.accept_waveform(sr, audio);
        recognizer.decode(&stream);

        let raw_result = stream
            .get_result()
            .ok_or_else(|| EngineError::DecodeError("stream.get_result() returned None".into()))?;
        let raw_text = raw_result.text;

        // Clean SenseVoice language tags from output
        let (cleaned_text, detected_lang) = clean_sensevoice_text(&raw_text);

        // Calculate audio-based duration estimate
        let audio_duration =
            Duration::from_secs_f32(audio.len() as f32 / sample_rate as f32);

        // Build a single segment for the full utterance.
        // Sherpa-onnx v1.13.2 Rust API's OfflineRecognizerResult has per-token
        // timestamps/durations but no segment-level boundaries. We construct
        // one segment covering the full audio duration.
        let segments = if cleaned_text.is_empty() {
            Vec::new()
        } else {
            vec![Segment {
                text: cleaned_text.clone(),
                start: Duration::ZERO,
                end: audio_duration,
            }]
        };

        // sherpa-onnx Rust API does not expose per-frame confidence scores.
        // Returns 1.0 (full confidence) as a placeholder. Revisit when upstream adds this.
        let confidence = if cleaned_text.is_empty() { 0.0 } else { 1.0 };

        // Determine displayed language: detected tag wins, else configured
        let language = if detected_lang != "auto" {
            detected_lang
        } else {
            self.cfg.language.clone()
        };

        Ok(RecognitionResult {
            text: cleaned_text,
            language,
            confidence,
            duration: audio_duration,
            segments,
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

    fn is_loaded(&self) -> bool {
        self.recognizer.is_some()
    }

    fn close(&mut self) -> Result<(), EngineError> {
        self.recognizer = None;
        Ok(())
    }
}

// ── WhisperCppEngine ──

/// Whisper.cpp engine implementation backed by `whisper-rs`.
///
/// Loads GGML/GGUF format Whisper models via the whisper.cpp C library.
#[cfg(feature = "engine-whisper-cpp")]
pub struct WhisperCppEngine {
    cfg: ModelConfig,
    ctx: Option<WhisperContext>,
}

// Manual Debug impl: WhisperContext does not impl Debug.
#[cfg(feature = "engine-whisper-cpp")]
impl fmt::Debug for WhisperCppEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WhisperCppEngine")
            .field("cfg", &self.cfg)
            .field("model_loaded", &self.ctx.is_some())
            .finish()
    }
}

// Safety: WhisperContext wraps a raw C pointer (whisper_context*).
// It is Send (ownership moves between threads) but not Sync.
#[cfg(feature = "engine-whisper-cpp")]
unsafe impl Send for WhisperCppEngine {}

#[cfg(feature = "engine-whisper-cpp")]
impl WhisperCppEngine {
    /// Create a new uninitialized Whisper.cpp engine (no model loaded).
    pub fn new() -> Self {
        Self {
            cfg: ModelConfig {
                model_type: ModelType::WhisperCpp,
                model_path: String::new(),
                tokens_path: String::new(),
                device: DeviceType::Auto,
                language: String::from("auto"),
                num_threads: 0,
            },
            ctx: None,
        }
    }

    /// Returns `true` if a model is currently loaded.
    pub fn is_loaded(&self) -> bool {
        self.ctx.is_some()
    }
}

#[cfg(feature = "engine-whisper-cpp")]
impl Default for WhisperCppEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "engine-whisper-cpp")]
impl Engine for WhisperCppEngine {
    fn load_model(&mut self, mut cfg: ModelConfig) -> Result<(), EngineError> {
        // Resolve Auto device (default to CPU for whisper.cpp)
        if cfg.device == DeviceType::Auto {
            cfg.device = DeviceType::Cpu;
        }

        // Resolve thread count: default to half of available parallelism
        if cfg.num_threads <= 0 {
            let count = std::thread::available_parallelism()
                .map(NonZeroUsize::get)
                .unwrap_or(4);
            cfg.num_threads = std::cmp::max(1, count / 2) as i32;
        }

        // Check that the model file exists
        if !std::path::Path::new(&cfg.model_path).exists() {
            return Err(EngineError::WhisperModelNotFound(cfg.model_path.clone()));
        }

        // Store config
        self.cfg = cfg;

        // Build whisper context parameters
        let ctx_params = WhisperContextParameters::default();

        // Load the model
        let ctx = WhisperContext::new_with_params(&self.cfg.model_path, ctx_params)
            .map_err(|e| EngineError::ModelLoadError(format!(
                "whisper-rs: failed to create context from '{}': {}",
                self.cfg.model_path, e
            )))?;

        self.ctx = Some(ctx);
        Ok(())
    }

    fn recognize(
        &mut self,
        audio: &[f32],
        sample_rate: u32,
    ) -> Result<RecognitionResult, EngineError> {
        let ctx = self
            .ctx
            .as_ref()
            .ok_or(EngineError::ModelNotLoaded)?;

        // Create a processing state from the context
        let mut state = ctx
            .create_state()
            .map_err(|e| EngineError::WhisperError(format!("create_state: {e}")))?;

        // Configure full transcription parameters
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Set number of threads
        params.set_n_threads(self.cfg.num_threads);

        // Set language if not auto
        let lang = self.cfg.language.clone();
        if lang != "auto" {
            params.set_language(Some(&lang));
        }

        // Suppress blank (non-speech tokens)
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);

        // Run inference
        state
            .full(params, audio)
            .map_err(|e| EngineError::WhisperError(format!("full inference: {e}")))?;

        // Collect segments
        let n_segments = state.full_n_segments();
        let mut segments: Vec<Segment> = Vec::with_capacity(n_segments as usize);
        let mut full_text = String::new();

        for i in 0..n_segments {
            let seg = state
                .get_segment(i)
                .ok_or_else(|| EngineError::WhisperError(format!("segment {i} not found")))?;
            let text = seg
                .to_str()
                .map_err(|e| EngineError::WhisperError(format!("segment text: {e}")))?;

            // Timestamps are in centiseconds (10ms units)
            let start_centis = seg.start_timestamp();
            let end_centis = seg.end_timestamp();

            segments.push(Segment {
                text: text.to_string(),
                start: Duration::from_millis(start_centis.max(0) as u64 * 10),
                end: Duration::from_millis(end_centis.max(0) as u64 * 10),
            });

            if !full_text.is_empty() {
                full_text.push(' ');
            }
            full_text.push_str(text);
        }

        // Calculate audio duration
        let audio_duration =
            Duration::from_secs_f32(audio.len() as f32 / sample_rate as f32);

        // Determine confidence:
        // whisper-rs does not expose per-token probability on the Rust API.
        // Returns 1.0 (full confidence) as placeholder. Revisit when upstream adds this.
        let confidence = if full_text.is_empty() { 0.0 } else { 1.0 };

        Ok(RecognitionResult {
            text: full_text,
            language: self.cfg.language.clone(),
            confidence,
            duration: audio_duration,
            segments,
        })
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            id: String::from("whisper-cpp"),
            model_type: ModelType::WhisperCpp,
            name: String::from("Whisper (whisper.cpp)"),
            description: String::from("Whisper model via whisper.cpp"),
            size_bytes: 0,
            languages: vec![
                String::from("en"),
                String::from("zh"),
                String::from("ja"),
                String::from("de"),
                String::from("fr"),
                String::from("es"),
                String::from("ru"),
            ],
            device: self.cfg.device,
        }
    }

    fn is_loaded(&self) -> bool {
        self.ctx.is_some()
    }

    fn close(&mut self) -> Result<(), EngineError> {
        // Drop the context; the WhisperContext destructor frees the C resources
        self.ctx = None;
        Ok(())
    }
}

// ── Free Functions ──

/// Strip SenseVoice language/control tags from transcription text
/// and return (cleaned_text, detected_language).
///
/// SenseVoice prefixes output with tags like `<|zh|>`, `<|en|>`, `<|ja|>`,
/// `<|ko|>`, `<|yue|>`. Returns `"auto"` as detected language if no tag found.
#[cfg(feature = "engine-sensevoice")]
fn clean_sensevoice_text(text: &str) -> (String, String) {
    let mut detected = "auto";
    let mut cleaned = text.to_string();

    // Known SenseVoice language tags — check and strip each
    for tag in &["<|zh|>", "<|en|>", "<|ja|>", "<|ko|>", "<|yue|>"] {
        if cleaned.contains(tag) {
            detected = &tag[2..tag.len() - 2]; // extract "zh" from "<|zh|>"
            cleaned = cleaned.replace(tag, "");
        }
    }

    // Also strip other standard SenseVoice control tags:
    // <|sot|>, <|eot|>, <|nospeech|>, <|emotion|>, <|text_only|>, etc.
    let control_tags = [
        "<|sot|>",
        "<|eot|>",
        "<|nospeech|>",
        "<|text_only|>",
        "<|emotion|>",
    ];
    for tag in &control_tags {
        cleaned = cleaned.replace(tag, "");
    }

    (cleaned.trim().to_string(), detected.to_string())
}

/// Probe the system for the best available compute device.
///
/// Port of Go `ProbeDevice()` from `core/engine/device.go`.
/// Uses DirectML on Windows, CPU fallback elsewhere.
pub fn probe_device() -> DeviceType {
    #[cfg(target_os = "windows")]
    {
        // For Windows, use DirectML by default (best GPU acceleration for sherpa-onnx)
        DeviceType::DirectML
    }
    #[cfg(not(target_os = "windows"))]
    {
        // macOS/Linux fallback to CPU
        DeviceType::Cpu
    }
}

/// Create a new engine for the given model type.
///
/// Port of Go `New()` from `core/engine/engine.go`.
pub fn new_engine(model_type: ModelType) -> Result<Box<dyn Engine>, EngineError> {
    match model_type {
        #[cfg(feature = "engine-sensevoice")]
        ModelType::SenseVoice => Ok(Box::new(SenseVoiceEngine::new())),
        #[cfg(feature = "engine-whisper-cpp")]
        ModelType::WhisperCpp => Ok(Box::new(WhisperCppEngine::new())),
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
        assert_eq!("directml".parse::<DeviceType>().unwrap(), DeviceType::DirectML);
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

    #[test]
    #[cfg(feature = "engine-sensevoice")]
    fn test_new_sensevoice_engine() {
        let engine = new_engine(ModelType::SenseVoice).unwrap();
        let info = engine.model_info();
        assert_eq!(info.id, "sensevoice-small");
        assert!(!engine.is_loaded());
    }

    #[test]
    #[cfg(feature = "engine-sensevoice")]
    fn test_recognize_before_load_errors() {
        let mut engine = SenseVoiceEngine::new();
        let result = engine.recognize(&[], 16000);
        assert!(matches!(result, Err(EngineError::ModelNotLoaded)));
    }

    #[test]
    #[cfg(feature = "engine-sensevoice")]
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
    #[cfg(feature = "engine-sensevoice")]
    fn test_close_resets_recognizer() {
        let mut engine = SenseVoiceEngine::new();
        assert!(!engine.is_loaded());
        let _ = engine.close();
        assert!(!engine.is_loaded());
    }

    #[test]
    fn test_probe_device_returns_platform_appropriate() {
        #[cfg(target_os = "windows")]
        assert_eq!(probe_device(), DeviceType::DirectML);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(probe_device(), DeviceType::Cpu);
    }

    #[test]
    #[cfg(feature = "engine-sensevoice")]
    fn test_new_engine_sensevoice_type() {
        assert!(new_engine(ModelType::SenseVoice).is_ok());
    }

    #[test]
    #[cfg(feature = "engine-whisper-cpp")]
    fn test_new_engine_whispercpp_type() {
        assert!(new_engine(ModelType::WhisperCpp).is_ok());
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
    #[cfg(feature = "engine-sensevoice")]
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
        // Will fail because paths are empty, but config should be resolved first
        let result = engine.load_model(cfg);
        assert!(result.is_err());
        // Config should have been resolved before the attempt
        #[cfg(target_os = "windows")]
        assert_eq!(engine.cfg.device, DeviceType::DirectML);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(engine.cfg.device, DeviceType::Cpu);
        assert!(engine.cfg.num_threads > 0);
    }

    #[test]
    #[cfg(feature = "engine-sensevoice")]
    fn test_load_model_nonexistent_path_errors() {
        let mut engine = SenseVoiceEngine::new();
        let cfg = ModelConfig {
            model_type: ModelType::SenseVoice,
            model_path: String::from("/nonexistent/model.onnx"),
            tokens_path: String::from("/nonexistent/tokens.txt"),
            device: DeviceType::Cpu,
            language: String::from("auto"),
            num_threads: 2,
        };
        let result = engine.load_model(cfg);
        assert!(matches!(result, Err(EngineError::ModelLoadError(_))));
    }

    #[test]
    #[cfg(feature = "engine-sensevoice")]
    fn test_clean_sensevoice_text_strips_tags() {
        let (text, lang) = clean_sensevoice_text("<|zh|>你好世界");
        assert_eq!(text, "你好世界");
        assert_eq!(lang, "zh");
    }

    #[test]
    #[cfg(feature = "engine-sensevoice")]
    fn test_clean_sensevoice_text_no_tags() {
        let (text, lang) = clean_sensevoice_text("hello world");
        assert_eq!(text, "hello world");
        assert_eq!(lang, "auto");
    }

    #[test]
    #[cfg(feature = "engine-sensevoice")]
    fn test_clean_sensevoice_text_multiple_tags() {
        let (text, lang) =
            clean_sensevoice_text("<|sot|><|zh|><|text_only|>今天天气不错");
        assert_eq!(text, "今天天气不错");
        assert_eq!(lang, "zh");
    }

    // ── WhisperCppEngine tests ──

    #[test]
    #[cfg(feature = "engine-whisper-cpp")]
    fn test_new_whispercpp_engine() {
        let engine = new_engine(ModelType::WhisperCpp).unwrap();
        let info = engine.model_info();
        assert_eq!(info.id, "whisper-cpp");
        assert!(!engine.is_loaded());
    }

    #[test]
    #[cfg(feature = "engine-whisper-cpp")]
    fn test_whispercpp_recognize_before_load_errors() {
        let mut engine = WhisperCppEngine::new();
        let result = engine.recognize(&[], 16000);
        assert!(matches!(result, Err(EngineError::ModelNotLoaded)));
    }

    #[test]
    #[cfg(feature = "engine-whisper-cpp")]
    fn test_whispercpp_model_info() {
        let engine = WhisperCppEngine::new();
        let info = engine.model_info();
        assert_eq!(info.id, "whisper-cpp");
        assert_eq!(info.model_type, ModelType::WhisperCpp);
        assert_eq!(info.name, "Whisper (whisper.cpp)");
        assert!(info.languages.contains(&String::from("en")));
    }

    #[test]
    #[cfg(feature = "engine-whisper-cpp")]
    fn test_whispercpp_close_resets_model() {
        let mut engine = WhisperCppEngine::new();
        assert!(!engine.is_loaded());
        let _ = engine.close();
        assert!(!engine.is_loaded());
    }

    #[test]
    #[cfg(feature = "engine-whisper-cpp")]
    fn test_whispercpp_load_model_nonexistent_path_errors() {
        let mut engine = WhisperCppEngine::new();
        let cfg = ModelConfig {
            model_type: ModelType::WhisperCpp,
            model_path: String::from("/nonexistent/ggml-model.bin"),
            tokens_path: String::new(),
            device: DeviceType::Cpu,
            language: String::from("auto"),
            num_threads: 2,
        };
        let result = engine.load_model(cfg);
        assert!(matches!(result, Err(EngineError::WhisperModelNotFound(_))));
    }
}
