<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { get } from 'svelte/store';
  import { invoke } from '@tauri-apps/api/core';
  import { Chart, registerables } from 'chart.js';
  import { ViewHeader, MetricCard, TimeRangePicker, UsageWindowCard } from '$lib/components';
  import { settings } from '$lib/stores/settings';
  import { t, tx } from '$lib/i18n';
  import { formatCost as fmt } from '$lib/format';
  import { formatCostAxis } from '$lib/format';
  import { isConnected, lastUpdated, timeRange as timeRangeStore, customTimeRange as customTimeRangeStore, totalCost } from '$lib/stores';
  import type { DashboardMetrics, TimeRange, CustomTimeRange, PreviousPeriod } from '$lib/types';

  let metrics: DashboardMetrics | null = null;
  let loading = true;
  let error: string | null = null;

  let costChartCanvas: HTMLCanvasElement;
  let modelChartCanvas: HTMLCanvasElement;
  let costChart: Chart | null = null;
  let modelChart: Chart | null = null;

  // macOS dark theme colors (matching Swift version)
  const colors = {
    green: '#00ff88',     // Bright green for positive
    sky: '#00d9ff',       // Cyan (primary accent)
    mauve: '#a855f7',     // Purple
    peach: '#ffb347',     // Orange
    pink: '#ff6b9d',
    yellow: '#ffd93d',
    red: '#ff6b6b',       // Softer red
    blue: '#007aff',      // macOS blue
    teal: '#00d9ff',      // Teal/Cyan
    surface0: '#2c2c2e',  // Card background
    surface1: '#3a3a3c',  // Hover state
    overlay0: '#8e8e93',  // Muted text
    text: '#ffffff',
    subtext1: '#e5e5e7',
  };

  Chart.register(...registerables);

  // Track previous URL to detect settings changes
  let previousPrometheusUrl = '';

  // Re-fetch when Prometheus URL changes (after initial load)
  $: if ($settings.prometheusUrl && previousPrometheusUrl && $settings.prometheusUrl !== previousPrometheusUrl) {
    previousPrometheusUrl = $settings.prometheusUrl;
    fetchMetrics();
  }

  // ---- Period comparison + 30-day forecast -------------------------------------------------
  const PRESET_SECS: Record<string, number> = {
    '15m': 900, '1h': 3600, '4h': 4 * 3600, '1d': 86400, '7d': 7 * 86400, '30d': 30 * 86400, '90d': 90 * 86400,
  };

  function rangeSeconds(range: TimeRange, custom: CustomTimeRange | null): number {
    if (range === 'custom') return custom ? custom.end - custom.start : 0;
    return PRESET_SECS[range] ?? 0;
  }

  let previous: PreviousPeriod | null = null;

  async function fetchPrevious() {
    const secs = rangeSeconds($timeRangeStore, $customTimeRangeStore);
    if (secs <= 0) { previous = null; return; }
    try {
      previous = await invoke<PreviousPeriod>('get_previous_period', {
        rangeSecs: secs,
        prometheusUrl: $settings.prometheusUrl,
      });
    } catch {
      previous = null; // deltas are optional; hide them on failure
    }
  }

  /** % change, or null when the previous value is 0 (undefined growth). */
  function pctDelta(cur: number, prev: number | undefined): number | null {
    return prev && prev > 0 ? ((cur - prev) / prev) * 100 : null;
  }

  $: rangeSecs = rangeSeconds($timeRangeStore, $customTimeRangeStore);
  // Linear extrapolation of the selected range to 30 days; hidden for ranges shorter than 1 day.
  $: forecast30 = metrics && rangeSecs >= 86400 ? (metrics.totalCostUsd / rangeSecs) * 30 * 86400 : null;

  // ---- Export (CSV / JSON into ~/Downloads) ----
  let exportMsg = '';

  async function exportReport(kind: 'csv' | 'json') {
    if (!metrics) return;
    const m = metrics;
    const rows: [string, number | string][] = [
      ['range', $timeRangeStore],
      ['total_cost_usd', m.totalCostUsd],
      ['total_tokens', m.totalTokens],
      ['input_tokens', m.inputTokens],
      ['output_tokens', m.outputTokens],
      ['cache_read_tokens', m.cacheReadTokens],
      ['cache_creation_tokens', m.cacheCreationTokens],
      ['active_time_seconds', m.activeTimeSeconds],
      ['sessions', m.sessionCount],
      ['lines_added', m.linesAdded],
      ['lines_removed', m.linesRemoved],
      ['commits', m.commitCount],
      ['pull_requests', m.pullRequestCount],
    ];
    const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
    const content = kind === 'json'
      ? JSON.stringify({ exportedAt: new Date().toISOString(), ...Object.fromEntries(rows), tokensByModel: m.tokensByModel }, null, 2)
      : ['metric,value', ...rows.map(([k, v]) => `${k},${v}`),
         ...m.tokensByModel.map((x) => `tokens_model:${x.model.replace(/,/g, ' ')},${x.tokens}`)].join('\n') + '\n';
    try {
      const path = await invoke<string>('save_export', { fileName: `claude-code-usage-${stamp}.${kind}`, content });
      exportMsg = `${$t('export.saved')}: ${path}`;
    } catch (e) {
      exportMsg = String(e);
    }
    setTimeout(() => (exportMsg = ''), 8000);
  }

  async function fetchMetrics() {
    // Update tracked URL on each fetch
    previousPrometheusUrl = $settings.prometheusUrl;
    loading = true;
    error = null;
    try {
      const params: Record<string, unknown> = {
        timeRange: $timeRangeStore,
        prometheusUrl: $settings.prometheusUrl,
      };

      // Add custom range timestamps if using custom time range
      if ($timeRangeStore === 'custom' && $customTimeRangeStore) {
        params.customStart = $customTimeRangeStore.start;
        params.customEnd = $customTimeRangeStore.end;
      }

      metrics = await invoke<DashboardMetrics>('get_dashboard_metrics', params);
      totalCost.set(metrics.totalCostUsd);
      isConnected.set(true);
      lastUpdated.set(new Date());
      void fetchPrevious();

      // Update system tray with cost and connection status
      try {
        await invoke('update_tray_stats', {
          totalCost: metrics.totalCostUsd,
          isConnected: true,
        });
      } catch {
        // Silently ignore tray update errors (e.g., on Windows where title may not be supported)
      }

      await tick();
      updateCharts();
    } catch (e) {
      error = e as string;
      isConnected.set(false);
      // Update tray to show disconnected status
      try {
        await invoke('update_tray_stats', {
          totalCost: 0,
          isConnected: false,
        });
      } catch {
        // Silently ignore tray update errors
      }
    } finally {
      loading = false;
    }
  }

  function handleTimeRangeChange(value: TimeRange) {
    timeRangeStore.set(value);
    if (value !== 'custom') {
      // Clear custom range when switching to preset
      customTimeRangeStore.set(null);
    }
    fetchMetrics();
  }

  function handleCustomRangeChange(range: CustomTimeRange) {
    customTimeRangeStore.set(range);
  }

  $: formatCost = $fmt;

  // Efficiency ratios: null when the divisor is 0 (rendered as "-").
  $: cacheHitRate = metrics && metrics.cacheReadTokens + metrics.inputTokens > 0
    ? (metrics.cacheReadTokens / (metrics.cacheReadTokens + metrics.inputTokens)) * 100
    : null;
  $: costPerCommit = metrics && metrics.commitCount > 0 ? metrics.totalCostUsd / metrics.commitCount : null;
  $: costPerPr = metrics && metrics.pullRequestCount > 0 ? metrics.totalCostUsd / metrics.pullRequestCount : null;
  $: costPer100 = metrics && metrics.linesAdded > 0 ? (metrics.totalCostUsd / metrics.linesAdded) * 100 : null;

  function formatTokens(n: number): string {
    if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(2)}B`;
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return n.toString();
  }

  function formatTime(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);
    if (hours > 0) return `${hours}h ${minutes}m`;
    if (minutes > 0) return `${minutes}m ${secs}s`;
    return `${secs}s`;
  }

  function getTimeRangeLabel(range: TimeRange): string {
    if (range === 'custom' && $customTimeRangeStore) {
      const start = new Date($customTimeRangeStore.start * 1000);
      const end = new Date($customTimeRangeStore.end * 1000);
      const formatDate = (d: Date) => d.toLocaleDateString([], { month: 'short', day: 'numeric' });
      return `${formatDate(start)} - ${formatDate(end)}`;
    }
    const labels: Record<Exclude<TimeRange, 'custom'>, string> = {
      '15m': 'Past 15 minutes',
      '1h': 'Past hour',
      '4h': 'Past 4 hours',
      '1d': 'Past day',
      '7d': 'Past week',
      '30d': 'Past month',
      '90d': 'Past 3 months',
    };
    return labels[range as Exclude<TimeRange, 'custom'>] || 'Custom range';
  }

  function getCostByModel(model: { model: string; tokens: number }): number {
    if (!metrics || metrics.totalTokens === 0) return 0;
    const proportion = model.tokens / metrics.totalTokens;
    return metrics.totalCostUsd * proportion;
  }

  function formatModelName(modelName: string): string {
    // Extract friendly name like "Haiku 4.5" or "Opus 4.5" from full model name
    // Model names look like: "claude-3-5-haiku-20241022" or "claude-opus-4-5-20251101"
    const lower = modelName.toLowerCase();

    // Extract version like "3-5" or "4-5" and convert to "3.5" or "4.5"
    const versionMatch = lower.match(/(\d)-(\d)/);
    const version = versionMatch ? `${versionMatch[1]}.${versionMatch[2]}` : '';

    if (lower.includes('haiku')) {
      return version ? `Haiku ${version}` : 'Haiku';
    }
    if (lower.includes('opus')) {
      return version ? `Opus ${version}` : 'Opus';
    }
    if (lower.includes('sonnet')) {
      return version ? `Sonnet ${version}` : 'Sonnet';
    }
    // Fallback: return last part of model name
    return modelName.split('.').pop() || modelName;
  }

  // Consolidate models by friendly name
  function getConsolidatedModels(): { model: string; tokens: number }[] {
    if (!metrics) return [];

    const consolidated = new Map<string, number>();
    for (const m of metrics.tokensByModel) {
      const friendlyName = formatModelName(m.model);
      consolidated.set(friendlyName, (consolidated.get(friendlyName) || 0) + m.tokens);
    }

    return Array.from(consolidated.entries())
      .map(([model, tokens]) => ({ model, tokens }))
      .sort((a, b) => b.tokens - a.tokens);
  }

  function getCostByConsolidatedModel(model: { model: string; tokens: number }): number {
    if (!metrics || metrics.totalTokens === 0) return 0;
    const proportion = model.tokens / metrics.totalTokens;
    return metrics.totalCostUsd * proportion;
  }

  function updateCharts() {
    if (!metrics) return;

    // Cumulative cost over period chart
    if (costChartCanvas && metrics.tokensOverTime.length > 0) {
      costChart?.destroy();
      const ctx = costChartCanvas.getContext('2d');
      if (ctx) {
        const gradient = ctx.createLinearGradient(0, 0, 0, 128);
        gradient.addColorStop(0, 'rgba(0, 217, 255, 0.4)');   // Cyan accent
        gradient.addColorStop(1, 'rgba(0, 217, 255, 0)');

        // Backend returns rate per step via rate()
        // Compute cumulative sum then scale to match total (matching Swift app)
        let cumulative = 0;
        const rawCumulativeData = metrics.tokensOverTime.map(p => {
          cumulative += p.value;
          return cumulative;
        });

        // Scale cumulative values so final value matches totalCostUsd
        const rawTotal = rawCumulativeData[rawCumulativeData.length - 1] || 0;
        const targetTotal = metrics.totalCostUsd;
        const scaleFactor = (rawTotal > 0 && targetTotal > 0) ? targetTotal / rawTotal : 1;
        const cumulativeData = rawCumulativeData.map(v => v * scaleFactor);

        costChart = new Chart(costChartCanvas, {
          type: 'line',
          data: {
            labels: metrics.tokensOverTime.map(p => {
              const date = new Date(p.timestamp * 1000);
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
            }),
            datasets: [{
              data: cumulativeData,
              borderColor: colors.sky,
              backgroundColor: gradient,
              fill: true,
              tension: 0,  // Angular lines like Swift version
              pointRadius: 0,
              borderWidth: 2,
            }],
          },
          options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: { legend: { display: false } },
            scales: {
              x: {
                grid: {
                  color: 'rgba(142, 142, 147, 0.2)',  // Semi-transparent grid lines
                  drawTicks: true,
                },
                ticks: {
                  color: colors.overlay0,
                  maxTicksLimit: 6,
                  font: { size: 10 },
                },
              },
              y: {
                grid: {
                  color: 'rgba(142, 142, 147, 0.2)',  // Semi-transparent grid lines
                },
                ticks: {
                  color: colors.overlay0,
                  callback: (v) => get(formatCostAxis)(Number(v)),
                  font: { size: 10 },
                  maxTicksLimit: 5,
                },
                position: 'right',
                beginAtZero: true,
              },
            },
          },
        });
      }
    }

    // Model breakdown chart (using consolidated models)
    const consolidatedModels = getConsolidatedModels();
    if (modelChartCanvas && consolidatedModels.length > 0) {
      modelChart?.destroy();
      const chartColors = [colors.mauve, colors.sky, colors.green, colors.peach, colors.pink];
      const costData = consolidatedModels.map(m => getCostByConsolidatedModel(m));

      modelChart = new Chart(modelChartCanvas, {
        type: 'doughnut',
        data: {
          labels: consolidatedModels.map(m => m.model),
          datasets: [{
            data: costData,
            backgroundColor: chartColors.slice(0, consolidatedModels.length),
            borderWidth: 0,
          }],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          cutout: '70%',
          plugins: { legend: { display: false } },
        },
      });
    }
  }

  onMount(() => {
    fetchMetrics();
    const interval = setInterval(fetchMetrics, $settings.refreshInterval * 1000);
    return () => {
      clearInterval(interval);
      costChart?.destroy();
      modelChart?.destroy();
    };
  });

  onDestroy(() => {
    costChart?.destroy();
    modelChart?.destroy();
  });

  const chartColors = [colors.mauve, colors.sky, colors.green, colors.peach, colors.pink];
</script>

<div class="flex flex-col h-full">
  <!-- Header Row: Title (left) + Time Range (right) -->
  <div class="flex items-center justify-between mb-4">
    <!-- Left: Dashboard title -->
    <div class="flex-shrink-0">
      <h1 class="text-lg font-semibold text-text-primary">{$t('dash.title')}</h1>
      <p class="text-xs text-text-muted">{$tx(getTimeRangeLabel($timeRangeStore))}</p>
    </div>

    <!-- Right: Export + time range picker -->
    <div class="flex-shrink-0 flex items-center gap-2">
      {#if metrics}
        <button on:click={() => exportReport('csv')} class="px-2 py-1 text-xs rounded-md bg-bg-card border border-border-secondary text-text-secondary hover:bg-bg-card-hover">CSV</button>
        <button on:click={() => exportReport('json')} class="px-2 py-1 text-xs rounded-md bg-bg-card border border-border-secondary text-text-secondary hover:bg-bg-card-hover">JSON</button>
      {/if}
      <TimeRangePicker
        value={$timeRangeStore}
        onChange={handleTimeRangeChange}
        customRange={$customTimeRangeStore}
        onCustomRangeChange={handleCustomRangeChange}
      />
    </div>
  </div>

  {#if exportMsg}
    <div class="mb-2 text-xs text-text-secondary break-all">{exportMsg}</div>
  {/if}

  {#if loading && !metrics}
    <div class="flex items-center justify-center h-32">
      <div class="text-xs text-text-muted">Loading metrics...</div>
    </div>
  {:else if error}
    <div class="bg-red/10 border border-red/50 rounded-md p-3 mb-4">
      <p class="text-xs text-red">{error}</p>
    </div>
    <p class="text-xs text-text-muted mb-3">{$t('offline.note')}</p>
    <UsageWindowCard />
  {:else if metrics}
    <!-- Key Metrics Section -->
    <div class="mb-4">
      <div class="flex items-center justify-between mb-2">
        <span class="text-xs font-medium text-text-muted uppercase tracking-wider">{$t('dash.keyMetrics')}</span>
        <span class="text-xs text-text-muted">Double-click to copy</span>
      </div>
      <!-- First row: Total Spend, Active Time, Total Tokens -->
      <div class="grid grid-cols-3 gap-2 mb-2">
        <MetricCard label={$t('dash.totalSpend')} value={formatCost(metrics.totalCostUsd)} subtitle={$tx(getTimeRangeLabel($timeRangeStore))} color="green" highlight={true} delta={pctDelta(metrics.totalCostUsd, previous?.totalCostUsd)} lowerIsBetter={true} />
        <MetricCard label={$t('dash.activeTime')} value={formatTime(metrics.activeTimeSeconds)} subtitle={$t('dash.activeTime.sub')} color="cyan" highlight={true} delta={pctDelta(metrics.activeTimeSeconds, previous?.activeTimeSeconds)} />
        <MetricCard label={$t('dash.totalTokens')} value={formatTokens(metrics.totalTokens)} subtitle={$t('dash.totalTokens.sub')} color="purple" highlight={true} delta={pctDelta(metrics.totalTokens, previous?.totalTokens)} />
      </div>
      {#if forecast30 !== null}
        <div class="text-xs text-text-muted mb-2">{$t('dash.forecast')}: <span class="text-text-secondary">{formatCost(forecast30)}</span></div>
      {/if}
      <!-- Second row: Lines Added, Lines Removed, Commits, PRs -->
      <div class="grid grid-cols-4 gap-2">
        <MetricCard label={$t('dash.linesAdded')} value={`+${metrics.linesAdded.toLocaleString()}`} subtitle={$t('dash.linesAdded.sub')} color="green" />
        <MetricCard label={$t('dash.linesRemoved')} value={`-${metrics.linesRemoved.toLocaleString()}`} subtitle={$t('dash.linesRemoved.sub')} color="red" />
        <MetricCard label={$t('dash.commits')} value={metrics.commitCount.toString()} subtitle={$t('dash.commits.sub')} color="purple" />
        <MetricCard label={$t('dash.prs')} value={metrics.pullRequestCount?.toString() || '0'} subtitle={$t('dash.prs.sub')} color="orange" />
      </div>
    </div>

    <!-- 5-hour usage window (local transcripts) -->
    <UsageWindowCard />

    <!-- Efficiency -->
    <div class="mb-4">
      <div class="mb-2">
        <span class="text-xs font-medium text-text-muted uppercase tracking-wider">{$t('eff.title')}</span>
      </div>
      <div class="grid grid-cols-4 gap-2">
        <MetricCard label={$t('eff.cacheHit')} value={cacheHitRate === null ? '-' : `${cacheHitRate.toFixed(1)}%`} color="cyan" />
        <MetricCard label={$t('eff.perCommit')} value={costPerCommit === null ? '-' : formatCost(costPerCommit)} color="purple" />
        <MetricCard label={$t('eff.perPr')} value={costPerPr === null ? '-' : formatCost(costPerPr)} color="orange" />
        <MetricCard label={$t('eff.per100')} value={costPer100 === null ? '-' : formatCost(costPer100)} color="green" />
      </div>
    </div>

    <!-- Total Cost Over Period Chart -->
    <div class="bg-bg-card rounded-md p-4 mb-4">
      <div class="flex items-center justify-between mb-1">
        <span class="text-xs font-medium text-text-muted uppercase tracking-wider">{$t('dash.costOverTime')}</span>
        <span class="text-xs text-text-muted">{$tx(getTimeRangeLabel($timeRangeStore))}</span>
      </div>
      <div class="h-40">
        <canvas bind:this={costChartCanvas}></canvas>
      </div>
    </div>

    <!-- Cost By Model Section -->
    <div class="bg-bg-card rounded-md p-4">
      <div class="mb-3 text-center">
        <span class="text-xs font-medium text-text-muted uppercase tracking-wider">{$t('dash.costByModel')}</span>
        <span class="text-xs text-text-muted ml-2">breakdown</span>
      </div>
      <div class="flex items-center justify-center gap-8">
        <!-- Larger donut chart (25% bigger: 128px -> 160px) -->
        <div class="w-40 h-40 flex-shrink-0">
          <canvas bind:this={modelChartCanvas}></canvas>
        </div>
        <div class="space-y-3">
          {#each getConsolidatedModels().slice(0, 5) as model, i}
            {@const modelCost = getCostByConsolidatedModel(model)}
            <div class="flex items-center justify-between gap-4">
              <div class="flex items-center gap-2">
                <div class="w-2.5 h-2.5 rounded-full flex-shrink-0" style="background-color: {chartColors[i % chartColors.length]}"></div>
                <span class="text-lg font-bold text-text-secondary min-w-[120px]">{model.model}</span>
              </div>
              <span class="text-lg font-bold text-text-primary flex-shrink-0">{formatCost(modelCost)}</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>
