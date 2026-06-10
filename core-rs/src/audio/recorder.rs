use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::types::{AudioChunk, AudioError, RecorderConfig, SAMPLE_RATE};
use crate::audio::traits::AudioRecorder;

/// Shared state between the audio callback thread and the main thread.
pub(crate) struct SharedState {
    /// Accumulated audio buffer (f32, mono).
    pub(crate) buffer: Vec<f32>,
    /// Live chunk subscribers.
    pub(crate) subscribers: Vec<std::sync::mpsc::SyncSender<AudioChunk>>,
}

impl SharedState {
    pub(crate) fn push_samples(&mut self, data: &[f32], sample_rate: u32) {
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
    pub(crate) shared: Arc<Mutex<SharedState>>,
    stream: Mutex<Option<cpal::Stream>>,
    recording: AtomicBool,
    started_at: Mutex<Option<SystemTime>>,
    /// Actual sample rate reported by the audio device when `start()` was
    /// called.  Returned by `drain()` so the ASR engine knows the true rate.
    sample_rate: Mutex<u32>,
}

// SAFETY: cpal::Stream is !Send on Windows WASAPI due to PhantomData<*mut ()>,
// but we always access it behind a Mutex and never from audio-callback threads.
unsafe impl Send for Recorder {}
unsafe impl Sync for Recorder {}

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
            sample_rate: Mutex::new(SAMPLE_RATE),
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

    /// Build a cpal input stream at the device's default config.
    ///
    /// Returns `(stream, actual_sample_rate)` so the caller can resample to
    /// 16 kHz if needed before passing audio to the ASR engine.
    fn build_input_stream(
        device: &cpal::Device,
        shared: &Arc<Mutex<SharedState>>,
    ) -> Result<(cpal::Stream, u32), AudioError> {
        let cfg = device
            .default_input_config()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?;
        let fmt = cfg.sample_format();
        let rate = cfg.sample_rate().0;
        let stream_cfg: cpal::StreamConfig = cfg.into();

        let stream = match fmt {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &stream_cfg,
                {
                    let shared = Arc::clone(shared);
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut state) = shared.try_lock() {
                            state.push_samples(data, rate);
                        }
                    }
                },
                |err| eprintln!("[vtl] audio input error: {err}"),
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &stream_cfg,
                {
                    let shared = Arc::clone(shared);
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut state) = shared.try_lock() {
                            let converted: Vec<f32> =
                                data.iter().map(|&s| s as f32 / 32768.0).collect();
                            state.push_samples(&converted, rate);
                        }
                    }
                },
                |err| eprintln!("[vtl] audio input error: {err}"),
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &stream_cfg,
                {
                    let shared = Arc::clone(shared);
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut state) = shared.try_lock() {
                            let converted: Vec<f32> = data
                                .iter()
                                .map(|&s| (s as f32 / 32768.0) - 1.0)
                                .collect();
                            state.push_samples(&converted, rate);
                        }
                    }
                },
                |err| eprintln!("[vtl] audio input error: {err}"),
                None,
            ),
            _ => return Err(AudioError::FormatNotSupported),
        }
        .map_err(|e| AudioError::DeviceError(e.to_string()))?;
        stream.play().map_err(|e| AudioError::DeviceError(format!("stream play failed: {e}")))?;

        println!("[vtl] built input stream: {} Hz, fmt={:?}", rate, fmt);
        Ok((stream, rate))
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRecorder for Recorder {
    fn start(&self, cfg: RecorderConfig) -> Result<(), AudioError> {
        if self.recording.load(Ordering::Acquire) {
            return Err(AudioError::AlreadyRecording);
        }
        let device = Self::resolve_device(&cfg.device_id)?;
        let (stream, dev_rate) = Self::build_input_stream(&device, &self.shared)?;

        // Track the actual device sample rate so drain() returns it correctly.
        *self.sample_rate.lock().unwrap() = dev_rate;

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
        let actual_rate = *self.sample_rate.lock().unwrap();
        let mut state = self.shared.lock().unwrap();
        let samples = std::mem::take(&mut state.buffer);
        Ok(AudioChunk {
            samples,
            sample_rate: actual_rate,
            captured_at: started_at,
        })
    }

    fn subscribe(&self) -> std::sync::mpsc::Receiver<AudioChunk> {
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        self.shared.lock().unwrap().subscribers.push(tx);
        rx
    }
}
