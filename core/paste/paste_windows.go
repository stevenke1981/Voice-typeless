//go:build windows

package paste

import (
	"sync"
	"time"
)

// readClipboard returns the current Unicode text from the Windows clipboard.
func readClipboard() (string, error) {
	// TODO: OpenClipboard(0) → GetClipboardData(CF_UNICODETEXT) → CloseClipboard
	return "", nil
}

// writeClipboard sets the Windows clipboard to the given Unicode text.
func writeClipboard(text string) error {
	// TODO: OpenClipboard(0) → EmptyClipboard → SetClipboardData(CF_UNICODETEXT) → CloseClipboard
	_ = text
	return nil
}

// windowsPaster pastes text using SendInput Ctrl+V via clipboard.
type windowsPaster struct {
	mu    sync.Mutex
	cfg   PasteConfig
	guard ClipboardGuard
}

func newPlatformPaster(cfg PasteConfig) Paster {
	return &windowsPaster{
		cfg:   cfg,
		guard: NewClipboardGuard(time.Duration(cfg.ClipboardHoldMs) * time.Millisecond),
	}
}

func (p *windowsPaster) Paste(text string) error {
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
	// TODO: SendInput Ctrl+V keydown + keyup

	if p.cfg.RestoreClipboard {
		return p.guard.Restore()
	}
	return nil
}

func (p *windowsPaster) Configure(cfg PasteConfig) {
	p.mu.Lock()
	p.cfg = cfg
	p.guard = NewClipboardGuard(time.Duration(cfg.ClipboardHoldMs) * time.Millisecond)
	p.mu.Unlock()
}

func (p *windowsPaster) Close() error { return nil }
