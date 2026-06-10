<script lang="ts">
  import type { LocalConfig } from './types';
  import DevicePicker from '../DevicePicker.svelte';
  import { t } from '../../i18n.svelte';

  interface Props { config: LocalConfig; }
  const { config }: Props = $props();
</script>

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
