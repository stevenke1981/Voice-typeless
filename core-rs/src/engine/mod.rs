pub mod traits;
pub mod types;

#[cfg(feature = "engine-sensevoice")]
pub mod sensevoice;

#[cfg(feature = "engine-whisper-cpp")]
pub mod whisper_cpp;

// ── Re-exports ──

pub use traits::Engine;
pub use types::*;

#[cfg(feature = "engine-sensevoice")]
pub use sensevoice::{clean_sensevoice_text, SenseVoiceEngine};

#[cfg(feature = "engine-whisper-cpp")]
pub use whisper_cpp::WhisperCppEngine;

// ── Free Functions ──

/// Probe the system for the best available compute device.
///
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
/// Create an engine implementation for the requested model type.
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
    fn test_probe_device_returns_platform_appropriate() {
        #[cfg(target_os = "windows")]
        assert_eq!(probe_device(), DeviceType::DirectML);
        #[cfg(not(target_os = "windows"))]
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
}
