<script lang="ts">
  import type { DailyActivityPoint } from '$lib/types';
  import { createEventDispatcher } from 'svelte';
  import { t } from '$lib/i18n';

  const dispatch = createEventDispatcher<{ select: string }>();

  /** Daily message counts keyed by YYYY-MM-DD (from stats-cache). */
  export let points: DailyActivityPoint[];
  export let weeks = 12;

  const pad = (n: number) => String(n).padStart(2, '0');
  const keyOf = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

  $: byDate = new Map(points.map((p) => [p.date.slice(0, 10), p.value]));

  // Columns = weeks (oldest first), rows = Sun..Sat; the last column ends on today.
  $: grid = (() => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const end = new Date(today);
    end.setDate(end.getDate() + (6 - end.getDay())); // Saturday of the current week
    const start = new Date(end);
    start.setDate(start.getDate() - weeks * 7 + 1);
    const cols: { key: string; value: number; future: boolean }[][] = [];
    for (let w = 0; w < weeks; w++) {
      const col = [];
      for (let d = 0; d < 7; d++) {
        const day = new Date(start);
        day.setDate(start.getDate() + w * 7 + d);
        col.push({ key: keyOf(day), value: byDate.get(keyOf(day)) ?? 0, future: day > today });
      }
      cols.push(col);
    }
    return cols;
  })();

  $: max = Math.max(1, ...grid.flat().map((c) => c.value));

  function level(v: number): number {
    if (v <= 0) return 0;
    return Math.min(4, Math.ceil((v / max) * 4));
  }

  const shades = [
    'rgba(142, 142, 147, 0.15)',
    'rgba(0, 255, 136, 0.25)',
    'rgba(0, 255, 136, 0.45)',
    'rgba(0, 255, 136, 0.7)',
    'rgba(0, 255, 136, 1)',
  ];
</script>

<div class="bg-bg-card rounded-md p-3 mb-3">
  <div class="flex items-center justify-between mb-2">
    <div class="text-xs font-medium text-text-muted uppercase tracking-wider">{$t('heat.title')}</div>
    <span class="text-xs text-text-muted">{$t('heat.weeks', { n: weeks })}</span>
  </div>
  <div class="flex gap-1 justify-center">
    {#each grid as col}
      <div class="flex flex-col gap-1">
        {#each col as cell}
          <button
            type="button"
            class="w-3.5 h-3.5 rounded-sm p-0 border-0 {cell.value > 0 ? 'cursor-pointer hover:ring-1 hover:ring-white/60' : 'cursor-default'}"
            style="background: {cell.future ? 'transparent' : shades[level(cell.value)]};"
            title={cell.future ? '' : `${cell.key}: ${cell.value.toLocaleString()}`}
            disabled={cell.future || cell.value <= 0}
            on:click={() => dispatch('select', cell.key)}
          ></button>
        {/each}
      </div>
    {/each}
  </div>
</div>
