/**
 * Tauri event listeners — typed wrappers around @tauri-apps/api/event.
 *
 * Call `setupEventListeners()` exactly once, inside `onMount` of the root
 * App component.  All listeners mutate `appState` directly.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { appState } from '../stores/appState.svelte';
import { startRecording, stopRecording, cancelRecording, retryEngine } from './commands';

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
  duration_ms: number;
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

export interface ModelErrorPayload {
  message: string;
}

export interface VadSilencePayload {
  duration_ms: number;
}

export interface DebugHotkeyEventPayload {
  action: string;
  accelerator: string;
  state: string;
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

      // Store debug info
      appState.lastRecordingDebug = {
        samples: (e.payload as any).sample_count ?? 0,
        duration_ms: e.payload.duration_ms,
        text_len: e.payload.text.length,
      };

      // Auto-paste is handled by the Rust backend in stop_recording.
      // The frontend only updates UI state.
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
      appState.engineLoaded = true;
      appState.errorMessage = ''; // clear engine-not-loaded error
    }),
  );

  // ── model-error (engine failed to load at startup) ────────────────────────
  unlisteners.push(
    await listen<ModelErrorPayload>('model-error', (e) => {
      appState.status = 'error';
      appState.errorMessage = e.payload.message;
      appState.modelLoadProgress = 0;
      appState.modelLoadStage = '';
      appState.engineLoaded = false;
    }),
  );

  // ── vad-silence-detected (informational; VAD auto-stop is handled in Core) ─
  unlisteners.push(
    await listen<VadSilencePayload>('vad-silence-detected', (_e) => {
      // Core will follow up with recording-stopped; nothing extra needed here.
    }),
  );

  // ── vad-auto-stop (emitted by Core VAD monitor on 3s silence) ──────────────
  unlisteners.push(
    await listen<void>('vad-auto-stop', () => {
      if (appState.status === 'recording') {
        stopRecording().catch(() => {});
      }
    }),
  );

  // ── model-required (emitted when engine loading fails) ──────────────────────
  unlisteners.push(
    await listen<ModelErrorPayload>('model-required', (e) => {
      appState.status = 'error';
      appState.errorMessage = e.payload.message;
      appState.modelLoadProgress = 0;
      appState.modelLoadStage = 'download';
    }),
  );

  // ── model-downloaded (emitted after background download completes) ─────────
  unlisteners.push(
    await listen<{ modelId: string; path: string }>('model-downloaded', () => {
      // Model files are on disk — try to load the engine
      appState.modelLoadStage = 'load';
      appState.modelLoadProgress = 0.5;
      retryEngine().catch((err) => {
        appState.status = 'error';
        appState.errorMessage = `Engine load failed: ${err}`;
        appState.modelLoadProgress = 0;
        appState.modelLoadStage = '';
      });
    }),
  );

  // ── hotkey-ptt-pressed ─────────────────────────────────────────────────────
  unlisteners.push(
    await listen<void>('hotkey-ptt-pressed', () => {
      if (appState.status === 'idle' || appState.status === 'error') {
        appState.errorMessage = '';    // clear previous error
        startRecording('push_to_talk').catch((err) => {
          appState.status = 'idle';
          appState.errorMessage = String(err);
        });
      }
    }),
  );

  // ── hotkey-ptt-released ────────────────────────────────────────────────────
  unlisteners.push(
    await listen<void>('hotkey-ptt-released', () => {
      if (appState.status === 'recording') {
        stopRecording().catch(() => {});
      }
    }),
  );

  // ── hotkey-free-speech (toggle) ────────────────────────────────────────────
  unlisteners.push(
    await listen<void>('hotkey-free-speech', () => {
      if (appState.status === 'idle' || appState.status === 'error') {
        appState.errorMessage = '';
        startRecording('free_speech').catch((err) => {
          appState.status = 'idle';
          appState.errorMessage = String(err);
        });
      } else if (appState.status === 'recording') {
        stopRecording().catch(() => {});
      }
    }),
  );

  // ── hotkey-cancel ──────────────────────────────────────────────────────────
  unlisteners.push(
    await listen<void>('hotkey-cancel', () => {
      if (appState.status === 'recording' || appState.status === 'processing') {
        cancelRecording().catch(() => {});
      }
    }),
  );

  // ── debug-hotkey-event (populates appState.lastHotkeyEvent) ────────────────
  unlisteners.push(
    await listen<DebugHotkeyEventPayload>('debug-hotkey-event', (e) => {
      appState.lastHotkeyEvent = {
        action: e.payload.action,
        accelerator: e.payload.accelerator,
        state: e.payload.state,
        receivedAt: Date.now(),
      };
    }),
  );

  // ── hotkey-registration (registration success/failure per hotkey) ─────────
  unlisteners.push(
    await listen<{ results: Array<{ action: string; hotkey: string; ok: boolean; error: string }> }>('hotkey-registration', (e) => {
      for (const r of e.payload.results) {
        appState.hotkeyRegistration[r.action] = { ok: r.ok, error: r.error };
        // Also update the hotkey display strings to what was actually configured
        if (r.action === 'push_to_talk') appState.hotkeyConfig.push_to_talk = r.hotkey;
        if (r.action === 'free_speech')  appState.hotkeyConfig.free_speech  = r.hotkey;
        if (r.action === 'cancel')       appState.hotkeyConfig.cancel       = r.hotkey;
      }
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
