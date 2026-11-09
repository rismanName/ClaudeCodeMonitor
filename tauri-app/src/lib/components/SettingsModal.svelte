<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getVersion } from '@tauri-apps/api/app';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { settings, saveSettings } from '$lib/stores/settings';
  import type { Settings } from '$lib/types';
  import { onMount } from 'svelte';
  import { t } from '$lib/i18n';

  export let open: boolean;
  export let onClose: () => void;

  let localSettings: Settings = { ...$settings };
  let testStatus: 'idle' | 'testing' | 'success' | 'error' = 'idle';
  let updateStatus: 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'none' | 'error' = 'idle';
  let updateVersion: string = '';
  let updateError: string = '';
  let appVersion: string = '';

  onMount(async () => {
    appVersion = await getVersion();
  });

  $: if (open) {
    localSettings = { ...$settings };
    testStatus = 'idle';
  }

  async function testConnection() {
    testStatus = 'testing';
    try {
      const result = await invoke<boolean>('test_connection', {
        url: localSettings.prometheusUrl,
      });
      testStatus = result ? 'success' : 'error';
    } catch {
      testStatus = 'error';
    }
  }

  async function checkForUpdates() {
    updateStatus = 'checking';
    updateError = '';
    try {
      const update = await check();
      if (update) {
        updateVersion = update.version;
        updateStatus = 'available';
      } else {
        updateStatus = 'none';
      }
    } catch (e) {
      updateError = String(e);
      updateStatus = 'error';
    }
  }

  async function downloadAndInstall() {
    updateStatus = 'downloading';
    try {
      const update = await check();
      if (update) {
        await update.downloadAndInstall();
        updateStatus = 'ready';
      }
    } catch (e) {
      updateError = String(e);
      updateStatus = 'error';
    }
  }

  async function restartApp() {
    await relaunch();
  }

  async function save() {
    await saveSettings({ ...localSettings });
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if open}
  <div class="fixed inset-0 bg-crust/80 flex items-center justify-center z-50">
    <div class="bg-bg-primary border border-border-secondary rounded-lg p-6 w-full max-w-md shadow-card">
      <h2 class="text-lg font-bold text-text-primary mb-4">{$t('set.title')}</h2>

      <div class="space-y-4">
        <!-- Prometheus URL -->
        <div>
          <label class="block text-sm text-text-secondary mb-1" for="prometheus-url">{$t('set.promUrl')}</label>
          <div class="flex gap-2">
            <input
              id="prometheus-url"
              type="text"
              bind:value={localSettings.prometheusUrl}
              class="flex-1 bg-bg-card border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue"
            />
            <button
              on:click={testConnection}
              class="px-3 py-2 bg-bg-card border border-border-secondary rounded-md text-text-secondary hover:bg-bg-card-hover transition-colors"
            >
              {#if testStatus === 'testing'}
                {$t('set.testing')}
              {:else if testStatus === 'success'}
                <span class="text-green">{$t('set.pass')}</span>
              {:else if testStatus === 'error'}
                <span class="text-red">{$t('set.fail')}</span>
              {:else}
                {$t('set.test')}
              {/if}
            </button>
          </div>
        </div>

        <!-- Refresh Interval -->
        <div>
          <label class="block text-sm text-text-secondary mb-1" for="refresh-interval">{$t('set.refresh')}</label>
          <input
            id="refresh-interval"
            type="number"
            bind:value={localSettings.refreshInterval}
            min="5"
            max="300"
            class="w-full bg-bg-card border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue"
          />
        </div>

        <!-- Pricing Provider -->
        <div>
          <label class="block text-sm text-text-secondary mb-1" for="pricing-provider">{$t('set.pricing')}</label>
          <select
            id="pricing-provider"
            bind:value={localSettings.pricingProvider}
            class="w-full bg-bg-card border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue"
          >
            <option value="anthropic">Anthropic (Direct API)</option>
            <option value="aws-bedrock">AWS Bedrock</option>
            <option value="google-vertex">Google Vertex AI</option>
          </select>
        </div>

        <!-- Currency -->
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block text-sm text-text-secondary mb-1" for="currency">{$t('set.currency')}</label>
            <select
              id="currency"
              bind:value={localSettings.currency}
              class="w-full bg-bg-card border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue"
            >
              <option value="IDR">Rupiah (IDR)</option>
              <option value="USD">US Dollar (USD)</option>
            </select>
          </div>
          <div>
            <label class="block text-sm text-text-secondary mb-1" for="idr-rate">{$t('set.rate')}</label>
            <input
              id="idr-rate"
              type="number"
              min="1"
              bind:value={localSettings.usdToIdrRate}
              class="w-full bg-bg-card border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue"
            />
          </div>
        </div>

        <!-- Budget & plan limit -->
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block text-sm text-text-secondary mb-1" for="daily-budget">{$t('set.dailyBudget')}</label>
            <input
              id="daily-budget"
              type="number"
              min="0"
              step="0.5"
              bind:value={localSettings.dailyBudgetUsd}
              class="w-full bg-bg-card border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue"
            />
          </div>
          <div>
            <label class="block text-sm text-text-secondary mb-1" for="plan-limit">{$t('set.planLimit')}</label>
            <input
              id="plan-limit"
              type="number"
              min="0"
              step="100000"
              bind:value={localSettings.planLimitTokens}
              class="w-full bg-bg-card border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue"
            />
          </div>
        </div>
        <p class="text-xs text-text-muted -mt-2">{$t('set.budget.hint')}</p>

        <!-- Language -->
        <div>
          <label class="block text-sm text-text-secondary mb-1" for="language">{$t('set.language')}</label>
          <select
            id="language"
            bind:value={localSettings.language}
            class="w-full bg-bg-card border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue"
          >
            <option value="id">Bahasa Indonesia</option>
            <option value="en">English</option>
          </select>
        </div>

        <!-- Auto-start monitoring stack -->
        <div>
          <label class="flex items-center gap-2 text-sm text-text-secondary">
            <input type="checkbox" bind:checked={localSettings.autoStartStack} />
            {$t('set.autoStart')}
          </label>
          <p class="text-xs text-text-muted mt-1">{$t('set.autoStart.hint')}</p>
          {#if localSettings.autoStartStack}
            <label class="block text-sm text-text-secondary mt-2 mb-1" for="compose-dir">{$t('set.composeDir')}</label>
            <input
              id="compose-dir"
              type="text"
              bind:value={localSettings.composeDir}
              placeholder="/path/to/ClaudeCodeMonitor"
              class="w-full bg-bg-card border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue"
            />
            <p class="text-xs text-text-muted mt-1">{$t('set.composeDir.hint')}</p>
          {/if}
        </div>

        <!-- Version -->
        <div class="pt-4 border-t border-border-secondary">
          <label class="block text-sm text-text-secondary mb-1">{$t('set.version')}</label>
          <span class="text-text-primary">{appVersion || '...'}</span>
        </div>

        <!-- Updates -->
        <div class="pt-4">
          <label class="block text-sm text-text-secondary mb-2">{$t('set.updates')}</label>
          <div class="flex items-center gap-3">
            {#if updateStatus === 'idle'}
              <button
                on:click={checkForUpdates}
                class="px-3 py-2 bg-bg-card border border-border-secondary rounded-md text-text-secondary hover:bg-bg-card-hover transition-colors"
              >
                {$t('set.checkUpdates')}
              </button>
            {:else if updateStatus === 'checking'}
              <span class="text-text-muted text-sm">{$t('set.checking')}</span>
            {:else if updateStatus === 'none'}
              <span class="text-green text-sm">{$t('set.upToDate')}</span>
              <button
                on:click={checkForUpdates}
                class="px-3 py-2 bg-bg-card border border-border-secondary rounded-md text-text-secondary hover:bg-bg-card-hover transition-colors text-sm"
              >
                {$t('set.checkAgain')}
              </button>
            {:else if updateStatus === 'available'}
              <span class="text-text-secondary text-sm">{$t('set.available', { v: updateVersion })}</span>
              <button
                on:click={downloadAndInstall}
                class="px-3 py-2 bg-blue text-crust rounded-md hover:bg-blue/90 transition-colors text-sm font-medium"
              >
                {$t('set.install')}
              </button>
            {:else if updateStatus === 'downloading'}
              <span class="text-text-muted text-sm">{$t('set.downloading')}</span>
            {:else if updateStatus === 'ready'}
              <span class="text-green text-sm">{$t('set.ready')}</span>
              <button
                on:click={restartApp}
                class="px-3 py-2 bg-green text-crust rounded-md hover:bg-green/90 transition-colors text-sm font-medium"
              >
                {$t('set.restart')}
              </button>
            {:else if updateStatus === 'error'}
              <span class="text-red text-sm">{$t('set.error')}: {updateError}</span>
              <button
                on:click={checkForUpdates}
                class="px-3 py-2 bg-bg-card border border-border-secondary rounded-md text-text-secondary hover:bg-bg-card-hover transition-colors text-sm"
              >
                {$t('set.retry')}
              </button>
            {/if}
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex justify-end gap-3 mt-6">
        <button
          on:click={onClose}
          class="px-4 py-2 text-text-muted hover:text-text-primary transition-colors"
        >
          {$t('set.cancel')}
        </button>
        <button
          on:click={save}
          class="px-4 py-2 bg-blue text-crust rounded-md hover:bg-blue/90 transition-colors font-medium"
        >
          {$t('set.save')}
        </button>
      </div>
    </div>
  </div>
{/if}
