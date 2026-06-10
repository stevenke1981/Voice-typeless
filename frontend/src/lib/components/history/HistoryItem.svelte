<script lang="ts">
  /**
   * HistoryItem — renders a single history recognition result.
   */

  import type { HistoryItem } from '../../tauri/commands';

  interface Props {
    item: HistoryItem;
    copiedId: string | null;
    pendingDeleteId: string | null;
    onCopy: (item: HistoryItem) => void;
    onDelete: (id: string) => void;
    onConfirmDelete: (id: string) => void;
    onCancelDelete: () => void;
    langLabel: (lang: string) => string;
    formatTime: (unixSeconds: number) => string;
  }

  const {
    item,
    copiedId,
    pendingDeleteId,
    onCopy,
    onDelete,
    onConfirmDelete,
    onCancelDelete,
    langLabel,
    formatTime,
  }: Props = $props();
</script>

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
        <button class="action-btn confirm-yes" onclick={() => onConfirmDelete(item.id)} aria-label="Confirm delete">✓</button>
        <button class="action-btn confirm-no" onclick={onCancelDelete} aria-label="Cancel delete">✕</button>
      {:else}
        <button
          class="action-btn copy-btn"
          class:copied={copiedId === item.id}
          onclick={() => onCopy(item)}
          aria-label={copiedId === item.id ? 'Copied!' : 'Copy to clipboard'}
          title="Copy"
        >
          {copiedId === item.id ? '✓' : '⎘'}
        </button>
        <button
          class="action-btn delete-btn"
          onclick={() => onDelete(item.id)}
          aria-label="Delete this result"
          title="Delete"
        >
          ✕
        </button>
      {/if}
    </div>
  </div>
</li>

<style>
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

  /* Inline confirmation (shared classes also used in parent) */
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
</style>
