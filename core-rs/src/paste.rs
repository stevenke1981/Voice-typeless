//! Clipboard paste operations — Win32 implementation.
//!
//! This module mirrors `core/paste/` from the Go implementation and
//! provides the paste-method abstraction, a clipboard guard for
//! save/restore semantics, and platform-specific clipboard I/O
//! via Win32 `OpenClipboard` / `SetClipboardData` (Windows)
//! or no-op stubs (other platforms).

use std::sync::{LazyLock, Mutex};

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
// Platform clipboard — Win32 / stub
// ---------------------------------------------------------------------------

/// Global lock serializing clipboard access (Win32 clipboard is process-wide).
static CLIPBOARD_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Run `f` while holding the global clipboard lock.
///
/// On Windows, `OpenClipboard` can only succeed from one thread at a time
/// per process. This lock serialises all clipboard I/O to prevent
/// test-level and runtime contention.
fn with_clipboard_lock<T>(f: impl FnOnce() -> T) -> T {
    let _guard = CLIPBOARD_LOCK.lock().expect("clipboard lock poisoned");
    f()
}

#[cfg(windows)]
mod clipboard_sys {
    use super::PasteError;
    use std::ffi::c_void;
    use std::ptr;

    type HANDLE = *mut c_void;
    type BOOL = i32;
    type UINT = u32;

    const CF_UNICODETEXT: UINT = 13;
    const GMEM_MOVEABLE: UINT = 0x0002;
    const GMEM_ZEROINIT: UINT = 0x0040;

    extern "system" {
        fn OpenClipboard(hWnd: HANDLE) -> BOOL;
        fn CloseClipboard() -> BOOL;
        fn EmptyClipboard() -> BOOL;
        fn SetClipboardData(uFormat: UINT, hMem: HANDLE) -> HANDLE;
        fn GetClipboardData(uFormat: UINT) -> HANDLE;
        fn GlobalAlloc(uFlags: UINT, dwBytes: usize) -> HANDLE;
        fn GlobalLock(hMem: HANDLE) -> *mut c_void;
        fn GlobalUnlock(hMem: HANDLE) -> BOOL;
        fn GlobalFree(hMem: HANDLE) -> HANDLE;
    }

    /// Read Unicode text from the system clipboard via Win32 API.
    pub fn read_text() -> Result<String, PasteError> {
        unsafe {
            if OpenClipboard(ptr::null_mut()) == 0 {
                return Err(PasteError::ClipboardError(
                    "OpenClipboard failed".into(),
                ));
            }

            let handle = GetClipboardData(CF_UNICODETEXT);
            if handle.is_null() {
                CloseClipboard();
                return Ok(String::new());
            }

            let ptr = GlobalLock(handle) as *const u16;
            if ptr.is_null() {
                CloseClipboard();
                return Err(PasteError::ClipboardError(
                    "GlobalLock failed".into(),
                ));
            }

            // Find null-terminator position
            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
            }

            let result = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
            GlobalUnlock(handle);
            CloseClipboard();
            Ok(result)
        }
    }

    /// Write Unicode text to the system clipboard via Win32 API.
    pub fn write_text(text: &str) -> Result<(), PasteError> {
        unsafe {
            let utf16: Vec<u16> =
                text.encode_utf16().chain(std::iter::once(0)).collect();
            let byte_size = utf16.len() * 2; // u16 = 2 bytes

            let hmem = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, byte_size);
            if hmem.is_null() {
                return Err(PasteError::ClipboardError(
                    "GlobalAlloc failed".into(),
                ));
            }

            let ptr = GlobalLock(hmem) as *mut u16;
            if ptr.is_null() {
                GlobalFree(hmem);
                return Err(PasteError::ClipboardError(
                    "GlobalLock failed".into(),
                ));
            }
            ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
            GlobalUnlock(hmem);

            if OpenClipboard(ptr::null_mut()) == 0 {
                GlobalFree(hmem);
                return Err(PasteError::ClipboardError(
                    "OpenClipboard failed".into(),
                ));
            }
            EmptyClipboard();
            let result = SetClipboardData(CF_UNICODETEXT, hmem);
            CloseClipboard();
            if result.is_null() {
                return Err(PasteError::ClipboardError(
                    "SetClipboardData failed".into(),
                ));
            }
            Ok(())
        }
    }
}

#[cfg(not(windows))]
mod clipboard_sys {
    use super::PasteError;
    use arboard::Clipboard;

    pub fn read_text() -> Result<String, PasteError> {
        let mut clip = Clipboard::new()
            .map_err(|e| PasteError::ClipboardError(format!("arboard init: {}", e)))?;
        clip.get_text()
            .map_err(|e| PasteError::ClipboardError(format!("arboard read: {}", e)))
    }

    pub fn write_text(text: &str) -> Result<(), PasteError> {
        let mut clip = Clipboard::new()
            .map_err(|e| PasteError::ClipboardError(format!("arboard init: {}", e)))?;
        clip.set_text(text)
            .map_err(|e| PasteError::ClipboardError(format!("arboard write: {}", e)))
    }
}

/// Reads the current text content from the system clipboard.
pub fn read_clipboard() -> Result<String, PasteError> {
    with_clipboard_lock(|| clipboard_sys::read_text())
}

/// Writes text content to the system clipboard.
pub fn write_clipboard(text: &str) -> Result<(), PasteError> {
    with_clipboard_lock(|| clipboard_sys::write_text(text))
}

/// Send Ctrl+V (or Cmd+V on macOS) keystrokes to paste from clipboard.
///
/// Used for applications that do not react to clipboard changes alone.
#[cfg(windows)]
pub fn send_paste_keys() -> Result<(), PasteError> {
    // TODO: implement SendInput / keybd_event simulation
    Ok(())
}

#[cfg(not(windows))]
pub fn send_paste_keys() -> Result<(), PasteError> {
    Err(PasteError::PasteFailed(
        "keyboard paste requires platform-specific implementation; use Tauri-enigo layer".into()
    ))
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
/// Returns a [`WindowsPaster`] on all platforms (clipboard I/O is
/// real Win32 on Windows, no-op elsewhere).
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
    fn test_new_paster_paste_ok() {
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
    fn test_read_clipboard_ok() {
        let result = read_clipboard();
        // On Windows this returns the actual clipboard content;
        // on other platforms it returns empty string. Either way: Ok.
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_clipboard_ok() {
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
