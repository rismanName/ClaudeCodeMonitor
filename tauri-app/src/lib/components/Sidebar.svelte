<script lang="ts">
  import { formatCost as fmt } from '$lib/format';
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { isConnected, lastUpdated, totalCost } from '$lib/stores';
  import { settings } from '$lib/stores/settings';
  import { t } from '$lib/i18n';
  import { sections, sectionOf } from '$lib/nav';

  export let activeView: string;
  /** Receives a section id (the page decides which of its views to show). */
  export let onNavigate: (sectionId: string) => void;
  export let onOpenSettings: () => void = () => {};
  export let onOpenPalette: () => void = () => {};

  let elapsedSeconds = 0;
  let interval: ReturnType<typeof setInterval>;

  function updateElapsed() {
    if ($lastUpdated) {
      elapsedSeconds = Math.floor((Date.now() - $lastUpdated.getTime()) / 1000);
    }
  }

  function formatElapsed(seconds: number): string {
    if (seconds < 60) return `${seconds} ${get(t)('time.sec')}`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)} ${get(t)('time.min')}`;
    return `${Math.floor(seconds / 3600)} ${get(t)('time.hr')}`;
  }

  onMount(() => {
    updateElapsed();
    interval = setInterval(updateElapsed, 1000);
  });

  onDestroy(() => {
    if (interval) clearInterval(interval);
  });

  $: formatCost = $fmt;
  $: activeSection = sectionOf(activeView).id;

  function getIcon(name: string): string {
    const icons: Record<string, string> = {
      folder: 'M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2V7z',
      wallet: 'M3 7a2 2 0 012-2h14a2 2 0 012 2v2H3V7zm0 2h18v8a2 2 0 01-2 2H5a2 2 0 01-2-2V9zm13 4h2',
      check: 'M5 13l4 4L19 7',
      grid: 'M4 4h6v6H4V4zm10 0h6v6h-6V4zM4 14h6v6H4v-6zm10 0h6v6h-6v-6z',
      activity: 'M22 12h-4l-3 9L9 3l-3 9H2',
      lightbulb: 'M9 21h6m-3-3v3m0-18a6 6 0 016 6c0 2.22-1.21 4.16-3 5.2V17a1 1 0 01-1 1h-4a1 1 0 01-1-1v-2.8c-1.79-1.04-3-2.98-3-5.2a6 6 0 016-6z',
      terminal: 'M4 17l6-6-6-6m8 14h8',
      database: 'M12 2C6.48 2 2 4.24 2 7v10c0 2.76 4.48 5 10 5s10-2.24 10-5V7c0-2.76-4.48-5-10-5zm0 3c4.42 0 8 1.79 8 4s-3.58 4-8 4-8-1.79-8-4 3.58-4 8-4z',
      flask: 'M9 3h6v2H9V3zm1 2v5.17l-5 8.33V20h14v-1.5l-5-8.33V5h-4z',
      heartbeat: 'M22 12h-4l-3 9L9 3l-3 9H2M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 .53-.06 1.05-.17 1.54',
    };
    return icons[name] || icons.grid;
  }
</script>

<aside class="w-56 h-screen bg-bg-secondary border-r border-border-secondary flex flex-col">
  <!-- Brand -->
  <div class="px-4 pt-5 pb-4 flex items-center gap-2.5">
    <div class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 bg-accent-primary/15 border border-accent-primary/30">
      <svg class="w-4.5 h-4.5 text-accent-primary" width="18" height="18" viewBox="0 0 24 24" fill="none">
        <path d="M2 12h4l2-6 4 12 2-6h8" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </div>
    <div class="flex-1 min-w-0 leading-tight">
      <div class="text-sm font-semibold text-text-primary">Claude Monitor</div>
      <div class="text-xs text-text-muted">{formatCost($totalCost)} · {$t('status.total')}</div>
    </div>
  </div>

  <!-- Command palette entry (⌘K) -->
  <div class="px-3 pb-3">
    <button on:click={onOpenPalette} class="w-full flex items-center justify-between rounded-lg border border-border-secondary bg-bg-card px-2.5 py-1.5 text-xs text-text-muted hover:text-text-primary transition-colors">
      <span>{$t('pal.search')}</span>
      <kbd class="text-[10px] rounded bg-bg-primary px-1.5 py-0.5 border border-border-secondary">⌘K</kbd>
    </button>
  </div>

  <!-- Navigation -->
  <nav class="flex-1 overflow-y-auto px-3 pb-2">
    {#each sections as section}
      {@const active = activeSection === section.id}
      <button
        class="w-full text-left flex items-center gap-2.5 px-2.5 py-2 rounded-lg mb-1 transition-colors
          {active ? 'bg-accent-primary/12 text-text-primary' : 'text-text-muted hover:text-text-primary hover:bg-bg-card'}"
        title={$t(section.desc)}
        on:click={() => onNavigate(section.id)}
      >
        <svg class="w-4 h-4 flex-shrink-0 {active ? 'text-accent-primary' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.8">
          <path stroke-linecap="round" stroke-linejoin="round" d={getIcon(section.icon)} />
        </svg>
        <span class="text-sm {active ? 'font-medium' : ''}">{$t(section.label)}</span>
      </button>
    {/each}
  </nav>

  <!-- Status footer -->
  <div class="px-4 py-3 border-t border-border-secondary flex items-center justify-between">
    <div class="min-w-0">
      <div class="flex items-center gap-1.5">
        <span class="w-1.5 h-1.5 rounded-full {$isConnected ? 'bg-green' : 'bg-red'}"></span>
        <span class="text-xs {$isConnected ? 'text-green' : 'text-red'}">{$isConnected ? $t('status.connected') : $t('status.disconnected')}</span>
      </div>
      <div class="text-[11px] text-text-muted mt-0.5">{$lastUpdated ? formatElapsed(elapsedSeconds) : $t('status.never')}</div>
    </div>
    <button on:click={onOpenSettings} class="p-1.5 rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-card transition-colors" title={$t('settings.tooltip')}>
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="1.8">
        <path stroke-linecap="round" stroke-linejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    </button>
  </div>
</aside>
