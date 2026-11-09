<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { UsageWindowCard, ViewHeader } from '$lib/components';
  import { settings, saveSettings } from '$lib/stores/settings';
  import { formatCost } from '$lib/format';
  import { t } from '$lib/i18n';
  import { weekdayNames } from '$lib/usageWindow';

  interface CostSpike { isSpike: boolean; lastHourUsd: number; avgHourlyUsd: number }

  let spentUsd = 0;
  let spike: CostSpike | null = null;
  let budgetInput = $settings.dailyBudgetUsd;
  let limitInput = $settings.planLimitTokens;
  let weeklyInput = $settings.weeklyLimitTokens;
  let resetDayInput = $settings.weeklyResetDay;
  let resetTimeInput = $settings.weeklyResetTime;
  $: days = weekdayNames($settings.language);
  let saved = false;
  let timer: ReturnType<typeof setInterval>;

  async function refresh() {
    try {
      // Total from Prometheus (not summed from sessions, which only lists sessions present in history.jsonl).
      const m = await invoke<{ totalCostUsd: number }>('get_dashboard_metrics', {
        timeRange: '1d',
        prometheusUrl: $settings.prometheusUrl,
      });
      spentUsd = m.totalCostUsd || 0;
    } catch { /* keep last value */ }
    try {
      spike = await invoke<CostSpike>('get_cost_spike', { prometheusUrl: $settings.prometheusUrl });
    } catch { spike = null; }
  }

  async function save() {
    await saveSettings({
      ...$settings,
      dailyBudgetUsd: Math.max(0, Number(budgetInput) || 0),
      planLimitTokens: Math.max(0, Math.round(Number(limitInput) || 0)),
      weeklyLimitTokens: Math.max(0, Math.round(Number(weeklyInput) || 0)),
      weeklyResetDay: Math.min(6, Math.max(0, Math.round(Number(resetDayInput) || 0))),
      weeklyResetTime: /^\d{1,2}:\d{2}$/.test(resetTimeInput) ? resetTimeInput : '23:00',
    });
    saved = true;
    setTimeout(() => (saved = false), 2500);
  }

  $: budget = $settings.dailyBudgetUsd;
  $: pct = budget > 0 ? Math.min(100, (spentUsd / budget) * 100) : 0;
  $: barColor = pct >= 100 ? 'bg-red' : pct >= 80 ? 'bg-peach' : 'bg-green';

  onMount(() => {
    refresh();
    timer = setInterval(refresh, 60_000);
  });
  onDestroy(() => timer && clearInterval(timer));
</script>

<div>
  <ViewHeader category="summary" title={$t('nav.budget')} subtitle={$t('nav.budget.desc')} />

  <div class="bg-bg-card rounded-xl p-4 mb-4">
    <div class="flex items-center justify-between mb-2">
      <span class="text-xs font-medium text-text-muted uppercase tracking-wider">{$t('budget.title')}</span>
      {#if budget > 0}<span class="text-xs text-text-muted">{pct.toFixed(0)}%</span>{/if}
    </div>
    <div class="text-lg font-bold text-text-primary mb-1">
      {$formatCost(spentUsd)}
      {#if budget > 0}<span class="text-xs font-normal text-text-muted">/ {$formatCost(budget)}</span>{/if}
    </div>
    {#if budget > 0}
      <div class="h-2 rounded-full bg-bg-primary overflow-hidden"><div class="h-full {barColor} transition-all" style="width: {pct}%"></div></div>
    {:else}
      <p class="text-xs text-text-muted">{$t('budget.notSet')}</p>
    {/if}
    {#if spike}
      <p class="mt-2 text-xs {spike.isSpike ? 'text-peach' : 'text-text-muted'}">
        {$t('budget.lastHour')}: {$formatCost(spike.lastHourUsd)} · {$t('budget.usual')}: {$formatCost(spike.avgHourlyUsd)}/{$t('proj.hourShort')}
        {#if spike.isSpike} — {$t('budget.spike')}{/if}
      </p>
    {/if}
  </div>

  <UsageWindowCard />

  <div class="bg-bg-card rounded-xl p-4">
    <div class="text-xs font-medium text-text-muted uppercase tracking-wider mb-3">{$t('budget.limits')}</div>
    <div class="grid grid-cols-2 gap-3 mb-3">
      <div>
        <label class="block text-sm text-text-secondary mb-1" for="b-budget">{$t('set.dailyBudget')}</label>
        <input id="b-budget" type="number" min="0" step="0.5" bind:value={budgetInput}
          class="w-full bg-bg-primary border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue" />
      </div>
      <div>
        <label class="block text-sm text-text-secondary mb-1" for="b-limit">{$t('set.planLimit')}</label>
        <input id="b-limit" type="number" min="0" step="100000" bind:value={limitInput}
          class="w-full bg-bg-primary border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue" />
      </div>
    </div>
    <div class="grid grid-cols-3 gap-3 mb-3">
      <div>
        <label class="block text-sm text-text-secondary mb-1" for="b-weekly">{$t('set.weeklyLimit')}</label>
        <input id="b-weekly" type="number" min="0" step="1000000" bind:value={weeklyInput}
          class="w-full bg-bg-primary border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue" />
      </div>
      <div>
        <label class="block text-sm text-text-secondary mb-1" for="b-day">{$t('set.weeklyResetDay')}</label>
        <select id="b-day" bind:value={resetDayInput}
          class="w-full bg-bg-primary border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue">
          {#each days as d, i}<option value={i}>{d}</option>{/each}
        </select>
      </div>
      <div>
        <label class="block text-sm text-text-secondary mb-1" for="b-time">{$t('set.weeklyResetTime')}</label>
        <input id="b-time" type="time" bind:value={resetTimeInput}
          class="w-full bg-bg-primary border border-border-secondary rounded-md px-3 py-2 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue" />
      </div>
    </div>
    <div class="flex items-center gap-3">
      <button on:click={save} class="px-4 py-2 bg-blue text-crust rounded-md hover:bg-blue/90 transition-colors font-medium text-sm">{$t('set.save')}</button>
      {#if saved}<span class="text-xs text-green">{$t('budget.saved')}</span>{/if}
      <span class="text-xs text-text-muted">{$t('set.budget.hint')}</span>
    </div>
  </div>
</div>
