<script lang="ts">
  export let tabs: { id: string; label: string }[];
  export let active: string;
  export let onChange: (id: string) => void;
</script>

<nav class="flex gap-1 border-b border-border-secondary mb-5">
  {#each tabs as tab}
    <button
      class="px-3 py-2 text-sm transition-colors relative {active === tab.id ? 'text-text-primary font-medium' : 'text-text-muted hover:text-text-primary'}"
      on:click={() => onChange(tab.id)}
    >
      {tab.label}
      {#if active === tab.id}<div class="absolute -bottom-px left-0 right-0 h-0.5 bg-accent-primary"></div>{/if}
    </button>
  {/each}
</nav>
