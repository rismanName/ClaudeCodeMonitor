<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { settings } from '$lib/stores/settings';
  import { t } from '$lib/i18n';

  interface UsageWindows {
    blockActive: boolean;
    blockStart: number;
    blockEnd: number;
    blockTokens: number;
    minutesRemaining: number;
    weekTokens: number;
  }

  let usage: UsageWindows | null = null;
  let timer: ReturnType<typeof setInterval>;

  async function refresh() {
    try {
      usage = await invoke<UsageWindows>('get_usage_windows');
    } catch (e) {
      console.error('get_usage_windows failed:', e);
    }
  }

  function fmtTokens(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return n.toString();
  }

  $: limit = $settings.planLimitTokens;
  $: pct = usage && limit > 0 ? Math.min(100, (usage.blockTokens / limit) * 100) : 0;
  $: barColor = pct >= 90 ? 'bg-red' : pct >= 70 ? 'bg-peach' : 'bg-green';

  onMount(() => {
    refresh();
    timer = setInterval(refresh, 60_000);
  });
  onDestroy(() => timer && clearInterval(timer));
</script>

<div class="bg-bg-card rounded-md p-4 mb-4">
  <div class="flex items-center justify-between mb-2">
    <span class="text-xs font-medium text-text-muted uppercase tracking-wider">{$t('usage.title')}</span>
    {#if usage?.blockActive}
      <span class="text-xs text-text-muted">{$t('usage.remaining', { m: usage.minutesRemaining })}</span>
    {:else}
      <span class="text-xs text-text-muted">{$t('usage.noBlock')}</span>
    {/if}
  </div>

  {#if usage}
    <div class="flex items-baseline justify-between mb-1">
      <span class="text-lg font-bold text-text-primary">{fmtTokens(usage.blockTokens)}</span>
      {#if limit > 0}
        <span class="text-xs text-text-muted">/ {fmtTokens(limit)} ({pct.toFixed(0)}%)</span>
      {/if}
    </div>
    {#if limit > 0}
      <div class="h-2 rounded-full bg-bg-primary overflow-hidden">
        <div class="h-full {barColor} transition-all" style="width: {pct}%"></div>
      </div>
    {:else}
      <p class="text-xs text-text-muted">{$t('usage.setLimit')}</p>
    {/if}
    <div class="mt-2 text-xs text-text-muted">
      {$t('usage.week')}: <span class="text-text-secondary">{fmtTokens(usage.weekTokens)}</span>
    </div>
  {/if}
</div>
