/**
 * Tauri event listeners — typed wrappers around @tauri-apps/api/event.
 *
 * Call `setupEventListeners()` exactly once, inside `onMount` of the root
 * App component.  All listeners mutate `appState` directly.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { appState } from '../stores/appState.svelte';

// ─── IPC Event Payload Types (mirrors architecture.md §4.2) ──────────────────

export interface RecordingStartedPayload {
  /** Unix timestamp (ms) of when the Core layer began capture. */
  timestamp: number;
}

export interface RecordingStoppedPayload {
  /** Total duration of the audio capture in milliseconds. */
  duration_ms: number;
}

export interface RecognitionResultPayload {
  text: string;
  language: string;
  confidence: number;
  segments?: Array<{ text: string; start_ms: number; end_ms: number }>;
}

export interface RecognitionErrorPayload {
  message: string;
  /** Machine-readable code, e.g. "MODEL_NOT_LOADED" | "AUDIO_DEVICE_ERROR". */
  code: string;
}

export interface ModelLoadingPayload {
  /** Progress 0.0–1.0. */
  progress: number;
  stage: 'download' | 'load' | 'warmup';
}

export interface ModelReadyPayload {
  modelId: string;
  device: 'directml' | 'cuda' | 'cpu';
}

export interface VadSilencePayload {
  duration_ms: number;
}

// ─── Listener Registry ────────────────────────────────────────────────────────

/** Collected unlisten functions; call `teardownEventListeners()` on app exit. */
const unlisteners: UnlistenFn[] = [];

/**
 * Register all Tauri → Frontend event handlers.
 * Safe to call only once (guards are not required — Tauri deduplicates by name).
 */
export async function setupEventListeners(): Promise<void> {
  // ── recording-started ──────────────────────────────────────────────────────
  unlisteners.push(
    await listen<RecordingStartedPayload>('recording-started', (_e) => {
      appState.status = 'recording';
      appState.recordingDuration = 0;
      appState.errorMessage = '';
    }),
  );

  // ── recording-stopped ─────────────────────────────────────────────────────
  unlisteners.push(
    await listen<RecordingStoppedPayload>('recording-stopped', (e) => {
      appState.status = 'processing';
      appState.recordingDuration = e.payload.duration_ms;
    }),
  );

  // ── recognition-result ────────────────────────────────────────────────────
  unlisteners.push(
    await listen<RecognitionResultPayload>('recognition-result', (e) => {
      appState.status = 'idle';
      appState.currentText = e.payload.text;
    }),
  );

  // ── recognition-error ─────────────────────────────────────────────────────
  unlisteners.push(
    await listen<RecognitionErrorPayload>('recognition-error', (e) => {
      appState.status = 'error';
      appState.errorMessage = `[${e.payload.code}] ${e.payload.message}`;
    }),
  );

  // ── recording-cancelled (emitted by Core when Esc is pressed) ─────────────
  unlisteners.push(
    await listen<void>('recording-cancelled', () => {
      appState.status = 'idle';
      appState.recordingDuration = 0;
      appState.errorMessage = '';
    }),
  );

  // ── model-loading (progress updates during model init) ────────────────────
  unlisteners.push(
    await listen<ModelLoadingPayload>('model-loading', (e) => {
      appState.modelLoadProgress = e.payload.progress;
      appState.modelLoadStage = e.payload.stage;
    }),
  );

  // ── model-ready ───────────────────────────────────────────────────────────
  unlisteners.push(
    await listen<ModelReadyPayload>('model-ready', (e) => {
      appState.activeModel = e.payload.modelId;
      appState.activeInferenceDevice = e.payload.device;
      appState.modelLoadProgress = 0;
      appState.modelLoadStage = '';
    }),
  );

  // ── vad-silence-detected (informational; VAD auto-stop is handled in Core) ─
  unlisteners.push(
    await listen<VadSilencePayload>('vad-silence-detected', (_e) => {
      // Core will follow up with recording-stopped; nothing extra needed here.
    }),
  );
}

/**
 * Release all active Tauri event subscriptions.
 * Call in the application's destroy lifecycle if needed.
 */
export function teardownEventListeners(): void {
  for (const unlisten of unlisteners) {
    unlisten();
  }
  unlisteners.length = 0;
}
