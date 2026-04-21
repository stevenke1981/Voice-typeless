//go:build !windows && !darwin

package paste

import (
	"sync"
	"time"
)

// readClipboard is a stub for unsupported platforms.
func readClipboard() (string, error) { return "", nil }

// writeClipboard is a stub for unsupported platforms.
func writeClipboard(text string) error { _ = text; return nil }

// stubPaster is a no-op Paster for unsupported platforms.
type stubPaster struct {
	mu    sync.Mutex
	cfg   PasteConfig
	guard ClipboardGuard
}

func newPlatformPaster(cfg PasteConfig) Paster {
	return &stubPaster{
		cfg:   cfg,
		guard: NewClipboardGuard(time.Duration(cfg.ClipboardHoldMs) * time.Millisecond),
	}
}

func (p *stubPaster) Paste(text string) error  { _ = text; return nil }
func (p *stubPaster) Configure(cfg PasteConfig) {
	p.mu.Lock()
	p.cfg = cfg
	p.mu.Unlock()
}
func (p *stubPaster) Close() error { return nil }
