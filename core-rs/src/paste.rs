//! Clipboard paste operations — stub implementation.
//!
//! This module mirrors `core/paste/` from the Go implementation and
//! provides the paste-method abstraction, a clipboard guard for
//! save/restore semantics, and platform-specific clipboard I/O
//! (all stubs pending real Win32 API integration).

use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during paste and clipboard operations.
#[derive(Debug, thiserror::Error)]
pub enum PasteError {
    #[error("clipboard error: {0}")]
    ClipboardError(String),
    #[error("paste failed: {0}")]
    PasteFailed(String),
    #[error("clipboard guard error: {0}")]
    GuardError(String),
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// The method used to simulate a paste into the focused application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMethod {
    /// Write text to the system clipboard.
    Clipboard,
    /// Use keyboard-event simulation (e.g. SendInput / Ctrl+V).
    SendInput,
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for paste behaviour.
#[derive(Debug, Clone)]
pub struct PasteConfig {
    /// Which paste method to use.
    pub method: PasteMethod,
    /// How long (in ms) to hold the clipboard before restoring it.
    pub clipboard_hold_ms: u32,
    /// Whether to save and restore the original clipboard content.
    pub restore_clipboard: bool,
}

impl Default for PasteConfig {
    fn default() -> Self {
        Self {
            method: PasteMethod::Clipboard,
            clipboard_hold_ms: 150,
            restore_clipboard: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Abstraction for performing a paste operation.
pub trait Paster {
    /// Paste `text` into the currently focused application.
    fn paste(&self, text: &str) -> Result<(), PasteError>;

    /// Replace the current configuration at runtime.
    fn configure(&self, cfg: PasteConfig);

    /// Release any resources held by the paster.
    fn close(&self) -> Result<(), PasteError>;
}

/// Saves and restores the system clipboard content.
pub trait ClipboardGuard {
    /// Read and store the current clipboard content.
    fn save(&mut self) -> Result<(), PasteError>;

    /// Write the stored content back to the clipboard.
    fn restore(&mut self) -> Result<(), PasteError>;

    /// Return the configured hold duration.
    fn hold_duration(&self) -> std::time::Duration;
}

// ---------------------------------------------------------------------------
// Platform clipboard stubs
// ---------------------------------------------------------------------------

/// Reads the current text content from the system clipboard.
///
/// # Stub
///
/// Always returns an empty string. Real implementation will use
/// Win32 `OpenClipboard` / `GetClipboardData` on Windows.
pub fn read_clipboard() -> Result<String, PasteError> {
    Ok(String::new())
}

/// Writes text content to the system clipboard.
///
/// # Stub
///
/// No-op. Real implementation will use
/// Win32 `OpenClipboard` / `SetClipboardData` on Windows.
pub fn write_clipboard(_text: &str) -> Result<(), PasteError> {
    Ok(())
}

// ---------------------------------------------------------------------------
// ClipboardGuard implementation
// ---------------------------------------------------------------------------

/// Default stub implementation of [`ClipboardGuard`].
#[derive(Debug)]
pub struct ClipboardGuardImpl {
    saved: Mutex<String>,
    hold_dur: std::time::Duration,
}

impl ClipboardGuardImpl {
    /// Create a new guard with the given hold duration.
    pub fn new(hold_duration: std::time::Duration) -> Self {
        Self {
            saved: Mutex::new(String::new()),
            hold_dur: hold_duration,
        }
    }
}

impl ClipboardGuard for ClipboardGuardImpl {
    fn save(&mut self) -> Result<(), PasteError> {
        let content = read_clipboard()?;
        *self
            .saved
            .lock()
            .map_err(|e| PasteError::GuardError(e.to_string()))? = content;
        Ok(())
    }

    fn restore(&mut self) -> Result<(), PasteError> {
        let content = self
            .saved
            .lock()
            .map_err(|e| PasteError::GuardError(e.to_string()))?
            .clone();
        write_clipboard(&content)?;
        Ok(())
    }

    fn hold_duration(&self) -> std::time::Duration {
        self.hold_dur
    }
}

// ---------------------------------------------------------------------------
// Paster implementation (Windows stub)
// ---------------------------------------------------------------------------

/// Windows-specific stub implementation of [`Paster`].
///
/// # Flow
///
/// 1. Save clipboard content if `restore_clipboard` is enabled.
/// 2. Write the recognised text to the clipboard.
/// 3. Sleep for the configured hold duration.
/// 4. (TODO) Emit Ctrl+V via SendInput for applications that do not
///    react to clipboard changes alone.
/// 5. Restore the original clipboard content if saving was enabled.
#[derive(Debug)]
pub struct WindowsPaster {
    cfg: Mutex<PasteConfig>,
    guard: Mutex<ClipboardGuardImpl>,
}

impl WindowsPaster {
    /// Create a new `WindowsPaster` with the given configuration.
    pub fn new(cfg: PasteConfig) -> Self {
        let hold_dur = std::time::Duration::from_millis(cfg.clipboard_hold_ms as u64);
        Self {
            cfg: Mutex::new(cfg),
            guard: Mutex::new(ClipboardGuardImpl::new(hold_dur)),
        }
    }
}

impl Paster for WindowsPaster {
    fn paste(&self, text: &str) -> Result<(), PasteError> {
        // Extract values under the cfg lock, then drop it before touching
        // the guard to avoid any potential lock-ordering trouble.
        let (restore, hold_ms) = {
            let cfg = self
                .cfg
                .lock()
                .map_err(|e| PasteError::PasteFailed(e.to_string()))?;
            (cfg.restore_clipboard, cfg.clipboard_hold_ms)
        };

        // 1. Save clipboard if configured.
        if restore {
            let mut guard = self
                .guard
                .lock()
                .map_err(|e| PasteError::PasteFailed(e.to_string()))?;
            guard.save()?;
        }

        // 2. Write text to the clipboard.
        write_clipboard(text)?;

        // 3. Wait for the content to be available.
        std::thread::sleep(std::time::Duration::from_millis(hold_ms as u64));

        // 4. TODO: SendInput Ctrl+V for apps that do not react to clipboard alone.

        // 5. Restore clipboard if configured.
        if restore {
            let mut guard = self
                .guard
                .lock()
                .map_err(|e| PasteError::PasteFailed(e.to_string()))?;
            guard.restore()?;
        }

        Ok(())
    }

    fn configure(&self, cfg: PasteConfig) {
        let hold_dur = std::time::Duration::from_millis(cfg.clipboard_hold_ms as u64);
        // Update config and rebuild guard with the new hold duration.
        if let Ok(mut c) = self.cfg.lock() {
            *c = cfg;
        }
        if let Ok(mut g) = self.guard.lock() {
            *g = ClipboardGuardImpl::new(hold_dur);
        }
    }

    fn close(&self) -> Result<(), PasteError> {
        // No-op: nothing to clean up in the stub.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Constructor functions
// ---------------------------------------------------------------------------

/// Create a new [`Paster`] with the given configuration.
///
/// Currently returns a [`WindowsPaster`] stub on all platforms.
pub fn new_paster(cfg: PasteConfig) -> impl Paster {
    WindowsPaster::new(cfg)
}

/// Create a new [`ClipboardGuard`] with the given hold duration.
pub fn new_clipboard_guard(hold_duration: std::time::Duration) -> impl ClipboardGuard {
    ClipboardGuardImpl::new(hold_duration)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_paste_config() {
        let cfg = PasteConfig::default();
        assert_eq!(cfg.method, PasteMethod::Clipboard);
        assert_eq!(cfg.clipboard_hold_ms, 150);
        assert!(cfg.restore_clipboard);
    }

    #[test]
    fn test_clipboard_guard_save_restore() {
        let mut guard = ClipboardGuardImpl::new(std::time::Duration::from_millis(150));
        // Both save and restore are stubs — they should succeed.
        assert!(guard.save().is_ok());
        assert!(guard.restore().is_ok());
    }

    #[test]
    fn test_new_paster_returns_stub() {
        let paster = new_paster(PasteConfig::default());
        assert!(paster.paste("hello").is_ok());
    }

    #[test]
    fn test_paster_configure() {
        let paster = new_paster(PasteConfig::default());
        let new_cfg = PasteConfig {
            method: PasteMethod::SendInput,
            clipboard_hold_ms: 200,
            restore_clipboard: false,
        };
        paster.configure(new_cfg);
        assert!(paster.paste("world").is_ok());
    }

    #[test]
    fn test_read_clipboard_stub() {
        let result = read_clipboard();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_write_clipboard_stub() {
        let result = write_clipboard("test content");
        assert!(result.is_ok());
    }

    #[test]
    fn test_clipboard_guard_hold_duration() {
        let duration = std::time::Duration::from_millis(300);
        let guard = ClipboardGuardImpl::new(duration);
        assert_eq!(guard.hold_duration(), duration);
    }

    #[test]
    fn test_paste_error_display() {
        let err = PasteError::ClipboardError("broken".into());
        assert_eq!(err.to_string(), "clipboard error: broken");

        let err = PasteError::PasteFailed("timeout".into());
        assert_eq!(err.to_string(), "paste failed: timeout");

        let err = PasteError::GuardError("poisoned".into());
        assert_eq!(err.to_string(), "clipboard guard error: poisoned");
    }

    #[test]
    fn test_paster_close() {
        let paster = new_paster(PasteConfig::default());
        assert!(paster.close().is_ok());
    }
}
