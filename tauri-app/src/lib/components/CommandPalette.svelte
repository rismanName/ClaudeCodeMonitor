<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { settings } from '$lib/stores/settings';
  import { onboardingOpen, setAlert, clearAlert } from '$lib/stores';
  import { sections } from '$lib/nav';
  import { t } from '$lib/i18n';
  import type { TranslationKey } from '$lib/i18n';

  export let onClose: () => void;
  export let onNavigate: (view: string) => void;
  export let onOpenSettings: () => void;

  interface Item { id: string; label: string; tag: string; run: () => void | Promise<void> }

  let query = '';
  let index = 0;
  let input: HTMLInputElement;

  /** Result feedback goes through the shared alert strip (id 'action'), auto-clearing after a few seconds. */
  function notify(severity: 'info' | 'critical', text: string) {
    setAlert({ id: 'action', severity, text });
    setTimeout(() => clearAlert('action'), 6000);
  }

  const stackAction = (cmd: 'ensure_monitoring_stack' | 'restart_monitoring_stack') => async () => {
    notify('info', $t('stack.starting'));
    try {
      const r = await invoke<{ state: string; code: string; detail: string }>(cmd, { composeDir: $settings.composeDir || null });
      if (r.state === 'error') notify('critical', `${$t(`stack.error.${r.code}` as TranslationKey)}${r.detail ? ` (${r.detail})` : ''}`);
      else notify('info', $t('stack.started'));
    } catch (e) { notify('critical', String(e)); }
  };

  async function backup() {
    try {
      const r = await invoke<{ path: string }>('backup_prometheus', { prometheusUrl: $settings.prometheusUrl });
      notify('info', `${$t('export.saved')}: ${r.path}`);
    } catch (e) {
      notify('critical', String(e) === 'admin_api_disabled' ? $t('backup.needAdmin') : String(e));
    }
  }

  // Rebuilt when the language changes ($t is reactive).
  $: items = [
    ...sections.flatMap((s) =>
      s.views.map((v): Item => ({ id: `go-${v.id}`, label: $t(v.label), tag: $t(s.label), run: () => onNavigate(v.id) }))
    ),
    { id: 'act-start', label: $t('pal.startStack'), tag: $t('pal.action'), run: stackAction('ensure_monitoring_stack') },
    { id: 'act-restart', label: $t('pal.restartStack'), tag: $t('pal.action'), run: stackAction('restart_monitoring_stack') },
    { id: 'act-backup', label: $t('pal.backup'), tag: $t('pal.action'), run: backup },
    { id: 'act-guide', label: $t('onb.rerun'), tag: $t('pal.action'), run: () => onboardingOpen.set(true) },
    { id: 'act-settings', label: $t('pal.settings'), tag: $t('pal.action'), run: onOpenSettings },
  ] as Item[];

  /** Higher = better; -1 = no match. Substring (earlier is better) beats in-order subsequence. */
  export function score(q: string, text: string): number {
    const needle = q.trim().toLowerCase();
    if (!needle) return 0;
    const hay = text.toLowerCase();
    const at = hay.indexOf(needle);
    if (at >= 0) return 1000 - at;
    let pos = 0;
    for (const ch of needle) {
      pos = hay.indexOf(ch, pos);
      if (pos < 0) return -1;
      pos++;
    }
    return 100 - hay.length;
  }

  $: results = (query.trim()
    ? items
        .map((it) => ({ it, s: Math.max(score(query, it.label), score(query, it.tag) - 500) }))
        .filter((r) => r.s >= 0)
        .sort((a, b) => b.s - a.s)
        .map((r) => r.it)
    : items) as Item[];

  $: if (index >= results.length) index = Math.max(0, results.length - 1);

  async function move(delta: number) {
    if (!results.length) return;
    index = (index + delta + results.length) % results.length;
    await tick();
    document.getElementById(`pal-item-${index}`)?.scrollIntoView({ block: 'nearest' });
  }

  function choose(it: Item | undefined) {
    if (!it) return;
    onClose();
    void it.run();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { e.preventDefault(); onClose(); }
    else if (e.key === 'ArrowDown') { e.preventDefault(); move(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); move(-1); }
    else if (e.key === 'Enter') { e.preventDefault(); choose(results[index]); }
  }

  onMount(() => input?.focus());
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-24 px-6" role="presentation" on:click={onClose}>
  <div class="w-full max-w-lg rounded-2xl border border-border-primary bg-bg-primary shadow-card overflow-hidden" role="dialog" aria-modal="true" aria-label={$t('pal.placeholder')} tabindex="-1" on:click|stopPropagation>
    <input
      bind:this={input}
      bind:value={query}
      on:keydown={onKeydown}
      on:input={() => (index = 0)}
      placeholder={$t('pal.placeholder')}
      class="w-full bg-transparent px-4 py-3 text-sm text-text-primary placeholder-text-muted border-b border-border-secondary focus:outline-none"
    />
    <div class="max-h-80 overflow-y-auto py-1" role="listbox">
      {#each results as it, i (it.id)}
        <button
          id="pal-item-{i}"
          role="option"
          aria-selected={i === index}
          class="w-full flex items-center justify-between gap-3 px-4 py-2 text-left text-sm {i === index ? 'bg-accent-primary/12 text-text-primary' : 'text-text-secondary'}"
          on:mousemove={() => (index = i)}
          on:click={() => choose(it)}
        >
          <span class="truncate">{it.label}</span>
          <span class="text-[11px] text-text-muted flex-shrink-0">{it.tag}</span>
        </button>
      {:else}
        <div class="px-4 py-6 text-center text-sm text-text-muted">{$t('pal.empty')}</div>
      {/each}
    </div>
  </div>
</div>
