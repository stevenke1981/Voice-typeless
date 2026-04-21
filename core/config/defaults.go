package config

// DefaultConfig returns the out-of-box application configuration.
func DefaultConfig() AppConfig {
	return AppConfig{
		Hotkey: HotkeyConfig{
			PushToTalk: "Alt+Space",
			FreeSpeech: "Ctrl+Shift+V",
			Cancel:     "Escape",
		},
		Audio: AudioConfig{
			DeviceID:     "default",
			SampleRate:   16000,
			Channels:     1,
			EnableSounds: true,
		},
		Model: ModelConfig{
			ActiveModelID: "sensevoice-small",
			ModelsDir:     "", // populated at runtime from AppData
			Device:        "auto",
		},
		Text: TextConfig{
			Language:                  "auto",
			FilterFillerWords:         true,
			MixedLanguageOptimization: true,
			CustomDictionary:          []string{},
		},
		UI: UIConfig{
			Theme:                 "dark",
			Language:              "en",
			ShowFloatingIndicator: true,
			IndicatorPosition:     PositionConfig{X: 100, Y: 100},
			HistoryRetentionDays:  30,
			MaxHistoryItems:       50,
		},
		System: SystemConfig{
			AutoStart:      false,
			MinimizeToTray: true,
			CheckUpdates:   true,
		},
	}
}
