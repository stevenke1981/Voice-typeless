package hotkey_test

import (
	"context"
	"testing"

	"github.com/vtl/core/hotkey"
)

func TestParseKeyCombo_Valid(t *testing.T) {
	cases := []struct {
		input string
		mods  hotkey.Modifier
		key   string
	}{
		{"Alt+Space", hotkey.ModAlt, "Space"},
		{"Ctrl+Shift+V", hotkey.ModCtrl | hotkey.ModShift, "V"},
		{"Escape", hotkey.ModNone, "Escape"},
	}
	for _, c := range cases {
		combo, err := hotkey.ParseKeyCombo(c.input)
		if err != nil {
			t.Errorf("ParseKeyCombo(%q) error: %v", c.input, err)
			continue
		}
		if combo.Modifiers != c.mods {
			t.Errorf("ParseKeyCombo(%q) mods = %v, want %v", c.input, combo.Modifiers, c.mods)
		}
		if combo.Key != c.key {
			t.Errorf("ParseKeyCombo(%q) key = %q, want %q", c.input, combo.Key, c.key)
		}
	}
}

func TestParseKeyCombo_Invalid(t *testing.T) {
	_, err := hotkey.ParseKeyCombo("")
	if err == nil {
		t.Error("expected error for empty combo")
	}
}

func TestKeyCombo_String(t *testing.T) {
	combo := hotkey.KeyCombo{Modifiers: hotkey.ModCtrl | hotkey.ModShift, Key: "V"}
	s := combo.String()
	if s == "" {
		t.Error("expected non-empty String()")
	}
}

func TestHotkeyManager_StubRun(t *testing.T) {
	mgr := hotkey.New()
	ctx, cancel := context.WithCancel(context.Background())
	cancel() // cancel immediately
	err := mgr.Run(ctx)
	// Run should return ctx.Err() when cancelled.
	if err == nil {
		t.Error("expected Run to return context error")
	}
}
