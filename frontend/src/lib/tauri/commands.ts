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
  is_default: boolean;
  channels: number;
  sample_rates: number[];
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

/**
 * Paste text by writing to clipboard and simulating keyboard paste (Ctrl+V / Cmd+V).
 * Falls back gracefully if paste simulation is not available on the current platform.
 */
export async function pasteText(text: string): Promise<void> {
    await invoke('paste_text', { text });
}

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

// ─── Model commands (architecture §6.1) ──────────────────────────────────────

export interface ModelInfo {
  id: string;
  name: string;
  type: "sensevoice" | "whisper-tiny" | "custom-onnx";
  size_bytes: number;
  languages: string[];
  is_active: boolean;
  is_downloaded: boolean;
  device: "directml" | "cuda" | "cpu" | null;
}

export const getModelList = () =>
  invoke<ModelInfo[]>("get_model_list");

export const setActiveModel = (modelId: string) =>
  invoke<void>("set_active_model", { modelId });

export const retryEngine = () =>
  invoke<void>("retry_engine");

export interface HotkeyRegEntry {
  action: string;
  hotkey: string;
  ok: boolean;
  error: string;
}

export interface EngineStatus {
  loaded: boolean;
  model_id: string;
  device: string;
  hotkey_registration: HotkeyRegEntry[];
}

export const getEngineStatus = () =>
  invoke<EngineStatus>("get_engine_status");
