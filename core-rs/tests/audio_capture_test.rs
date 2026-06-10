//! Quick hardware audio capture test.
//!
//! Run manually: cargo test --test audio_capture_test -- --nocapture --ignored
//!
//! Verifies that cpal can capture actual audio samples from the microphone.

use std::time::Duration;
use vtl_core::audio::{AudioRecorder, Recorder, RecorderConfig};

#[test]
#[ignore]
fn test_hardware_audio_capture() {
    let recorder = Recorder::new();
    let cfg = RecorderConfig {
        device_id: "default".to_string(),
        sample_rate: 16000,
        channels: 1,
        buffer_size: 0,
    };

    println!("Starting recording...");
    recorder.start(cfg).expect("start_recording failed");
    
    // Record for 2 seconds
    std::thread::sleep(Duration::from_secs(2));
    
    println!("Stopping recording...");
    recorder.stop().expect("stop_recording failed");
    
    let chunk = recorder.drain().expect("drain failed");
    let duration_ms = (chunk.samples.len() as u64 * 1000) / chunk.sample_rate as u64;
    
    println!(
        "Captured: samples={}, rate={}Hz, duration={}ms",
        chunk.samples.len(),
        chunk.sample_rate,
        duration_ms,
    );
    
    assert!(chunk.samples.len() > 0, "Expected >0 samples but got 0 — audio capture not working!");
    assert!(duration_ms >= 500, "Expected at least 500ms of audio, got {}ms", duration_ms);
}
