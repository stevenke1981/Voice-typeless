use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, atomic::AtomicUsize};

use cpal::traits::{DeviceTrait, HostTrait};

use crate::audio::types::AudioError;
use crate::audio::traits::AudioPlayer;

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

// SAFETY: Same rationale as Recorder — cpal::Stream behind a Mutex.
unsafe impl Send for Player {}
unsafe impl Sync for Player {}

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

impl Default for Player {
    fn default() -> Self {
        Self::new()
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
