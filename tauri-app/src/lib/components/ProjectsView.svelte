<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { TimeRangePicker, ViewHeader, MetricCard } from '$lib/components';
  import { settings } from '$lib/stores/settings';
  import { timeRange as timeRangeStore, sessionProjectFilter, drillOrigin, requestedView } from '$lib/stores';
  import { formatCost } from '$lib/format';
  import { t } from '$lib/i18n';
  import type { SessionsData, TimeRange } from '$lib/types';

  let data: SessionsData | null = null;
  let loading = true;
  let error: string | null = null;

  async function fetchData() {
    loading = true;
    error = null;
    try {
      data = await invoke<SessionsData>('get_sessions_data', {
        timeRange: $timeRangeStore,
        prometheusUrl: $settings.prometheusUrl,
      });
    } catch (e) {
      error = e as string;
    } finally {
      loading = false;
    }
  }

  function handleRange(v: TimeRange) {
    timeRangeStore.set(v);
    fetchData();
  }

  function fmtTokens(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return n.toString();
  }

  function fmtHours(sec: number): string {
    const h = sec / 3600;
    return h >= 1 ? `${h.toFixed(1)} ${$t('proj.hours')}` : `${Math.round(sec / 60)} ${$t('proj.minutes')}`;
  }

  // Projects with usage, most expensive first.
  $: projects = (data?.projects ?? []).filter((p) => p.totalTokens > 0).slice().sort((a, b) => b.totalCostUsd - a.totalCostUsd);
  $: totalCost = projects.reduce((s, p) => s + p.totalCostUsd, 0);
  $: maxCost = Math.max(0.0001, ...projects.map((p) => p.totalCostUsd));

  /** Drill down: open Session History filtered to this project; Esc / the back link returns here. */
  function openSessions(p: { project: string; projectPath: string | null }) {
    sessionProjectFilter.set({ key: p.projectPath ?? p.project, label: p.project });
    drillOrigin.set('projects');
    requestedView.set('sessions');
  }

  onMount(fetchData);
</script>

<div>
  <ViewHeader category="sessions" title={$t('nav.projects')} subtitle={$t('nav.projects.desc')}>
    <svelte:fragment slot="actions">
      <TimeRangePicker value={$timeRangeStore} onChange={handleRange} />
    </svelte:fragment>
  </ViewHeader>

  {#if loading && !data}
    <div class="text-xs text-text-muted">…</div>
  {:else if error}
    <div class="bg-red/10 border border-red/50 rounded-md p-3 text-xs text-red">{error}</div>
  {:else if projects.length === 0}
    <div class="bg-bg-card rounded-xl p-4 text-sm text-text-secondary">{$t('proj.empty')}</div>
  {:else}
    <div class="grid grid-cols-3 gap-2 mb-3">
      <MetricCard label={$t('proj.count')} value={projects.length.toString()} color="yellow" />
      <MetricCard label={$t('proj.totalCost')} value={$formatCost(totalCost)} color="green" highlight={true} />
      <MetricCard label={$t('proj.top')} value={projects[0].project} subtitle={`${$formatCost(projects[0].totalCostUsd)} · ${totalCost > 0 ? Math.round((projects[0].totalCostUsd / totalCost) * 100) : 0}%`} color="purple" />
    </div>

    <div class="bg-bg-card rounded-xl p-4 space-y-3">
      {#each projects as p}
        <button type="button" class="block w-full text-left rounded-lg -mx-2 px-2 py-1.5 hover:bg-bg-card-hover transition-colors" title={$t('proj.openSessions')} on:click={() => openSessions(p)}>
          <div class="flex items-baseline justify-between gap-3 mb-1">
            <span class="text-sm font-semibold text-text-primary truncate" title={p.projectPath ?? ''}>{p.project}</span>
            <span class="text-sm font-bold text-accent-green flex-shrink-0">{$formatCost(p.totalCostUsd)}</span>
          </div>
          <div class="h-1.5 rounded-full bg-bg-primary overflow-hidden">
            <div class="h-full bg-green" style="width: {(p.totalCostUsd / maxCost) * 100}%"></div>
          </div>
          <div class="mt-1 flex gap-4 text-xs text-text-muted">
            <span>{fmtTokens(p.totalTokens)} {$t('proj.tokens')}</span>
            <span>{p.sessionCount} {$t('proj.sessions')}</span>
            <span>{fmtHours(p.activeTimeSeconds)}</span>
            {#if p.activeTimeSeconds > 0}
              <span>{$formatCost(p.totalCostUsd / (p.activeTimeSeconds / 3600))}/{$t('proj.hourShort')}</span>
            {/if}
          </div>
        </button>
      {/each}
    </div>
  {/if}
</div>
