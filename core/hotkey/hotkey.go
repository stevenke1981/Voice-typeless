// Package hotkey provides cross-platform global hotkey registration and dispatch.
package hotkey

import "context"

// Modifier represents a keyboard modifier key bitmask.
type Modifier uint32

const (
	ModNone  Modifier = 0
	ModCtrl  Modifier = 1 << iota // 2
	ModShift                      // 4
	ModAlt                        // 8
	ModSuper                      // 16 — Win key / Cmd key
)

// KeyCombo describes a hotkey combination.
type KeyCombo struct {
	Modifiers Modifier
	Key       string // e.g., "Space", "V", "F1"
}

// HotkeyAction identifies which action a hotkey triggers.
type HotkeyAction string

const (
	ActionPushToTalk HotkeyAction = "push_to_talk"
	ActionFreeSpeech HotkeyAction = "free_speech"
	ActionCancel     HotkeyAction = "cancel"
)

// HotkeyEvent is emitted when a registered hotkey is pressed or released.
type HotkeyEvent struct {
	Action  HotkeyAction
	Pressed bool // true = key down, false = key up
	Combo   KeyCombo
}

// HotkeyConfig maps actions to key combos.
type HotkeyConfig struct {
	PushToTalk KeyCombo
	FreeSpeech KeyCombo
	Cancel     KeyCombo
}

// HotkeyManager registers global hotkeys and emits events.
// Implementations are platform-specific (Windows: RegisterHotKey, macOS: Carbon).
type HotkeyManager interface {
	// Register registers all hotkeys in cfg.
	// Returns an error if any combo is already in use by another application.
	Register(cfg HotkeyConfig) error

	// Unregister releases all registered hotkeys.
	Unregister() error

	// Events returns a channel that receives HotkeyEvents.
	// The channel is buffered (capacity 16).
	Events() <-chan HotkeyEvent

	// Run blocks and processes OS hotkey messages until ctx is cancelled.
	Run(ctx context.Context) error
}

// New creates a platform-appropriate HotkeyManager.
// The returned manager is a no-op stub until platform-specific code is wired in.
func New() HotkeyManager { return &stubManager{events: make(chan HotkeyEvent, 16)} }

// stubManager is a no-op HotkeyManager used until platform drivers are ready.
type stubManager struct {
	events chan HotkeyEvent
}

func (s *stubManager) Register(cfg HotkeyConfig) error { return nil }
func (s *stubManager) Unregister() error               { return nil }
func (s *stubManager) Events() <-chan HotkeyEvent      { return s.events }
func (s *stubManager) Run(ctx context.Context) error {
	<-ctx.Done()
	return ctx.Err()
}
