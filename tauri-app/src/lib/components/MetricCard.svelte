<script lang="ts">
  export let label: string;
  export let value: string;
  export let subtitle: string = '';
  export let color: 'green' | 'cyan' | 'purple' | 'orange' | 'yellow' | 'red' | 'blue' | 'teal' | 'pink' = 'cyan';
  export let showBar: boolean = false;
  export let barPercent: number = 0;
  export let highlight: boolean = false;
  /** % change vs previous period; null = unknown / previous was 0 (nothing rendered). */
  export let delta: number | null = null;
  /** For cost-like metrics, a decrease is good. */
  export let lowerIsBetter: boolean = false;

  $: deltaGood = delta === null || delta === 0 ? null : (delta < 0) === lowerIsBetter;

  const colorClasses: Record<string, { dot: string; bar: string; glow: string; border: string }> = {
    green: { dot: 'bg-green', bar: 'bg-green', glow: 'rgba(52, 211, 153, 0.15)', border: 'rgba(52, 211, 153, 0.4)' },
    cyan: { dot: 'bg-sky', bar: 'bg-sky', glow: 'rgba(124, 156, 255, 0.15)', border: 'rgba(124, 156, 255, 0.4)' },
    purple: { dot: 'bg-mauve', bar: 'bg-mauve', glow: 'rgba(167, 139, 250, 0.15)', border: 'rgba(167, 139, 250, 0.4)' },
    orange: { dot: 'bg-peach', bar: 'bg-peach', glow: 'rgba(251, 146, 60, 0.15)', border: 'rgba(251, 146, 60, 0.4)' },
    yellow: { dot: 'bg-yellow', bar: 'bg-yellow', glow: 'rgba(251, 191, 36, 0.15)', border: 'rgba(251, 191, 36, 0.4)' },
    red: { dot: 'bg-red', bar: 'bg-red', glow: 'rgba(248, 113, 113, 0.15)', border: 'rgba(248, 113, 113, 0.4)' },
    blue: { dot: 'bg-blue', bar: 'bg-blue', glow: 'rgba(91, 141, 239, 0.15)', border: 'rgba(91, 141, 239, 0.4)' },
    teal: { dot: 'bg-teal', bar: 'bg-teal', glow: 'rgba(124, 156, 255, 0.15)', border: 'rgba(124, 156, 255, 0.4)' },
    pink: { dot: 'bg-pink', bar: 'bg-pink', glow: 'rgba(244, 114, 182, 0.15)', border: 'rgba(244, 114, 182, 0.4)' },
  };

  $: highlightStyle = highlight ? `border-color: ${colorClasses[color].border};` : '';
</script>

<div class="bg-bg-card rounded-xl px-4 py-3 hover:bg-bg-card-hover transition-colors" style={highlightStyle}>
  <div class="flex items-center gap-1.5 mb-1.5">
    <div class="w-1.5 h-1.5 rounded-full {colorClasses[color].dot}"></div>
    <span class="text-[11px] font-medium text-text-muted uppercase tracking-wider">{label}</span>
  </div>
  <div class="flex items-baseline gap-2">
    <div class="text-xl font-semibold text-text-primary leading-tight">{value}</div>
    {#if delta !== null}
      <span class="text-xs font-medium {deltaGood === null ? 'text-text-muted' : deltaGood ? 'text-green' : 'text-red'}">
        {delta > 0 ? '▲' : delta < 0 ? '▼' : '•'} {Math.abs(delta).toFixed(0)}%
      </span>
    {/if}
  </div>
  {#if subtitle}
    <div class="text-xs text-text-muted mt-0.5">{subtitle}</div>
  {/if}
  {#if showBar}
    <div class="mt-2.5 h-1 bg-bg-primary rounded-full overflow-hidden">
      <div class="h-full {colorClasses[color].bar} transition-all" style="width: {barPercent}%"></div>
    </div>
  {/if}
</div>
