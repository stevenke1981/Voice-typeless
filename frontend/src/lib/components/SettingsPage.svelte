<script lang="ts">
  /**
   * SettingsPage — full application configuration UI.
   *
   * Sections (delegated to sub-components):
   *  1. Hotkeys   — push-to-talk / free-speech / cancel combos
   *  2. Audio     — microphone device picker + notification sounds toggle
   *  3. Model     — active model + inference device
   *  4. Text      — filler-word filter toggle, VAD silence threshold
   *  5. UI        — theme, floating indicator toggle, history retention
   *  6. System    — auto-start, minimize to tray, update checks
   *
   * Design decisions:
   *  - Uses a local `LocalConfig` type that extends the stub AppConfig from
   *    commands.ts with the additional fields defined in architecture.md §6.
   *    `setConfig` is called with a type assertion since the backend Rust layer
   *    accepts arbitrary Partial<AppConfig>.
   *  - Hotkey capture: clicking a hotkey field puts it into "listening" mode;
   *    the next key combo (with modifiers) is captured and written to state.
   *  - Theme changes are applied immediately to `document.documentElement`.
   *  - All changes are staged locally; nothing is persisted until "Save".
   */

  import { onMount } from 'svelte';
  import { getConfig, setConfig, getAutostartEnabled, type AppConfig } from '../tauri/commands';
  import { appState } from '../stores/appState.svelte';
  import { t, setLang } from '../i18n.svelte';
  import type { LocalConfig } from './settings/types';
  import { DEFAULT_CONFIG } from './settings/types';

  import HotkeySection from './settings/HotkeySection.svelte';
  import AudioSection from './settings/AudioSection.svelte';
  import ModelSection from './settings/ModelSection.svelte';
  import TextSection from './settings/TextSection.svelte';
  import UISection from './settings/UISection.svelte';
  import SystemSection from './settings/SystemSection.svelte';
  import SettingsFooter from './settings/SettingsFooter.svelte';

  // ─── Props ───────────────────────────────────────────────────────────────────

  interface Props {
    /** Callback invoked when the user clicks the back button. */
    onClose?: () => void;
  }

  const { onClose = undefined }: Props = $props();

  // ─── State ─────────────────────────────────────────────────────────────────

  let config = $state<LocalConfig>(structuredClone(DEFAULT_CONFIG));
  let isLoading = $state(true);
  let isSaving = $state(false);
  let saveError = $state('');
  let saveSuccess = $state(false);

  /** Key being actively recorded (null = none). */
  let recordingKey = $state<keyof LocalConfig['hotkey'] | null>(null);

  // ─── Lifecycle ─────────────────────────────────────────────────────────────

  onMount(async () => {
    isLoading = true;
    try {
      const raw: AppConfig = await getConfig();

      // Merge server response into local config (server fields take precedence).
      // The cast lets us read extended fields if the backend supports them.
      const extended = raw as unknown as Partial<LocalConfig>;

      if (raw.hotkey) {
        config.hotkey.push_to_talk = raw.hotkey.push_to_talk ?? DEFAULT_CONFIG.hotkey.push_to_talk;
        config.hotkey.free_speech  = raw.hotkey.free_speech  ?? DEFAULT_CONFIG.hotkey.free_speech;
        config.hotkey.cancel       = raw.hotkey.cancel       ?? DEFAULT_CONFIG.hotkey.cancel;
      }
      if (raw.audio) {
        config.audio.device_id     = raw.audio.device_id     ?? DEFAULT_CONFIG.audio.device_id;
        config.audio.enable_sounds = raw.audio.enable_sounds ?? DEFAULT_CONFIG.audio.enable_sounds;
        config.audio.sound_volume  = (extended.audio as any)?.sound_volume ?? DEFAULT_CONFIG.audio.sound_volume;
      }
      if (raw.model) {
        config.model.active_model_id = raw.model.active_model_id ?? DEFAULT_CONFIG.model.active_model_id;
        config.model.device          = raw.model.device          ?? DEFAULT_CONFIG.model.device;
      }
      if (extended.text) {
        config.text.filter_filler_words          = extended.text.filter_filler_words          ?? DEFAULT_CONFIG.text.filter_filler_words;
        config.text.mixed_language_optimization  = extended.text.mixed_language_optimization  ?? DEFAULT_CONFIG.text.mixed_language_optimization;
        config.text.vad_silence_threshold_ms     = extended.text.vad_silence_threshold_ms     ?? DEFAULT_CONFIG.text.vad_silence_threshold_ms;
      }
      if (raw.ui) {
        config.ui.theme                   = raw.ui.theme                   ?? DEFAULT_CONFIG.ui.theme;
        config.ui.language                = raw.ui.language                ?? DEFAULT_CONFIG.ui.language;
        config.ui.show_floating_indicator = raw.ui.show_floating_indicator ?? DEFAULT_CONFIG.ui.show_floating_indicator;
        config.ui.history_retention_days  = (extended.ui as any)?.history_retention_days ?? DEFAULT_CONFIG.ui.history_retention_days;
        config.ui.max_history_items       = (extended.ui as any)?.max_history_items      ?? DEFAULT_CONFIG.ui.max_history_items;
      }
      if (extended.system) {
        config.system.auto_start      = extended.system.auto_start      ?? DEFAULT_CONFIG.system.auto_start;
        config.system.minimize_to_tray = extended.system.minimize_to_tray ?? DEFAULT_CONFIG.system.minimize_to_tray;
        config.system.check_updates   = extended.system.check_updates   ?? DEFAULT_CONFIG.system.check_updates;
      }

      // Load real autostart state from OS registry
      try {
        config.system.auto_start = await getAutostartEnabled();
      } catch {
        /* ignore on non-Windows */
      }
    } catch {
      /* First run: defaults are fine */
    } finally {
      isLoading = false;
    }
  });

  // ─── Theme application ─────────────────────────────────────────────────────

  $effect(() => {
    const theme = config.ui.theme;
    appState.theme = theme as 'dark' | 'light' | 'system';
    const root = document.documentElement;
    root.classList.remove('theme-dark', 'theme-light');
    if (theme === 'dark') {
      root.classList.add('theme-dark');
    } else if (theme === 'light') {
      root.classList.add('theme-light');
    } else {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      root.classList.add(prefersDark ? 'theme-dark' : 'theme-light');
    }
  });

  // Sync language selector → i18n module
  $effect(() => { setLang(config.ui.language as 'en' | 'zh'); });

  // ─── Save ──────────────────────────────────────────────────────────────────

  async function save(): Promise<void> {
    isSaving = true;
    saveError = '';
    saveSuccess = false;
    try {
      // Cast to Partial<AppConfig> — the Rust handler accepts arbitrary JSON.
      await setConfig(config as unknown as Partial<AppConfig>);
      // @ts-ignore — hotkeyConfig is added by the parallel agent
      appState.hotkeyConfig = { ...config.hotkey };
      saveSuccess = true;
      setTimeout(() => { saveSuccess = false; }, 2500);
    } catch (err) {
      saveError = err instanceof Error ? err.message : String(err);
    } finally {
      isSaving = false;
    }
  }

  function resetDefaults(): void {
    config = structuredClone(DEFAULT_CONFIG);
  }

  // ─── Hotkey capture ────────────────────────────────────────────────────────

  /**
   * Start listening for the next key combo and write it to config.hotkey[field].
   */
  function startCapture(field: keyof LocalConfig['hotkey']): void {
    recordingKey = field;
  }

  function onHotkeyKeydown(
    field: keyof LocalConfig['hotkey'],
    e: KeyboardEvent,
  ): void {
    if (recordingKey !== field) return;
    e.preventDefault();
    e.stopPropagation();

    // Ignore bare modifier presses
    if (['Control', 'Alt', 'Shift', 'Meta', 'Process'].includes(e.key)) return;

    const parts: string[] = [];
    if (e.ctrlKey)  parts.push('Ctrl');
    if (e.altKey)   parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');
    if (e.metaKey)  parts.push('Super');

    // Normalise key name
    const keyName =
      e.key === ' '          ? 'Space'
      : e.key === 'Escape'   ? 'Escape'
      : e.key === 'Enter'    ? 'Enter'
      : e.key === 'Backspace' ? 'Backspace'
      : e.key === 'Tab'      ? 'Tab'
      : e.key === 'Delete'   ? 'Delete'
      : e.key.length === 1   ? e.key.toUpperCase()
      : e.key; // F1–F24, ArrowLeft, etc.

    parts.push(keyName);
    config.hotkey[field] = parts.join('+');
    recordingKey = null;
  }

  function onHotkeyBlur(): void {
    recordingKey = null;
  }
</script>

<!-- ── Page shell ──────────────────────────────────────────────────────────── -->
<div class="settings-page" aria-label="Settings" aria-busy={isLoading}>

  <!-- Back button row -->
  {#if onClose}
    <div class="settings-back-row">
      <button
        class="settings-back-btn"
        onclick={onClose}
        aria-label="Back to main page"
      >
        <span aria-hidden="true">←</span> Back
      </button>
    </div>
  {/if}

  {#if isLoading}
    <!-- Loading skeleton -->
    <div class="loading-skeleton" role="status" aria-live="polite">
      <span class="sr-only">Loading settings…</span>
      <div class="skeleton-row"></div>
      <div class="skeleton-row short"></div>
      <div class="skeleton-row"></div>
    </div>

  {:else}
    <HotkeySection
      {config}
      {recordingKey}
      {startCapture}
      {onHotkeyKeydown}
      {onHotkeyBlur}
    />
    <AudioSection {config} />
    <ModelSection {config} />
    <TextSection {config} />
    <UISection {config} />
    <SystemSection {config} />
    <SettingsFooter
      {config}
      {isSaving}
      {saveError}
      {saveSuccess}
      {save}
      {resetDefaults}
    />
  {/if}
</div>

<style>
  /* ── Page shell ────────────────────────────────────────────────────────────── */
  .settings-page {
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 0 0 24px;
    height: 100%;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--vtl-border) transparent;
  }

  .settings-page::-webkit-scrollbar { width: 4px; }
  .settings-page::-webkit-scrollbar-thumb {
    background: var(--vtl-border);
    border-radius: 2px;
  }

  /* Screen-reader only helper */
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  /* ── Shared section / field / control styles (global so children inherit) ── */
  :global(.settings-section) {
    padding: 20px 20px 0;
    border-bottom: 1px solid rgba(74, 74, 82, 0.4);
    padding-bottom: 20px;
  }

  :global(.section-heading) {
    margin: 0 0 4px;
    font-size: 12px;
    font-weight: 700;
    color: var(--vtl-gray);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  :global(.section-desc) {
    margin: 0 0 14px;
    font-size: 11px;
    color: var(--vtl-border);
  }

  :global(.field-group) {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  :global(.field-row) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 12px;
  }

  :global(.toggle-row) {
    align-items: flex-start;
    padding: 4px 0;
  }

  :global(.field-label) {
    flex: 1;
    font-size: 13px;
    color: var(--vtl-text-dark);
    display: flex;
    flex-direction: column;
    gap: 2px;
    cursor: default;
    min-width: 0;
  }

  :global(.field-hint) {
    font-size: 10px;
    color: var(--vtl-gray);
    font-weight: 400;
    line-height: 1.4;
  }

  /* Tailwind-like helper for DevicePicker spacing */
  :global(.mb-field) { margin-bottom: 12px; }

  /* ── Back button ────────────────────────────────────────────────────────────── */
  .settings-back-row {
    padding: 4px 8px 0;
    border-bottom: 1px solid rgba(74, 74, 82, 0.3);
  }

  .settings-back-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    color: var(--vtl-gray);
    font-size: 13px;
    font-family: inherit;
    cursor: pointer;
    padding: 8px 8px;
    border-radius: 6px;
    transition: color 0.15s, background 0.15s;
  }

  .settings-back-btn:hover {
    color: var(--vtl-text-dark);
    background: rgba(255, 255, 255, 0.04);
  }

  .settings-back-btn:focus-visible {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 2px;
  }

  :global(.toggle-btn) {
    flex-shrink: 0;
    width: 40px;
    height: 22px;
    background: var(--vtl-border);
    border: none;
    border-radius: 999px;
    cursor: pointer;
    padding: 0;
    position: relative;
    transition: background 0.2s;
    margin-top: 2px;
  }

  :global(.toggle-btn.on) {
    background: var(--vtl-teal);
  }

  :global(.toggle-btn:focus-visible) {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 2px;
  }

  :global(.toggle-thumb) {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    background: var(--vtl-text-dark);
    border-radius: 50%;
    transition: left 0.2s;
    pointer-events: none;
  }

  :global(.toggle-btn.on .toggle-thumb) {
    left: calc(100% - 19px);
  }

  :global(.range-input) {
    flex-shrink: 0;
    width: 140px;
    height: 4px;
    accent-color: var(--vtl-teal);
    cursor: pointer;
    background: var(--vtl-border);
    border-radius: 2px;
    appearance: none;
    -webkit-appearance: none;
    outline: none;
  }

  :global(.range-input::-webkit-slider-thumb) {
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--vtl-teal);
    cursor: pointer;
    box-shadow: 0 0 6px rgba(0, 230, 200, 0.40);
  }

  :global(.range-input:focus-visible) {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 3px;
  }

  :global(.select-input) {
    flex-shrink: 0;
    min-width: 180px;
    background: var(--vtl-bg-dark-2);
    border: 1px solid var(--vtl-border);
    border-radius: 8px;
    color: var(--vtl-text-dark);
    padding: 7px 12px;
    font-size: 13px;
    font-family: inherit;
    cursor: pointer;
    appearance: none;
    -webkit-appearance: none;
    outline: none;
    transition: border-color 0.15s;
  }

  :global(.select-input:focus-visible) {
    border-color: var(--vtl-teal);
    box-shadow: 0 0 0 2px rgba(0, 230, 200, 0.15);
  }

  :global(.select-input option) {
    background: var(--vtl-bg-dark-2);
    color: var(--vtl-text-dark);
  }

  /* ── Loading skeleton ─────────────────────────────────────────────────────── */
  .loading-skeleton {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .skeleton-row {
    height: 40px;
    border-radius: 8px;
    background: linear-gradient(
      90deg,
      var(--vtl-bg-dark-2) 0%,
      rgba(74, 74, 82, 0.3) 50%,
      var(--vtl-bg-dark-2) 100%
    );
    background-size: 200% 100%;
    animation: vtl-shimmer 1.6s ease-in-out infinite;
  }

  .skeleton-row.short { width: 60%; }

  @keyframes vtl-shimmer {
    0%   { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
</style>
