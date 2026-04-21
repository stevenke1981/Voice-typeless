/**
 * Central application state — Svelte 5 runes only.
 * NO writable/readable stores; uses $state / $derived at module scope.
 *
 * This file MUST have the `.svelte.ts` extension so the Svelte compiler
 * processes rune syntax ($state, $derived) correctly.
 */

// ─── Types ────────────────────────────────────────────────────────────────────

export type AppStatus = 'idle' | 'recording' | 'processing' | 'error';

export type ModelId =
  | 'sensevoice-small'
  | 'whisper-tiny'
  | (string & {}); // allow arbitrary model IDs

// ─── Reactive State ───────────────────────────────────────────────────────────

/**
 * Single source of truth for the entire frontend application.
 * Mutate fields directly — Svelte 5 tracks them via the Proxy.
 */
export const appState = $state({
  /** Current recording/processing lifecycle state. */
  status: 'idle' as AppStatus,

  /** Most recently recognised text (updated on recognition-result). */
  currentText: '',

  /**
   * Duration of the last recording in milliseconds.
   * Reset to 0 on recording-started; set to actual duration on recording-stopped.
   */
  recordingDuration: 0,

  /** Human-readable error description when status === 'error'. */
  errorMessage: '',

  /** Active microphone device ID ('default' = OS default). */
  activeDevice: 'default',

  /** Currently loaded speech model identifier. */
  activeModel: 'sensevoice-small' as ModelId,

  /** Active inference hardware ('auto' probes DirectML → CUDA → CPU). */
  activeInferenceDevice: 'auto' as 'auto' | 'directml' | 'cuda' | 'cpu',

  /**
   * Model loading progress 0.0–1.0 (non-zero while model-loading events fire).
   * Resets to 0 when model-ready fires.
   */
  modelLoadProgress: 0,

  /** Stage reported by model-loading event. */
  modelLoadStage: '' as '' | 'download' | 'load' | 'warmup',

  /** Active UI theme — persisted in config.json via set_config. */
  theme: 'dark' as 'dark' | 'light' | 'system',
});

// ─── Derived Values (exported as functions — Svelte 5 module rule) ───────────

/** True while audio is being captured from the microphone. */
export function isRecording(): boolean { return appState.status === 'recording'; }

/** True while inference is running (between recording-stopped and recognition-result). */
export function isProcessing(): boolean { return appState.status === 'processing'; }

/** True when the app is idle and ready to record. */
export function isIdle(): boolean { return appState.status === 'idle'; }

/** True when an error has occurred. */
export function hasError(): boolean { return appState.status === 'error'; }

/**
 * True whenever the indicator should be visible (any non-idle status).
 * Convenience alias for template class bindings.
 */
export function isActive(): boolean { return appState.status !== 'idle'; }

// ─── Helpers ─────────────────────────────────────────────────────────────────

/** Reset error state and return to idle. */
export function clearError(): void {
  appState.status = 'idle';
  appState.errorMessage = '';
}
