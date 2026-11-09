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

/** Friendly model name, e.g. "claude-opus-4-5-20251101" → "Opus 4.5". */
export function formatModelName(modelName: string): string {
  // Extract friendly name like "Haiku 4.5" or "Opus 4.5" from full model name
  // Model names look like: "claude-3-5-haiku-20241022" or "claude-opus-4-5-20251101"
  const lower = modelName.toLowerCase();

  // Extract version like "3-5" or "4-5" and convert to "3.5" or "4.5"
  const versionMatch = lower.match(/(\d)-(\d)/);
  const version = versionMatch ? `${versionMatch[1]}.${versionMatch[2]}` : '';

  if (lower.includes('haiku')) {
    return version ? `Haiku ${version}` : 'Haiku';
  }
  if (lower.includes('opus')) {
    return version ? `Opus ${version}` : 'Opus';
  }
  if (lower.includes('sonnet')) {
    return version ? `Sonnet ${version}` : 'Sonnet';
  }
  // Fallback: return last part of model name
  return modelName.split('.').pop() || modelName;
}
