<script lang="ts">
  import { onMount } from 'svelte';
  import {
    Sidebar,
    SummaryView,
    TokenMetricsView,
    SessionsView,
    LocalStatsCacheView,
    SmokeTestView,
    PrometheusHealthView,
    SettingsModal,
    BudgetBanner,
    SpikeBanner,
  } from '$lib/components';
  import { isConnected } from '$lib/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { loadSettings, settings } from '$lib/stores/settings';
  import { t, type TranslationKey } from '$lib/i18n';

  interface StackStatus { state: 'running' | 'started' | 'error'; code: string; detail: string }
  let stackBanner: { kind: 'info' | 'ok' | 'error'; text: string } | null = null;

  // Start Docker + compose stack when Prometheus/OTLP ports are down, so the app works standalone.
  async function ensureStack() {
    stackBanner = { kind: 'info', text: $t('stack.starting') };
    try {
      const r = await invoke<StackStatus>('ensure_monitoring_stack', {
        composeDir: $settings.composeDir || null,
      });
      if (r.state === 'running') {
        stackBanner = null;
      } else if (r.state === 'started') {
        stackBanner = { kind: 'ok', text: $t('stack.started') };
        setTimeout(() => (stackBanner = null), 5000);
      } else {
        const key = `stack.error.${r.code}` as TranslationKey;
        stackBanner = { kind: 'error', text: r.detail ? `${$t(key)} (${r.detail})` : $t(key) };
      }
    } catch (e) {
      stackBanner = { kind: 'error', text: String(e) };
    }
  }

  let activeView = 'summary';
  let showSettings = false;

  function handleNavigate(view: string) {
    activeView = view;
  }

  // Listen for keyboard shortcut to open settings
  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === ',') {
      e.preventDefault();
      showSettings = !showSettings;
    }
  }

  onMount(() => {
    loadSettings().then(() => {
      if ($settings.autoStartStack) ensureStack();
    });
    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="flex h-screen bg-bg-primary">
  <Sidebar {activeView} onNavigate={handleNavigate} onOpenSettings={() => (showSettings = true)} />

  <main class="flex-1 overflow-y-auto p-4">
    {#if stackBanner}
      <div
        class="mb-3 flex items-center justify-between rounded-md border px-3 py-2 text-xs
          {stackBanner.kind === 'error' ? 'bg-red/10 border-red/50 text-red' : stackBanner.kind === 'ok' ? 'bg-green/10 border-green/50 text-green' : 'bg-bg-card border-border-secondary text-text-secondary'}"
      >
        <span>{stackBanner.text}</span>
        <button class="ml-3 underline" on:click={() => (stackBanner = null)}>{$t('stack.dismiss')}</button>
      </div>
    {/if}
    <BudgetBanner />
    <SpikeBanner />
    {#if activeView === 'summary'}
      <SummaryView />
    {:else if activeView === 'tokens'}
      <TokenMetricsView />
    {:else if activeView === 'sessions'}
      <SessionsView />
    {:else if activeView === 'stats-cache'}
      <LocalStatsCacheView />
    {:else if activeView === 'prometheus-health'}
      <PrometheusHealthView />
    {:else if activeView === 'smoke-test'}
      <SmokeTestView />
    {/if}
  </main>
</div>

<SettingsModal open={showSettings} onClose={() => (showSettings = false)} />
