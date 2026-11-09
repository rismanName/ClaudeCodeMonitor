import type { TranslationKey } from '$lib/i18n';

export type ViewId =
  | 'summary' | 'budget'
  | 'projects' | 'sessions' | 'tokens'
  | 'insights' | 'stats-cache'
  | 'setup' | 'prometheus-health' | 'smoke-test';

export interface NavView { id: ViewId; label: TranslationKey; desc: TranslationKey }
export interface NavSection { id: string; icon: string; label: TranslationKey; desc: TranslationKey; views: NavView[] }

/** Four task-based sections; each holds the views shown as tabs inside it. */
export const sections: NavSection[] = [
  {
    id: 'today', icon: 'grid', label: 'nav.today', desc: 'nav.today.desc',
    views: [
      { id: 'summary', label: 'nav.dashboard', desc: 'nav.dashboard.desc' },
      { id: 'budget', label: 'nav.budget', desc: 'nav.budget.desc' },
    ],
  },
  {
    id: 'explore', icon: 'folder', label: 'nav.explore', desc: 'nav.explore.desc',
    views: [
      { id: 'projects', label: 'nav.projects', desc: 'nav.projects.desc' },
      { id: 'sessions', label: 'nav.sessions', desc: 'nav.sessions.desc' },
      { id: 'tokens', label: 'nav.tokens', desc: 'nav.tokens.desc' },
    ],
  },
  {
    id: 'insights', icon: 'lightbulb', label: 'nav.insights', desc: 'nav.insights.desc',
    views: [
      { id: 'insights', label: 'nav.insights', desc: 'nav.insights.desc' },
      { id: 'stats-cache', label: 'nav.statsCache', desc: 'nav.statsCache.desc' },
    ],
  },
  {
    id: 'system', icon: 'heartbeat', label: 'nav.system', desc: 'nav.system.desc',
    views: [
      { id: 'setup', label: 'nav.setup', desc: 'nav.setup.desc' },
      { id: 'prometheus-health', label: 'nav.health', desc: 'nav.health.desc' },
      { id: 'smoke-test', label: 'nav.smoke', desc: 'nav.smoke.desc' },
    ],
  },
];

export function sectionOf(view: string): NavSection {
  return sections.find((s) => s.views.some((v) => v.id === view)) ?? sections[0];
}
