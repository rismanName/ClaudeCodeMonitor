<script lang="ts">
  export let category: string = '';
  export let title: string;
  export let subtitle: string = '';
  void category;
</script>

<div class="flex items-end justify-between mb-5">
  <div>
    <h1 class="text-2xl font-semibold text-text-primary leading-tight tracking-tight">{title}</h1>
    {#if subtitle}
      <p class="text-sm text-text-muted mt-0.5">{subtitle}</p>
    {/if}
  </div>
  <slot name="actions" />
</div>
