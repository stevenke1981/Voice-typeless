// Package config manages the Voice-typeless application configuration.
// Config is stored as JSON at the OS-appropriate AppData path.
package config

import (
	"encoding/json"
	"os"
	"path/filepath"
)

// AppConfig is the complete application configuration schema.
type AppConfig struct {
	Hotkey HotkeyConfig `json:"hotkey"`
	Audio  AudioConfig  `json:"audio"`
	Model  ModelConfig  `json:"model"`
	Text   TextConfig   `json:"text"`
	UI     UIConfig     `json:"ui"`
	System SystemConfig `json:"system"`
}

// HotkeyConfig stores hotkey bindings as human-readable strings (e.g. "Alt+Space").
type HotkeyConfig struct {
	PushToTalk string `json:"push_to_talk"`
	FreeSpeech string `json:"free_speech"`
	Cancel     string `json:"cancel"`
}

// AudioConfig controls microphone and sound settings.
type AudioConfig struct {
	DeviceID     string `json:"device_id"`
	SampleRate   uint32 `json:"sample_rate"`
	Channels     uint8  `json:"channels"`
	EnableSounds bool   `json:"enable_sounds"`
}

// ModelConfig selects the active speech recognition model.
type ModelConfig struct {
	ActiveModelID string `json:"active_model_id"`
	ModelsDir     string `json:"models_dir"` // populated at runtime from AppData
	Device        string `json:"device"`     // "auto", "directml", "cuda", "cpu"
}

// TextConfig controls text post-processing behaviour.
type TextConfig struct {
	Language                  string   `json:"language"`
	FilterFillerWords         bool     `json:"filter_filler_words"`
	MixedLanguageOptimization bool     `json:"mixed_language_optimization"`
	CustomDictionary          []string `json:"custom_dictionary"`
}

// UIConfig controls appearance and UI behaviour.
type UIConfig struct {
	Theme                 string         `json:"theme"` // "dark", "light", "system"
	Language              string         `json:"language"`
	ShowFloatingIndicator bool           `json:"show_floating_indicator"`
	IndicatorPosition     PositionConfig `json:"indicator_position"`
	HistoryRetentionDays  int            `json:"history_retention_days"`
	MaxHistoryItems       int            `json:"max_history_items"`
}

// PositionConfig holds screen coordinates for the floating indicator.
type PositionConfig struct {
	X int `json:"x"`
	Y int `json:"y"`
}

// SystemConfig controls OS-level integration.
type SystemConfig struct {
	AutoStart      bool `json:"auto_start"`
	MinimizeToTray bool `json:"minimize_to_tray"`
	CheckUpdates   bool `json:"check_updates"`
}

// Load reads the config from disk.
// Returns DefaultConfig() if the file does not exist yet.
func Load() (*AppConfig, error) {
	path, err := configPath()
	if err != nil {
		return nil, err
	}

	data, err := os.ReadFile(path)
	if os.IsNotExist(err) {
		cfg := DefaultConfig()
		return &cfg, nil
	}
	if err != nil {
		return nil, err
	}

	var cfg AppConfig
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, err
	}
	return &cfg, nil
}

// Save writes cfg to disk atomically (write to .tmp then rename).
func Save(cfg *AppConfig) error {
	path, err := configPath()
	if err != nil {
		return err
	}

	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}

	data, err := json.MarshalIndent(cfg, "", "  ")
	if err != nil {
		return err
	}

	tmp := path + ".tmp"
	if err := os.WriteFile(tmp, data, 0o600); err != nil {
		return err
	}
	return os.Rename(tmp, path)
}

func configPath() (string, error) {
	dir, err := os.UserConfigDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dir, "VoiceTypeless", "config.json"), nil
}
