use std::fmt;
use std::num::NonZeroUsize;
use std::time::Duration;

use crate::engine::traits::Engine;
use crate::engine::types::{
    DeviceType, EngineError, ModelConfig, ModelInfo, ModelType, RecognitionResult, Segment,
};

#[cfg(feature = "engine-whisper-cpp")]
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

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
        let ctx =
            WhisperContext::new_with_params(&self.cfg.model_path, ctx_params).map_err(|e| {
                EngineError::ModelLoadError(format!(
                    "whisper-rs: failed to create context from '{}': {}",
                    self.cfg.model_path, e
                ))
            })?;

        self.ctx = Some(ctx);
        Ok(())
    }

    fn recognize(
        &mut self,
        audio: &[f32],
        sample_rate: u32,
    ) -> Result<RecognitionResult, EngineError> {
        let ctx = self.ctx.as_ref().ok_or(EngineError::ModelNotLoaded)?;

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
        let audio_duration = Duration::from_secs_f32(audio.len() as f32 / sample_rate as f32);

        // Determine confidence:
        // whisper-rs does not expose per-token probability on the Rust API.
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

// ── Tests ──

#[cfg(test)]
#[cfg(feature = "engine-whisper-cpp")]
mod tests {
    use super::*;

    #[test]
    fn test_new_whispercpp_engine() {
        let engine = WhisperCppEngine::new();
        let info = engine.model_info();
        assert_eq!(info.id, "whisper-cpp");
        assert!(!engine.is_loaded());
    }

    #[test]
    fn test_whispercpp_recognize_before_load_errors() {
        let mut engine = WhisperCppEngine::new();
        let result = engine.recognize(&[], 16000);
        assert!(matches!(result, Err(EngineError::ModelNotLoaded)));
    }

    #[test]
    fn test_whispercpp_model_info() {
        let engine = WhisperCppEngine::new();
        let info = engine.model_info();
        assert_eq!(info.id, "whisper-cpp");
        assert_eq!(info.model_type, ModelType::WhisperCpp);
        assert_eq!(info.name, "Whisper (whisper.cpp)");
        assert!(info.languages.contains(&String::from("en")));
    }

    #[test]
    fn test_whispercpp_close_resets_model() {
        let mut engine = WhisperCppEngine::new();
        assert!(!engine.is_loaded());
        let _ = engine.close();
        assert!(!engine.is_loaded());
    }

    #[test]
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
