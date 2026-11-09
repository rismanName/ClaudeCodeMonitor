<script lang="ts">
  import { formatCost as fmt } from '$lib/format';
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { isConnected, lastUpdated, totalCost } from '$lib/stores';
  import { settings } from '$lib/stores/settings';
  import { t } from '$lib/i18n';
  import type { TranslationKey } from '$lib/i18n';

  export let activeView: string;
  export let onNavigate: (view: string) => void;
  export let onOpenSettings: () => void = () => {};

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

  // Each tab has a unique accent color for the selected state
  const tabColors: Record<string, { border: string; glow: string; text: string }> = {
    'summary': { border: '#00d9ff', glow: 'rgba(0, 217, 255, 0.15)', text: '#00d9ff' },      // Cyan - primary
    'tokens': { border: '#00ff88', glow: 'rgba(0, 255, 136, 0.15)', text: '#00ff88' },       // Green - performance
    'insights': { border: '#a855f7', glow: 'rgba(168, 85, 247, 0.15)', text: '#a855f7' },    // Purple - analysis
    'sessions': { border: '#ff6b9d', glow: 'rgba(255, 107, 157, 0.15)', text: '#ff6b9d' },    // Pink - activity
    'stats-cache': { border: '#007aff', glow: 'rgba(0, 122, 255, 0.15)', text: '#007aff' },  // Blue - data
    'prometheus-health': { border: '#ff6b6b', glow: 'rgba(255, 107, 107, 0.15)', text: '#ff6b6b' }, // Red - infrastructure
    'projects': { border: '#ffd93d', glow: 'rgba(255, 217, 61, 0.15)', text: '#ffd93d' },   // Yellow - projects
    'budget': { border: '#00ff88', glow: 'rgba(0, 255, 136, 0.15)', text: '#00ff88' },       // Green - money
    'setup': { border: '#00d9ff', glow: 'rgba(0, 217, 255, 0.15)', text: '#00d9ff' },         // Cyan - onboarding
    'smoke-test': { border: '#ffb347', glow: 'rgba(255, 179, 71, 0.15)', text: '#ffb347' }, // Orange - developer
  };

  type NavItem = { id: string; icon: string; label: TranslationKey; desc: TranslationKey };
  const groups: { title: TranslationKey; items: NavItem[] }[] = [
    {
      title: 'nav.prometheus',
      items: [
        { id: 'summary', icon: 'grid', label: 'nav.dashboard', desc: 'nav.dashboard.desc' },
        { id: 'tokens', icon: 'activity', label: 'nav.tokens', desc: 'nav.tokens.desc' },
      ],
    },
    {
      title: 'nav.analysis',
      items: [
        { id: 'projects', icon: 'folder', label: 'nav.projects', desc: 'nav.projects.desc' },
        { id: 'budget', icon: 'wallet', label: 'nav.budget', desc: 'nav.budget.desc' },
        { id: 'insights', icon: 'lightbulb', label: 'nav.insights', desc: 'nav.insights.desc' },
      ],
    },
    {
      title: 'nav.config',
      items: [
        { id: 'sessions', icon: 'terminal', label: 'nav.sessions', desc: 'nav.sessions.desc' },
        { id: 'stats-cache', icon: 'database', label: 'nav.statsCache', desc: 'nav.statsCache.desc' },
      ],
    },
    {
      title: 'nav.developer',
      items: [
        { id: 'setup', icon: 'check', label: 'nav.setup', desc: 'nav.setup.desc' },
        { id: 'prometheus-health', icon: 'heartbeat', label: 'nav.health', desc: 'nav.health.desc' },
        { id: 'smoke-test', icon: 'flask', label: 'nav.smoke', desc: 'nav.smoke.desc' },
      ],
    },
  ];

  $: formatCost = $fmt;

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

<aside class="w-52 h-screen bg-bg-primary border-r border-border-secondary flex flex-col">
  <!-- Logo Header -->
  <div class="px-3 py-3 flex items-center gap-2">
    <!-- Custom Logo: Gradient background with pulse/monitor line -->
    <div class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 relative overflow-hidden"
         style="background: linear-gradient(135deg, #00d9ff 0%, #007aff 50%, #a855f7 100%);">
      <!-- Pulse/Activity line representing monitoring -->
      <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none">
        <!-- Activity pulse line -->
        <path
          d="M2 12h4l2-6 4 12 2-6h8"
          stroke="white"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
          style="filter: drop-shadow(0 1px 2px rgba(0,0,0,0.3));"
        />
      </svg>
    </div>
    <div class="flex-1 min-w-0">
      <div class="text-xs text-text-muted uppercase tracking-wider leading-none">Claude Code</div>
      <div class="text-sm font-semibold text-text-primary">Monitor</div>
    </div>
    <div class="px-3 py-1 text-sm font-bold rounded" style="background: rgba(0, 255, 136, 0.15); color: #00ff88;">
      {formatCost($totalCost)}
    </div>
  </div>

  <!-- Navigation -->
  <nav class="flex-1 overflow-y-auto px-2 py-1">
    {#each groups as group, gi}
      <div class={gi < groups.length - 1 ? 'mb-3' : ''}>
        <div class="px-2 py-1.5 text-xs font-medium text-text-muted uppercase tracking-wider">
          {$t(group.title)}
        </div>
        {#each group.items as item}
          {@const colors = tabColors[item.id]}
          <button
            class="w-full text-left px-2 py-1.5 rounded-md mb-0.5 transition-all duration-200
              {activeView === item.id
                ? 'text-text-primary'
                : 'text-text-secondary hover:text-text-primary hover:bg-bg-card-hover'}"
            style={activeView === item.id
              ? `background: ${colors.glow}; border-left: 2px solid ${colors.border}; box-shadow: inset 0 0 12px ${colors.glow};`
              : 'border-left: 2px solid transparent;'}
            on:click={() => onNavigate(item.id)}
          >
            <div class="flex items-center gap-2">
              <svg
                class="w-4 h-4 flex-shrink-0 transition-colors duration-200"
                style={activeView === item.id ? `color: ${colors.text};` : ''}
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                stroke-width="2"
              >
                <path stroke-linecap="round" stroke-linejoin="round" d={getIcon(item.icon)} />
              </svg>
              <span class="text-sm font-medium truncate">{$t(item.label)}</span>
            </div>
            {#if activeView === item.id}
              <div class="text-xs text-text-muted mt-0.5 ml-6 truncate">{$t(item.desc)}</div>
            {/if}
          </button>
        {/each}
      </div>
    {/each}
  </nav>

  <!-- Status Footer -->
  <div class="px-3 py-2 border-t border-border-secondary">
    <div class="flex items-center justify-between mb-0.5">
      <div class="flex items-center gap-1.5">
        <div class="w-2 h-2 rounded-full {$isConnected ? 'bg-accent-green' : 'bg-accent-red'}"></div>
        <span class="text-sm font-bold {$isConnected ? 'text-accent-green' : 'text-accent-red'}">
          {$isConnected ? $t('status.connected') : $t('status.disconnected')}
        </span>
      </div>
      <button
        on:click={onOpenSettings}
        class="p-1.5 rounded-md text-text-muted hover:text-text-primary hover:bg-bg-card-hover transition-colors"
        title={$t('settings.tooltip')}
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
          <path stroke-linecap="round" stroke-linejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      </button>
    </div>
    <div class="flex items-center gap-1 text-xs text-text-muted">
      <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" stroke-width="2"/>
        <path stroke-width="2" d="M12 6v6l4 2"/>
      </svg>
      <span>{$lastUpdated ? formatElapsed(elapsedSeconds) : $t('status.never')}</span>
    </div>
  </div>
</aside>
