<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
  import { get } from 'svelte/store';
  import { settings } from '$lib/stores/settings';
  import { formatCost } from '$lib/format';
  import { t } from '$lib/i18n';
  import { setAlert, clearAlert } from '$lib/stores';

  interface CostSpike { isSpike: boolean; lastHourUsd: number; avgHourlyUsd: number }

  let spike: CostSpike | null = null;
  let dismissedDay = '';
  let timer: ReturnType<typeof setInterval>;

  const today = () => new Date().toDateString();

  async function refresh() {
    try {
      spike = await invoke<CostSpike>('get_cost_spike', { prometheusUrl: $settings.prometheusUrl });
      if (spike.isSpike) notifyOnce(spike);
    } catch {
      spike = null; // Prometheus down: no spike info
    }
  }

  async function notifyOnce(s: CostSpike) {
    const key = 'spike-notified';
    try {
      if (localStorage.getItem(key) === today()) return;
      let ok = await isPermissionGranted();
      if (!ok) ok = (await requestPermission()) === 'granted';
      if (!ok) return;
      sendNotification({
        title: 'Claude Code Monitor',
        body: get(t)('spike.msg', { last: get(formatCost)(s.lastHourUsd), avg: get(formatCost)(s.avgHourlyUsd) }),
      });
      localStorage.setItem(key, today());
    } catch {
      /* banner still shows */
    }
  }

  function dismiss() {
    dismissedDay = today();
    try { localStorage.setItem('spike-dismissed', dismissedDay); } catch { /* ignore */ }
  }

  $: if (spike?.isSpike && dismissedDay !== today()) {
    setAlert({
      id: 'spike',
      severity: 'warn',
      text: $t('spike.msg', { last: $formatCost(spike.lastHourUsd), avg: $formatCost(spike.avgHourlyUsd) }),
      onDismiss: dismiss,
    });
  } else {
    clearAlert('spike');
  }

  onMount(() => {
    try { dismissedDay = localStorage.getItem('spike-dismissed') ?? ''; } catch { /* ignore */ }
    refresh();
    timer = setInterval(refresh, 5 * 60_000);
  });
  onDestroy(() => timer && clearInterval(timer));
</script>
