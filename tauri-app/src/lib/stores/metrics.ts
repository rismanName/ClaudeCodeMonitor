import { writable } from 'svelte/store';
import type { DashboardMetrics, TimeRange, CustomTimeRange } from '$lib/types';

export const metrics = writable<DashboardMetrics | null>(null);
export const isLoading = writable(false);
export const error = writable<string | null>(null);
export const timeRange = writable<TimeRange>('1d');
export const customTimeRange = writable<CustomTimeRange | null>(null);
export const isConnected = writable(false);
export const lastUpdated = writable<Date | null>(null);
/** YYYY-MM-DD: when set, Session History shows only that local day (set from the activity heatmap). */
export const sessionDateFilter = writable<string | null>(null);
/** One-shot navigation request consumed by +page.svelte. */
export const requestedView = writable<string | null>(null);
/** Drill-down from Projects: Session History shows only this project's sessions (key = projectPath ?? project name). */
export const sessionProjectFilter = writable<{ key: string; label: string } | null>(null);
/** View id to return to (Esc / back link) after a drill-down; set together with sessionProjectFilter. */
export const drillOrigin = writable<string | null>(null);
export const totalCost = writable<number>(0);
/** First-run setup wizard visibility (opened automatically on launch, or from Setup Status). */
export const onboardingOpen = writable(false);
