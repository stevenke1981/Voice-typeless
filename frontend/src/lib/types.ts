/**
 * Voice-typeless canonical TypeScript types.
 *
 * Source of truth derived from architecture.md §4.4 (IPC payload types)
 * and §6.1 (Configuration Schema — AppConfig canonical definition).
 *
 * Naming convention:
 *   CamelCase for all frontend-facing types (architecture §6.1).
 *   IPC types in commands.ts use snake_case to match the Go/Tauri protocol.
 */

// ─── IPC Payload Types (architecture §4.4) ───────────────────────────────────

export interface RecognitionResult {
  text: string;
  language: string;
  confidence: number;
  duration_ms: number;
  segments?: Segment[];
}

export interface Segment {
  text: string;
  start_ms: number;
  end_ms: number;
}

export interface DeviceInfo {
  id: string;
  name: string;
  is_default: boolean;
}

export interface DeviceList {
  devices: DeviceInfo[];
  active_device_id: string;
}

export interface HistoryItem {
  id: string;
  text: string;
  language: string;
  confidence: number;
  duration_ms: number;
  created_at: number; // Unix ms
}

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

export interface Stats {
  total_items: number;
  total_chars: number;
  languages: Record<string, number>;
}

// ─── Configuration Types (architecture §6.1) ─────────────────────────────────

export interface HotkeyConfig {
  /** Human-readable combo, e.g. "Alt+Space". */
  pushToTalk: string;
  freeSpeech: string;
  cancel: string;
}

export interface AudioConfig {
  /** Device ID string or "default". */
  deviceId: string;
  /** 16000 for speech; 44100 for high-quality capture. */
  sampleRate: 16000 | 44100;
  channels: 1 | 2;
  enableSounds: boolean;
  /** Volume 0.0–1.0 for notification sounds. */
  soundVolume: number;
}

export interface ModelConfig {
  activeModelId: string;
  /** Absolute path to models directory. */
  modelsDir: string;
  /** "auto" probes DirectML → CUDA → CPU at startup. */
  device: "auto" | "directml" | "cuda" | "cpu";
}

export type SupportedLanguage =
  | "auto"
  | "zh" | "en" | "ja" | "ko"
  | "fr" | "de" | "es" | "ru" | "it" | "pt";

export interface TextConfig {
  language: SupportedLanguage;
  filterFillerWords: boolean;
  /** Insert spaces at CJK/Latin boundaries; capitalize after sentence end. */
  mixedLanguageOptimization: boolean;
  /** User-defined replacement pairs: { input: "a i", output: "AI" }. */
  customDictionary: Array<{ input: string; output: string }>;
  /** Max silence before auto-stop in free-speech mode (ms). Default 3000. */
  vadSilenceThresholdMs: number;
}

export interface IndicatorPosition {
  x: number;
  y: number;
  /** Which display the indicator was last seen on (for multi-monitor). */
  displayId?: string;
}

export interface UIConfig {
  theme: "dark" | "light" | "system";
  /** UI display language. */
  language: "zh" | "en";
  showFloatingIndicator: boolean;
  indicatorPosition: IndicatorPosition;
  /** How many days to retain history items. 0 = forever. */
  historyRetentionDays: number;
  /** Maximum number of history items to store. */
  maxHistoryItems: number;
}

export interface SystemConfig {
  autoStart: boolean;
  minimizeToTray: boolean;
  /** Check GitHub releases for updates at startup. */
  checkUpdates: boolean;
  /** Log level: "debug" | "info" | "warn" | "error" */
  logLevel: string;
}

export interface AppConfig {
  /** Config file schema version for migration. */
  version: number;
  hotkey: HotkeyConfig;
  audio: AudioConfig;
  model: ModelConfig;
  text: TextConfig;
  ui: UIConfig;
  system: SystemConfig;
}
