<script lang="ts">
  /**
   * DevicePicker — microphone device selector.
   *
   * Loads available devices via the `getDevices` Tauri command.
   * Persists the selection via `setDevice` and mirrors it into appState.
   */

  import { onMount } from 'svelte';
  import { getDevices, setDevice, type DeviceInfo } from '../tauri/commands';
  import { appState } from '../stores/appState.svelte';

  // ─── Props ─────────────────────────────────────────────────────────────────

  interface Props {
    /** Optional additional CSS class(es) for the root element. */
    class?: string;
  }

  const { class: className = '' }: Props = $props();

  // ─── State ─────────────────────────────────────────────────────────────────

  let devices = $state<DeviceInfo[]>([]);
  let isLoading = $state(false);
  let loadError = $state('');

  /** Mirrors appState.activeDevice so the <select> stays in sync. */
  let selected = $state(appState.activeDevice);

  // ─── Lifecycle ─────────────────────────────────────────────────────────────

  onMount(async () => {
    isLoading = true;
    loadError = '';
    try {
      devices = await getDevices();
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      isLoading = false;
    }
  });

  // ─── Handlers ──────────────────────────────────────────────────────────────

  async function onChange(e: Event): Promise<void> {
    const select = e.target as HTMLSelectElement;
    const deviceId = select.value;
    loadError = '';
    try {
      await setDevice(deviceId);
      appState.activeDevice = deviceId;
      selected = deviceId;
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
      // Revert select to previous value
      select.value = appState.activeDevice;
    }
  }
</script>

<div class="device-picker {className}">
  <label for="vtl-mic-select" class="picker-label">
    <span class="label-icon" aria-hidden="true">🎙</span>
    Microphone
  </label>

  <div class="select-wrapper" class:loading={isLoading}>
    <select
      id="vtl-mic-select"
      class="picker-select"
      value={selected}
      onchange={onChange}
      disabled={isLoading}
      aria-label="Select microphone device"
      aria-busy={isLoading}
    >
      <!-- Always present "default" option -->
      <option value="default">
        {isLoading ? 'Loading devices…' : 'Default Microphone'}
      </option>

      {#each devices as device (device.id)}
        <option value={device.id}>{device.name}</option>
      {/each}
    </select>

    <!-- Custom dropdown arrow -->
    <span class="select-arrow" aria-hidden="true">▾</span>
  </div>

  {#if loadError}
    <p class="picker-error" role="alert">
      <span aria-hidden="true">⚠</span> {loadError}
    </p>
  {/if}
</div>

<style>
  /* ── Root container ───────────────────────────────────────────────────────── */
  .device-picker {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  /* ── Label ────────────────────────────────────────────────────────────────── */
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

  /* ── Select wrapper (for custom arrow) ───────────────────────────────────── */
  .select-wrapper {
    position: relative;
    transition: opacity 0.15s;
  }

  .select-wrapper.loading { opacity: 0.6; }

  /* ── Select ───────────────────────────────────────────────────────────────── */
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
    /* Remove native arrow */
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

  /* Dark mode option background (WebKit) */
  .picker-select option {
    background: var(--vtl-bg-dark-2);
    color: var(--vtl-text-dark);
  }

  /* ── Custom dropdown arrow ────────────────────────────────────────────────── */
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

  /* ── Error message ────────────────────────────────────────────────────────── */
  .picker-error {
    margin: 0;
    font-size: 11px;
    color: #ff6b6b;
    display: flex;
    align-items: center;
    gap: 4px;
  }
</style>
