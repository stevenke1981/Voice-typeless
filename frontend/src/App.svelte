<script lang="ts">
  /**
   * App.svelte — root application shell for Voice-typeless.
   *
   * Responsibilities:
   *  - Mount FloatingIndicator (always rendered, shows/hides itself)
   *  - Call setupEventListeners() once on mount to connect Tauri IPC
   *  - Render a top nav bar (logo + settings toggle)
   *  - Swap between HistoryPanel and SettingsPage based on showSettings
   *  - Apply dark class to <html> immediately (dark-first design)
   *  - Reflect status in footer indicator dot
   *  - Apply subtle background shift on recording/processing
   */

  import { onMount } from 'svelte';
  import FloatingIndicator from './lib/components/FloatingIndicator.svelte';
  import HistoryPanel from './lib/components/HistoryPanel.svelte';
  import SettingsPage from './lib/components/SettingsPage.svelte';
  import { setupEventListeners, teardownEventListeners } from './lib/tauri/events';
  import { appState, isRecording } from './lib/stores/appState.svelte';
  import { getConfig, getEngineStatus } from './lib/tauri/commands';

  // ─── Navigation state ────────────────────────────────────────────────────

  let showSettings = $state(false);

  // ─── Theme ───────────────────────────────────────────────────────────────

  function applyTheme(theme: 'dark' | 'light' | 'system'): void {
    const root = document.documentElement;
    root.classList.remove('theme-dark', 'theme-light');
    if (theme === 'light') {
      root.classList.add('theme-light');
    } else if (theme === 'dark') {
      root.classList.add('theme-dark');
    } else {
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      root.classList.add(prefersDark ? 'theme-dark' : 'theme-light');
    }
  }

  $effect(() => { applyTheme(appState.theme); });

  // ─── Lifecycle ───────────────────────────────────────────────────────────

  onMount(() => {
    (async () => {
      await setupEventListeners();

      // Poll initial engine + hotkey registration status (guards against
      // events lost during setup → webview race where IPC events arrived
      // before the frontend was ready to receive them).
      try {
        const status = await getEngineStatus();
        if (status.loaded) {
          appState.engineLoaded = true;
          appState.modelLoadProgress = 0;
          appState.modelLoadStage = '';
        }
        if (status.hotkey_registration?.length) {
          for (const r of status.hotkey_registration) {
            appState.hotkeyRegistration[r.action] = { ok: r.ok, error: r.error };
          }
        }
      } catch {
        // silent — we'll retry later if needed
      }

      // Load persisted config (theme + hotkeys)
      try {
        const cfg = await getConfig();
        appState.theme = (cfg.ui?.theme ?? 'dark') as 'dark' | 'light' | 'system';
        appState.hotkeyConfig = {
          push_to_talk: (cfg.hotkey as any)?.push_to_talk ?? 'Alt+Space',
          free_speech:  (cfg.hotkey as any)?.free_speech  ?? 'Ctrl+Shift+V',
          cancel:       (cfg.hotkey as any)?.cancel       ?? 'Ctrl+Shift+Escape',
        };
      } catch {
        appState.theme = 'dark';
      }
      applyTheme(appState.theme);
    })();

    return () => {
      teardownEventListeners();
    };
  });

  // ─── Status label ────────────────────────────────────────────────────────

  const STATUS_LABELS: Record<string, string> = {
    idle:       'Ready',
    recording:  'Recording…',
    processing: 'Processing…',
    error:      'Error',
  };

  const statusLabel = $derived(STATUS_LABELS[appState.status] ?? appState.status);

  // ─── Hotkey debug — show active combos + last event ────────────────────────

  /** Hotkey action → display label map. */
  const HK_LABELS: Record<string, string> = {
    ptt:         'PTT',
    free_speech: 'Free',
    cancel:      'Cancel',
  };

  const HOTKEY_ACTIONS = ['push_to_talk', 'free_speech', 'cancel'] as const;

  /** True briefly after each hotkey event (self-clears after 1.5 s). */
  let hotkeyFlash = $state(false);

  /** Short string like "PTT pressed" or "Free released". */
  const lastHotkeyLabel = $derived(
    appState.lastHotkeyEvent.action
      ? `${HK_LABELS[appState.lastHotkeyEvent.action] ?? appState.lastHotkeyEvent.action} ${appState.lastHotkeyEvent.state.toLowerCase()}`
      : '',
  );

  // Trigger flash whenever lastHotkeyEvent changes; auto-clear after 1.5 s
  $effect(() => {
    const ev = appState.lastHotkeyEvent;
    if (ev.receivedAt > 0) {
      hotkeyFlash = true;
      const timer = setTimeout(() => { hotkeyFlash = false; }, 1500);
      return () => clearTimeout(timer);
    }
  });
</script>

<!-- Always-mounted floating indicator (renders conditionally inside component) -->
<FloatingIndicator />

<!-- ── Main application shell ────────────────────────────────────────────── -->
<div
  class="app-shell"
  class:status-recording={appState.status === 'recording'}
  class:status-processing={appState.status === 'processing'}
  class:status-error={appState.status === 'error'}
>
  <!-- ── Top navigation bar ────────────────────────────────────────────── -->
  <header class="app-header">
    <!-- Logo -->
    <div class="logo" aria-label="Voice-typeless">
      <span class="logo-mark" aria-hidden="true">
        <!-- Minimal SVG wave-to-cursor icon -->
        <svg width="18" height="18" viewBox="0 0 18 18" fill="none" aria-hidden="true">
          <path
            d="M2 9 Q4 4 6 9 Q8 14 10 9 Q12 4 14 9"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            fill="none"
          />
          <line
            x1="16" y1="6"
            x2="16" y2="12"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
          />
        </svg>
      </span>
      <span class="logo-voice">Voice</span><span class="logo-typeless">-typeless</span>
    </div>

    <!-- Nav controls -->
    <nav class="header-nav" aria-label="Main navigation">
      {#if showSettings}
        <button
          class="icon-btn back-btn"
          onclick={() => (showSettings = false)}
          aria-label="Back to main page"
          title="Back"
        >
          ←
        </button>
      {/if}
      <button
        class="icon-btn"
        class:active={showSettings}
        onclick={() => (showSettings = !showSettings)}
        aria-label={showSettings ? 'Close settings' : 'Open settings'}
        aria-pressed={showSettings}
        title="Settings"
      >
        ⚙
      </button>
    </nav>
  </header>

  <!-- ── Main content area ─────────────────────────────────────────────── -->
  <main class="app-main" id="main-content" tabindex="-1">
    {#if showSettings}
      <SettingsPage onClose={() => (showSettings = false)} />
    {:else}
      <HistoryPanel />
    {/if}
  </main>

  <!-- ── Status footer ─────────────────────────────────────────────────── -->
  <footer class="app-footer">
    <!-- Live region for status changes, polled by screen readers -->
    <div
      class="status-indicator"
      role="status"
      aria-live="polite"
      aria-atomic="true"
      aria-label="Application status: {statusLabel}"
    >
      <span
        class="status-dot"
      class:recording={isRecording()}
        class:processing={appState.status === 'processing'}
        class:error={appState.status === 'error'}
        aria-hidden="true"
      ></span>
      <span class="status-label">{statusLabel}</span>
    </div>

    <!-- Current model info -->
    <span class="footer-model" aria-label="Active model: {appState.activeModel}">
      {appState.activeModel}
    </span>

    <!-- Error message (compact, truncated) -->
    {#if appState.status === 'error' && appState.errorMessage}
      <span class="footer-error" title={appState.errorMessage} aria-live="assertive">
        {appState.errorMessage.slice(0, 60)}{appState.errorMessage.length > 60 ? '…' : ''}
      </span>
    {/if}

    <!-- ── Hotkey debug overlay ──────────────────────────────────────────── -->
    <div class="footer-hkdebug" aria-label="Hotkey debug">
      <!-- Engine load status -->
      <span
        class="hk-engine"
        class:loaded={appState.engineLoaded}
        class:loading={!appState.engineLoaded && appState.modelLoadStage !== ''}
        title={appState.engineLoaded ? 'Engine loaded' : 'Engine not loaded'}
      >
        {appState.engineLoaded ? '✓' : '✗'}
      </span>

      <!-- Registered hotkey combos (color-coded by registration status) -->
      <span class="hkdebug-combos" title="Registered hotkeys">
        {#each HOTKEY_ACTIONS as action}
          {@const reg = appState.hotkeyRegistration[action]}
          {@const hk = appState.hotkeyConfig[action]}
          <span
            class="hk-key"
            class:hk-ok={reg?.ok}
            class:hk-fail={reg && !reg.ok}
            title={reg && !reg.ok ? `Registration failed: ${reg.error}` : hk}
          >
            {hk}
          </span>
        {/each}
      </span>

      <!-- Last-recording debug (sample count, text length) -->
      {#if appState.lastRecordingDebug.samples > 0 || appState.lastRecordingDebug.text_len > 0}
        <span class="hkdebug-samples" title="Last recording: samples / text length">
          {appState.lastRecordingDebug.samples > 0
            ? `${(appState.lastRecordingDebug.samples / 1000).toFixed(1)}k`
            : '0'}s
          |
          {appState.lastRecordingDebug.text_len}c
        </span>
      {/if}

      <!-- Flashing last-hotkey indicator -->
      {#if lastHotkeyLabel}
        <span
          class="hkdebug-event"
          class:flash={hotkeyFlash}
          title="Last hotkey event: {appState.lastHotkeyEvent.accelerator}"
        >
          {lastHotkeyLabel}
        </span>
      {/if}
    </div>
  </footer>
</div>

<style>
  /* ── Shell layout ─────────────────────────────────────────────────────────── */
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background: var(--vtl-bg-dark);
    color: var(--vtl-text-dark);
    /* Smooth background shift on status changes */
    transition: background 0.4s ease;
  }

  /* Subtle teal tint while recording */
  .app-shell.status-recording {
    background: color-mix(in srgb, var(--vtl-bg-dark) 96%, var(--vtl-teal) 4%);
  }

  /* Subtle indigo tint while processing */
  .app-shell.status-processing {
    background: color-mix(in srgb, var(--vtl-bg-dark) 97%, var(--vtl-indigo) 3%);
  }

  /* Subtle red tint on error */
  .app-shell.status-error {
    background: color-mix(in srgb, var(--vtl-bg-dark) 97%, #ff4444 3%);
  }

  /* ── Header ──────────────────────────────────────────────────────────────── */
  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--vtl-border);
    flex-shrink: 0;
    /* Subtle glass effect matching the floating indicator style */
    background: rgba(15, 15, 18, 0.85);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  /* ── Logo ────────────────────────────────────────────────────────────────── */
  .logo {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;
    font-weight: 700;
    user-select: none;
    -webkit-user-select: none;
  }

  .logo-mark {
    display: flex;
    align-items: center;
    color: var(--vtl-teal);
  }

  .logo-voice {
    color: var(--vtl-teal);
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .logo-typeless {
    color: var(--vtl-text-dark);
    font-weight: 300;
    opacity: 0.85;
  }

  /* ── Header nav ──────────────────────────────────────────────────────────── */
  .header-nav {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .icon-btn {
    background: none;
    border: none;
    color: var(--vtl-gray);
    font-size: 17px;
    line-height: 1;
    cursor: pointer;
    padding: 5px 7px;
    border-radius: 7px;
    transition: color 0.15s, background 0.15s;
  }

  .icon-btn:hover {
    color: var(--vtl-text-dark);
    background: rgba(255, 255, 255, 0.06);
  }

  .icon-btn.active {
    color: var(--vtl-teal);
    background: rgba(0, 230, 200, 0.08);
  }

  .icon-btn:focus-visible {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 2px;
  }

  /* Distinct back button styling */
  .back-btn {
    font-size: 15px;
    color: var(--vtl-gray);
  }

  .back-btn:hover {
    color: var(--vtl-teal);
  }

  /* ── Main content ────────────────────────────────────────────────────────── */
  .app-main {
    flex: 1;
    overflow-y: auto;
    /* Remove default outline on programmatic focus (skip-link target) */
    outline: none;
    scrollbar-width: thin;
    scrollbar-color: var(--vtl-border) transparent;
  }

  .app-main::-webkit-scrollbar { width: 4px; }
  .app-main::-webkit-scrollbar-thumb {
    background: var(--vtl-border);
    border-radius: 2px;
  }

  /* ── Footer ──────────────────────────────────────────────────────────────── */
  .app-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    border-top: 1px solid var(--vtl-border);
    flex-shrink: 0;
    background: rgba(15, 15, 18, 0.70);
    min-height: 32px;
  }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  /* Animated status dot */
  .status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--vtl-border);
    flex-shrink: 0;
    transition: background 0.2s, box-shadow 0.2s;
  }

  .status-dot.recording {
    background: var(--vtl-teal);
    box-shadow: 0 0 6px rgba(0, 230, 200, 0.60);
    animation: vtl-dot-pulse 1.4s ease-in-out infinite;
  }

  .status-dot.processing {
    background: var(--vtl-indigo);
    box-shadow: 0 0 6px rgba(91, 78, 255, 0.50);
  }

  .status-dot.error {
    background: #ff6b6b;
    box-shadow: 0 0 6px rgba(255, 107, 107, 0.50);
  }

  @keyframes vtl-dot-pulse {
    0%, 100% { transform: scale(1);    opacity: 1;    }
    50%       { transform: scale(1.35); opacity: 0.75; }
  }

  .status-label {
    font-size: 11px;
    color: var(--vtl-gray);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .footer-model {
    margin-left: auto;
    font-size: 10px;
    color: var(--vtl-border);
    font-family: 'JetBrains Mono', monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 120px;
  }

  .footer-error {
    font-size: 10px;
    color: #ff6b6b;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 160px;
  }

  /* ── Hotkey debug bar ──────────────────────────────────────────────────── */
  .footer-hkdebug {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
  }

  .hk-engine {
    font-size: 10px;
    font-family: 'JetBrains Mono', monospace;
    width: 14px;
    text-align: center;
    transition: color 0.2s;
  }
  .hk-engine.loaded { color: #22FFAA; }
  .hk-engine.loading { color: #FFD700; animation: hk-pulse 0.8s ease-in-out infinite; }
  .hk-engine:not(.loaded):not(.loading) { color: #ff6b6b; }

  @keyframes hk-pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.4; }
  }

  .hkdebug-combos {
    display: flex;
    gap: 4px;
  }

  .hk-key {
    font-size: 9px;
    font-family: 'JetBrains Mono', monospace;
    color: var(--vtl-border);
    background: rgba(255, 255, 255, 0.04);
    padding: 1px 4px;
    border-radius: 3px;
    white-space: nowrap;
  }

  .hkdebug-event {
    font-size: 9px;
    font-family: 'JetBrains Mono', monospace;
    color: var(--vtl-teal);
    padding: 1px 4px;
    border-radius: 3px;
    transition: background 0.15s, color 0.15s;
    white-space: nowrap;
  }

  .hkdebug-event.flash {
    background: rgba(0, 230, 200, 0.15);
    color: var(--vtl-teal);
    animation: hkflash-pulse 0.3s ease-out;
  }

  @keyframes hkflash-pulse {
    0%   { transform: scale(1);   opacity: 1;   }
    50%  { transform: scale(1.2); opacity: 0.8; }
    100% { transform: scale(1);   opacity: 1;   }
  }

  /* Hotkey registration status */
  .hk-key.hk-ok {
    color: #22FFAA;
    border-left: 2px solid #22FFAA;
    padding-left: 5px;
  }
  .hk-key.hk-fail {
    color: #ff6b6b;
    border-left: 2px solid #ff6b6b;
    padding-left: 5px;
    text-decoration: line-through;
    opacity: 0.7;
  }

  /* Recording debug (sample count + text length) */
  .hkdebug-samples {
    font-size: 9px;
    font-family: 'JetBrains Mono', monospace;
    color: var(--vtl-gray);
    background: rgba(255, 255, 255, 0.04);
    padding: 1px 4px;
    border-radius: 3px;
    white-space: nowrap;
  }
</style>
