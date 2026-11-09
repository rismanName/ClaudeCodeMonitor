<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { settings } from '$lib/stores/settings';
  import { formatCost } from '$lib/format';
  import { t } from '$lib/i18n';

  // Spend over a rolling 24h window (not calendar day), from the Sessions data source.
  // TODO(budget-notify): Also send a macOS notification once per threshold per day
  //   (tauri-plugin-notification + capability) and tint the tray title orange/red; today only this in-app banner exists.
  let spentUsd = 0;
  let dismissedFor = ''; // "warn"|"over" level the user dismissed
  let timer: ReturnType<typeof setInterval>;

  async function refresh() {
    if ($settings.dailyBudgetUsd <= 0) return;
    try {
      const data = await invoke<{ sessions: { totalCostUsd: number }[] }>('get_sessions_data', {
        timeRange: '24h',
        prometheusUrl: $settings.prometheusUrl,
      });
      spentUsd = data.sessions.reduce((sum, s) => sum + (s.totalCostUsd || 0), 0);
    } catch {
      /* Prometheus/history unavailable: keep last value */
    }
  }

  $: budget = $settings.dailyBudgetUsd;
  $: ratio = budget > 0 ? spentUsd / budget : 0;
  $: level = ratio >= 1 ? 'over' : ratio >= 0.8 ? 'warn' : '';
  $: if (level !== dismissedFor && level === '') dismissedFor = '';

  onMount(() => {
    refresh();
    timer = setInterval(refresh, 60_000);
  });
  onDestroy(() => timer && clearInterval(timer));
</script>

{#if budget > 0 && level && dismissedFor !== level}
  <div
    class="mb-3 flex items-center justify-between rounded-md border px-3 py-2 text-xs
      {level === 'over' ? 'bg-red/10 border-red/50 text-red' : 'bg-peach/10 border-peach/50 text-peach'}"
  >
    <span>
      {#if level === 'over'}
        {$t('budget.over', { spent: $formatCost(spentUsd), limit: $formatCost(budget) })}
      {:else}
        {$t('budget.warn', { pct: Math.round(ratio * 100), spent: $formatCost(spentUsd), limit: $formatCost(budget) })}
      {/if}
    </span>
    <button class="ml-3 underline" on:click={() => (dismissedFor = level)}>{$t('stack.dismiss')}</button>
  </div>
{/if}
