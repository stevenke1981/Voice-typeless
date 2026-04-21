import { invoke } from "@tauri-apps/api/core";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface RecognitionResult {
  text: string;
  language: string;
  confidence: number;
  duration_ms: number;
}

export interface DeviceInfo {
  id: string;
  name: string;
}

export interface HistoryItem {
  id: string;
  text: string;
  language: string;
  timestamp: number;
}

export interface AppConfig {
  hotkey: {
    push_to_talk: string;
    free_speech: string;
    cancel: string;
  };
  audio: {
    device_id: string;
    enable_sounds: boolean;
  };
  model: {
    active_model_id: string;
    device: "auto" | "directml" | "cuda" | "cpu";
  };
  ui: {
    theme: "dark" | "light" | "system";
    language: "zh" | "en";
    show_floating_indicator: boolean;
  };
}

// ─── Commands ─────────────────────────────────────────────────────────────────

export const startRecording = (mode: "push_to_talk" | "free_speech") =>
  invoke<void>("start_recording", { mode });

export const stopRecording = () =>
  invoke<RecognitionResult>("stop_recording");

export const cancelRecording = () =>
  invoke<void>("cancel_recording");

export const getDevices = () =>
  invoke<DeviceInfo[]>("get_devices");

export const setDevice = (deviceId: string) =>
  invoke<void>("set_device", { deviceId });

export const getHistory = (limit = 50) =>
  invoke<HistoryItem[]>("get_history", { limit });

export const deleteHistoryItem = (id: string) =>
  invoke<void>("delete_history_item", { id });

export const getConfig = () =>
  invoke<AppConfig>("get_config");

export const setConfig = (config: Partial<AppConfig>) =>
  invoke<void>("set_config", { config });

// ─── New commands (Features 3–9) ──────────────────────────────────────────────

export interface Stats {
  total_items: number;
  total_chars: number;
  languages: Record<string, number>;
}

export const clearHistory = () =>
  invoke<void>("clear_history");

export const exportHistoryText = () =>
  invoke<string>("export_history_text");

export const getStats = () =>
  invoke<Stats>("get_stats");

export const runDemo = () =>
  invoke<void>("run_demo");

export const getAutostartEnabled = () =>
  invoke<boolean>("get_autostart_enabled");

export const setAutostartEnabled = (enable: boolean) =>
  invoke<void>("set_autostart_enabled", { enable });
