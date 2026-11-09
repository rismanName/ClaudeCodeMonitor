<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
  import { get } from 'svelte/store';
  import { settings } from '$lib/stores/settings';
  import { formatCost } from '$lib/format';
  import { t } from '$lib/i18n';
  import { setAlert, clearAlert } from '$lib/stores';

  // Spend over a rolling 24h window (not calendar day), from the Sessions data source.
  let spentUsd = 0;
  let timer: ReturnType<typeof setInterval>;

  async function refresh() {
    if ($settings.dailyBudgetUsd <= 0) return;
    try {
      // Total from Prometheus (not summed from sessions, which only lists sessions present in history.jsonl).
      const m = await invoke<{ totalCostUsd: number }>('get_dashboard_metrics', {
        timeRange: '1d',
        prometheusUrl: $settings.prometheusUrl,
      });
      spentUsd = m.totalCostUsd || 0;
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
  $: message =
    level === 'over'
      ? $t('budget.over', { spent: $formatCost(spentUsd), limit: $formatCost(budget) })
      : level === 'warn'
        ? $t('budget.warn', { pct: Math.round(ratio * 100), spent: $formatCost(spentUsd), limit: $formatCost(budget) })
        : '';

  // Publish to the shared alert strip (AlertStrip renders it; dismissal is handled there).
  $: if (budget > 0 && level) setAlert({ id: 'budget', severity: level === 'over' ? 'critical' : 'warn', text: message });
  $: if (!(budget > 0 && level)) clearAlert('budget');

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
