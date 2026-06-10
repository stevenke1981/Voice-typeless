<script lang="ts">
  import type { LocalConfig } from './types';
  import { t } from '../../i18n.svelte';

  interface Props {
    config: LocalConfig;
    recordingKey: keyof LocalConfig['hotkey'] | null;
    startCapture: (field: keyof LocalConfig['hotkey']) => void;
    onHotkeyKeydown: (field: keyof LocalConfig['hotkey'], e: KeyboardEvent) => void;
    onHotkeyBlur: () => void;
  }

  const { config, recordingKey, startCapture, onHotkeyKeydown, onHotkeyBlur }: Props = $props();

  const HOTKEY_LABELS: Record<keyof LocalConfig['hotkey'], string> = {
    push_to_talk: 'settings.hotkeys.pushToTalk',
    free_speech:  'settings.hotkeys.freeSpeech',
    cancel:       'settings.hotkeys.cancel',
  };
</script>

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

<style>
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
</style>
