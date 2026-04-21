//go:build darwin

package paste

import (
	"sync"
	"time"
)

// readClipboard returns the general pasteboard text on macOS.
func readClipboard() (string, error) {
	// TODO: NSPasteboard.generalPasteboard().string(forType: .string)
	return "", nil
}

// writeClipboard writes text to the macOS general pasteboard.
func writeClipboard(text string) error {
	// TODO: NSPasteboard write
	_ = text
	return nil
}

// darwinPaster pastes via NSPasteboard + CGEvent Cmd+V.
type darwinPaster struct {
	mu    sync.Mutex
	cfg   PasteConfig
	guard ClipboardGuard
}

func newPlatformPaster(cfg PasteConfig) Paster {
	return &darwinPaster{
		cfg:   cfg,
		guard: NewClipboardGuard(time.Duration(cfg.ClipboardHoldMs) * time.Millisecond),
	}
}

func (p *darwinPaster) Paste(text string) error {
	p.mu.Lock()
	defer p.mu.Unlock()

	if p.cfg.RestoreClipboard {
		if err := p.guard.Save(); err != nil {
			return err
		}
	}

	if err := writeClipboard(text); err != nil {
		return err
	}
	time.Sleep(p.guard.HoldDuration())
	// TODO: send CGEvent Command+V keydown + keyup

	if p.cfg.RestoreClipboard {
		return p.guard.Restore()
	}
	return nil
}

func (p *darwinPaster) Configure(cfg PasteConfig) {
	p.mu.Lock()
	p.cfg = cfg
	p.guard = NewClipboardGuard(time.Duration(cfg.ClipboardHoldMs) * time.Millisecond)
	p.mu.Unlock()
}

func (p *darwinPaster) Close() error { return nil }
