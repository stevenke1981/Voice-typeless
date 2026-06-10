<script lang="ts">
  import type { LocalConfig } from './types';
  import { t } from '../../i18n.svelte';

  interface Props { config: LocalConfig; }
  const { config }: Props = $props();

  const VAD_MIN = 1000;
  const VAD_MAX = 10000;

  const vadLabel = $derived.by(() => {
    const ms = config.text.vad_silence_threshold_ms;
    return `${(ms / 1000).toFixed(1)} s`;
  });
</script>

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
