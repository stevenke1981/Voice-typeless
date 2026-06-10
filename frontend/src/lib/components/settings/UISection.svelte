<script lang="ts">
  import type { LocalConfig } from './types';
  import { t } from '../../i18n.svelte';

  interface Props { config: LocalConfig; }
  const { config }: Props = $props();

  const THEME_OPTIONS: Array<{ value: LocalConfig['ui']['theme']; label: string }> = [
    { value: 'dark',   label: '🌙 Dark'   },
    { value: 'light',  label: '☀ Light'  },
    { value: 'system', label: '⚙ System' },
  ];

  const LANG_OPTIONS: Array<{ value: LocalConfig['ui']['language']; label: string }> = [
    { value: 'en', label: 'English' },
    { value: 'zh', label: '中文'    },
  ];

  const retentionLabel = $derived.by(() => {
    const days = config.ui.history_retention_days;
    return days === 0 ? 'Keep forever' : `${days} day${days !== 1 ? 's' : ''}`;
  });
</script>

<section class="settings-section" aria-labelledby="ui-heading">
  <h3 id="ui-heading" class="section-heading">
    <span aria-hidden="true">🎨</span> {t('settings.section.interface')}
  </h3>

  <!-- Theme selection -->
  <div class="field-row">
    <span class="field-label" id="theme-group-label">{t('settings.ui.theme')}</span>
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

<style>
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
</style>
