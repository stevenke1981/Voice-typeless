package hotkey

import (
	"fmt"
	"strings"
)

// ParseKeyCombo parses a human-readable combo string like "Alt+Space" or "Ctrl+Shift+V".
// Returns an error if the string is not recognisable.
func ParseKeyCombo(s string) (KeyCombo, error) {
	if s == "" {
		return KeyCombo{}, fmt.Errorf("hotkey: empty key combo string")
	}
	parts := strings.Split(s, "+")
	var mods Modifier
	key := ""
	for i, p := range parts {
		p = strings.TrimSpace(p)
		isLast := i == len(parts)-1
		switch strings.ToLower(p) {
		case "ctrl", "control":
			mods |= ModCtrl
		case "shift":
			mods |= ModShift
		case "alt":
			mods |= ModAlt
		case "super", "win", "cmd", "command":
			mods |= ModSuper
		default:
			if !isLast {
				return KeyCombo{}, fmt.Errorf("hotkey: unexpected modifier %q in %q", p, s)
			}
			key = p
		}
	}
	if key == "" {
		return KeyCombo{}, fmt.Errorf("hotkey: no key specified in %q", s)
	}
	return KeyCombo{Modifiers: mods, Key: key}, nil
}

// String returns the canonical string representation, e.g. "Ctrl+Shift+V".
func (k KeyCombo) String() string {
	var parts []string
	if k.Modifiers&ModCtrl != 0 {
		parts = append(parts, "Ctrl")
	}
	if k.Modifiers&ModShift != 0 {
		parts = append(parts, "Shift")
	}
	if k.Modifiers&ModAlt != 0 {
		parts = append(parts, "Alt")
	}
	if k.Modifiers&ModSuper != 0 {
		parts = append(parts, "Super")
	}
	parts = append(parts, k.Key)
	return strings.Join(parts, "+")
}
