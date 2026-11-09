<script lang="ts">
  import { visibleAlerts, dismissAlert } from '$lib/stores';
  import type { AlertSeverity } from '$lib/stores';
  import { t } from '$lib/i18n';

  let open = false;

  $: top = $visibleAlerts[0];
  $: more = $visibleAlerts.length - 1;
  $: if (more <= 0) open = false;

  const tone: Record<AlertSeverity, string> = {
    info: 'bg-bg-card border-border-secondary text-text-secondary',
    warn: 'bg-peach/10 border-peach/50 text-peach',
    critical: 'bg-red/10 border-red/50 text-red',
  };
  const dot: Record<AlertSeverity, string> = { info: 'bg-sky', warn: 'bg-peach', critical: 'bg-red' };
</script>

<svelte:window on:click={() => (open = false)} />

{#if top}
  <!-- One strip at a time: the most severe alert; "+N" opens the list of all active alerts. -->
  <div class="relative mb-3" on:click|stopPropagation role="presentation">
    <div class="flex items-center gap-2 rounded-md border px-3 py-2 text-xs {tone[top.severity]}">
      <span class="flex-1">{top.text}</span>
      {#if more > 0}
        <button class="px-1.5 py-0.5 rounded bg-black/20 font-medium" title={$t('alerts.all')} on:click={() => (open = !open)}>+{more}</button>
      {/if}
      <button class="underline" on:click={() => dismissAlert(top.id)}>{$t('stack.dismiss')}</button>
    </div>
    {#if open}
      <div class="absolute z-20 left-0 right-0 mt-1 rounded-md border border-border-primary bg-bg-secondary shadow-card divide-y divide-border-secondary">
        <div class="px-3 py-1.5 text-[11px] font-medium text-text-muted uppercase tracking-wider">{$t('alerts.all')}</div>
        {#each $visibleAlerts as a (a.id)}
          <div class="flex items-center gap-2 px-3 py-2 text-xs text-text-secondary">
            <span class="w-1.5 h-1.5 rounded-full flex-shrink-0 {dot[a.severity]}"></span>
            <span class="flex-1">{a.text}</span>
            <button class="underline text-text-muted" on:click={() => dismissAlert(a.id)}>{$t('stack.dismiss')}</button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}
