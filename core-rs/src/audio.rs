//! Audio capture, playback, and device enumeration.
//!
//! This module provides the core audio interfaces for Voice-typeless:
//! - [`AudioRecorder`] trait for recording from microphones
//! - [`AudioPlayer`] trait for playing back audio signals
//! - [`DeviceEnumerator`] trait for discovering audio devices
//! - Voice Activity Detection (VAD) via [`is_speech`]
//!
//! The implementations are stubs — real hardware audio access
//! (via malgo/miniaudio or similar) is deferred. They track
//! recording state, buffer data, and manage subscribers, but
//! do not actually capture or play sound.

use std::sync::Mutex;

/// Sample rate used throughout the engine (16 kHz).
pub const SAMPLE_RATE: u32 = 16000;

/// Errors that can occur during audio operations.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// Returned when attempting to start recording while already recording.
    #[error("already recording")]
    AlreadyRecording,

    /// Returned when attempting to stop or drain while not recording.
    #[error("not recording")]
    NotRecording,

    /// Returned when attempting to drain while still recording (call stop first).
    #[error("drain while recording")]
    DrainWhileRecording,

    /// Wraps a device-level error.
    #[error("device error: {0}")]
    DeviceError(String),
}

/// Information about an audio input/output device.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub channels: u32,
    pub sample_rates: Vec<u32>,
}

/// Configuration for the audio recorder.
///
/// Default: device_id = "default", sample_rate = 16000, channels = 1, buffer_size = 0.
/// A `buffer_size` of 0 means 16000 * 30 = 30 seconds of ring buffer.
#[derive(Debug, Clone)]
pub struct RecorderConfig {
    /// Device ID; use "default" for the system default device.
    pub device_id: String,
    /// Sample rate in Hz (must be 16000 for direct engine use).
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo).
    pub channels: u32,
    /// Ring buffer size in samples (0 = 16000 * 30 = 30 seconds).
    pub buffer_size: u32,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            device_id: "default".to_string(),
            sample_rate: SAMPLE_RATE,
            channels: 1,
            buffer_size: 0,
        }
    }
}

/// A chunk of captured audio samples with metadata.
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub captured_at: std::time::SystemTime,
}

// ── Voice Activity Detection ───────────────────────────────────────────────

/// Configuration for the voice activity detector (VAD).
///
/// Default: energy_threshold = 0.02, silence_duration_ms = 3000.
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// RMS energy threshold above which audio is considered speech (default 0.02).
    pub energy_threshold: f32,
    /// Silence duration in milliseconds before considering speech ended (default 3000).
    pub silence_duration_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.02,
            silence_duration_ms: 3000,
        }
    }
}

/// Returns `true` if the audio chunk contains speech based on RMS energy.
///
/// Uses a simple energy-based VAD: if the root-mean-square of the samples
/// exceeds the configured threshold, the chunk is classified as speech.
pub fn is_speech(chunk: &[f32], cfg: VadConfig) -> bool {
    if chunk.is_empty() {
        return false;
    }
    let sum: f64 = chunk.iter().map(|&s| (s as f64) * (s as f64)).sum();
    let rms = (sum / chunk.len() as f64).sqrt();
    (rms as f32) > cfg.energy_threshold
}

// ── Traits ─────────────────────────────────────────────────────────────────

/// Interface for recording audio from a microphone.
pub trait AudioRecorder {
    /// Start recording with the given configuration.
    fn start(&self, cfg: RecorderConfig) -> Result<(), AudioError>;
    /// Stop recording.
    fn stop(&self) -> Result<(), AudioError>;
    /// Cancel the current recording (discard buffered data).
    fn cancel(&self);
    /// Drain captured audio after stopping. Returns an error if still recording
    /// or if no recording session has been started.
    fn drain(&self) -> Result<AudioChunk, AudioError>;
    /// Subscribe to live audio chunks delivered during recording.
    fn subscribe(&self) -> std::sync::mpsc::Receiver<AudioChunk>;
}

/// Interface for playing audio signals.
pub trait AudioPlayer {
    /// Start playback.
    fn play_start(&self) -> Result<(), AudioError>;
    /// Stop playback.
    fn play_stop(&self) -> Result<(), AudioError>;
    /// Cancel playback immediately.
    fn play_cancel(&self) -> Result<(), AudioError>;
    /// Enable or disable playback.
    fn set_enabled(&self, enabled: bool);
    /// Set playback volume (0.0 – 1.0).
    fn set_volume(&self, volume: f64);
    /// Close the player and release resources.
    fn close(&self) -> Result<(), AudioError>;
}

/// Interface for enumerating audio input devices.
pub trait DeviceEnumerator {
    /// List all available input devices.
    fn list_input_devices(&self) -> Result<Vec<DeviceInfo>, AudioError>;
    /// Get the default input device.
    fn default_input_device(&self) -> Result<DeviceInfo, AudioError>;
}

// ── Stub implementations ───────────────────────────────────────────────────

struct RecorderInner {
    recording: bool,
    started_at: Option<std::time::SystemTime>,
    buffer: Vec<f32>,
    subscribers: Vec<std::sync::mpsc::Sender<AudioChunk>>,
    config: RecorderConfig,
}

/// Stub audio recorder.
///
/// Tracks recording state and buffers data in memory but does not
/// actually capture from a hardware microphone.
pub struct Recorder {
    inner: Mutex<RecorderInner>,
}

impl Recorder {
    /// Create a new `Recorder` in the stopped state.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RecorderInner {
                recording: false,
                started_at: None,
                buffer: Vec::new(),
                subscribers: Vec::new(),
                config: RecorderConfig::default(),
            }),
        }
    }
}

impl AudioRecorder for Recorder {
    fn start(&self, cfg: RecorderConfig) -> Result<(), AudioError> {
        let mut inner = self.inner.lock().unwrap();
        if inner.recording {
            return Err(AudioError::AlreadyRecording);
        }
        inner.recording = true;
        inner.started_at = Some(std::time::SystemTime::now());
        inner.config = cfg;
        inner.buffer.clear();
        Ok(())
    }

    fn stop(&self) -> Result<(), AudioError> {
        let mut inner = self.inner.lock().unwrap();
        if !inner.recording {
            return Err(AudioError::NotRecording);
        }
        inner.recording = false;
        Ok(())
    }

    fn cancel(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.recording = false;
        inner.started_at = None;
        inner.buffer.clear();
    }

    fn drain(&self) -> Result<AudioChunk, AudioError> {
        let mut inner = self.inner.lock().unwrap();
        if inner.recording {
            return Err(AudioError::DrainWhileRecording);
        }
        let started_at = inner.started_at.take().ok_or(AudioError::NotRecording)?;
        let chunk = AudioChunk {
            samples: std::mem::take(&mut inner.buffer),
            sample_rate: inner.config.sample_rate,
            captured_at: started_at,
        };
        Ok(chunk)
    }

    fn subscribe(&self) -> std::sync::mpsc::Receiver<AudioChunk> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut inner = self.inner.lock().unwrap();
        inner.subscribers.push(tx);
        rx
    }
}

// ── Player ─────────────────────────────────────────────────────────────────

struct PlayerInner {
    enabled: bool,
    volume: f64,
}

/// Stub audio player.
///
/// Tracks enabled/disabled state and volume but does not actually
/// play sound through any hardware device.
pub struct Player {
    inner: Mutex<PlayerInner>,
}

impl Player {
    /// Create a new `Player` with default settings (enabled, volume 1.0).
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PlayerInner {
                enabled: true,
                volume: 1.0,
            }),
        }
    }
}

impl AudioPlayer for Player {
    fn play_start(&self) -> Result<(), AudioError> {
        // TODO: implement playback via malgo/miniaudio
        Ok(())
    }

    fn play_stop(&self) -> Result<(), AudioError> {
        Ok(())
    }

    fn play_cancel(&self) -> Result<(), AudioError> {
        Ok(())
    }

    fn set_enabled(&self, enabled: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.enabled = enabled;
    }

    fn set_volume(&self, volume: f64) {
        let mut inner = self.inner.lock().unwrap();
        inner.volume = volume;
    }

    fn close(&self) -> Result<(), AudioError> {
        Ok(())
    }
}

// ── Enumerator ─────────────────────────────────────────────────────────────

/// Stub device enumerator.
///
/// Returns a hardcoded default device. Real device discovery
/// is deferred.
pub struct Enumerator;

impl DeviceEnumerator for Enumerator {
    fn list_input_devices(&self) -> Result<Vec<DeviceInfo>, AudioError> {
        Ok(vec![DeviceInfo {
            id: "default".to_string(),
            name: "Default Microphone".to_string(),
            is_default: true,
            channels: 1,
            sample_rates: vec![SAMPLE_RATE, 44100, 48000],
        }])
    }

    fn default_input_device(&self) -> Result<DeviceInfo, AudioError> {
        let devices = self.list_input_devices()?;
        devices
            .into_iter()
            .find(|d| d.is_default)
            .ok_or_else(|| AudioError::DeviceError("no default device found".to_string()))
    }
}

// ── Constructor functions ──────────────────────────────────────────────────

/// Create a new stub audio recorder.
pub fn new_recorder() -> impl AudioRecorder {
    Recorder::new()
}

/// Create a new stub audio player.
pub fn new_player() -> impl AudioPlayer {
    Player::new()
}

/// Create a new stub device enumerator.
pub fn new_enumerator() -> impl DeviceEnumerator {
    Enumerator
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_default_config() {
        let cfg = VadConfig::default();
        assert_eq!(cfg.energy_threshold, 0.02);
        assert_eq!(cfg.silence_duration_ms, 3000);
    }

    #[test]
    fn test_is_speech_empty_chunk() {
        let cfg = VadConfig::default();
        assert!(!is_speech(&[], cfg));
    }

    #[test]
    fn test_new_recorder_starts_not_recording() {
        let recorder = new_recorder();
        let result = recorder.drain();
        assert!(matches!(result, Err(AudioError::NotRecording)));
    }

    #[test]
    fn test_recorder_start_stop_drain_cycle() {
        let recorder = new_recorder();
        let cfg = RecorderConfig::default();
        assert!(recorder.start(cfg).is_ok());
        assert!(recorder.stop().is_ok());
        let result = recorder.drain();
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert!(chunk.samples.is_empty());
        assert_eq!(chunk.sample_rate, SAMPLE_RATE);
    }

    #[test]
    fn test_recorder_double_start_errors() {
        let recorder = new_recorder();
        let cfg = RecorderConfig::default();
        assert!(recorder.start(cfg.clone()).is_ok());
        let result = recorder.start(cfg);
        assert!(matches!(result, Err(AudioError::AlreadyRecording)));
    }

    #[test]
    fn test_new_player_returns_stub() {
        let player = new_player();
        assert!(player.play_start().is_ok());
        assert!(player.play_stop().is_ok());
        assert!(player.play_cancel().is_ok());
        player.set_enabled(false);
        player.set_volume(0.5);
        assert!(player.close().is_ok());
    }

    #[test]
    fn test_enumerator_returns_default_device() {
        let enumerator = new_enumerator();
        let devices = enumerator.list_input_devices().unwrap();
        assert!(!devices.is_empty());
        let default = enumerator.default_input_device().unwrap();
        assert!(default.is_default);
        assert_eq!(default.name, "Default Microphone");
    }
}
