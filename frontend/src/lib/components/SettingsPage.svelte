<script lang="ts">
  /**
   * SettingsPage — full application configuration UI.
   *
   * Sections:
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
  import { getConfig, setConfig, getAutostartEnabled, setAutostartEnabled, type AppConfig } from '../tauri/commands';
  import { appState } from '../stores/appState.svelte';
  import DevicePicker from './DevicePicker.svelte';
  import { t, setLang } from '../i18n.svelte';

  // ─── Extended config type ─────────────────────────────────────────────────
  //
  // The stub AppConfig in commands.ts is intentionally minimal.
  // We define a richer local type that mirrors architecture.md §6 and use it
  // for internal state.  When saving, we cast to `Partial<AppConfig>` because
  // the Tauri `set_config` command accepts any JSON object on the Rust side.

  interface LocalConfig {
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

  const DEFAULT_CONFIG: LocalConfig = {
    hotkey: { push_to_talk: 'Alt+Space', free_speech: 'Ctrl+Shift+V', cancel: 'Escape' },
    audio: { device_id: 'default', enable_sounds: true, sound_volume: 0.8 },
    model: { active_model_id: 'sensevoice-small', device: 'auto' },
    text: { filter_filler_words: true, mixed_language_optimization: true, vad_silence_threshold_ms: 3000 },
    ui: { theme: 'dark', language: 'en', show_floating_indicator: true, history_retention_days: 30, max_history_items: 50 },
    system: { auto_start: false, minimize_to_tray: true, check_updates: true },
  };

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

  // ─── Helpers ───────────────────────────────────────────────────────────────

  const HOTKEY_LABELS: Record<keyof LocalConfig['hotkey'], string> = {
    push_to_talk: 'settings.hotkeys.pushToTalk',
    free_speech:  'settings.hotkeys.freeSpeech',
    cancel:       'settings.hotkeys.cancel',
  };

  const THEME_OPTIONS: Array<{ value: LocalConfig['ui']['theme']; label: string }> = [
    { value: 'dark',   label: '🌙 Dark'   },
    { value: 'light',  label: '☀ Light'  },
    { value: 'system', label: '⚙ System' },
  ];

  const MODEL_OPTIONS = [
    { id: 'sensevoice-small', label: 'SenseVoice Small (recommended)' },
    { id: 'whisper-tiny',     label: 'Whisper Tiny (fallback)'       },
  ];

  const DEVICE_OPTIONS: Array<{ value: LocalConfig['model']['device']; label: string }> = [
    { value: 'auto',     label: 'Auto (DirectML → CUDA → CPU)' },
    { value: 'directml', label: 'DirectML (Windows GPU)'       },
    { value: 'cuda',     label: 'CUDA (NVIDIA GPU)'            },
    { value: 'cpu',      label: 'CPU only'                     },
  ];

  const LANG_OPTIONS: Array<{ value: LocalConfig['ui']['language']; label: string }> = [
    { value: 'en', label: 'English' },
    { value: 'zh', label: '中文'    },
  ];

  const VAD_MIN = 1000;
  const VAD_MAX = 10000;

  const retentionLabel = $derived.by(() => {
    const days = config.ui.history_retention_days;
    return days === 0 ? 'Keep forever' : `${days} day${days !== 1 ? 's' : ''}`;
  });

  const vadLabel = $derived.by(() => {
    const ms = config.text.vad_silence_threshold_ms;
    return `${(ms / 1000).toFixed(1)} s`;
  });
</script>

<!-- ── Page shell ──────────────────────────────────────────────────────────── -->
<div class="settings-page" aria-label="Settings" aria-busy={isLoading}>

  {#if isLoading}
    <!-- Loading skeleton -->
    <div class="loading-skeleton" role="status" aria-live="polite">
      <span class="sr-only">Loading settings…</span>
      <div class="skeleton-row"></div>
      <div class="skeleton-row short"></div>
      <div class="skeleton-row"></div>
    </div>

  {:else}

    <!-- ── Section: Hotkeys ──────────────────────────────────────────────── -->
    <section class="settings-section" aria-labelledby="hotkey-heading">
      <h3 id="hotkey-heading" class="section-heading">
        <span aria-hidden="true">⌨</span> {t('settings.section.hotkeys')}
      </h3>
      <p class="section-desc">{t('settings.hotkeys.desc')}</p>

      <div class="field-group">
        {#each Object.keys(HOTKEY_LABELS) as field (field)}
          {@const key = field as keyof LocalConfig['hotkey']}
          {@const isCapturing = recordingKey === key}
          <div class="field-row">
            <label
              for="hotkey-{field}"
              class="field-label"
            >
              {t(HOTKEY_LABELS[key])}
            </label>
            <button
              id="hotkey-{field}"
              class="hotkey-input"
              class:capturing={isCapturing}
              onclick={() => startCapture(key)}
              onkeydown={(e) => onHotkeyKeydown(key, e)}
              onblur={onHotkeyBlur}
              aria-label="{t(HOTKEY_LABELS[key])} hotkey: {config.hotkey[key]}. Click to change."
              aria-pressed={isCapturing}
              title={isCapturing ? 'Press a key combination…' : 'Click to record hotkey'}
              type="button"
            >
              {#if isCapturing}
                <span class="capturing-hint">Press keys…</span>
              {:else}
                <kbd class="kbd">{config.hotkey[key]}</kbd>
              {/if}
            </button>
          </div>
        {/each}
      </div>
    </section>

    <!-- ── Section: Audio ────────────────────────────────────────────────── -->
    <section class="settings-section" aria-labelledby="audio-heading">
      <h3 id="audio-heading" class="section-heading">
        <span aria-hidden="true">🎙</span> {t('settings.section.audio')}
      </h3>

      <!-- Embedded device picker -->
      <DevicePicker class="mb-field" />

      <!-- Notification sounds toggle -->
      <div class="field-row toggle-row">
        <label for="enable-sounds" class="field-label">
          {t('settings.audio.sounds')}
          <span class="field-hint">{t('settings.audio.soundsHint')}</span>
        </label>
        <button
          id="enable-sounds"
          class="toggle-btn"
          class:on={config.audio.enable_sounds}
          onclick={() => (config.audio.enable_sounds = !config.audio.enable_sounds)}
          role="switch"
          aria-checked={config.audio.enable_sounds}
          aria-label="Notification sounds: {config.audio.enable_sounds ? 'enabled' : 'disabled'}"
        >
          <span class="toggle-thumb"></span>
        </button>
      </div>

      <!-- Volume slider (visible only when sounds enabled) -->
      {#if config.audio.enable_sounds}
        <div class="field-row">
          <label for="sound-volume" class="field-label">
            {t('settings.audio.volume')}
            <span class="field-hint">{Math.round(config.audio.sound_volume * 100)}%</span>
          </label>
          <input
            id="sound-volume"
            type="range"
            class="range-input"
            min="0"
            max="1"
            step="0.05"
            bind:value={config.audio.sound_volume}
            aria-label="Sound volume: {Math.round(config.audio.sound_volume * 100)}%"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(config.audio.sound_volume * 100)}
          />
        </div>
      {/if}
    </section>

    <!-- ── Section: Model ────────────────────────────────────────────────── -->
    <section class="settings-section" aria-labelledby="model-heading">
      <h3 id="model-heading" class="section-heading">
        <span aria-hidden="true">🧠</span> {t('settings.section.model')}
      </h3>

      <div class="field-row">
        <label for="active-model" class="field-label">{t('settings.model.active')}</label>
        <select
          id="active-model"
          class="select-input"
          bind:value={config.model.active_model_id}
          aria-label="Select speech model"
        >
          {#each MODEL_OPTIONS as m (m.id)}
            <option value={m.id}>{m.label}</option>
          {/each}
        </select>
      </div>

      <div class="field-row">
        <label for="inference-device" class="field-label">
          {t('settings.model.device')}
          <span class="field-hint">{t('settings.model.deviceHint')}</span>
        </label>
        <select
          id="inference-device"
          class="select-input"
          bind:value={config.model.device}
          aria-label="Select inference hardware device"
        >
          {#each DEVICE_OPTIONS as d (d.value)}
            <option value={d.value}>{d.label}</option>
          {/each}
        </select>
      </div>
    </section>

    <!-- ── Section: Text Processing ─────────────────────────────────────── -->
    <section class="settings-section" aria-labelledby="text-heading">
      <h3 id="text-heading" class="section-heading">
        <span aria-hidden="true">✍</span> {t('settings.section.text')}
      </h3>

      <!-- Filler word filter -->
      <div class="field-row toggle-row">
        <label for="filter-filler" class="field-label">
          {t('settings.text.filterFiller')}
          <span class="field-hint">{t('settings.text.filterFillerHint')}</span>
        </label>
        <button
          id="filter-filler"
          class="toggle-btn"
          class:on={config.text.filter_filler_words}
          onclick={() => (config.text.filter_filler_words = !config.text.filter_filler_words)}
          role="switch"
          aria-checked={config.text.filter_filler_words}
          aria-label="Filter filler words: {config.text.filter_filler_words ? 'enabled' : 'disabled'}"
        >
          <span class="toggle-thumb"></span>
        </button>
      </div>

      <!-- Mixed-language optimisation -->
      <div class="field-row toggle-row">
        <label for="lang-mix" class="field-label">
          {t('settings.text.mixedLang')}
          <span class="field-hint">{t('settings.text.mixedLangHint')}</span>
        </label>
        <button
          id="lang-mix"
          class="toggle-btn"
          class:on={config.text.mixed_language_optimization}
          onclick={() => (config.text.mixed_language_optimization = !config.text.mixed_language_optimization)}
          role="switch"
          aria-checked={config.text.mixed_language_optimization}
          aria-label="Mixed-language optimisation: {config.text.mixed_language_optimization ? 'enabled' : 'disabled'}"
        >
          <span class="toggle-thumb"></span>
        </button>
      </div>

      <!-- VAD silence threshold -->
      <div class="field-row">
        <label for="vad-threshold" class="field-label">
          {t('settings.text.vadThreshold')}
          <span class="field-hint">{vadLabel} of silence triggers stop in free-speech mode</span>
        </label>
        <input
          id="vad-threshold"
          type="range"
          class="range-input"
          min={VAD_MIN}
          max={VAD_MAX}
          step={500}
          bind:value={config.text.vad_silence_threshold_ms}
          aria-label="Auto-stop silence threshold: {vadLabel}"
          aria-valuemin={VAD_MIN}
          aria-valuemax={VAD_MAX}
          aria-valuenow={config.text.vad_silence_threshold_ms}
          aria-valuetext={vadLabel}
        />
      </div>
    </section>

    <!-- ── Section: UI ────────────────────────────────────────────────────── -->
    <section class="settings-section" aria-labelledby="ui-heading">
      <h3 id="ui-heading" class="section-heading">
        <span aria-hidden="true">🎨</span> {t('settings.section.interface')}
      </h3>

      <!-- Theme selection -->
      <div class="field-row">
        <label class="field-label" id="theme-group-label">{t('settings.ui.theme')}</label>
        <div
          class="theme-buttons"
          role="group"
          aria-labelledby="theme-group-label"
        >
          {#each THEME_OPTIONS as themeOpt (themeOpt.value)}
            <button
              class="theme-btn"
              class:active={config.ui.theme === themeOpt.value}
              onclick={() => (config.ui.theme = themeOpt.value)}
              aria-pressed={config.ui.theme === themeOpt.value}
              aria-label="Theme: {themeOpt.label}"
            >
              {themeOpt.label}
            </button>
          {/each}
        </div>
      </div>

      <!-- UI language -->
      <div class="field-row">
        <label for="ui-language" class="field-label">{t('settings.ui.language')}</label>
        <select
          id="ui-language"
          class="select-input select-narrow"
          bind:value={config.ui.language}
          aria-label="Interface display language"
        >
          {#each LANG_OPTIONS as l (l.value)}
            <option value={l.value}>{l.label}</option>
          {/each}
        </select>
      </div>

      <!-- Floating indicator toggle -->
      <div class="field-row toggle-row">
        <label for="show-indicator" class="field-label">
          {t('settings.ui.indicator')}
          <span class="field-hint">{t('settings.ui.indicatorHint')}</span>
        </label>
        <button
          id="show-indicator"
          class="toggle-btn"
          class:on={config.ui.show_floating_indicator}
          onclick={() => (config.ui.show_floating_indicator = !config.ui.show_floating_indicator)}
          role="switch"
          aria-checked={config.ui.show_floating_indicator}
          aria-label="Floating indicator: {config.ui.show_floating_indicator ? 'enabled' : 'disabled'}"
        >
          <span class="toggle-thumb"></span>
        </button>
      </div>

      <!-- History retention slider -->
      <div class="field-row">
        <label for="history-retention" class="field-label">
          {t('settings.ui.retention')}
          <span class="field-hint">{retentionLabel}</span>
        </label>
        <input
          id="history-retention"
          type="range"
          class="range-input"
          min="0"
          max="365"
          step="1"
          bind:value={config.ui.history_retention_days}
          aria-label="History retention: {retentionLabel}"
          aria-valuemin={0}
          aria-valuemax={365}
          aria-valuenow={config.ui.history_retention_days}
          aria-valuetext={retentionLabel}
        />
      </div>

      <!-- Max history items -->
      <div class="field-row">
        <label for="max-history" class="field-label">
          {t('settings.ui.maxHistory')}
          <span class="field-hint">{config.ui.max_history_items} items</span>
        </label>
        <input
          id="max-history"
          type="range"
          class="range-input"
          min="10"
          max="500"
          step="10"
          bind:value={config.ui.max_history_items}
          aria-label="Max history items: {config.ui.max_history_items}"
          aria-valuemin={10}
          aria-valuemax={500}
          aria-valuenow={config.ui.max_history_items}
        />
      </div>
    </section>

    <!-- ── Section: System ────────────────────────────────────────────────── -->
    <section class="settings-section" aria-labelledby="system-heading">
      <h3 id="system-heading" class="section-heading">
        <span aria-hidden="true">⚙</span> {t('settings.section.system')}
      </h3>

      <div class="field-row toggle-row">
        <label for="auto-start" class="field-label">
          {t('settings.system.autoStart')}
          <span class="field-hint">{t('settings.system.autoStartHint')}</span>
        </label>
        <button
          id="auto-start"
          class="toggle-btn"
          class:on={config.system.auto_start}
          onclick={async () => {
            const next = !config.system.auto_start;
            config.system.auto_start = next;
            try { await setAutostartEnabled(next); } catch { config.system.auto_start = !next; }
          }}
          role="switch"
          aria-checked={config.system.auto_start}
          aria-label="Launch at login: {config.system.auto_start ? 'enabled' : 'disabled'}"
        >
          <span class="toggle-thumb"></span>
        </button>
      </div>

      <div class="field-row toggle-row">
        <label for="minimize-tray" class="field-label">
          {t('settings.system.tray')}
          <span class="field-hint">{t('settings.system.trayHint')}</span>
        </label>
        <button
          id="minimize-tray"
          class="toggle-btn"
          class:on={config.system.minimize_to_tray}
          onclick={() => (config.system.minimize_to_tray = !config.system.minimize_to_tray)}
          role="switch"
          aria-checked={config.system.minimize_to_tray}
          aria-label="Minimize to tray: {config.system.minimize_to_tray ? 'enabled' : 'disabled'}"
        >
          <span class="toggle-thumb"></span>
        </button>
      </div>

      <div class="field-row toggle-row">
        <label for="check-updates" class="field-label">
          {t('settings.system.updates')}
          <span class="field-hint">{t('settings.system.updatesHint')}</span>
        </label>
        <button
          id="check-updates"
          class="toggle-btn"
          class:on={config.system.check_updates}
          onclick={() => (config.system.check_updates = !config.system.check_updates)}
          role="switch"
          aria-checked={config.system.check_updates}
          aria-label="Check for updates: {config.system.check_updates ? 'enabled' : 'disabled'}"
        >
          <span class="toggle-thumb"></span>
        </button>
      </div>
    </section>

    <!-- ── Footer: Save / Reset ──────────────────────────────────────────── -->
    <footer class="settings-footer">
      {#if saveError}
        <p class="footer-error" role="alert">
          <span aria-hidden="true">⚠</span> {saveError}
        </p>
      {/if}
      {#if saveSuccess}
        <p class="footer-success" role="status" aria-live="polite">
          <span aria-hidden="true">✓</span> {t('settings.saved')}
        </p>
      {/if}

      <div class="footer-actions">
        <button
          class="btn-ghost"
          onclick={resetDefaults}
          aria-label="Reset all settings to defaults"
        >
          {t('settings.reset')}
        </button>
        <button
          class="btn-primary"
          onclick={save}
          disabled={isSaving}
          aria-label={isSaving ? t('settings.saving') : t('settings.save')}
          aria-busy={isSaving}
        >
          {isSaving ? t('settings.saving') : t('settings.save')}
        </button>
      </div>
    </footer>

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

  /* ── Section ─────────────────────────────────────────────────────────────── */
  .settings-section {
    padding: 20px 20px 0;
    border-bottom: 1px solid rgba(74, 74, 82, 0.4);
    padding-bottom: 20px;
  }

  .section-heading {
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

  .section-desc {
    margin: 0 0 14px;
    font-size: 11px;
    color: var(--vtl-border);
  }

  /* ── Field layout ────────────────────────────────────────────────────────── */
  .field-group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .field-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 12px;
  }

  .toggle-row {
    align-items: flex-start;
    padding: 4px 0;
  }

  .field-label {
    flex: 1;
    font-size: 13px;
    color: var(--vtl-text-dark);
    display: flex;
    flex-direction: column;
    gap: 2px;
    cursor: default;
    min-width: 0;
  }

  .field-hint {
    font-size: 10px;
    color: var(--vtl-gray);
    font-weight: 400;
    line-height: 1.4;
  }

  /* Tailwind-like helper for DevicePicker spacing */
  :global(.mb-field) { margin-bottom: 12px; }

  /* ── Hotkey input button ─────────────────────────────────────────────────── */
  .hotkey-input {
    background: var(--vtl-bg-dark-2);
    border: 1px solid var(--vtl-border);
    border-radius: 8px;
    padding: 6px 12px;
    min-width: 140px;
    text-align: center;
    cursor: pointer;
    color: var(--vtl-text-dark);
    font-family: inherit;
    font-size: 13px;
    transition: border-color 0.15s, box-shadow 0.15s, background 0.15s;
    flex-shrink: 0;
  }

  .hotkey-input:hover {
    border-color: var(--vtl-teal);
  }

  .hotkey-input:focus-visible {
    outline: none;
    border-color: var(--vtl-teal);
    box-shadow: 0 0 0 2px rgba(0, 230, 200, 0.20);
  }

  .hotkey-input.capturing {
    border-color: var(--vtl-indigo);
    background: rgba(91, 78, 255, 0.08);
    box-shadow: 0 0 0 2px rgba(91, 78, 255, 0.20);
    animation: vtl-pulse 1.2s ease-in-out infinite;
  }

  @keyframes vtl-pulse {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.7; }
  }

  .capturing-hint {
    color: var(--vtl-indigo);
    font-size: 11px;
    font-style: italic;
  }

  .kbd {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: var(--vtl-teal);
  }

  /* ── Toggle switch ────────────────────────────────────────────────────────── */
  .toggle-btn {
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

  .toggle-btn.on {
    background: var(--vtl-teal);
  }

  .toggle-btn:focus-visible {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 2px;
  }

  .toggle-thumb {
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

  .toggle-btn.on .toggle-thumb {
    left: calc(100% - 19px);
  }

  /* ── Range slider ─────────────────────────────────────────────────────────── */
  .range-input {
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

  .range-input::-webkit-slider-thumb {
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--vtl-teal);
    cursor: pointer;
    box-shadow: 0 0 6px rgba(0, 230, 200, 0.40);
  }

  .range-input:focus-visible {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 3px;
  }

  /* ── Select input ─────────────────────────────────────────────────────────── */
  .select-input {
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

  .select-input:focus-visible {
    border-color: var(--vtl-teal);
    box-shadow: 0 0 0 2px rgba(0, 230, 200, 0.15);
  }

  .select-input option {
    background: var(--vtl-bg-dark-2);
    color: var(--vtl-text-dark);
  }

  .select-narrow { min-width: 120px; }

  /* ── Theme button group ───────────────────────────────────────────────────── */
  .theme-buttons {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .theme-btn {
    background: var(--vtl-bg-dark-2);
    border: 1px solid var(--vtl-border);
    border-radius: 7px;
    color: var(--vtl-gray);
    padding: 6px 10px;
    font-size: 11px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s, background 0.15s;
    white-space: nowrap;
  }

  .theme-btn:hover {
    border-color: var(--vtl-teal);
    color: var(--vtl-text-dark);
  }

  .theme-btn.active {
    border-color: var(--vtl-teal);
    background: rgba(0, 230, 200, 0.10);
    color: var(--vtl-teal);
  }

  .theme-btn:focus-visible {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 2px;
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

  /* ── Footer ───────────────────────────────────────────────────────────────── */
  .settings-footer {
    padding: 20px 20px 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .footer-error {
    margin: 0;
    font-size: 12px;
    color: #ff6b6b;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .footer-success {
    margin: 0;
    font-size: 12px;
    color: var(--vtl-green);
    display: flex;
    align-items: center;
    gap: 6px;
    animation: vtl-fadein 0.2s ease;
  }

  @keyframes vtl-fadein {
    from { opacity: 0; transform: translateY(4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .footer-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
  }

  /* Ghost secondary button */
  .btn-ghost {
    background: none;
    border: 1px solid var(--vtl-border);
    border-radius: 8px;
    color: var(--vtl-gray);
    padding: 8px 16px;
    font-size: 13px;
    font-family: inherit;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }

  .btn-ghost:hover {
    border-color: var(--vtl-text-dark);
    color: var(--vtl-text-dark);
  }

  .btn-ghost:focus-visible {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 2px;
  }

  /* Primary action button */
  .btn-primary {
    background: var(--vtl-teal);
    border: none;
    border-radius: 8px;
    color: var(--vtl-bg-dark);
    padding: 8px 20px;
    font-size: 13px;
    font-weight: 700;
    font-family: inherit;
    cursor: pointer;
    transition: opacity 0.15s, box-shadow 0.15s;
    box-shadow: 0 0 10px rgba(0, 230, 200, 0.20);
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.90;
    box-shadow: 0 0 16px rgba(0, 230, 200, 0.35);
  }

  .btn-primary:focus-visible {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 3px;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
