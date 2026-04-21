<script lang="ts">
  /**
   * HistoryPanel — scrollable list of the last 50 recognition results.
   *
   * Features:
   *  - Search/filter history with live results count
   *  - Stats row (total recordings, chars, top language)
   *  - Clear All with inline confirmation
   *  - Export all to clipboard
   *  - Demo mode button
   *  - Per-item copy + delete with inline confirmation
   */

  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    getHistory, deleteHistoryItem, clearHistory,
    exportHistoryText, getStats, runDemo,
    type HistoryItem, type Stats,
  } from '../tauri/commands';

  // ─── State ─────────────────────────────────────────────────────────────────

  let items          = $state<HistoryItem[]>([]);
  let isLoading      = $state(false);
  let loadError      = $state('');
  let copiedId       = $state<string | null>(null);
  let pendingDeleteId = $state<string | null>(null);
  let searchText     = $state('');
  let isExporting    = $state(false);
  let exportCopied   = $state(false);
  let pendingClearAll = $state(false);
  let isDemoRunning  = $state(false);
  let stats          = $state<Stats | null>(null);

  // ─── Derived ───────────────────────────────────────────────────────────────

  const filteredItems = $derived(
    searchText.trim()
      ? items.filter(item =>
          item.text.toLowerCase().includes(searchText.toLowerCase())
        )
      : items
  );

  const topLanguage = $derived.by(() => {
    if (!stats || Object.keys(stats.languages).length === 0) return '';
    const sorted = Object.entries(stats.languages).sort(([, a], [, b]) => b - a);
    return sorted[0]?.[0]?.toUpperCase() ?? '';
  });

  // ─── Data Operations ───────────────────────────────────────────────────────

  async function fetchStats(): Promise<void> {
    try {
      stats = await getStats();
    } catch {
      /* stats are informational only */
    }
  }

  async function refresh(): Promise<void> {
    isLoading = true;
    loadError = '';
    try {
      items = await getHistory(50);
      await fetchStats();
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      isLoading = false;
    }
  }

  async function copyItem(item: HistoryItem): Promise<void> {
    try {
      await navigator.clipboard.writeText(item.text);
      copiedId = item.id;
      setTimeout(() => { copiedId = null; }, 1500);
    } catch {
      /* Clipboard access may be blocked in some WebView contexts */
    }
  }

  function requestDelete(id: string): void { pendingDeleteId = id; }
  function cancelDelete(): void { pendingDeleteId = null; }

  async function confirmDelete(id: string): Promise<void> {
    try {
      await deleteHistoryItem(id);
      pendingDeleteId = null;
      await refresh();
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
      pendingDeleteId = null;
    }
  }

  function requestClearAll(): void { pendingClearAll = true; }
  function cancelClearAll(): void { pendingClearAll = false; }

  async function confirmClearAll(): Promise<void> {
    try {
      await clearHistory();
      pendingClearAll = false;
      items = [];
      stats = null;
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
      pendingClearAll = false;
    }
  }

  async function exportAll(): Promise<void> {
    if (isExporting || items.length === 0) return;
    isExporting = true;
    try {
      const text = await exportHistoryText();
      if (text) {
        await navigator.clipboard.writeText(text);
        exportCopied = true;
        setTimeout(() => { exportCopied = false; }, 1500);
      }
    } catch {
      /* ignore */
    } finally {
      isExporting = false;
    }
  }

  async function startDemo(): Promise<void> {
    if (isDemoRunning) return;
    isDemoRunning = true;
    try {
      await runDemo();
    } catch {
      /* ignore */
    } finally {
      isDemoRunning = false;
    }
  }

  // ─── Lifecycle ─────────────────────────────────────────────────────────────

  onMount(() => {
    let unlisten: (() => void) | undefined;

    // Initial load + set up the Tauri event subscription
    refresh().then(() =>
      listen('recognition-result', () => { refresh(); }).then(fn => { unlisten = fn; })
    );

    return () => { if (unlisten) unlisten(); };
  });

  // ─── Helpers ───────────────────────────────────────────────────────────────

  function formatTime(unixSeconds: number): string {
    return new Date(unixSeconds * 1000).toLocaleTimeString(undefined, {
      hour: '2-digit', minute: '2-digit', second: '2-digit',
    });
  }

  function langLabel(lang: string): string {
    const MAP: Record<string, string> = {
      zh: 'ZH', en: 'EN', ja: 'JA', ko: 'KO',
      fr: 'FR', de: 'DE', es: 'ES', ru: 'RU',
      it: 'IT', pt: 'PT', auto: 'AUTO',
    };
    return MAP[lang.toLowerCase()] ?? lang.toUpperCase().slice(0, 4);
  }
</script>

<section class="history-panel" aria-label="Recognition history">
  <!-- ── Header ─────────────────────────────────────────────────────────── -->
  <div class="panel-header">
    <div class="header-left">
      <h2 class="section-title">History</h2>
      {#if stats && stats.total_items > 0}
        <span class="stats-badge" aria-label="Statistics">
          {stats.total_items} · {stats.total_chars}c{topLanguage ? ` · ${topLanguage}` : ''}
        </span>
      {/if}
    </div>
    <div class="header-actions">
      <!-- Demo button -->
      <button
        class="header-btn demo-btn"
        onclick={startDemo}
        disabled={isDemoRunning}
        aria-label="Run demo"
        title="Try Demo"
      >
        {isDemoRunning ? '…' : '▶'}
      </button>

      <!-- Export to clipboard -->
      <button
        class="header-btn"
        class:copied={exportCopied}
        onclick={exportAll}
        disabled={isExporting || items.length === 0}
        aria-label={exportCopied ? 'Copied!' : 'Export history to clipboard'}
        title="Export all"
      >
        {exportCopied ? '✓' : '⎘'}
      </button>

      <!-- Clear All with inline confirmation -->
      {#if pendingClearAll}
        <span class="confirm-label" aria-live="assertive">Clear all?</span>
        <button class="header-btn confirm-yes" onclick={confirmClearAll} aria-label="Confirm clear all">✓</button>
        <button class="header-btn confirm-no" onclick={cancelClearAll} aria-label="Cancel clear all">✕</button>
      {:else}
        <button
          class="header-btn clear-btn"
          onclick={requestClearAll}
          disabled={items.length === 0}
          aria-label="Clear all history"
          title="Clear all"
        >
          🗑
        </button>
      {/if}

      <!-- Refresh -->
      <button
        class="refresh-btn"
        onclick={refresh}
        disabled={isLoading}
        aria-label="Refresh history"
        title="Refresh"
      >
        <span class="refresh-icon" class:spinning={isLoading} aria-hidden="true">↻</span>
      </button>
    </div>
  </div>

  <!-- ── Search ─────────────────────────────────────────────────────────── -->
  <div class="search-row">
    <div class="search-input-wrap">
      <span class="search-icon" aria-hidden="true">🔍</span>
      <input
        type="search"
        class="search-input"
        placeholder="Search history…"
        bind:value={searchText}
        aria-label="Search history"
      />
      {#if searchText}
        <button class="search-clear" onclick={() => (searchText = '')} aria-label="Clear search">✕</button>
      {/if}
    </div>
    {#if searchText.trim()}
      <span class="search-count" aria-live="polite">
        {filteredItems.length} result{filteredItems.length !== 1 ? 's' : ''}
      </span>
    {/if}
  </div>

  <!-- ── Error banner ───────────────────────────────────────────────────── -->
  {#if loadError}
    <div class="error-banner" role="alert">
      <span class="error-icon" aria-hidden="true">⚠</span>
      {loadError}
      <button class="dismiss-btn" onclick={() => (loadError = '')} aria-label="Dismiss error">✕</button>
    </div>
  {/if}

  <!-- ── Empty state ────────────────────────────────────────────────────── -->
  {#if !isLoading && filteredItems.length === 0 && !loadError}
    <div class="empty-state" role="status" aria-live="polite">
      <div class="empty-icon" aria-hidden="true">🎤</div>
      {#if searchText.trim()}
        <p class="empty-title">No results for "{searchText}"</p>
        <p class="empty-sub">Try a different search term.</p>
      {:else}
        <p class="empty-title">No history yet</p>
        <p class="empty-sub">Press your hotkey and start speaking, or click ▶ for a demo.</p>
      {/if}
    </div>

  <!-- ── History list ───────────────────────────────────────────────────── -->
  {:else}
    <ul class="history-list" aria-label="Recognition results">
      {#each filteredItems as item (item.id)}
        <li class="history-item" aria-label="Recognition result: {item.text.slice(0, 40)}">
          <p class="item-text">{item.text}</p>
          <div class="item-meta">
            <span
              class="lang-badge"
              aria-label="Language: {item.language}"
              title="Detected language: {item.language}"
            >
              {langLabel(item.language)}
            </span>
            <time
              class="timestamp"
              datetime={new Date(item.timestamp * 1000).toISOString()}
              title={new Date(item.timestamp * 1000).toLocaleString()}
            >
              {formatTime(item.timestamp)}
            </time>
            <span class="meta-spacer" aria-hidden="true"></span>
            <div class="actions" role="group" aria-label="Actions for this result">
              {#if pendingDeleteId === item.id}
                <span class="confirm-label" aria-live="assertive">Delete?</span>
                <button class="action-btn confirm-yes" onclick={() => confirmDelete(item.id)} aria-label="Confirm delete">✓</button>
                <button class="action-btn confirm-no" onclick={cancelDelete} aria-label="Cancel delete">✕</button>
              {:else}
                <button
                  class="action-btn copy-btn"
                  class:copied={copiedId === item.id}
                  onclick={() => copyItem(item)}
                  aria-label={copiedId === item.id ? 'Copied!' : 'Copy to clipboard'}
                  title="Copy"
                >
                  {copiedId === item.id ? '✓' : '⎘'}
                </button>
                <button
                  class="action-btn delete-btn"
                  onclick={() => requestDelete(item.id)}
                  aria-label="Delete this result"
                  title="Delete"
                >
                  ✕
                </button>
              {/if}
            </div>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  /* ── Panel shell ─────────────────────────────────────────────────────────── */
  .history-panel {
    display: flex;
    flex-direction: column;
    padding: 16px;
    gap: 10px;
    height: 100%;
    overflow: hidden;
  }

  /* ── Header ──────────────────────────────────────────────────────────────── */
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
    gap: 6px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .stats-badge {
    font-size: 10px;
    color: var(--vtl-border);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 130px;
  }

  .section-title {
    margin: 0;
    font-size: 11px;
    font-weight: 700;
    color: var(--vtl-gray);
    text-transform: uppercase;
    letter-spacing: 0.10em;
  }

  .refresh-btn {
    background: none;
    border: none;
    color: var(--vtl-gray);
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 6px;
    font-size: 16px;
    line-height: 1;
    transition: color 0.15s, background 0.15s;
  }

  .refresh-btn:hover:not(:disabled) {
    color: var(--vtl-text-dark);
    background: rgba(255, 255, 255, 0.06);
  }

  .refresh-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .refresh-icon {
    display: inline-block;
    transition: transform 0.4s ease;
  }

  .refresh-icon.spinning {
    animation: vtl-spin 0.8s linear infinite;
  }

  @keyframes vtl-spin {
    to { transform: rotate(360deg); }
  }

  /* ── Error banner ─────────────────────────────────────────────────────────── */
  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: rgba(255, 107, 107, 0.10);
    border: 1px solid rgba(255, 107, 107, 0.30);
    border-radius: 8px;
    color: #ff6b6b;
    font-size: 12px;
    flex-shrink: 0;
  }

  .error-icon { font-size: 14px; }

  .dismiss-btn {
    margin-left: auto;
    background: none;
    border: none;
    color: #ff6b6b;
    cursor: pointer;
    padding: 0 2px;
    font-size: 13px;
    opacity: 0.7;
    transition: opacity 0.15s;
  }

  .dismiss-btn:hover { opacity: 1; }

  /* ── Empty state ─────────────────────────────────────────────────────────── */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 8px;
    padding: 48px 16px;
    text-align: center;
  }

  .empty-icon {
    font-size: 36px;
    opacity: 0.4;
  }

  .empty-title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--vtl-gray);
  }

  .empty-sub {
    margin: 0;
    font-size: 12px;
    color: var(--vtl-border);
  }

  /* ── History list ─────────────────────────────────────────────────────────── */
  .history-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow-y: auto;
    /* Scrollbar styling */
    scrollbar-width: thin;
    scrollbar-color: var(--vtl-border) transparent;
  }

  .history-list::-webkit-scrollbar { width: 4px; }
  .history-list::-webkit-scrollbar-track { background: transparent; }
  .history-list::-webkit-scrollbar-thumb {
    background: var(--vtl-border);
    border-radius: 2px;
  }

  /* ── History item ─────────────────────────────────────────────────────────── */
  .history-item {
    background: var(--vtl-bg-dark-2);
    border: 1px solid var(--vtl-border);
    border-radius: 8px;
    padding: 10px 12px;
    transition: border-color 0.15s;
    /* Slide-in animation for new items */
    animation: vtl-item-in 0.18s ease-out;
  }

  .history-item:hover {
    border-color: rgba(74, 74, 82, 0.9);
  }

  @keyframes vtl-item-in {
    from { opacity: 0; transform: translateY(-4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .item-text {
    margin: 0 0 8px;
    font-size: 13px;
    line-height: 1.55;
    color: var(--vtl-text-dark);
    word-break: break-word;
    white-space: pre-wrap;
  }

  /* ── Meta row ─────────────────────────────────────────────────────────────── */
  .item-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: nowrap;
  }

  .lang-badge {
    flex-shrink: 0;
    font-size: 9px;
    font-weight: 700;
    background: rgba(0, 230, 200, 0.10);
    color: var(--vtl-teal);
    border: 1px solid rgba(0, 230, 200, 0.20);
    border-radius: 4px;
    padding: 2px 5px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-family: 'JetBrains Mono', monospace;
  }

  .timestamp {
    font-size: 11px;
    color: var(--vtl-gray);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .meta-spacer { flex: 1; }

  /* ── Actions ──────────────────────────────────────────────────────────────── */
  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .action-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px 7px;
    border-radius: 5px;
    font-size: 13px;
    color: var(--vtl-gray);
    transition: color 0.15s, background 0.15s;
    line-height: 1;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.07);
  }

  /* Copy button: flash green on success */
  .copy-btn.copied {
    color: var(--vtl-green);
    background: rgba(34, 255, 170, 0.08);
  }

  /* Delete button: red on hover */
  .delete-btn:hover {
    color: #ff6b6b;
    background: rgba(255, 107, 107, 0.08);
  }

  /* Inline confirmation */
  .confirm-label {
    font-size: 11px;
    color: #ff6b6b;
    font-weight: 600;
    white-space: nowrap;
    margin-right: 2px;
  }

  .confirm-yes {
    color: var(--vtl-green);
  }

  .confirm-yes:hover {
    background: rgba(34, 255, 170, 0.10);
  }

  .confirm-no:hover {
    color: #ff6b6b;
    background: rgba(255, 107, 107, 0.10);
  }

  /* ── Header action buttons ───────────────────────────────────────────────── */
  .header-btn {
    background: none;
    border: none;
    color: var(--vtl-gray);
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 6px;
    font-size: 13px;
    line-height: 1;
    transition: color 0.15s, background 0.15s;
  }

  .header-btn:hover:not(:disabled) {
    color: var(--vtl-text-dark);
    background: rgba(255, 255, 255, 0.06);
  }

  .header-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .header-btn.copied {
    color: var(--vtl-green);
  }

  .demo-btn:hover:not(:disabled) {
    color: var(--vtl-teal);
    background: rgba(0, 230, 200, 0.08);
  }

  .clear-btn:hover:not(:disabled) {
    color: #ff6b6b;
    background: rgba(255, 107, 107, 0.08);
  }

  /* ── Search ──────────────────────────────────────────────────────────────── */
  .search-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .search-input-wrap {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--vtl-bg-dark-2);
    border: 1px solid var(--vtl-border);
    border-radius: 8px;
    padding: 5px 10px;
    transition: border-color 0.15s;
  }

  .search-input-wrap:focus-within {
    border-color: var(--vtl-teal);
  }

  .search-icon {
    font-size: 12px;
    opacity: 0.5;
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    font-size: 12px;
    color: var(--vtl-text-dark);
    font-family: inherit;
    min-width: 0;
  }

  .search-input::placeholder { color: var(--vtl-border); }
  .search-input::-webkit-search-cancel-button { display: none; }

  .search-clear {
    background: none;
    border: none;
    color: var(--vtl-border);
    cursor: pointer;
    font-size: 11px;
    padding: 0 2px;
    line-height: 1;
    flex-shrink: 0;
    transition: color 0.15s;
  }

  .search-clear:hover { color: var(--vtl-gray); }

  .search-count {
    font-size: 11px;
    color: var(--vtl-gray);
    white-space: nowrap;
    flex-shrink: 0;
  }
</style>
