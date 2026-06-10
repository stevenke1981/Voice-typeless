<script lang="ts">
  import type { LocalConfig } from './types';
  import { t } from '../../i18n.svelte';

  interface Props {
    config: LocalConfig;
    isSaving: boolean;
    saveError: string;
    saveSuccess: boolean;
    save: () => Promise<void>;
    resetDefaults: () => void;
  }

  const { config, isSaving, saveError, saveSuccess, save, resetDefaults }: Props = $props();
</script>

<footer class="settings-footer">
  {#if saveError}
    <p class="footer-error" role="alert">
      <span aria-hidden="true">⚠</span> {saveError}
    </p>
  {/if}
  {#if saveSuccess}
    <p class="footer-success" role="status" aria-live="polite">
      <span aria-hidden="true">✓</span> {t('settings.saved')}
    </p>
  {/if}

  <div class="footer-actions">
    <button
      class="btn-ghost"
      onclick={resetDefaults}
      aria-label="Reset all settings to defaults"
    >
      {t('settings.reset')}
    </button>
    <button
      class="btn-primary"
      onclick={save}
      disabled={isSaving}
      aria-label={isSaving ? t('settings.saving') : t('settings.save')}
      aria-busy={isSaving}
    >
      {isSaving ? t('settings.saving') : t('settings.save')}
    </button>
  </div>
</footer>

<style>
  .settings-footer {
    padding: 20px 20px 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .footer-error {
    margin: 0;
    font-size: 12px;
    color: #ff6b6b;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .footer-success {
    margin: 0;
    font-size: 12px;
    color: var(--vtl-green);
    display: flex;
    align-items: center;
    gap: 6px;
    animation: vtl-fadein 0.2s ease;
  }

  @keyframes vtl-fadein {
    from { opacity: 0; transform: translateY(4px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .footer-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
  }

  .btn-ghost {
    background: none;
    border: 1px solid var(--vtl-border);
    border-radius: 8px;
    color: var(--vtl-gray);
    padding: 8px 16px;
    font-size: 13px;
    font-family: inherit;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }

  .btn-ghost:hover {
    border-color: var(--vtl-text-dark);
    color: var(--vtl-text-dark);
  }

  .btn-ghost:focus-visible {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 2px;
  }

  .btn-primary {
    background: var(--vtl-teal);
    border: none;
    border-radius: 8px;
    color: var(--vtl-bg-dark);
    padding: 8px 20px;
    font-size: 13px;
    font-weight: 700;
    font-family: inherit;
    cursor: pointer;
    transition: opacity 0.15s, box-shadow 0.15s;
    box-shadow: 0 0 10px rgba(0, 230, 200, 0.20);
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.90;
    box-shadow: 0 0 16px rgba(0, 230, 200, 0.35);
  }

  .btn-primary:focus-visible {
    outline: 2px solid var(--vtl-teal);
    outline-offset: 3px;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
