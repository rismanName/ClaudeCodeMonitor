<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { settings, saveSettings } from '$lib/stores/settings';
  import { t } from '$lib/i18n';
  import { formatCost } from '$lib/format';
  import { lastWeeklyReset, nextWeeklyReset, formatClock, formatResetDay } from '$lib/usageWindow';

  interface UsageWindows {
    blockActive: boolean;
    blockStart: number;
    blockEnd: number;
    blockTokens: number;
    blockCostUsd: number;
    minutesRemaining: number;
    weekTokens: number;
    weekCostUsd: number;
    weekCacheSavedUsd: number;
  }

  let usage: UsageWindows | null = null;
  let now = new Date();
  let timer: ReturnType<typeof setInterval>;

  async function refresh() {
    now = new Date();
    try {
      // The weekly period starts at the user's weekly reset (not a rolling 7 days), like Claude Desktop.
      const weekStart = Math.floor(lastWeeklyReset($settings.weeklyResetDay, $settings.weeklyResetTime, now).getTime() / 1000);
      usage = await invoke<UsageWindows>('get_usage_windows', { pricingProvider: $settings.pricingProvider, weekStart });
    } catch (e) {
      console.error('get_usage_windows failed:', e);
    }
  }

  function fmtTokens(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return n.toString();
  }

  const colorFor = (p: number) => (p >= 90 ? 'bg-red' : p >= 70 ? 'bg-peach' : 'bg-green');

  $: sessionLimit = $settings.planLimitTokens;
  $: weekLimit = $settings.weeklyLimitTokens;
  $: sessionPct = usage && sessionLimit > 0 ? Math.min(100, (usage.blockTokens / sessionLimit) * 100) : 0;
  $: weekPct = usage && weekLimit > 0 ? Math.min(100, (usage.weekTokens / weekLimit) * 100) : 0;
  // Shown the way Claude Desktop does ("5% used"), plus what is left.
  $: sessionLeftTokens = usage ? Math.max(0, sessionLimit - usage.blockTokens) : 0;
  $: weekLeftTokens = usage ? Math.max(0, weekLimit - usage.weekTokens) : 0;
  $: sessionResets = usage?.blockActive ? $t('usage.resetsAt', { t: formatClock(new Date(usage.blockEnd * 1000)) }) : $t('usage.noSession');
  $: weekResets = $t('usage.resetsOn', { d: formatResetDay(nextWeeklyReset($settings.weeklyResetDay, $settings.weeklyResetTime, now), $settings.language) });

  // Calibration: derive the limits from the percentages Claude Desktop shows right now.
  let showCalibrate = false;
  let calSession = '';
  let calWeek = '';
  let calMsg = '';

  async function calibrate() {
    if (!usage) return;
    const s = Number(calSession);
    const w = Number(calWeek);
    const next = { ...$settings };
    let changed = false;
    if (s > 0 && usage.blockTokens > 0) { next.planLimitTokens = Math.round((usage.blockTokens * 100) / s); changed = true; }
    if (w > 0 && usage.weekTokens > 0) { next.weeklyLimitTokens = Math.round((usage.weekTokens * 100) / w); changed = true; }
    if (!changed) { calMsg = $t('usage.calibrate.none'); return; }
    await saveSettings(next);
    calMsg = $t('usage.calibrate.done');
    calSession = '';
    calWeek = '';
  }

  onMount(() => {
    refresh();
    timer = setInterval(refresh, 60_000);
  });
  onDestroy(() => timer && clearInterval(timer));
</script>

<div class="bg-bg-card rounded-xl p-4 mb-4">
  <div class="flex items-center justify-between mb-3">
    <span class="text-xs font-medium text-text-muted uppercase tracking-wider">{$t('usage.title')}</span>
    <button class="text-xs text-text-muted underline hover:text-text-primary" on:click={() => (showCalibrate = !showCalibrate)}>{$t('usage.calibrate')}</button>
  </div>

  {#if usage}
    <!-- Current session (5-hour block) -->
    <div class="mb-4">
      <div class="flex items-baseline justify-between">
        <span class="text-sm font-medium text-text-primary">{$t('usage.session')}</span>
        {#if sessionLimit > 0}<span class="text-sm text-text-secondary">{$t('usage.used', { pct: sessionPct.toFixed(0) })} <span class="text-text-muted">· {$t('usage.left', { pct: (100 - sessionPct).toFixed(0) })}</span></span>{/if}
      </div>
      {#if sessionLimit > 0}
        <div class="mt-1.5 h-2 rounded-full bg-bg-primary overflow-hidden"><div class="h-full {colorFor(sessionPct)} transition-all" style="width: {sessionPct}%"></div></div>
      {/if}
      <div class="mt-1 flex items-center justify-between text-xs text-text-muted">
        <span>{sessionResets}{#if usage.blockActive} · {$t('usage.remaining', { m: usage.minutesRemaining })}{/if}</span>
        <span>{fmtTokens(usage.blockTokens)}{#if sessionLimit > 0} / {fmtTokens(sessionLimit)} · {$t('usage.leftTokens', { n: fmtTokens(sessionLeftTokens) })}{/if} · {$formatCost(usage.blockCostUsd)}</span>
      </div>
      {#if sessionLimit <= 0}<p class="mt-1 text-xs text-text-muted">{$t('usage.setLimit')}</p>{/if}
    </div>

    <!-- This week (since the weekly reset) -->
    <div>
      <div class="flex items-baseline justify-between">
        <span class="text-sm font-medium text-text-primary">{$t('usage.thisWeek')}</span>
        {#if weekLimit > 0}<span class="text-sm text-text-secondary">{$t('usage.used', { pct: weekPct.toFixed(0) })} <span class="text-text-muted">· {$t('usage.left', { pct: (100 - weekPct).toFixed(0) })}</span></span>{/if}
      </div>
      {#if weekLimit > 0}
        <div class="mt-1.5 h-2 rounded-full bg-bg-primary overflow-hidden"><div class="h-full {colorFor(weekPct)} transition-all" style="width: {weekPct}%"></div></div>
      {/if}
      <div class="mt-1 flex items-center justify-between text-xs text-text-muted">
        <span>{weekResets}</span>
        <span>{fmtTokens(usage.weekTokens)}{#if weekLimit > 0} / {fmtTokens(weekLimit)} · {$t('usage.leftTokens', { n: fmtTokens(weekLeftTokens) })}{/if} · {$formatCost(usage.weekCostUsd)}</span>
      </div>
      {#if weekLimit <= 0}<p class="mt-1 text-xs text-text-muted">{$t('usage.setWeekLimit')}</p>{/if}
    </div>

    <div class="mt-3 text-xs text-text-muted">
      {$t('usage.cacheSaved')}: <span class="text-green">{$formatCost(usage.weekCacheSavedUsd)}</span>
      <span class="ml-2">· {$t('usage.estimate')}</span>
    </div>

    {#if showCalibrate}
      <div class="mt-3 rounded-lg border border-border-secondary p-3">
        <p class="text-xs text-text-muted mb-2">{$t('usage.calibrate.hint')}</p>
        <div class="grid grid-cols-2 gap-3 mb-2">
          <label class="text-xs text-text-secondary">{$t('usage.session')} (%)
            <input type="number" min="1" max="100" bind:value={calSession} class="mt-1 w-full bg-bg-primary border border-border-secondary rounded-md px-2 py-1.5 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue" />
          </label>
          <label class="text-xs text-text-secondary">{$t('usage.thisWeek')} (%)
            <input type="number" min="1" max="100" bind:value={calWeek} class="mt-1 w-full bg-bg-primary border border-border-secondary rounded-md px-2 py-1.5 text-text-primary focus:outline-none focus:ring-2 focus:ring-blue" />
          </label>
        </div>
        <div class="flex items-center gap-3">
          <button on:click={calibrate} class="px-3 py-1.5 bg-blue text-crust rounded-md text-xs font-medium">{$t('usage.apply')}</button>
          {#if calMsg}<span class="text-xs text-text-muted">{calMsg}</span>{/if}
        </div>
      </div>
    {/if}
  {/if}
</div>
