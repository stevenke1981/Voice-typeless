use crate::audio::types::VadConfig;

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
