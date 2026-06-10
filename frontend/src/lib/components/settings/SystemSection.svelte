<script lang="ts">
  import type { LocalConfig } from './types';
  import { t } from '../../i18n.svelte';
  import { setAutostartEnabled } from '../../tauri/commands';

  interface Props { config: LocalConfig; }
  const { config }: Props = $props();
</script>

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
