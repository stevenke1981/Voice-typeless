use std::time::SystemTime;

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

    /// No audio input/output hardware found.
    #[error("no audio hardware available")]
    NoAudioHardware,

    /// Audio format not supported by the implementation.
    #[error("audio format not supported")]
    FormatNotSupported,

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
    pub captured_at: SystemTime,
}

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
