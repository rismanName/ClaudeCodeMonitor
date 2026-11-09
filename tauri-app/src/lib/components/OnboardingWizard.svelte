<script lang="ts">
  import { onMount } from 'svelte';
  import { settings } from '$lib/stores/settings';
  import { onboardingOpen } from '$lib/stores';
  import { t } from '$lib/i18n';
  import type { TranslationKey } from '$lib/i18n';
  import { loadSetup, setupSteps, missingRequired, fixStack, fixTelemetry, fixAutostart, skipOnboarding } from '$lib/setup';
  import type { SetupSnapshot, StepId } from '$lib/setup';

  let snap: SetupSnapshot = { status: null, telemetryOn: null, autostartOn: null };
  let busy: StepId | '' = '';
  let message = '';

  async function refresh() { snap = await loadSetup(); }

  $: steps = setupSteps(snap);
  $: missing = missingRequired(snap).length;
  // The step the user should act on now: first failing required step, else the optional one.
  $: current = steps.find((s) => s.required && s.ok === false) ?? steps.find((s) => !s.required && s.ok === false) ?? null;

  async function run(id: StepId) {
    busy = id;
    message = '';
    if (id === 'stack') message = await fixStack($settings.composeDir, (code) => $t(`stack.error.${code}` as TranslationKey));
    else if (id === 'telemetry') message = await fixTelemetry();
    else if (id === 'autostart') message = await fixAutostart();
    busy = '';
    await refresh();
  }

  const actionLabel: Partial<Record<StepId, TranslationKey>> = { stack: 'setup.fix.start', telemetry: 'set.telemetry.do', autostart: 'set.telemetry.do' };

  function skip() { skipOnboarding(); onboardingOpen.set(false); }
  function close() { onboardingOpen.set(false); }

  onMount(refresh);
</script>

<div class="fixed inset-0 z-40 flex items-center justify-center bg-black/60 p-6" role="dialog" aria-modal="true" aria-label={$t('onb.title')}>
  <div class="w-full max-w-md rounded-2xl border border-border-primary bg-bg-primary shadow-card p-6">
    <h2 class="text-xl font-semibold tracking-tight text-text-primary">{$t('onb.title')}</h2>
    <p class="text-sm text-text-muted mt-1 mb-5">{missing === 0 ? $t('onb.allDone') : $t('onb.subtitle')}</p>

    <ol class="space-y-2 mb-4">
      {#each steps as step, i}
        {@const isCurrent = current?.id === step.id}
        <li class="rounded-xl border px-3 py-2.5 {isCurrent ? 'border-accent-primary/50 bg-accent-primary/5' : 'border-border-secondary'}">
          <div class="flex items-center gap-3">
            <span class="w-5 text-center font-bold {step.ok ? 'text-green' : step.ok === false ? (step.required ? 'text-red' : 'text-peach') : 'text-text-muted'}">
              {step.ok ? '✓' : step.ok === false ? (step.required ? '✗' : '!') : '?'}
            </span>
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium text-text-primary">
                {i + 1}. {$t(step.title)}{#if !step.required}<span class="ml-2 text-xs text-text-muted">{$t('setup.optional')}</span>{/if}
              </div>
              {#if isCurrent}<div class="text-xs text-text-muted mt-0.5">{$t(step.hint)}</div>{/if}
            </div>
            {#if isCurrent && actionLabel[step.id]}
              <button disabled={busy !== ''} on:click={() => run(step.id)} class="px-3 py-1.5 bg-accent-primary text-crust rounded-lg text-xs font-medium disabled:opacity-50">
                {busy === step.id ? '…' : $t(actionLabel[step.id] as TranslationKey)}
              </button>
            {/if}
          </div>
        </li>
      {/each}
    </ol>

    {#if message}<p class="mb-3 text-xs text-red break-all">{message}</p>{/if}

    <div class="flex items-center justify-between">
      <button class="text-xs text-text-muted underline" on:click={refresh}>{$t('setup.recheck')}</button>
      <div class="flex items-center gap-3">
        {#if missing > 0}<button class="text-xs text-text-muted underline" on:click={skip}>{$t('onb.skip')}</button>{/if}
        <button on:click={close} disabled={missing > 0}
          class="px-4 py-2 rounded-lg bg-accent-primary text-crust text-sm font-medium disabled:opacity-40">{$t('onb.done')}</button>
      </div>
    </div>
  </div>
</div>
