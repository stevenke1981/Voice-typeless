use crate::audio::types::{AudioChunk, AudioError, DeviceInfo, RecorderConfig};

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
