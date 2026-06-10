use std::fmt;
use std::num::NonZeroUsize;
use std::time::Duration;

use crate::engine::probe_device;
use crate::engine::traits::Engine;
use crate::engine::types::{
    DeviceType, EngineError, ModelConfig, ModelInfo, ModelType, RecognitionResult, Segment,
};

#[cfg(feature = "engine-sensevoice")]
use sherpa_onnx::{
    OfflineModelConfig, OfflineRecognizer, OfflineRecognizerConfig, OfflineSenseVoiceModelConfig,
};

/// SenseVoice engine implementation backed by sherpa-onnx `OfflineRecognizer`.
///
/// Offline SenseVoice engine backed by sherpa-onnx.
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

        // Resolve thread count: use all available parallelism
        if cfg.num_threads <= 0 {
            let count = std::thread::available_parallelism()
                .map(NonZeroUsize::get)
                .unwrap_or(4);
            cfg.num_threads = count as i32;
        }

        // Map device to sherpa-onnx provider string
        let provider = cfg.device.to_provider_str();

        // Build sherpa-onnx config using the resolved runtime settings.
        let recognizer_config = OfflineRecognizerConfig {
            model_config: OfflineModelConfig {
                sense_voice: OfflineSenseVoiceModelConfig {
                    model: Some(cfg.model_path.clone()),
                    language: Some(cfg.language.clone()),
                    use_itn: true,
                },
                tokens: Some(cfg.tokens_path.clone()),
                num_threads: cfg.num_threads,
                provider: Some(provider.to_string()),
                ..Default::default()
            },
            decoding_method: Some("greedy_search".into()),
            // Discourage blank/pad tokens to reduce false negatives.
            blank_penalty: 0.1,
            ..Default::default()
        };

        // Store config before attempting recognizer creation
        // so resolved device/threads are persisted even on failure
        self.cfg = cfg;

        // Create the recognizer
        let recognizer = OfflineRecognizer::create(&recognizer_config).ok_or_else(|| {
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

        // sherpa-onnx requires 16 kHz mono input — resample if needed
        const TARGET_RATE: u32 = 16000;
        let (waveform, used_rate) = if sample_rate != TARGET_RATE {
            (
                resample_linear(audio, sample_rate, TARGET_RATE),
                TARGET_RATE,
            )
        } else {
            (audio.to_vec(), sample_rate)
        };

        // Normalize audio to consistent peak level for better ASR accuracy
        let waveform = normalize_loudness(&waveform);

        // Create an offline stream, feed audio, decode
        let stream = recognizer.create_stream();
        stream.accept_waveform(used_rate as i32, &waveform);
        recognizer.decode(&stream);

        let raw_result = stream
            .get_result()
            .ok_or_else(|| EngineError::DecodeError("stream.get_result() returned None".into()))?;
        let raw_text = raw_result.text;

        // Clean SenseVoice language tags from output
        let (cleaned_text, detected_lang) = clean_sensevoice_text(&raw_text);

        // Calculate audio-based duration estimate
        let audio_duration = Duration::from_secs_f32(audio.len() as f32 / sample_rate as f32);

        // Build a single segment for the full utterance.
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

/// Strip SenseVoice language/control tags from transcription text
/// and return (cleaned_text, detected_language).
///
/// SenseVoice prefixes output with tags like `<|zh|>`, `<|en|>`, `<|ja|>`,
/// `<|ko|>`, `<|yue|>`. Returns `"auto"` as detected language if no tag found.
#[cfg(feature = "engine-sensevoice")]
pub fn clean_sensevoice_text(text: &str) -> (String, String) {
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

/// Linearly resample audio from `src_rate` to `dst_rate`.
///
/// Simple linear interpolation — sufficient for speech resampling between
/// common rates (48 kHz → 16 kHz, 44.1 kHz → 16 kHz, etc.).
#[cfg(feature = "engine-sensevoice")]
fn resample_linear(audio: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if src_rate == dst_rate || audio.is_empty() {
        return audio.to_vec();
    }
    let ratio = src_rate as f64 / dst_rate as f64;
    let out_len = (audio.len() as f64 / ratio).ceil() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let src_idx = src_pos as usize;
        let frac = src_pos - src_idx as f64;
        let a = audio[src_idx.min(audio.len() - 1)];
        let b = audio[(src_idx + 1).min(audio.len() - 1)];
        out.push((a as f64 + (b as f64 - a as f64) * frac) as f32);
    }
    out
}

/// Normalize audio to a target peak amplitude for consistent ASR input.
///
/// Brings the loudest sample to a fixed target level (0.9 = −0.9 dBFS),
/// scaling the rest proportionally.  Skips silent or already-clipped audio.
#[cfg(feature = "engine-sensevoice")]
fn normalize_loudness(audio: &[f32]) -> Vec<f32> {
    if audio.is_empty() {
        return audio.to_vec();
    }
    let peak = audio.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);
    // Skip if silent or already clipping — would just amplify noise or distort
    if !(1e-6..0.99).contains(&peak) {
        return audio.to_vec();
    }
    let target: f32 = 0.9; // ≈ −0.9 dBFS
    let gain = target / peak;
    audio
        .iter()
        .map(|&s| (s as f64 * gain as f64).clamp(-1.0, 1.0) as f32)
        .collect()
}

// ── Tests ──

#[cfg(test)]
#[cfg(feature = "engine-sensevoice")]
mod tests {
    use super::*;

    #[test]
    fn test_new_sensevoice_engine() {
        let engine = SenseVoiceEngine::new();
        let info = engine.model_info();
        assert_eq!(info.id, "sensevoice-small");
        assert!(!engine.is_loaded());
    }

    #[test]
    fn test_recognize_before_load_errors() {
        let mut engine = SenseVoiceEngine::new();
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
    fn test_close_resets_recognizer() {
        let mut engine = SenseVoiceEngine::new();
        assert!(!engine.is_loaded());
        let _ = engine.close();
        assert!(!engine.is_loaded());
    }

    #[test]
    fn test_new_engine_sensevoice_type() {
        assert!(super::super::new_engine(ModelType::SenseVoice).is_ok());
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
        let result = engine.load_model(cfg);
        assert!(result.is_err());
        #[cfg(target_os = "windows")]
        assert_eq!(engine.cfg.device, DeviceType::DirectML);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(engine.cfg.device, DeviceType::Cpu);
        assert!(engine.cfg.num_threads > 0);
    }

    #[test]
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
    fn test_normalize_loudness_empty() {
        assert_eq!(normalize_loudness(&[]), Vec::<f32>::new());
    }

    #[test]
    fn test_normalize_loudness_silence_unchanged() {
        let silent = vec![0.0f32, 0.0, 0.0];
        assert_eq!(normalize_loudness(&silent), silent);
    }

    #[test]
    fn test_normalize_loudness_scales_peak_to_target() {
        let quiet = vec![0.1f32, -0.05, 0.03];
        let normed = normalize_loudness(&quiet);
        let new_peak = normed.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);
        // Should be close to 0.9
        assert!((new_peak - 0.9).abs() < 0.01, "peak={new_peak}");
        // signs preserved
        assert!(normed[0] > 0.0);
        assert!(normed[1] < 0.0);
    }

    #[test]
    fn test_normalize_loudness_no_change_for_already_loud() {
        let loud = vec![0.99f32, -0.98, 0.95, -0.99];
        let normed = normalize_loudness(&loud);
        // Already has sample >= 0.99, should not be re-scaled
        assert_eq!(normed, loud);
    }

    #[test]
    fn test_normalize_loudness_preserves_shape() {
        // Use a monotonic signal that never crosses zero
        let signal: Vec<f32> = (1..=100).map(|i| 0.1 + (i as f32 / 100.0) * 0.5).collect();
        let normed = normalize_loudness(&signal);
        // Ratio between adjacent samples should be preserved
        for i in 1..signal.len() {
            let orig_ratio = signal[i] / signal[i - 1];
            let norm_ratio = normed[i] / normed[i - 1];
            assert!(
                (orig_ratio - norm_ratio).abs() < 0.001,
                "shape not preserved at index {i}: {orig_ratio} vs {norm_ratio}"
            );
        }
    }

    #[test]
    fn test_clean_sensevoice_text_strips_tags() {
        let (text, lang) = clean_sensevoice_text("<|zh|>你好世界");
        assert_eq!(text, "你好世界");
        assert_eq!(lang, "zh");
    }

    #[test]
    fn test_clean_sensevoice_text_no_tags() {
        let (text, lang) = clean_sensevoice_text("hello world");
        assert_eq!(text, "hello world");
        assert_eq!(lang, "auto");
    }

    #[test]
    fn test_clean_sensevoice_text_multiple_tags() {
        let (text, lang) = clean_sensevoice_text("<|sot|><|zh|><|text_only|>今天天气不错");
        assert_eq!(text, "今天天气不错");
        assert_eq!(lang, "zh");
    }
}
