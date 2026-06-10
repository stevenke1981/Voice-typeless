<script lang="ts">
  import type { LocalConfig } from './types';
  import { t } from '../../i18n.svelte';

  interface Props { config: LocalConfig; }
  const { config }: Props = $props();

  const MODEL_OPTIONS = [
    { id: 'sensevoice-small', label: 'SenseVoice Small (recommended)' },
    { id: 'whisper-tiny',     label: 'Whisper Tiny (fallback)'       },
  ];

  const DEVICE_OPTIONS: Array<{ value: LocalConfig['model']['device']; label: string }> = [
    { value: 'auto',     label: 'Auto (DirectML → CUDA → CPU)' },
    { value: 'directml', label: 'DirectML (Windows GPU)'       },
    { value: 'cuda',     label: 'CUDA (NVIDIA GPU)'            },
    { value: 'cpu',      label: 'CPU only'                     },
  ];
</script>

<section class="settings-section" aria-labelledby="model-heading">
  <h3 id="model-heading" class="section-heading">
    <span aria-hidden="true">🧠</span> {t('settings.section.model')}
  </h3>

  <div class="field-row">
    <label for="active-model" class="field-label">{t('settings.model.active')}</label>
    <select
      id="active-model"
      class="select-input"
      bind:value={config.model.active_model_id}
      aria-label="Select speech model"
    >
      {#each MODEL_OPTIONS as m (m.id)}
        <option value={m.id}>{m.label}</option>
      {/each}
    </select>
  </div>

  <div class="field-row">
    <label for="inference-device" class="field-label">
      {t('settings.model.device')}
      <span class="field-hint">{t('settings.model.deviceHint')}</span>
    </label>
    <select
      id="inference-device"
      class="select-input"
      bind:value={config.model.device}
      aria-label="Select inference hardware device"
    >
      {#each DEVICE_OPTIONS as d (d.value)}
        <option value={d.value}>{d.label}</option>
      {/each}
    </select>
  </div>
</section>
