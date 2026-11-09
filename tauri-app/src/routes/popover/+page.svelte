<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { loadSettings, settings } from '$lib/stores/settings';
  import { formatCost, formatModelName } from '$lib/format';
  import { t } from '$lib/i18n';
  import type { DashboardMetrics } from '$lib/types';

  interface UsageWindows {
    blockActive: boolean;
    blockTokens: number;
    blockCostUsd: number;
    minutesRemaining: number;
    weekTokens: number;
    weekCostUsd: number;
    weekCacheSavedUsd: number;
  }
  interface SetupStatus { prometheusUp: boolean; otlpUp: boolean }

  let m: DashboardMetrics | null = null;
  let usage: UsageWindows | null = null;
  let stack: SetupStatus | null = null;
  let restarting = false;
  let timer: ReturnType<typeof setInterval>;

  // Pinned = desktop-widget mode (stays open on top). Kept in localStorage, which both windows share.
  const PIN_KEY = 'widget-pinned';
  let pinned = false;

  async function setPinned(v: boolean) {
    pinned = v;
    try { localStorage.setItem(PIN_KEY, v ? '1' : '0'); } catch { /* not persisted */ }
    try { await invoke('set_popover_pinned', { pinned: v }); } catch { /* backend unavailable */ }
  }

  async function refresh() {
    await loadSettings(); // pick up budget / currency changes made in the main window
    try {
      // Totals come from Prometheus (not summed from sessions, which only lists sessions present in history.jsonl).
      m = await invoke<DashboardMetrics>('get_dashboard_metrics', { timeRange: '1d', prometheusUrl: $settings.prometheusUrl });
    } catch { /* keep last value */ }
    try { usage = await invoke<UsageWindows>('get_usage_windows', { pricingProvider: $settings.pricingProvider }); } catch { usage = null; }
    try { stack = await invoke<SetupStatus>('get_setup_status'); } catch { stack = null; }
  }

  async function restartStack() {
    restarting = true;
    try { await invoke('restart_monitoring_stack', { composeDir: $settings.composeDir || null }); } catch { /* status row shows result */ }
    restarting = false;
    refresh();
  }

  const openDashboard = () => invoke('open_dashboard');
  const quit = () => invoke('quit_app');

  function fmtTokens(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return Math.round(n).toString();
  }

  function fmtDuration(sec: number): string {
    const h = Math.floor(sec / 3600);
    const min = Math.floor((sec % 3600) / 60);
    return h > 0 ? `${h}j ${min}m` : `${min}m`;
  }

  $: spentUsd = m?.totalCostUsd ?? 0;
  $: budget = $settings.dailyBudgetUsd;
  $: pct = budget > 0 ? Math.min(100, (spentUsd / budget) * 100) : 0;
  $: barColor = pct >= 100 ? 'bg-red' : pct >= 80 ? 'bg-peach' : 'bg-green';
  $: limit = $settings.planLimitTokens;
  $: blockPct = usage && limit > 0 ? Math.min(100, (usage.blockTokens / limit) * 100) : 0;
  $: stackOk = !!stack && stack.prometheusUp && stack.otlpUp;

  // Token split (input / output / cache read / cache written) as a stacked bar.
  $: parts = m
    ? [
        { key: 'pop.input', value: m.inputTokens, color: 'bg-sky' },
        { key: 'pop.output', value: m.outputTokens, color: 'bg-mauve' },
        { key: 'pop.cacheRead', value: m.cacheReadTokens, color: 'bg-green' },
        { key: 'pop.cacheWrite', value: m.cacheCreationTokens, color: 'bg-peach' },
      ]
    : [];
  $: partsTotal = parts.reduce((s, p) => s + p.value, 0);
  $: cacheHit = m && m.cacheReadTokens + m.inputTokens > 0 ? (m.cacheReadTokens / (m.cacheReadTokens + m.inputTokens)) * 100 : null;

  // Models consolidated by friendly name, top 3.
  $: models = (() => {
    if (!m) return [];
    const map = new Map<string, number>();
    for (const x of m.tokensByModel) map.set(formatModelName(x.model), (map.get(formatModelName(x.model)) ?? 0) + x.tokens);
    const total = [...map.values()].reduce((a, b) => a + b, 0) || 1;
    return [...map.entries()].map(([name, tokens]) => ({ name, tokens, share: (tokens / total) * 100 })).sort((a, b) => b.tokens - a.tokens).slice(0, 3);
  })();

  // Cumulative tokens over the last 24h as an inline SVG area (no chart library needed for a widget).
  const W = 300, H = 40;
  $: spark = (() => {
    const pts = (m?.tokensOverTime ?? []).slice().sort((a, b) => a.timestamp - b.timestamp);
    if (pts.length < 2) return null;
    const t0 = pts[0].timestamp, t1 = pts[pts.length - 1].timestamp;
    // The backend returns a rate per step; the running sum has the shape of tokens used so far (same as the dashboard chart).
    let run = 0;
    const cum = pts.map((p) => (run += p.value));
    const vmax = run > 0 ? run : 1; // values are tiny per-second rates: never floor the divisor at 1
    const xy = pts.map((p, i) => `${(((p.timestamp - t0) / (t1 - t0 || 1)) * W).toFixed(1)},${(H - (cum[i] / vmax) * (H - 3) - 1).toFixed(1)}`);
    return { line: `M${xy.join(' L')}`, area: `M0,${H} L${xy.join(' L')} L${W},${H} Z` };
  })();

  onMount(() => {
    try { pinned = localStorage.getItem(PIN_KEY) === '1'; } catch { /* default unpinned */ }
    // Restore the widget on launch (also after login). Wait a moment so the tray icon has a screen position.
    if (pinned) setTimeout(() => invoke('set_popover_pinned', { pinned: true }).catch(() => {}), 1500);
    refresh();
    timer = setInterval(refresh, 30_000);
    window.addEventListener('focus', refresh); // hidden window keeps its state; refresh each time it is shown
  });
  onDestroy(() => {
    if (timer) clearInterval(timer);
    window.removeEventListener('focus', refresh);
  });
</script>

<div class="h-screen flex flex-col bg-bg-primary select-none">
  <!-- Header: drag to move (useful when pinned) + pin toggle -->
  <div class="flex items-center justify-between px-4 pt-3 pb-1" data-tauri-drag-region>
    <span class="text-xs font-medium text-text-muted pointer-events-none">Claude Monitor</span>
    <button on:click={() => setPinned(!pinned)} title={pinned ? $t('pop.unpin') : $t('pop.pin')} aria-pressed={pinned}
      class="p-1 rounded-md transition-colors {pinned ? 'text-accent-primary bg-accent-primary/15' : 'text-text-muted hover:text-text-primary hover:bg-bg-card'}">
      <svg class="w-4 h-4" viewBox="0 0 24 24" fill={pinned ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M9 3h6l-1 6 3 3v2H7v-2l3-3-1-6zM12 14v7" />
      </svg>
    </button>
  </div>

  <div class="flex-1 overflow-y-auto px-4 pb-2 space-y-3">
    <!-- Spend -->
    <div>
      <div class="text-[11px] font-medium text-text-muted uppercase tracking-wider">{$t('budget.title')}</div>
      <div class="flex items-baseline gap-2 mt-0.5">
        <div class="text-3xl font-semibold tracking-tight text-text-primary">{$formatCost(spentUsd)}</div>
        {#if budget > 0}<div class="text-xs text-text-muted">/ {$formatCost(budget)}</div>{/if}
      </div>
      {#if budget > 0}
        <div class="mt-1.5 h-1.5 rounded-full bg-bg-card overflow-hidden"><div class="h-full {barColor}" style="width: {pct}%"></div></div>
      {:else}
        <div class="mt-1 text-xs text-text-muted">{$t('budget.notSet')}</div>
      {/if}
      {#if usage}
        <div class="mt-1.5 text-[11px] text-text-muted">
          {$t('usage.week')}: <span class="text-text-secondary">{fmtTokens(usage.weekTokens)} · {$formatCost(usage.weekCostUsd)}</span>
        </div>
      {/if}
    </div>

    <!-- Tokens -->
    <div class="bg-bg-card rounded-xl px-3 py-2.5">
      <div class="flex items-baseline justify-between">
        <span class="text-[11px] font-medium text-text-muted uppercase tracking-wider">{$t('pop.tokens')}</span>
        {#if cacheHit !== null}<span class="text-[11px] text-text-muted">{$t('eff.cacheHit')} <span class="text-green font-medium">{cacheHit.toFixed(0)}%</span></span>{/if}
      </div>
      <div class="text-xl font-semibold text-text-primary mt-0.5">{fmtTokens(m?.totalTokens ?? 0)}</div>

      {#if partsTotal > 0}
        <div class="mt-2 flex h-1.5 rounded-full overflow-hidden bg-bg-primary">
          {#each parts as p}<div class="{p.color}" style="width: {(p.value / partsTotal) * 100}%"></div>{/each}
        </div>
        <div class="mt-2 grid grid-cols-2 gap-x-3 gap-y-1">
          {#each parts as p}
            <div class="flex items-center justify-between text-[11px]">
              <span class="flex items-center gap-1.5 text-text-muted"><span class="w-1.5 h-1.5 rounded-full {p.color}"></span>{$t(p.key as 'pop.input')}</span>
              <span class="text-text-secondary">{fmtTokens(p.value)}</span>
            </div>
          {/each}
        </div>
      {/if}

      {#if spark}
        <svg class="mt-2 w-full h-10" viewBox="0 0 {W} {H}" preserveAspectRatio="none" aria-hidden="true">
          <path d={spark.area} fill="rgba(124, 156, 255, 0.15)" />
          <path d={spark.line} fill="none" stroke="#7c9cff" stroke-width="1.5" vector-effect="non-scaling-stroke" />
        </svg>
      {/if}
    </div>

    <!-- Activity -->
    <div class="grid grid-cols-3 gap-2">
      {#each [
        { label: $t('dash.activeTime'), value: fmtDuration(m?.activeTimeSeconds ?? 0) },
        { label: $t('dash.commits'), value: String(m?.commitCount ?? 0) },
        { label: 'PR', value: String(m?.pullRequestCount ?? 0) },
      ] as s}
        <div class="bg-bg-card rounded-lg px-2 py-1.5 text-center">
          <div class="text-sm font-semibold text-text-primary">{s.value}</div>
          <div class="text-[10px] text-text-muted truncate">{s.label}</div>
        </div>
      {/each}
    </div>
    <div class="flex items-center justify-between text-[11px] text-text-muted px-1">
      <span>{$t('pop.lines')}</span>
      <span><span class="text-green">+{(m?.linesAdded ?? 0).toLocaleString()}</span> / <span class="text-red">−{(m?.linesRemoved ?? 0).toLocaleString()}</span></span>
    </div>

    <!-- Models -->
    {#if models.length > 0}
      <div class="bg-bg-card rounded-xl px-3 py-2.5">
        <div class="text-[11px] font-medium text-text-muted uppercase tracking-wider mb-1.5">{$t('pop.models')}</div>
        <div class="space-y-1.5">
          {#each models as mo}
            <div>
              <div class="flex items-center justify-between text-xs">
                <span class="text-text-primary">{mo.name}</span>
                <span class="text-text-muted">{fmtTokens(mo.tokens)} · {mo.share.toFixed(0)}%</span>
              </div>
              <div class="mt-0.5 h-1 rounded-full bg-bg-primary overflow-hidden"><div class="h-full bg-mauve" style="width: {mo.share}%"></div></div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- 5-hour window -->
    <div class="bg-bg-card rounded-xl px-3 py-2.5">
      <div class="text-[11px] font-medium text-text-muted uppercase tracking-wider">{$t('usage.title')}</div>
      {#if usage?.blockActive}
        <div class="mt-1 flex items-baseline justify-between">
          <span class="text-sm font-semibold text-text-primary">{fmtTokens(usage.blockTokens)} <span class="text-xs font-normal text-text-muted">· {$formatCost(usage.blockCostUsd)}</span></span>
          <span class="text-xs text-text-muted">{$t('usage.remaining', { m: usage.minutesRemaining })}</span>
        </div>
        {#if limit > 0}
          <div class="mt-1.5 h-1 rounded-full bg-bg-primary overflow-hidden"><div class="h-full {blockPct >= 90 ? 'bg-red' : blockPct >= 70 ? 'bg-peach' : 'bg-green'}" style="width: {blockPct}%"></div></div>
        {/if}
      {:else}
        <div class="mt-1 text-xs text-text-muted">{$t('usage.noBlock')}</div>
      {/if}
    </div>

    <!-- Stack status -->
    <div class="bg-bg-card rounded-xl px-3 py-2 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <span class="w-1.5 h-1.5 rounded-full {stackOk ? 'bg-green' : 'bg-red'}"></span>
        <span class="text-xs text-text-primary">{stackOk ? $t('pop.stackUp') : $t('pop.stackDown')}</span>
      </div>
      <button on:click={restartStack} disabled={restarting}
        class="text-xs px-2 py-1 rounded-md border border-border-secondary text-text-secondary hover:bg-bg-card-hover disabled:opacity-50">
        {restarting ? '…' : $t('pop.restartStack')}
      </button>
    </div>
  </div>

  <div class="flex gap-2 px-4 pb-4 pt-2">
    <button on:click={openDashboard} class="flex-1 px-3 py-2 rounded-lg bg-accent-primary text-crust text-sm font-medium hover:opacity-90">{$t('pop.openDashboard')}</button>
    <button on:click={quit} class="px-3 py-2 rounded-lg border border-border-secondary text-text-secondary text-sm hover:bg-bg-card-hover">{$t('pop.quit')}</button>
  </div>
</div>
