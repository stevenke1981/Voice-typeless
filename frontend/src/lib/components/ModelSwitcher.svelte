<script lang="ts">
  /**
   * ModelSwitcher — speech model selector & device indicator.
   *
   * Displays available models (SenseVoice, Whisper-tiny, custom ONNX),
   * shows the currently active model and inference device,
   * and allows switching between downloaded models.
   */

  import { onMount } from 'svelte';
  import { getModelList, setActiveModel, type ModelInfo } from '../tauri/commands';
  import { appState } from '../stores/appState.svelte';

  // ─── Props ─────────────────────────────────────────────────────────────────

  interface Props {
    /** Optional additional CSS class(es) for the root element. */
    class?: string;
  }

  const { class: className = '' }: Props = $props();

  // ─── State ─────────────────────────────────────────────────────────────────

  let models = $state<ModelInfo[]>([]);
  let isLoading = $state(false);
  let loadError = $state('');

  /** Mirrors appState.activeModel so the <select> stays in sync. */
  let selected = $state(appState.activeModel);

  // ─── Lifecycle ─────────────────────────────────────────────────────────────

  onMount(async () => {
    isLoading = true;
    loadError = '';
    try {
      models = await getModelList();
      appState.modelList = models.map(m => ({
        id: m.id,
        name: m.name,
        type: m.type,
        is_downloaded: m.is_downloaded,
        device: m.device,
      }));
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      isLoading = false;
    }
  });

  // ─── Handlers ──────────────────────────────────────────────────────────────

  async function onChange(e: Event): Promise<void> {
    const select = e.target as HTMLSelectElement;
    const modelId = select.value;
    loadError = '';
    try {
      await setActiveModel(modelId);
      appState.activeModel = modelId as typeof appState.activeModel;
      selected = modelId;
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
      select.value = appState.activeModel;
    }
  }

  // ─── Derived Values ────────────────────────────────────────────────────────

  /** The currently active ModelInfo (for showing device / status). */
  let activeModelInfo = $derived(
    models.find(m => m.id === appState.activeModel) ?? null
  );
</script>

<div class="model-switcher {className}">
  <label for="vtl-model-select" class="picker-label">
    <span class="label-icon" aria-hidden="true">⊞</span>
    Speech Model
  </label>

  <div class="select-wrapper" class:loading={isLoading}>
    <select
      id="vtl-model-select"
      class="picker-select"
      value={selected}
      onchange={onChange}
      disabled={isLoading}
      aria-label="Select speech recognition model"
      aria-busy={isLoading}
    >
      {#if isLoading}
        <option value="">Loading models…</option>
      {:else if models.length === 0}
        <option value="">No models available</option>
      {:else}
        {#each models as model (model.id)}
          <option value={model.id} disabled={!model.is_downloaded}>
            {model.name}
            {#if !model.is_downloaded}(not downloaded){/if}
          </option>
        {/each}
      {/if}
    </select>

    <!-- Custom dropdown arrow -->
    <span class="select-arrow" aria-hidden="true">▾</span>
  </div>

  <!-- Device & status indicator -->
  {#if activeModelInfo && activeModelInfo.is_downloaded}
    <p class="model-status">
      Device:
      <span class="device-badge" class:directml={activeModelInfo.device === 'directml'} class:cuda={activeModelInfo.device === 'cuda'} class:cpu={activeModelInfo.device === 'cpu'}>
        {activeModelInfo.device ?? 'auto'}
      </span>
    </p>
  {:else if activeModelInfo && !activeModelInfo.is_downloaded}
    <p class="model-status model-pending">Model not downloaded</p>
  {/if}

  {#if loadError}
    <p class="picker-error" role="alert">
      <span aria-hidden="true">⚠</span> {loadError}
    </p>
  {/if}
</div>

<style>
  /* ── Root container ────────────────────────────────────────────────────────── */
  .model-switcher {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  /* ── Label ─────────────────────────────────────────────────────────────────── */
  .picker-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--vtl-gray);
    text-transform: uppercase;
    letter-spacing: 0.07em;
    user-select: none;
  }

  .label-icon { font-size: 14px; }

  /* ── Select wrapper ────────────────────────────────────────────────────────── */
  .select-wrapper {
    position: relative;
    transition: opacity 0.15s;
  }

  .select-wrapper.loading { opacity: 0.6; }

  /* ── Select ────────────────────────────────────────────────────────────────── */
  .picker-select {
    width: 100%;
    background: var(--vtl-bg-dark-2);
    border: 1px solid var(--vtl-border);
    border-radius: 8px;
    color: var(--vtl-text-dark);
    padding: 8px 36px 8px 12px;
    font-size: 13px;
    font-family: inherit;
    cursor: pointer;
    appearance: none;
    -webkit-appearance: none;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .picker-select:focus {
    border-color: var(--vtl-teal);
    box-shadow: 0 0 0 2px rgba(0, 230, 200, 0.15);
  }

  .picker-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .picker-select option {
    background: var(--vtl-bg-dark-2);
    color: var(--vtl-text-dark);
  }

  /* ── Custom dropdown arrow ─────────────────────────────────────────────────── */
  .select-arrow {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--vtl-gray);
    font-size: 11px;
    pointer-events: none;
    user-select: none;
  }

  /* ── Status line ───────────────────────────────────────────────────────────── */
  .model-status {
    margin: 0;
    font-size: 11px;
    color: var(--vtl-gray);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .model-pending {
    color: #ffa94d;
  }

  .device-badge {
    display: inline-block;
    padding: 1px 7px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .device-badge.directml {
    background: rgba(0, 230, 200, 0.12);
    color: #00e6c8;
  }

  .device-badge.cuda {
    background: rgba(91, 78, 255, 0.15);
    color: #7b6eff;
  }

  .device-badge.cpu {
    background: rgba(160, 160, 168, 0.12);
    color: #a0a0a8;
  }

  /* ── Error message ─────────────────────────────────────────────────────────── */
  .picker-error {
    margin: 0;
    font-size: 11px;
    color: #ff6b6b;
    display: flex;
    align-items: center;
    gap: 4px;
  }
</style>
