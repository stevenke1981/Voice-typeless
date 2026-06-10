//! Automatic model downloader.
//!
//! When the required ASR model files are not found on disk,
//! this module fetches them from Hugging Face (or a fallback mirror),
//! writes them to `{config_dir}/VoiceTypeless/models/sensevoice-small/`,
//! and emits progress events so the frontend can show a download indicator.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Model manifest
// ---------------------------------------------------------------------------

/// Holds the remote URLs and local filenames for a downloadable model.
struct ModelSource {
    pub model_id: &'static str,
    pub files: &'static [ModelFile],
}

/// A single file in a model distribution.
struct ModelFile {
    /// File name as stored on disk (e.g. `model.int8.onnx`).
    pub local_name: &'static str,
    /// Primary download URL (Hugging Face resolve).
    pub url: &'static str,
    /// Fallback URL if the primary fails.
    pub fallback_url: &'static str,
    /// Approximate size in bytes (for progress estimation).
    pub size_hint: u64,
}

// ── Available models ──────────────────────────────────────────────────────────

/// Model sources indexed by `active_model_id`.
/// Each entry maps a model ID to the files that must be downloaded.
const MODEL_REGISTRY: &[ModelSource] = &[ModelSource {
    model_id: "sensevoice-small",
    files: &[
        ModelFile {
            local_name: "model.int8.onnx",
            url: "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-emocv-lid-2024-07-28/resolve/main/model.int8.onnx",
            fallback_url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-emocv-lid-2024-07-28/model.int8.onnx",
            size_hint: 38_000_000,
        },
        ModelFile {
            local_name: "tokens.txt",
            url: "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-emocv-lid-2024-07-28/resolve/main/tokens.txt",
            fallback_url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-emocv-lid-2024-07-28/tokens.txt",
            size_hint: 5_000,
        },
    ],
}];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Progress payload emitted during download.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Current file being downloaded.
    pub file_name: String,
    /// Total size of the current file (from Content-Length header, or size hint).
    pub total_bytes: u64,
    /// Bytes written to disk so far (for the current file).
    pub bytes_written: u64,
}

/// Overall download phase.
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadPhase {
    Downloading,
    Done,
    Error(String),
}

/// Returns `true` if the given model ID is registered for auto-download.
pub fn is_downloadable(model_id: &str) -> bool {
    MODEL_REGISTRY.iter().any(|m| m.model_id == model_id)
}

/// Download model files for the given `model_id`.
///
/// Files are placed in `base_dir / model_id /`.
/// `on_chunk` is called periodically with progress information so the caller
/// can emit progress events.
///
/// Returns the path to the downloaded model directory on success.
pub fn download_model<F>(
    base_dir: &Path,
    model_id: &str,
    mut on_progress: F,
) -> Result<PathBuf, String>
where
    F: FnMut(DownloadProgress),
{
    let source = MODEL_REGISTRY
        .iter()
        .find(|m| m.model_id == model_id)
        .ok_or_else(|| format!("unknown model id: {model_id}"))?;

    let target_dir = base_dir.join(model_id);
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("failed to create model directory: {e}"))?;

    for mf in source.files {
        let local_path = target_dir.join(mf.local_name);

        // Skip if file already exists and has reasonable size
        if local_path.exists() {
            if let Ok(meta) = local_path.metadata() {
                if meta.len() > 1000 {
                    // File exists and looks valid — skip download
                    continue;
                }
            }
        }

        // Try primary URL first, then fallback
        let response = match ureq::get(mf.url).call() {
            Ok(response) => response,
            Err(_) => ureq::get(mf.fallback_url)
                .call()
                .map_err(|e| format!("failed to download {}: {e}", mf.local_name))?,
        };

        // Determine total size from Content-Length header
        let total_size: u64 = response
            .header("content-length")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(mf.size_hint);

        // Stream response body to disk with progress callbacks
        let mut reader = response.into_reader();
        let mut file = fs::File::create(&local_path)
            .map_err(|e| format!("failed to create {local_path:?}: {e}"))?;
        let mut written: u64 = 0;
        let mut buf = [0u8; 65_536]; // 64 KiB chunks

        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|e| format!("read error for {}: {e}", mf.local_name))?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])
                .map_err(|e| format!("write error for {local_path:?}: {e}"))?;
            written += n as u64;
            on_progress(DownloadProgress {
                file_name: mf.local_name.to_string(),
                total_bytes: total_size,
                bytes_written: written,
            });
        }
    }

    Ok(target_dir)
}
