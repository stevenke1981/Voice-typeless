<script lang="ts">
  /**
   * FloatingIndicator — draggable, always-mounted overlay.
   *
   * Rendered by App.svelte unconditionally; shows/hides itself based on
   * appState.status.  Position is persisted to localStorage so it survives
   * page reloads and app restarts.
   *
   * Design spec (agents.md):
   *  - Semi-transparent dark pill (#1F1F25, 90 % opacity + backdrop-filter)
   *  - VTL teal border + glow when recording
   *  - 3-bar animated sound wave, pulsing via CSS keyframes
   *  - Live recording timer (MM:SS) counted locally; shows actual duration
   *    from appState.recordingDuration while processing
   *  - Draggable anywhere; position remembered in localStorage
   *  - Hidden (display:none) when status === 'idle'
   */

  import { appState, isRecording } from '../stores/appState.svelte';

  // ─── Position state ────────────────────────────────────────────────────────

  /** Load saved position eagerly so `pos` is correct on first render. */
  function loadSavedPos(): { x: number; y: number } {
    try {
      const raw = localStorage.getItem('vtl-indicator-pos');
      if (raw) return JSON.parse(raw) as { x: number; y: number };
    } catch {
      /* ignore parse errors */
    }
    // Default: top-right area, away from typical taskbars
    return { x: window.innerWidth - 220, y: 24 };
  }

  let pos = $state(loadSavedPos());
  let dragging = $state(false);
  let dragOffset = $state({ x: 0, y: 0 });

  /** Persist position whenever it changes. */
  $effect(() => {
    localStorage.setItem('vtl-indicator-pos', JSON.stringify(pos));
  });

  // ─── Live timer ────────────────────────────────────────────────────────────

  /**
   * `elapsedMs` counts up every second while recording.
   * When processing, we switch to `appState.recordingDuration` (actual value
   * reported by the Core sidecar).
   */
  let elapsedMs = $state(0);

  $effect(() => {
    if (isRecording()) {
      elapsedMs = 0;
      const id = setInterval(() => {
        elapsedMs += 1000;
      }, 1000);
      // Svelte 5 $effect cleanup: return a teardown function
      return () => clearInterval(id);
    }
  });

  /** Milliseconds to display in the timer label. */
  const displayMs = $derived(
    appState.status === 'processing' ? appState.recordingDuration : elapsedMs,
  );

  /** Formatted MM:SS string for the timer. */
  const timerLabel = $derived.by(() => {
    const totalSecs = Math.floor(displayMs / 1000);
    const mins = Math.floor(totalSecs / 60);
    const secs = totalSecs % 60;
    return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
  });

  // ─── Drag handlers ─────────────────────────────────────────────────────────

  function onMouseDown(e: MouseEvent) {
    // Only drag on primary button
    if (e.button !== 0) return;
    dragging = true;
    dragOffset = { x: e.clientX - pos.x, y: e.clientY - pos.y };
  }

  function onMouseMove(e: MouseEvent) {
    if (!dragging) return;
    // Clamp to viewport so the indicator is never dragged off-screen
    const x = Math.max(0, Math.min(window.innerWidth - 200, e.clientX - dragOffset.x));
    const y = Math.max(0, Math.min(window.innerHeight - 48, e.clientY - dragOffset.y));
    pos = { x, y };
  }

  function onMouseUp() {
    dragging = false;
  }
</script>

<!-- Capture mouse events globally so dragging works even outside the element -->
<svelte:window onmousemove={onMouseMove} onmouseup={onMouseUp} />

{#if appState.status !== 'idle'}
  <!--
    role="status" + aria-live="polite" so screen readers announce
    status changes without interrupting ongoing speech.
  -->
  <div
    class="vtl-indicator"
    class:recording={isRecording()}
    class:processing={appState.status === 'processing'}
    class:error={appState.status === 'error'}
    class:dragging
    style="left: {pos.x}px; top: {pos.y}px;"
    onmousedown={onMouseDown}
    role="status"
    aria-live="polite"
    aria-label="Voice-typeless recording indicator: {appState.status}"
    aria-atomic="true"
  >
    <!-- Animated wave bars (hidden from AT, purely decorative) -->
    <div class="wave-bars" aria-hidden="true">
      <span class="bar bar-1"></span>
      <span class="bar bar-2"></span>
      <span class="bar bar-3"></span>
    </div>

    <!-- Timer display -->
    <span class="timer" aria-label="Recording duration {timerLabel}">
      {timerLabel}
    </span>

    <!-- Status label for processing / error states -->
    {#if appState.status === 'processing'}
      <span class="status-text" aria-hidden="true">Processing…</span>
    {:else if appState.status === 'error'}
      <span class="status-text error-text" title={appState.errorMessage}>
        Error
      </span>
    {/if}
  </div>
{/if}

<style>
  /* ── Pill container ──────────────────────────────────────────────────────── */
  .vtl-indicator {
    position: fixed;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    /* Semi-transparent dark pill per design spec */
    background: rgba(31, 31, 37, 0.92);
    backdrop-filter: blur(14px) saturate(160%);
    -webkit-backdrop-filter: blur(14px) saturate(160%);
    border: 1px solid var(--vtl-border);
    border-radius: 999px;
    cursor: grab;
    user-select: none;
    -webkit-user-select: none;
    /* Above everything, including modals */
    z-index: 9999;
    /* Smooth transitions on state changes */
    transition:
      border-color 0.2s ease,
      box-shadow 0.2s ease,
      opacity 0.15s ease;
    font-family: 'JetBrains Mono', 'Fira Code', monospace;
    font-size: 13px;
    color: var(--vtl-text-dark);
    /* Hardware-accelerated positioning */
    will-change: left, top;
  }

  /* Recording state: teal glow border */
  .vtl-indicator.recording {
    border-color: var(--vtl-teal);
    box-shadow: 0 0 0 1px rgba(0, 230, 200, 0.15), 0 0 14px rgba(0, 230, 200, 0.28);
  }

  /* Processing state: indigo hint */
  .vtl-indicator.processing {
    border-color: var(--vtl-indigo);
    box-shadow: 0 0 14px rgba(91, 78, 255, 0.22);
  }

  /* Error state: red hint */
  .vtl-indicator.error {
    border-color: #ff6b6b;
    box-shadow: 0 0 14px rgba(255, 107, 107, 0.22);
  }

  .vtl-indicator:active,
  .vtl-indicator.dragging {
    cursor: grabbing;
  }

  /* ── Wave bars ───────────────────────────────────────────────────────────── */
  .wave-bars {
    display: flex;
    align-items: center;
    gap: 3px;
    height: 18px;
  }

  .bar {
    display: inline-block;
    width: 3px;
    border-radius: 2px;
    background: var(--vtl-teal);
    /* Default: animation paused (non-recording states) */
    animation: vtl-wave 1.2s ease-in-out infinite;
    animation-play-state: paused;
  }

  .bar-1 { height: 7px;  animation-delay: 0s;    }
  .bar-2 { height: 15px; animation-delay: 0.18s; }
  .bar-3 { height: 9px;  animation-delay: 0.36s; }

  /* Running animation only during active recording */
  .recording .bar {
    animation-play-state: running;
  }

  /* Processing bars: static indigo colour */
  .processing .bar {
    animation-play-state: paused;
    background: var(--vtl-indigo);
    height: 12px;
  }

  /* Error bars: static red colour */
  .error .bar {
    animation-play-state: paused;
    background: #ff6b6b;
    height: 12px;
  }

  @keyframes vtl-wave {
    0%, 100% { transform: scaleY(0.5); }
    50%       { transform: scaleY(1.0); }
  }

  /* ── Timer ───────────────────────────────────────────────────────────────── */
  .timer {
    color: var(--vtl-teal);
    /* Prevent layout shift as digits change */
    font-variant-numeric: tabular-nums;
    min-width: 40px;
    text-align: center;
    letter-spacing: 0.04em;
  }

  .processing .timer {
    color: var(--vtl-indigo);
  }

  .error .timer {
    color: #ff6b6b;
  }

  /* ── Status text ─────────────────────────────────────────────────────────── */
  .status-text {
    color: var(--vtl-gray);
    font-size: 11px;
    white-space: nowrap;
  }

  .error-text {
    color: #ff6b6b;
  }
</style>
