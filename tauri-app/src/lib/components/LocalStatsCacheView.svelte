<script lang="ts">
  import { tx } from '$lib/i18n';
  import { formatCost as fmt } from '$lib/format';
  import { onMount, onDestroy, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Chart, registerables } from 'chart.js';
  import { ViewHeader, MetricCard, ActivityHeatmap } from '$lib/components';
  import { settings } from '$lib/stores/settings';
  import { sessionDateFilter, requestedView } from '$lib/stores';
  import type { LocalStatsCacheData } from '$lib/types';

  let data: LocalStatsCacheData | null = null;
  let loading = true;
  let error: string | null = null;

  let dailyChartCanvas: HTMLCanvasElement;
  let modelChartCanvas: HTMLCanvasElement;
  let hourlyChartCanvas: HTMLCanvasElement;
  let charts: Chart[] = [];

  // macOS dark theme colors (matching Swift version)
  const colors = {
    green: '#34d399',     // Bright green for positive
    sky: '#7c9cff',       // Cyan (primary accent)
    mauve: '#a78bfa',     // Purple
    peach: '#fb923c',     // Orange
    pink: '#f472b6',
    yellow: '#fbbf24',
    red: '#f87171',       // Softer red
    blue: '#5b8def',      // macOS blue
    teal: '#7c9cff',      // Teal/Cyan
    surface0: '#18181b',  // Card background
    surface1: '#222227',  // Hover state
    overlay0: '#8b8b94',  // Muted text
  };

  Chart.register(...registerables);

  async function fetchData() {
    loading = true;
    error = null;
    try {
      data = await invoke<LocalStatsCacheData>('get_local_stats_cache', {
        pricingProvider: $settings.pricingProvider,
      });
      await tick();
      updateCharts();
    } catch (e) {
      error = e as string;
    } finally {
      loading = false;
    }
  }

  function formatTokens(n: number): string {
    if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(2)}B`;
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return n.toString();
  }

  $: formatCost = $fmt;

  function formatHour(hour: number | null): string {
    if (hour === null) return '—';
    const suffix = hour >= 12 ? 'PM' : 'AM';
    const h = hour % 12 || 12;
    return `${h} ${suffix}`;
  }

  function formatDate(iso: string | null): string {
    if (!iso) return '—';
    const date = new Date(iso);
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  }

  function formatModelName(model: string): string {
    // Extract friendly model name from full identifier
    // e.g., "claude-opus-4-5-20251101" -> "Opus 4.5"
    // e.g., "claude-3-5-haiku-20241022" -> "Haiku 3.5"
    // e.g., "claude-sonnet-4-5-20250929" -> "Sonnet 4.5"
    const lower = model.toLowerCase();

    if (lower.includes('opus')) {
      const match = lower.match(/opus[- ]?(\d)[- ]?(\d)?/);
      if (match) {
        return match[2] ? `Opus ${match[1]}.${match[2]}` : `Opus ${match[1]}`;
      }
      return 'Opus';
    }
    if (lower.includes('sonnet')) {
      const match = lower.match(/sonnet[- ]?(\d)[- ]?(\d)?/);
      if (match) {
        return match[2] ? `Sonnet ${match[1]}.${match[2]}` : `Sonnet ${match[1]}`;
      }
      return 'Sonnet';
    }
    if (lower.includes('haiku')) {
      const match = lower.match(/(\d)[- ](\d)[- ]haiku/) || lower.match(/haiku[- ]?(\d)[- ]?(\d)?/);
      if (match) {
        return match[2] ? `Haiku ${match[1]}.${match[2]}` : `Haiku ${match[1]}`;
      }
      return 'Haiku';
    }
    // Fallback: return the original model name
    return model;
  }

  function updateCharts() {
    if (!data) return;
    charts.forEach(c => c.destroy());
    charts = [];

    // Daily activity bar chart
    if (dailyChartCanvas && data.dailyActivity.length > 0) {
      const ctx = dailyChartCanvas.getContext('2d');
      if (ctx) {
        const chart = new Chart(dailyChartCanvas, {
          type: 'bar',
          data: {
            labels: data.dailyActivity.slice(-30).map(d => d.date.slice(5)),
            datasets: [{
              data: data.dailyActivity.slice(-30).map(d => d.value),
              backgroundColor: colors.sky,
              borderRadius: 2,
            }],
          },
          options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: { legend: { display: false } },
            scales: {
              x: { grid: { display: false }, ticks: { color: colors.overlay0, maxTicksLimit: 10 } },
              y: {
                grid: {
                  color: 'rgba(142, 142, 147, 0.3)',
                  drawTicks: true,
                },
                ticks: { color: colors.overlay0 },
              },
            },
          },
        });
        charts.push(chart);
      }
    }

    // Model breakdown doughnut
    if (modelChartCanvas && data.tokensByModel.length > 0) {
      const chartColors = [colors.mauve, colors.sky, colors.green, colors.peach, colors.pink, colors.blue];
      const chart = new Chart(modelChartCanvas, {
        type: 'doughnut',
        data: {
          labels: data.tokensByModel.map(m => m.model),
          datasets: [{
            data: data.tokensByModel.map(m => m.tokens),
            backgroundColor: chartColors.slice(0, data.tokensByModel.length),
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
      charts.push(chart);
    }

    // Hourly activity bar chart
    if (hourlyChartCanvas && data.activityByHour.length > 0) {
      const chart = new Chart(hourlyChartCanvas, {
        type: 'bar',
        data: {
          labels: data.activityByHour.map(h => `${h.hour}:00`),
          datasets: [{
            data: data.activityByHour.map(h => h.count),
            backgroundColor: colors.mauve,
            borderRadius: 2,
          }],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: { legend: { display: false } },
          scales: {
            x: { grid: { display: false }, ticks: { color: colors.overlay0, maxTicksLimit: 12 } },
            y: {
              grid: {
                color: 'rgba(142, 142, 147, 0.3)',
                drawTicks: true,
              },
              ticks: { color: colors.overlay0 },
            },
          },
        },
      });
      charts.push(chart);
    }
  }

  const chartColors = [colors.mauve, colors.sky, colors.green, colors.peach, colors.pink, colors.blue];

  onMount(fetchData);
  onDestroy(() => charts.forEach(c => c.destroy()));
</script>

<div>
  <ViewHeader category="stats-cache" title={$tx("Stats Cache")} subtitle="From ~/.claude/stats-cache.json" />

  {#if loading && !data}
    <div class="flex items-center justify-center h-32">
      <div class="text-xs text-text-muted">{$tx("Loading stats cache...")}</div>
    </div>
  {:else if error}
    <div class="bg-bg-card rounded-xl p-6">
      <div class="text-base font-semibold text-text-primary mb-2">{$tx("Stats Not Available Yet")}</div>
      <p class="text-sm text-text-secondary mb-4">
        Claude Code hasn't generated usage statistics yet. This file is created automatically by Claude Code in the background — there's no manual way to trigger it.
      </p>
      <ul class="text-sm text-text-secondary mb-4 list-disc list-inside space-y-1">
        <li>{$tx("Use Claude Code for a few sessions")}</li>
        <li>{$tx("Try running")} <code class="bg-bg-primary px-1 py-0.5 rounded text-text-muted">/stats</code> {$tx("in Claude Code")}</li>
        <li>{$tx("Check back later")}</li>
      </ul>
      <div class="text-xs text-text-muted mb-4">Looking for: ~/.claude/stats-cache.json</div>
      <button
        class="px-4 py-2 bg-accent-blue text-white text-sm font-medium rounded-md hover:opacity-90 transition-opacity"
        on:click={fetchData}
      >
        {$tx("Refresh")}
      </button>
    </div>
  {:else if data}
    <!-- Info Banner -->
    <div class="bg-bg-card rounded-xl px-3 py-2 mb-3 flex items-center gap-2">
      <span class="text-sm">ℹ️</span>
      <span class="text-xs text-text-muted">Data covers approximately the last 30 days. Source: Claude Code internal cache</span>
    </div>

    <!-- Summary Cards - 8 cards in 2 rows of 4 -->
    <div class="grid grid-cols-4 gap-2 mb-3">
      <MetricCard label={$tx("Est. Cost")} value={formatCost(data.estimatedCost)} subtitle={$tx("all time")} color="green" highlight={true} />
      <MetricCard label={$tx("Total Tokens")} value={formatTokens(data.totalTokens)} subtitle={$tx("all time")} color="cyan" highlight={true} />
      <MetricCard label={$tx("Sessions")} value={data.totalSessions.toString()} subtitle={$tx("all time")} color="green" />
      <MetricCard label={$tx("Active Days")} value={data.activeDays.toString()} subtitle={$tx("days used")} color="yellow" />
    </div>
    <div class="grid grid-cols-4 gap-2 mb-3">
      <MetricCard label={$tx("Messages")} value={data.totalMessages.toLocaleString()} subtitle={$tx("all time")} color="purple" />
      <MetricCard label={$tx("Avg/Day")} value={data.avgMessagesPerDay.toFixed(1)} subtitle={$tx("messages")} color="orange" />
      <MetricCard label={$tx("Peak Hour")} value={formatHour(data.peakHour)} subtitle={$tx("most active")} color="cyan" />
      <MetricCard label={$tx("First Session")} value={formatDate(data.firstSession)} subtitle={$tx("started using")} color="blue" />
    </div>

    <!-- Daily Activity Chart -->
    <div class="bg-bg-card rounded-xl p-3 mb-3">
      <div class="flex items-center justify-between mb-2">
        <div class="text-xs font-medium text-text-muted uppercase tracking-wider">{$tx("Daily Activity")}</div>
        <span class="text-xs text-text-muted">{$tx("Last 30 days")}</span>
      </div>
      <div class="h-32"><canvas bind:this={dailyChartCanvas}></canvas></div>
    </div>

    <ActivityHeatmap points={data.dailyActivity} on:select={(e) => { sessionDateFilter.set(e.detail); requestedView.set('sessions'); }} />

    <!-- Token Usage by Model -->
    <div class="bg-bg-card rounded-xl p-4 mb-3">
      <div class="text-xs font-medium text-text-muted uppercase tracking-wider mb-3">{$tx("Token Usage by Model")}</div>
      <div class="flex items-center gap-5">
        <div class="w-30 h-30 flex-shrink-0" style="width: 120px; height: 120px;">
          <canvas bind:this={modelChartCanvas}></canvas>
        </div>
        <div class="flex-1 space-y-2 min-w-0">
          {#each data.tokensByModel.slice(0, 5) as model, i}
            {@const percent = data.totalTokens > 0 ? (model.tokens / data.totalTokens) * 100 : 0}
            <div class="flex items-center gap-2">
              <div class="w-2 h-2 rounded-full flex-shrink-0" style="background-color: {chartColors[i % chartColors.length]}"></div>
              <span class="text-sm font-bold text-text-primary">{formatModelName(model.model)}</span>
              <span class="text-sm font-bold text-text-primary">({percent.toFixed(1)}%)</span>
            </div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Activity by Hour -->
    <div class="bg-bg-card rounded-xl p-3">
      <div class="text-xs font-medium text-text-muted uppercase tracking-wider mb-2">{$tx("Activity by Hour")}</div>
      <div class="h-28"><canvas bind:this={hourlyChartCanvas}></canvas></div>
    </div>
  {/if}
</div>
