<script lang="ts">
  import { onMount } from 'svelte';
  import { ViewHeader } from '$lib/components';
  import { settings } from '$lib/stores/settings';
  import { onboardingOpen } from '$lib/stores';
  import { t } from '$lib/i18n';
  import type { TranslationKey } from '$lib/i18n';
  import { loadSetup, fixStack as runFixStack, fixTelemetry as runFixTelemetry, fixAutostart as runFixAutostart } from '$lib/setup';
  import type { SetupStatus } from '$lib/setup';

  let status: SetupStatus | null = null;
  let telemetryOn: boolean | null = null;
  let autostartOn: boolean | null = null;
  let busy = '';
  let message = '';

  async function refresh() {
    ({ status, telemetryOn, autostartOn } = await loadSetup());
  }

  async function fixTelemetry() {
    busy = 'telemetry';
    message = await runFixTelemetry();
    busy = '';
    refresh();
  }

  async function fixStack() {
    busy = 'stack';
    message = await runFixStack($settings.composeDir, (code) => $t(`stack.error.${code}` as TranslationKey));
    busy = '';
    refresh();
  }

  async function fixAutostart() {
    busy = 'autostart';
    message = await runFixAutostart();
    busy = '';
    refresh();
  }

  type Row = { key: string; ok: boolean | null; title: TranslationKey; hint: TranslationKey; fix?: () => void; fixLabel?: TranslationKey; required: boolean };

  $: rows = status
    ? ([
        { key: 'engine', ok: status.engineFound, title: 'setup.engine', hint: 'setup.engine.hint', required: true },
        { key: 'running', ok: status.engineRunning, title: 'setup.engineRunning', hint: 'setup.engineRunning.hint', fix: fixStack, fixLabel: 'setup.fix.start', required: true },
        { key: 'prom', ok: status.prometheusUp, title: 'setup.prometheus', hint: 'setup.prometheus.hint', fix: fixStack, fixLabel: 'setup.fix.start', required: true },
        { key: 'otlp', ok: status.otlpUp, title: 'setup.otlp', hint: 'setup.otlp.hint', fix: fixStack, fixLabel: 'setup.fix.start', required: true },
        { key: 'telemetry', ok: telemetryOn, title: 'setup.telemetry', hint: 'setup.telemetry.hint', fix: fixTelemetry, fixLabel: 'set.telemetry.do', required: true },
        { key: 'claude', ok: status.claudeDirExists, title: 'setup.claudeDir', hint: 'setup.claudeDir.hint', required: true },
        { key: 'stats', ok: status.statsCacheExists, title: 'setup.statsCache', hint: 'setup.statsCache.hint', required: false },
        { key: 'autostart', ok: autostartOn, title: 'setup.autostart', hint: 'setup.autostart.hint', fix: fixAutostart, fixLabel: 'set.telemetry.do', required: false },
      ] as Row[])
    : [];

  $: missingRequired = rows.filter((r) => r.required && r.ok === false).length;

  onMount(refresh);
</script>

<div>
  <ViewHeader category="summary" title={$t('nav.setup')} subtitle={$t('nav.setup.desc')}>
    <svelte:fragment slot="actions">
      <button on:click={() => onboardingOpen.set(true)} class="px-2 py-1 text-xs rounded-md bg-bg-card border border-border-secondary text-text-secondary hover:bg-bg-card-hover">{$t('onb.rerun')}</button>
      <button on:click={refresh} class="px-2 py-1 text-xs rounded-md bg-bg-card border border-border-secondary text-text-secondary hover:bg-bg-card-hover">{$t('setup.recheck')}</button>
    </svelte:fragment>
  </ViewHeader>

  {#if status}
    <div class="mb-3 rounded-md border px-3 py-2 text-sm {missingRequired === 0 ? 'bg-green/10 border-green/50 text-green' : 'bg-peach/10 border-peach/50 text-peach'}">
      {missingRequired === 0 ? $t('setup.allGood') : $t('setup.missing', { n: missingRequired })}
    </div>

    <div class="bg-bg-card rounded-xl divide-y divide-border-secondary">
      {#each rows as row}
        <div class="flex items-center gap-3 px-4 py-3">
          <span class="w-5 text-center font-bold {row.ok ? 'text-green' : row.ok === false ? (row.required ? 'text-red' : 'text-peach') : 'text-text-muted'}">
            {row.ok ? '✓' : row.ok === false ? (row.required ? '✗' : '!') : '?'}
          </span>
          <div class="flex-1 min-w-0">
            <div class="text-sm font-medium text-text-primary">{$t(row.title)}{#if !row.required}<span class="ml-2 text-xs text-text-muted">{$t('setup.optional')}</span>{/if}</div>
            {#if row.ok !== true}<div class="text-xs text-text-muted">{$t(row.hint)}</div>{/if}
          </div>
          {#if row.ok === false && row.fix}
            <button disabled={busy !== ''} on:click={row.fix} class="px-3 py-1.5 bg-blue text-crust rounded-md text-xs font-medium disabled:opacity-50">
              {busy ? '…' : $t(row.fixLabel ?? 'set.telemetry.do')}
            </button>
          {/if}
        </div>
      {/each}
    </div>
    {#if message}<p class="mt-2 text-xs text-red break-all">{message}</p>{/if}
  {:else}
    <div class="text-xs text-text-muted">…</div>
  {/if}
</div>
