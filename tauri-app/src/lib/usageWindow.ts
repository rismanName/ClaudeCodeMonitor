/** Helpers for the plan-limit windows: when the weekly period started/ends and how to print reset times. */

const pad = (n: number) => String(n).padStart(2, '0');

/** "HH:MM" → [hours, minutes]; falls back to 00:00 for malformed input. */
export function parseClock(hhmm: string): [number, number] {
  const m = /^(\d{1,2}):(\d{2})$/.exec(hhmm ?? '');
  if (!m) return [0, 0];
  return [Math.min(23, Number(m[1])), Math.min(59, Number(m[2]))];
}

/** Most recent weekly reset at or before `now` (local time). */
export function lastWeeklyReset(day: number, time: string, now: Date = new Date()): Date {
  const [h, min] = parseClock(time);
  const d = new Date(now);
  d.setHours(h, min, 0, 0);
  d.setDate(d.getDate() - ((d.getDay() - day + 7) % 7));
  if (d.getTime() > now.getTime()) d.setDate(d.getDate() - 7);
  return d;
}

/** Next weekly reset strictly after `now`. */
export function nextWeeklyReset(day: number, time: string, now: Date = new Date()): Date {
  const d = lastWeeklyReset(day, time, now);
  d.setDate(d.getDate() + 7);
  return d;
}

export const formatClock = (d: Date) => `${pad(d.getHours())}:${pad(d.getMinutes())}`;

/** "Kamis 23:00" / "Thursday 23:00". */
export function formatResetDay(d: Date, lang: 'en' | 'id'): string {
  const weekday = d.toLocaleDateString(lang === 'id' ? 'id-ID' : 'en-US', { weekday: 'long' });
  return `${weekday} ${formatClock(d)}`;
}

/** Localized weekday names indexed 0 = Sunday … 6 = Saturday (for a select). */
export function weekdayNames(lang: 'en' | 'id'): string[] {
  const base = new Date(2024, 0, 7); // a Sunday
  return Array.from({ length: 7 }, (_, i) =>
    new Date(base.getFullYear(), base.getMonth(), base.getDate() + i).toLocaleDateString(lang === 'id' ? 'id-ID' : 'en-US', { weekday: 'long' })
  );
}
