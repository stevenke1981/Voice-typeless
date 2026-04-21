package paste_test

import (
	"testing"
	"time"

	"github.com/vtl/core/paste"
)

func TestNewPaster_NotNil(t *testing.T) {
	p := paste.NewPaster(paste.DefaultPasteConfig())
	if p == nil {
		t.Fatal("NewPaster returned nil")
	}
}

func TestNewClipboardGuard_NotNil(t *testing.T) {
	g := paste.NewClipboardGuard(150 * time.Millisecond)
	if g == nil {
		t.Fatal("NewClipboardGuard returned nil")
	}
}

func TestClipboardGuard_HoldDuration(t *testing.T) {
	hold := 200 * time.Millisecond
	g := paste.NewClipboardGuard(hold)
	if g.HoldDuration() != hold {
		t.Errorf("expected hold duration %v, got %v", hold, g.HoldDuration())
	}
}

func TestPaster_Configure(t *testing.T) {
	p := paste.NewPaster(paste.DefaultPasteConfig())
	cfg := paste.DefaultPasteConfig()
	cfg.ClipboardHoldMs = 300
	p.Configure(cfg) // must not panic
}

func TestPaster_Paste_Empty(t *testing.T) {
	p := paste.NewPaster(paste.DefaultPasteConfig())
	err := p.Paste("")
	if err != nil {
		t.Errorf("Paste empty string returned error: %v", err)
	}
}
