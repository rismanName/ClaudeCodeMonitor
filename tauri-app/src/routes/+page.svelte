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
    BudgetWatcher,
    AlertStrip,
    OnboardingWizard,
    CommandPalette,
    ProjectsView,
    BudgetView,
    SetupView,
    InsightsView,
    SpikeWatcher,
    TabNav,
  } from '$lib/components';
  import { isConnected, requestedView, setAlert, clearAlert, onboardingOpen } from '$lib/stores';
  import { invoke } from '@tauri-apps/api/core';
  import { loadSettings, settings } from '$lib/stores/settings';
  import { t, type TranslationKey } from '$lib/i18n';
  import { sections, sectionOf } from '$lib/nav';
  import { shouldAutoOnboard } from '$lib/setup';

  interface StackStatus { state: 'running' | 'started' | 'error'; code: string; detail: string }

  // Start Docker + compose stack when Prometheus/OTLP ports are down, so the app works standalone.
  // Progress/result is reported through the shared alert strip (id 'stack').
  async function ensureStack() {
    setAlert({ id: 'stack', severity: 'info', text: $t('stack.starting') });
    try {
      const r = await invoke<StackStatus>('ensure_monitoring_stack', {
        composeDir: $settings.composeDir || null,
      });
      if (r.state === 'running') {
        clearAlert('stack');
      } else if (r.state === 'started') {
        setAlert({ id: 'stack', severity: 'info', text: $t('stack.started') });
        setTimeout(() => clearAlert('stack'), 5000);
      } else {
        const key = `stack.error.${r.code}` as TranslationKey;
        setAlert({ id: 'stack', severity: 'critical', text: r.detail ? `${$t(key)} (${r.detail})` : $t(key) });
      }
    } catch (e) {
      setAlert({ id: 'stack', severity: 'critical', text: String(e) });
    }
  }

  let activeView = 'summary';
  let showSettings = false;
  let showPalette = false;

  $: if ($requestedView) {
    activeView = $requestedView;
    requestedView.set(null);
  }

  // Remember the last tab opened in each section so switching sections returns to it.
  const lastView: Record<string, string> = {};

  $: section = sectionOf(activeView);
  $: lastView[section.id] = activeView;

  function handleSection(id: string) {
    const target = sections.find((s) => s.id === id);
    if (target) activeView = lastView[id] ?? target.views[0].id;
  }

  // Listen for keyboard shortcut to open settings
  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      showPalette = !showPalette;
    }
    if ((e.metaKey || e.ctrlKey) && e.key === ',') {
      e.preventDefault();
      showSettings = !showSettings;
    }
  }

  onMount(() => {
    loadSettings().then(() => {
      // Let the stack finish starting first so the wizard does not flash while auto-start is still working.
      (async () => {
        if ($settings.autoStartStack) await ensureStack();
        if (await shouldAutoOnboard()) onboardingOpen.set(true);
      })();
    });
    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });
</script>

<div class="flex h-screen bg-bg-primary">
  <Sidebar {activeView} onNavigate={handleSection} onOpenSettings={() => (showSettings = true)} onOpenPalette={() => (showPalette = true)} />

  <main class="flex-1 overflow-y-auto">
    <div class="max-w-5xl mx-auto px-8 py-6">
    <BudgetWatcher />
    <SpikeWatcher />
    <AlertStrip />
    {#if section.views.length > 1}
      <TabNav
        tabs={section.views.map((v) => ({ id: v.id, label: $t(v.label) }))}
        active={activeView}
        onChange={(id) => (activeView = id)}
      />
    {/if}
    {#if activeView === 'summary'}
      <SummaryView />
    {:else if activeView === 'tokens'}
      <TokenMetricsView />
    {:else if activeView === 'projects'}
      <ProjectsView />
    {:else if activeView === 'budget'}
      <BudgetView />
    {:else if activeView === 'insights'}
      <InsightsView />
    {:else if activeView === 'setup'}
      <SetupView />
    {:else if activeView === 'sessions'}
      <SessionsView />
    {:else if activeView === 'stats-cache'}
      <LocalStatsCacheView />
    {:else if activeView === 'prometheus-health'}
      <PrometheusHealthView />
    {:else if activeView === 'smoke-test'}
      <SmokeTestView />
    {/if}
    </div>
  </main>
</div>

{#if $onboardingOpen}<OnboardingWizard />{/if}
{#if showPalette}
  <CommandPalette onClose={() => (showPalette = false)} onNavigate={(v) => (activeView = v)} onOpenSettings={() => (showSettings = true)} />
{/if}
<SettingsModal open={showSettings} onClose={() => (showSettings = false)} />
