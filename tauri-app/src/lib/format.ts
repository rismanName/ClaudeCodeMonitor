import { derived } from 'svelte/store';
import { settings } from '$lib/stores/settings';

/** Currency-aware cost formatter. Use as `$formatCost(n)` so it re-renders when currency/rate change. */
export const formatCost = derived(settings, ($s) => {
  return (usd: number): string => {
    if ($s.currency === 'IDR') {
      return `Rp ${Math.round(usd * $s.usdToIdrRate).toLocaleString('id-ID')}`;
    }
    if (usd >= 1) return `$${usd.toFixed(2)}`;
    if (usd >= 0.01) return `$${usd.toFixed(3)}`;
    return `$${usd.toFixed(4)}`;
  };
});

/** Axis-tick variant: no sub-unit precision. */
export const formatCostAxis = derived(settings, ($s) => {
  return (usd: number): string =>
    $s.currency === 'IDR'
      ? `Rp ${Math.round(usd * $s.usdToIdrRate).toLocaleString('id-ID')}`
      : `$${usd.toFixed(2)}`;
});
