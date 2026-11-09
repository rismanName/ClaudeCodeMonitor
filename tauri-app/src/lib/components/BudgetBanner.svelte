<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
  import { get } from 'svelte/store';
  import { settings } from '$lib/stores/settings';
  import { formatCost } from '$lib/format';
  import { t } from '$lib/i18n';

  // Spend over a rolling 24h window (not calendar day), from the Sessions data source.
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

  /** Send one OS notification per level per calendar day (remembered in localStorage). */
  async function notifyOnce(lvl: 'warn' | 'over', body: string) {
    const key = `budget-notified-${lvl}`;
    const today = new Date().toDateString();
    try {
      if (localStorage.getItem(key) === today) return;
      let ok = await isPermissionGranted();
      if (!ok) ok = (await requestPermission()) === 'granted';
      if (!ok) return;
      sendNotification({ title: 'Claude Code Monitor', body });
      localStorage.setItem(key, today);
    } catch {
      /* notifications unavailable: banner still shows */
    }
  }

  $: budget = $settings.dailyBudgetUsd;
  $: ratio = budget > 0 ? spentUsd / budget : 0;
  $: level = (budget > 0 && ratio >= 1 ? 'over' : budget > 0 && ratio >= 0.8 ? 'warn' : '') as '' | 'warn' | 'over';
  $: if (level === '') dismissedFor = '';

  $: message =
    level === 'over'
      ? $t('budget.over', { spent: $formatCost(spentUsd), limit: $formatCost(budget) })
      : level === 'warn'
        ? $t('budget.warn', { pct: Math.round(ratio * 100), spent: $formatCost(spentUsd), limit: $formatCost(budget) })
        : '';

  // Tray marker + notification react to the level; message is computed with the current language.
  $: invoke('set_tray_alert', { level: level === 'over' ? 2 : level === 'warn' ? 1 : 0 }).catch(() => {});
  $: if (level) notifyOnce(level, get(t)(level === 'over' ? 'budget.over' : 'budget.warn', {
    pct: Math.round(ratio * 100),
    spent: get(formatCost)(spentUsd),
    limit: get(formatCost)(budget),
  }));

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
    <span>{message}</span>
    <button class="ml-3 underline" on:click={() => (dismissedFor = level)}>{$t('stack.dismiss')}</button>
  </div>
{/if}
