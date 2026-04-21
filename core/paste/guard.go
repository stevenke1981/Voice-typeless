package paste

import (
	"sync"
	"time"
)

// clipboardGuard is the cross-platform ClipboardGuard implementation.
// It serialises clipboard access with a mutex and delegates read/write to
// platform-specific readClipboard / writeClipboard functions.
type clipboardGuard struct {
	mu      sync.Mutex
	saved   string
	holdDur time.Duration
}

func newPlatformGuard(holdDuration time.Duration) ClipboardGuard {
	return &clipboardGuard{holdDur: holdDuration}
}

func (g *clipboardGuard) Save() error {
	g.mu.Lock()
	defer g.mu.Unlock()
	text, err := readClipboard()
	if err != nil {
		// Non-fatal: proceed without save if clipboard is unreadable.
		g.saved = ""
		return nil
	}
	g.saved = text
	return nil
}

func (g *clipboardGuard) Restore() error {
	g.mu.Lock()
	defer g.mu.Unlock()
	if g.saved == "" {
		return nil
	}
	return writeClipboard(g.saved)
}

func (g *clipboardGuard) HoldDuration() time.Duration {
	return g.holdDur
}
