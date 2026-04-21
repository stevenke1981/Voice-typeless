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
  import { getConfig, startRecording, stopRecording, cancelRecording } from './lib/tauri/commands';

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

  // ─── Hotkey helpers ──────────────────────────────────────────────────────

  /**
   * Returns true if the KeyboardEvent matches a stored combo string.
   * Combo format: "Modifier+Key" e.g. "Alt+Space", "Ctrl+Shift+V", "Escape".
   * Comparison is case-insensitive.
   */
  function matchesHotkey(e: KeyboardEvent, combo: string): boolean {
    if (!combo) return false;
    const parts = combo.split('+');
    const keyPart = parts[parts.length - 1];
    const wantsCtrl  = parts.includes('Ctrl');
    const wantsAlt   = parts.includes('Alt');
    const wantsShift = parts.includes('Shift');
    const wantsMeta  = parts.includes('Meta') || parts.includes('Win');
    if (e.ctrlKey  !== wantsCtrl)  return false;
    if (e.altKey   !== wantsAlt)   return false;
    if (e.shiftKey !== wantsShift) return false;
    if (e.metaKey  !== wantsMeta)  return false;
    const evKey = e.key === ' ' ? 'Space' : e.key;
    return evKey.toLowerCase() === keyPart.toLowerCase();
  }

  // ─── Lifecycle ───────────────────────────────────────────────────────────

  onMount(() => {
    // Track push-to-talk state
    let pttHeld = false;
    let pttPhysicalKey = '';

    function handleKeyDown(e: KeyboardEvent): void {
      // Skip if focus is inside an interactive input element
      const tgt = e.target as HTMLElement;
      if (
        tgt instanceof HTMLInputElement ||
        tgt instanceof HTMLTextAreaElement ||
        tgt instanceof HTMLSelectElement ||
        tgt.isContentEditable
      ) return;

      const hk = appState.hotkeyConfig;

      // Cancel (highest priority)
      if (
        matchesHotkey(e, hk.cancel) &&
        (appState.status === 'recording' || appState.status === 'processing')
      ) {
        e.preventDefault();
        pttHeld = false;
        pttPhysicalKey = '';
        cancelRecording().catch(() => {});
        return;
      }

      // Push-to-talk keydown
      if (matchesHotkey(e, hk.push_to_talk) && !pttHeld) {
        if (appState.status === 'idle') {
          e.preventDefault();
          pttHeld = true;
          pttPhysicalKey = e.key === ' ' ? 'Space' : e.key;
          startRecording('push_to_talk').catch(() => {});
        }
        return;
      }

      // Free-speech toggle
      if (matchesHotkey(e, hk.free_speech)) {
        e.preventDefault();
        if (appState.status === 'idle') {
          startRecording('free_speech').catch(() => {});
        } else if (appState.status === 'recording') {
          stopRecording().catch(() => {});
        }
      }
    }

    function handleKeyUp(e: KeyboardEvent): void {
      if (!pttHeld) return;
      const relKey = e.key === ' ' ? 'Space' : e.key;
      if (relKey.toLowerCase() === pttPhysicalKey.toLowerCase()) {
        pttHeld = false;
        pttPhysicalKey = '';
        if (appState.status === 'recording') {
          stopRecording().catch(() => {});
        }
      }
    }

    (async () => {
      await setupEventListeners();

      // Load persisted config (theme + hotkeys)
      try {
        const cfg = await getConfig();
        appState.theme = (cfg.ui?.theme ?? 'dark') as 'dark' | 'light' | 'system';
        appState.hotkeyConfig = {
          push_to_talk: (cfg.hotkey as any)?.push_to_talk ?? 'Alt+Space',
          free_speech:  (cfg.hotkey as any)?.free_speech  ?? 'Ctrl+Shift+V',
          cancel:       (cfg.hotkey as any)?.cancel       ?? 'Escape',
        };
      } catch {
        appState.theme = 'dark';
      }
      applyTheme(appState.theme);

      document.addEventListener('keydown', handleKeyDown);
      document.addEventListener('keyup', handleKeyUp);
    })();

    return () => {
      teardownEventListeners();
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('keyup', handleKeyUp);
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
  <header class="app-header" role="banner">
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
      <button
        class="icon-btn"
        class:active={showSettings}
        onclick={() => (showSettings = !showSettings)}
        aria-label={showSettings ? 'Close settings' : 'Open settings'}
        aria-pressed={showSettings}
        title="Settings"
      >
        <!-- Settings gear (unicode) -->
        ⚙
      </button>
    </nav>
  </header>

  <!-- ── Main content area ─────────────────────────────────────────────── -->
  <main class="app-main" id="main-content" tabindex="-1">
    {#if showSettings}
      <SettingsPage />
    {:else}
      <HistoryPanel />
    {/if}
  </main>

  <!-- ── Status footer ─────────────────────────────────────────────────── -->
  <footer class="app-footer" role="contentinfo">
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
</style>
