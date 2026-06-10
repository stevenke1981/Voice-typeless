export interface LocalConfig {
  hotkey: {
    push_to_talk: string;
    free_speech: string;
    cancel: string;
  };
  audio: {
    device_id: string;
    enable_sounds: boolean;
    sound_volume: number; // 0.0–1.0
  };
  model: {
    active_model_id: string;
    device: 'auto' | 'directml' | 'cuda' | 'cpu';
  };
  text: {
    filter_filler_words: boolean;
    mixed_language_optimization: boolean;
    vad_silence_threshold_ms: number; // 1000–10000
  };
  ui: {
    theme: 'dark' | 'light' | 'system';
    language: 'zh' | 'en';
    show_floating_indicator: boolean;
    history_retention_days: number; // 0 = forever
    max_history_items: number;
  };
  system: {
    auto_start: boolean;
    minimize_to_tray: boolean;
    check_updates: boolean;
  };
}

export const DEFAULT_CONFIG: LocalConfig = {
  hotkey: { push_to_talk: 'Alt+Space', free_speech: 'Ctrl+Shift+V', cancel: 'Escape' },
  audio: { device_id: 'default', enable_sounds: true, sound_volume: 0.8 },
  model: { active_model_id: 'sensevoice-small', device: 'auto' },
  text: { filter_filler_words: true, mixed_language_optimization: true, vad_silence_threshold_ms: 3000 },
  ui: { theme: 'dark', language: 'en', show_floating_indicator: true, history_retention_days: 30, max_history_items: 50 },
  system: { auto_start: false, minimize_to_tray: true, check_updates: true },
};
