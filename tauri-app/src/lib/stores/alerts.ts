import { writable, derived, get } from 'svelte/store';

export type AlertSeverity = 'info' | 'warn' | 'critical';

export interface Alert {
  /** Stable id per source ('budget', 'spike', 'stack'); setting the same id replaces the alert. */
  id: string;
  severity: AlertSeverity;
  text: string;
  /** Called when the user dismisses it (e.g. to remember the dismissal for the rest of the day). */
  onDismiss?: () => void;
}

const rank: Record<AlertSeverity, number> = { info: 0, warn: 1, critical: 2 };

export const alerts = writable<Record<string, Alert>>({});
/** id → severity that was dismissed. A higher/different severity for the same id shows again. */
const dismissed = writable<Record<string, AlertSeverity>>({});

export function setAlert(a: Alert) {
  alerts.update((m) => ({ ...m, [a.id]: a }));
}

export function clearAlert(id: string) {
  alerts.update((m) => {
    if (!(id in m)) return m;
    const { [id]: _removed, ...rest } = m;
    return rest;
  });
  dismissed.update((d) => {
    if (!(id in d)) return d;
    const { [id]: _removed, ...rest } = d;
    return rest;
  });
}

export function dismissAlert(id: string) {
  const a = get(alerts)[id];
  if (!a) return;
  a.onDismiss?.();
  dismissed.update((d) => ({ ...d, [id]: a.severity }));
}

/** Active, not-dismissed alerts, most severe first. */
export const visibleAlerts = derived([alerts, dismissed], ([$a, $d]) =>
  Object.values($a)
    .filter((a) => $d[a.id] !== a.severity)
    .sort((x, y) => rank[y.severity] - rank[x.severity])
);
