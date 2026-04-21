// Package paste provides clipboard-safe text injection into the focused application.
// It saves, injects, and restores the clipboard to prevent data loss.
package paste

import "time"

// PasteMethod selects how text is inserted into the target application.
type PasteMethod string

const (
	// PasteMethodClipboard saves text to clipboard, sends Ctrl+V / Cmd+V.
	PasteMethodClipboard PasteMethod = "clipboard"
	// PasteMethodSendInput sends each character via synthetic keyboard events.
	// Slower but works in applications that intercept clipboard paste.
	PasteMethodSendInput PasteMethod = "sendinput"
)

// PasteConfig configures a Paster instance.
type PasteConfig struct {
	Method           PasteMethod
	ClipboardHoldMs  int  // Minimum ms to hold clipboard before restoring (default 150)
	RestoreClipboard bool // Whether to restore previous clipboard content
}

// DefaultPasteConfig returns sensible defaults.
func DefaultPasteConfig() PasteConfig {
	return PasteConfig{
		Method:           PasteMethodClipboard,
		ClipboardHoldMs:  150,
		RestoreClipboard: true,
	}
}

// Paster inserts text into the currently focused application.
type Paster interface {
	// Paste inserts text using the configured method.
	// Blocks until the paste operation is confirmed complete.
	Paste(text string) error

	// Configure updates the paster's config at runtime.
	Configure(cfg PasteConfig)

	// Close releases any held resources.
	Close() error
}

// ClipboardGuard saves and restores clipboard contents around a paste operation.
type ClipboardGuard interface {
	// Save captures the current clipboard contents.
	Save() error

	// Restore puts back the previously saved clipboard contents.
	// No-op if Save was not called.
	Restore() error

	// HoldDuration is the minimum delay between writing paste text and restoring.
	HoldDuration() time.Duration
}

// NewPaster creates a platform-appropriate Paster.
func NewPaster(cfg PasteConfig) Paster { return newPlatformPaster(cfg) }

// NewClipboardGuard creates a platform-appropriate ClipboardGuard.
func NewClipboardGuard(holdDuration time.Duration) ClipboardGuard {
	return newPlatformGuard(holdDuration)
}
