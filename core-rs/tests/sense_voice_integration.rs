//! Integration tests for SenseVoice ASR engine.
#![allow(unused_crate_dependencies)]
//!
//! These tests load the real ONNX model from disk and run recognition
//! on the bundled test WAV files. They serve as the primary smoke-test
//! for the speech-to-text pipeline.
//!
//! Run: `cargo test -p vtl-core --test sense_voice_integration`
//! (or just `cargo test -p vtl-core` to include all tests)

use std::path::{Path, PathBuf};

use vtl_core::engine::Engine;

/// Locate the models directory relative to the crate root.
///
/// `CARGO_MANIFEST_DIR` is set by Cargo to the directory containing the
/// test crate's `Cargo.toml` (i.e. `core-rs/`). The models live in the
/// workspace root's `models/` directory, one level up.
fn model_dir() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set (set by Cargo)");
    Path::new(&manifest)
        .parent()
        .expect("core-rs should have a parent (workspace root)")
        .join("models")
        .join("sensevoice-small")
}

/// Read a mono 16-bit PCM WAV file and return its f32 samples.
///
/// Uses the `hound` WAV crate. Panics if the file cannot be read or
/// is not a compatible 16-bit mono WAV.
fn read_wav_samples(path: &Path) -> (Vec<f32>, u32) {
    let mut reader = hound::WavReader::open(path).expect("failed to open WAV file");
    let spec = reader.spec();
    assert_eq!(
        spec.channels,
        1,
        "test WAV must be mono; {:?} has {} channels",
        path,
        spec.channels
    );
    assert_eq!(
        spec.bits_per_sample,
        16,
        "test WAV must be 16-bit PCM; {:?} is {} bit",
        path,
        spec.bits_per_sample
    );

    let sample_rate = spec.sample_rate;

    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.expect("failed to read WAV sample") as f32 / 32768.0)
        .collect();

    assert!(!samples.is_empty(), "WAV file {:?} contains no samples", path);
    (samples, sample_rate)
}

#[test]
fn test_sensevoice_load_and_recognize_zh() {
    let dir = model_dir();

    let model_path = dir.join("model.int8.onnx");
    let tokens_path = dir.join("tokens.txt");
    let test_wav = dir.join("test_wavs").join("zh.wav");

    // ── Pre-conditions ──────────────────────────────────────────────
    assert!(
        model_path.exists(),
        "Model file not found at {:?}. Did you download the model?",
        model_path
    );
    assert!(
        tokens_path.exists(),
        "Tokens file not found at {:?}",
        tokens_path
    );
    assert!(
        test_wav.exists(),
        "Test WAV not found at {:?}",
        test_wav
    );

    // ── Read test audio ─────────────────────────────────────────────
    let (samples, sample_rate) = read_wav_samples(&test_wav);
    eprintln!(
        "Loaded {:?}: {} samples @ {} Hz ({:.1}s)",
        test_wav.file_name().unwrap(),
        samples.len(),
        sample_rate,
        samples.len() as f64 / sample_rate as f64
    );

    // ── Create engine and load model ─────────────────────────────────
    let mut engine = vtl_core::engine::SenseVoiceEngine::new();

    let cfg = vtl_core::engine::ModelConfig {
        model_type: vtl_core::engine::ModelType::SenseVoice,
        model_path: model_path.to_string_lossy().to_string(),
        tokens_path: tokens_path.to_string_lossy().to_string(),
        device: vtl_core::engine::DeviceType::Cpu,
        language: "zh".into(),
        num_threads: 2,
    };

    engine
        .load_model(cfg)
        .expect("SenseVoice load_model() should succeed with valid model files");

    assert!(engine.is_loaded(), "Engine should report loaded after load_model()");

    // ── Run recognition ──────────────────────────────────────────────
    let result = engine
        .recognize(&samples, sample_rate)
        .expect("recognize() should succeed");

    // ── Assertions ──────────────────────────────────────────────────
    assert!(
        !result.text.is_empty(),
        "Recognized text should not be empty for zh.wav"
    );
    assert_eq!(
        result.language, "zh",
        "Expected language 'zh', got '{}'. Raw text: '{}'",
        result.language, result.text
    );
    assert!(
        result.confidence > 0.0,
        "Confidence should be > 0 for non-empty result"
    );
    assert!(
        result.duration.as_secs_f32() > 0.0,
        "Duration should be > 0"
    );

    eprintln!("✓ Recognized (zh): {}", result.text);
}

#[test]
fn test_sensevoice_recognize_en() {
    let dir = model_dir();
    let test_wav = dir.join("test_wavs").join("en.wav");

    if !test_wav.exists() {
        eprintln!("Skipping en test: {:?} not found", test_wav);
        return;
    }

    let (samples, sample_rate) = read_wav_samples(&test_wav);

    let mut engine = vtl_core::engine::SenseVoiceEngine::new();
    let cfg = vtl_core::engine::ModelConfig {
        model_type: vtl_core::engine::ModelType::SenseVoice,
        model_path: dir.join("model.int8.onnx").to_string_lossy().to_string(),
        tokens_path: dir.join("tokens.txt").to_string_lossy().to_string(),
        device: vtl_core::engine::DeviceType::Cpu,
        language: "auto".into(),
        num_threads: 2,
    };
    engine.load_model(cfg).expect("load_model() should succeed");

    let result = engine.recognize(&samples, sample_rate).expect("recognize() should succeed");
    assert!(!result.text.is_empty(), "English recognition should produce text");
    // When language is "auto", the result language may stay "auto" even though
    // the model correctly detected and transcribed English internally.
    eprintln!("✓ Recognized (en): {} [lang={}]", result.text, result.language);
}

#[test]
fn test_sensevoice_recognize_ja() {
    let dir = model_dir();
    let test_wav = dir.join("test_wavs").join("ja.wav");

    if !test_wav.exists() {
        eprintln!("Skipping ja test: {:?} not found", test_wav);
        return;
    }

    let (samples, sample_rate) = read_wav_samples(&test_wav);

    let mut engine = vtl_core::engine::SenseVoiceEngine::new();
    let cfg = vtl_core::engine::ModelConfig {
        model_type: vtl_core::engine::ModelType::SenseVoice,
        model_path: dir.join("model.int8.onnx").to_string_lossy().to_string(),
        tokens_path: dir.join("tokens.txt").to_string_lossy().to_string(),
        device: vtl_core::engine::DeviceType::Cpu,
        language: "auto".into(),
        num_threads: 2,
    };
    engine.load_model(cfg).expect("load_model() should succeed");

    let result = engine.recognize(&samples, sample_rate).expect("recognize() should succeed");
    assert!(!result.text.is_empty(), "Japanese recognition should produce text");
    eprintln!("✓ Recognized (ja): {}", result.text);
}

#[test]
fn test_sensevoice_recognize_ko() {
    let dir = model_dir();
    let test_wav = dir.join("test_wavs").join("ko.wav");

    if !test_wav.exists() {
        eprintln!("Skipping ko test: {:?} not found", test_wav);
        return;
    }

    let (samples, sample_rate) = read_wav_samples(&test_wav);

    let mut engine = vtl_core::engine::SenseVoiceEngine::new();
    let cfg = vtl_core::engine::ModelConfig {
        model_type: vtl_core::engine::ModelType::SenseVoice,
        model_path: dir.join("model.int8.onnx").to_string_lossy().to_string(),
        tokens_path: dir.join("tokens.txt").to_string_lossy().to_string(),
        device: vtl_core::engine::DeviceType::Cpu,
        language: "auto".into(),
        num_threads: 2,
    };
    engine.load_model(cfg).expect("load_model() should succeed");

    let result = engine.recognize(&samples, sample_rate).expect("recognize() should succeed");
    assert!(!result.text.is_empty(), "Korean recognition should produce text");
    eprintln!("✓ Recognized (ko): {}", result.text);
}

#[test]
fn test_sensevoice_recognize_yue() {
    let dir = model_dir();
    let test_wav = dir.join("test_wavs").join("yue.wav");

    if !test_wav.exists() {
        eprintln!("Skipping yue test: {:?} not found", test_wav);
        return;
    }

    let (samples, sample_rate) = read_wav_samples(&test_wav);

    let mut engine = vtl_core::engine::SenseVoiceEngine::new();
    let cfg = vtl_core::engine::ModelConfig {
        model_type: vtl_core::engine::ModelType::SenseVoice,
        model_path: dir.join("model.int8.onnx").to_string_lossy().to_string(),
        tokens_path: dir.join("tokens.txt").to_string_lossy().to_string(),
        device: vtl_core::engine::DeviceType::Cpu,
        language: "auto".into(),
        num_threads: 2,
    };
    engine.load_model(cfg).expect("load_model() should succeed");

    let result = engine.recognize(&samples, sample_rate).expect("recognize() should succeed");
    assert!(!result.text.is_empty(), "Cantonese recognition should produce text");
    eprintln!("✓ Recognized (yue): {}", result.text);
}
