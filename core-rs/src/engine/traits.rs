use crate::engine::types::{EngineError, ModelConfig, ModelInfo, RecognitionResult};

/// The abstract speech recognition engine interface.
///
/// Common contract implemented by offline speech engines.
pub trait Engine: Send {
    /// Load a model with the given configuration.
    fn load_model(&mut self, cfg: ModelConfig) -> Result<(), EngineError>;

    /// Transcribe audio samples and return the recognition result.
    fn recognize(
        &mut self,
        audio: &[f32],
        sample_rate: u32,
    ) -> Result<RecognitionResult, EngineError>;

    /// Return metadata about the loaded model.
    fn model_info(&self) -> ModelInfo;

    /// Close and release all engine resources.
    fn close(&mut self) -> Result<(), EngineError>;

    /// Returns `true` if a model is currently loaded.
    fn is_loaded(&self) -> bool {
        false
    }
}
