//! Audio capture, playback, and device enumeration.
//!
//! This module provides the core audio interfaces for Voice-typeless:
//! - [`AudioRecorder`] trait for recording from microphones
//! - [`AudioPlayer`] trait for playing back audio signals
//! - [`DeviceEnumerator`] trait for discovering audio devices
//! - Voice Activity Detection (VAD) via [`is_speech`]
//!
//! Backend: [cpal](https://crates.io/crates/cpal) v0.18 (WASAPI on Windows).

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use cpal::traits::{DeviceTrait, HostTrait};

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

// ── Data structures ──────────────────────────────────────────────────────────

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

// ── Recorder ─────────────────────────────────────────────────────────────────

/// Shared state between the audio callback thread and the main thread.
struct SharedState {
    /// Accumulated audio buffer (f32, mono).
    buffer: Vec<f32>,
    /// Live chunk subscribers.
    subscribers: Vec<std::sync::mpsc::SyncSender<AudioChunk>>,
}

impl SharedState {
    fn push_samples(&mut self, data: &[f32], sample_rate: u32) {
        self.buffer.extend_from_slice(data);
        let chunk = AudioChunk {
            samples: data.to_vec(),
            sample_rate,
            captured_at: SystemTime::now(),
        };
        self.subscribers.retain(|tx| {
            match tx.try_send(chunk.clone()) {
                Ok(()) => true,
                Err(std::sync::mpsc::TrySendError::Full(_)) => true,
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => false,
            }
        });
    }
}

/// Audio recorder backed by cpal.
///
/// Captures audio from a hardware microphone via cpal's WASAPI backend.
/// The audio callback pushes samples into a shared buffer; the main thread
/// reads it after calling [`stop`](AudioRecorder::stop).
pub struct Recorder {
    shared: Arc<Mutex<SharedState>>,
    stream: Mutex<Option<cpal::Stream>>,
    recording: AtomicBool,
    started_at: Mutex<Option<SystemTime>>,
}

impl Recorder {
    /// Create a new `Recorder` in the stopped state.
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Mutex::new(SharedState {
                buffer: Vec::new(),
                subscribers: Vec::new(),
            })),
            stream: Mutex::new(None),
            recording: AtomicBool::new(false),
            started_at: Mutex::new(None),
        }
    }

    /// Resolve a device ID string to a `cpal::Device`.
    fn resolve_device(device_id: &str) -> Result<cpal::Device, AudioError> {
        let host = cpal::default_host();
        if device_id == "default" {
            host.default_input_device()
                .ok_or(AudioError::NoAudioHardware)
        } else {
            host.input_devices()
                .map_err(|e| AudioError::DeviceError(e.to_string()))?
                .find(|d| d.name().ok().as_deref() == Some(device_id))
                .ok_or_else(|| AudioError::DeviceError(format!("device not found: {device_id}")))
        }
    }

    /// Build a cpal input stream, dispatching on the device's sample format.
    fn build_input_stream(
        device: &cpal::Device,
        shared: &Arc<Mutex<SharedState>>,
    ) -> Result<(cpal::Stream, u32), AudioError> {
        let config = device
            .default_input_config()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?;
        let sample_format = config.sample_format();
        let sample_rate = config.sample_rate().0;
        let stream_config: cpal::StreamConfig = config.into();
        let shared = Arc::clone(shared);

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut state) = shared.try_lock() {
                        state.push_samples(data, sample_rate);
                    }
                },
                |err| eprintln!("[vtl] audio input error: {err}"),
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &stream_config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut state) = shared.try_lock() {
                        let converted: Vec<f32> =
                            data.iter().map(|&s| s as f32 / 32768.0).collect();
                        state.push_samples(&converted, sample_rate);
                    }
                },
                |err| eprintln!("[vtl] audio input error: {err}"),
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &stream_config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut state) = shared.try_lock() {
                        let converted: Vec<f32> = data
                            .iter()
                            .map(|&s| (s as f32 / 32768.0) - 1.0)
                            .collect();
                        state.push_samples(&converted, sample_rate);
                    }
                },
                |err| eprintln!("[vtl] audio input error: {err}"),
                None,
            ),
            _ => return Err(AudioError::FormatNotSupported),
        }
        .map_err(|e| AudioError::DeviceError(e.to_string()))?;

        Ok((stream, sample_rate))
    }
}

impl AudioRecorder for Recorder {
    fn start(&self, cfg: RecorderConfig) -> Result<(), AudioError> {
        if self.recording.load(Ordering::Acquire) {
            return Err(AudioError::AlreadyRecording);
        }
        let device = Self::resolve_device(&cfg.device_id)?;
        let (stream, _rate) = Self::build_input_stream(&device, &self.shared)?;

        // Clear old state before starting fresh.
        {
            let mut state = self.shared.lock().unwrap();
            state.buffer.clear();
        }
        *self.started_at.lock().unwrap() = Some(SystemTime::now());
        *self.stream.lock().unwrap() = Some(stream);
        self.recording.store(true, Ordering::Release);
        Ok(())
    }

    fn stop(&self) -> Result<(), AudioError> {
        if !self.recording.load(Ordering::Acquire) {
            return Err(AudioError::NotRecording);
        }
        self.recording.store(false, Ordering::Release);
        // Dropping the stream stops the WASAPI audio thread and blocks until
        // the callback thread has exited. Safe to read buffer afterwards.
        drop(self.stream.lock().unwrap().take());
        Ok(())
    }

    fn cancel(&self) {
        self.recording.store(false, Ordering::Release);
        drop(self.stream.lock().unwrap().take());
        *self.started_at.lock().unwrap() = None;
        self.shared.lock().unwrap().buffer.clear();
    }

    fn drain(&self) -> Result<AudioChunk, AudioError> {
        if self.recording.load(Ordering::Acquire) {
            return Err(AudioError::DrainWhileRecording);
        }
        let started_at = self
            .started_at
            .lock()
            .unwrap()
            .take()
            .ok_or(AudioError::NotRecording)?;
        let mut state = self.shared.lock().unwrap();
        let samples = std::mem::take(&mut state.buffer);
        Ok(AudioChunk {
            samples,
            sample_rate: SAMPLE_RATE,
            captured_at: started_at,
        })
    }

    fn subscribe(&self) -> std::sync::mpsc::Receiver<AudioChunk> {
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        self.shared.lock().unwrap().subscribers.push(tx);
        rx
    }
}

// ── Player ─────────────────────────────────────────────────────────────────

/// Generate a short decaying sine tone (marimba-like) at the given sample rate.
fn generate_marimba_tone(sample_rate: u32, volume: f64) -> Vec<f32> {
    let duration_s = 0.2; // 200 ms
    let num_samples = (sample_rate as f64 * duration_s) as usize;
    let freq = 880.0; // A5
    (0..num_samples)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            let sample = (2.0 * std::f64::consts::PI * freq * t).sin();
            let envelope = (-8.0 * i as f64 / num_samples as f64).exp();
            (sample * envelope * volume) as f32
        })
        .collect()
}

/// Audio player backed by cpal.
///
/// Plays a short marimba-like indicator tone through the default output device.
pub struct Player {
    stream: Mutex<Option<cpal::Stream>>,
    enabled: AtomicBool,
    volume: Mutex<f64>,
}

impl Player {
    /// Create a new `Player` with default settings (enabled, volume 1.0).
    pub fn new() -> Self {
        Self {
            stream: Mutex::new(None),
            enabled: AtomicBool::new(true),
            volume: Mutex::new(1.0),
        }
    }
}

impl AudioPlayer for Player {
    fn play_start(&self) -> Result<(), AudioError> {
        if !self.enabled.load(Ordering::Acquire) {
            return Ok(());
        }
        if self.stream.lock().unwrap().is_some() {
            return Ok(());
        }

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::NoAudioHardware)?;
        let config = device
            .default_output_config()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?;
        let sample_format = config.sample_format();
        let sample_rate = config.sample_rate().0;
        let stream_config: cpal::StreamConfig = config.into();
        let vol = *self.volume.lock().unwrap();
        let tone = generate_marimba_tone(sample_rate, vol);
        let playhead = Arc::new(AtomicUsize::new(0));

        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                let ph = Arc::clone(&playhead);
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        let start = ph.fetch_add(data.len(), Ordering::Relaxed);
                        for (i, sample) in data.iter_mut().enumerate() {
                            *sample = tone.get(start + i).copied().unwrap_or(0.0);
                        }
                    },
                    |err| eprintln!("[vtl] audio output error: {err}"),
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let tone_i16: Vec<i16> = tone
                    .iter()
                    .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                    .collect();
                let ph = Arc::clone(&playhead);
                device.build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        let start = ph.fetch_add(data.len(), Ordering::Relaxed);
                        for (i, sample) in data.iter_mut().enumerate() {
                            *sample = tone_i16.get(start + i).copied().unwrap_or(0);
                        }
                    },
                    |err| eprintln!("[vtl] audio output error: {err}"),
                    None,
                )
            }
            _ => return Err(AudioError::FormatNotSupported),
        }
        .map_err(|e| AudioError::DeviceError(e.to_string()))?;

        *self.stream.lock().unwrap() = Some(stream);
        Ok(())
    }

    fn play_stop(&self) -> Result<(), AudioError> {
        self.stream.lock().unwrap().take();
        Ok(())
    }

    fn play_cancel(&self) -> Result<(), AudioError> {
        self.stream.lock().unwrap().take();
        Ok(())
    }

    fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
        if !enabled {
            let _ = self.play_stop();
        }
    }

    fn set_volume(&self, volume: f64) {
        *self.volume.lock().unwrap() = volume.clamp(0.0, 1.0);
    }

    fn close(&self) -> Result<(), AudioError> {
        self.stream.lock().unwrap().take();
        Ok(())
    }
}

// ── Enumerator ─────────────────────────────────────────────────────────────

/// Device enumerator backed by cpal.
pub struct Enumerator;

impl DeviceEnumerator for Enumerator {
    fn list_input_devices(&self) -> Result<Vec<DeviceInfo>, AudioError> {
        let host = cpal::default_host();
        let default_dev = host.default_input_device();
        let default_name = default_dev.as_ref().and_then(|d| d.name().ok());

        let devices: Vec<cpal::Device> = host
            .input_devices()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?
            .collect();

        let mut result = Vec::with_capacity(devices.len());
        for device in devices {
            let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
            let is_default = default_name.as_deref() == Some(name.as_str());

            let mut channels = 1u32;
            let mut sample_rates: Vec<u32> = Vec::new();
            if let Ok(configs) = device.supported_input_configs() {
                for cfg in configs {
                    channels = channels.max(cfg.channels().into());
                    let min = cfg.min_sample_rate().0;
                    let max = cfg.max_sample_rate().0;
                    sample_rates.push(min);
                    if max > min {
                        sample_rates.push(max);
                    }
                }
            }
            sample_rates.sort();
            sample_rates.dedup();

            result.push(DeviceInfo {
                id: name.clone(),
                name,
                is_default,
                channels,
                sample_rates,
            });
        }

        Ok(result)
    }

    fn default_input_device(&self) -> Result<DeviceInfo, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(AudioError::NoAudioHardware)?;
        let name = device
            .name()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?;

        let devices = self.list_input_devices()?;
        devices
            .into_iter()
            .find(|d| d.id == name)
            .ok_or_else(|| {
                AudioError::DeviceError("default device not found in device list".to_string())
            })
    }
}

// ── Constructor functions ──────────────────────────────────────────────────

/// Create a new audio recorder backed by cpal.
pub fn new_recorder() -> impl AudioRecorder {
    Recorder::new()
}

/// Create a new audio player backed by cpal.
pub fn new_player() -> impl AudioPlayer {
    Player::new()
}

/// Create a new device enumerator backed by cpal.
pub fn new_enumerator() -> impl DeviceEnumerator {
    Enumerator
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn has_audio_hardware() -> bool {
        cpal::default_host().default_input_device().is_some()
    }

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
        if !has_audio_hardware() {
            eprintln!("skipping hardware test — no audio device");
            return;
        }
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
        if !has_audio_hardware() {
            eprintln!("skipping hardware test — no audio device");
            return;
        }
        let recorder = new_recorder();
        let cfg = RecorderConfig::default();
        assert!(recorder.start(cfg.clone()).is_ok());
        let result = recorder.start(cfg);
        assert!(matches!(result, Err(AudioError::AlreadyRecording)));
    }

    #[test]
    fn test_player_start_stop() {
        if cpal::default_host().default_output_device().is_none() {
            eprintln!("skipping hardware test — no audio output device");
            return;
        }
        let player = new_player();
        // play_start may fail on headless CI; accept success or NoAudioHardware
        let r = player.play_start();
        if let Err(e) = &r {
            assert!(matches!(e, AudioError::NoAudioHardware));
        } else {
            assert!(r.is_ok());
            assert!(player.play_stop().is_ok());
        }
    }

    #[test]
    fn test_player_enable_disable() {
        let player = new_player();
        player.set_enabled(false);
        // When disabled, play_start is a no-op (returns Ok)
        assert!(player.play_start().is_ok());
        player.set_enabled(true);
        player.set_volume(0.5);
        assert!(player.close().is_ok());
    }

    #[test]
    fn test_enumerator_lists_devices() {
        let enumerator = new_enumerator();
        let devices = enumerator.list_input_devices().unwrap_or_default();
        // On headless CI the list may be empty; that is acceptable.
        if has_audio_hardware() {
            assert!(!devices.is_empty());
            for d in &devices {
                assert!(!d.id.is_empty());
                assert!(!d.name.is_empty());
            }
        }
    }

    #[test]
    fn test_enumerator_default_device() {
        if !has_audio_hardware() {
            eprintln!("skipping hardware test — no audio device");
            return;
        }
        let enumerator = new_enumerator();
        let default = enumerator.default_input_device().unwrap();
        assert!(default.is_default);
    }

    #[test]
    fn test_recorder_cancel() {
        if !has_audio_hardware() {
            eprintln!("skipping hardware test — no audio device");
            return;
        }
        let recorder = new_recorder();
        assert!(recorder.start(RecorderConfig::default()).is_ok());
        recorder.cancel();
        // After cancel, drain should error because started_at was cleared.
        let result = recorder.drain();
        assert!(matches!(result, Err(AudioError::NotRecording)));
    }

    #[test]
    fn test_is_speech_detects_silence() {
        let cfg = VadConfig::default();
        let silence = vec![0.0f32; 16000]; // one second of silence
        assert!(!is_speech(&silence, cfg));
    }

    #[test]
    fn test_is_speech_detects_audio() {
        let cfg = VadConfig {
            energy_threshold: 0.01,
            silence_duration_ms: 3000,
        };
        let mut samples = vec![0.0f32; 16000];
        // Fill the second half with a sine tone at moderate amplitude.
        for i in 8000..16000 {
            let t = (i - 8000) as f64 / 16000.0;
            samples[i] = (2.0 * std::f64::consts::PI * 440.0 * t).sin() as f32 * 0.2;
        }
        assert!(is_speech(&samples, cfg));
    }
}
