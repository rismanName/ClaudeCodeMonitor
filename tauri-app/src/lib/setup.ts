import { invoke } from '@tauri-apps/api/core';
import { isEnabled as isAutostartEnabled, enable as enableAutostart } from '@tauri-apps/plugin-autostart';
import type { TranslationKey } from '$lib/i18n';

export interface SetupStatus {
  engineFound: boolean;
  engineRunning: boolean;
  prometheusUp: boolean;
  otlpUp: boolean;
  claudeDirExists: boolean;
  statsCacheExists: boolean;
}

export interface SetupSnapshot {
  status: SetupStatus | null;
  telemetryOn: boolean | null;
  autostartOn: boolean | null;
}

export type StepId = 'engine' | 'stack' | 'telemetry' | 'claude' | 'autostart';

export interface SetupStep {
  id: StepId;
  /** null = could not be determined. */
  ok: boolean | null;
  required: boolean;
  title: TranslationKey;
  hint: TranslationKey;
}

const SKIP_KEY = 'onboarding-skipped';

export async function loadSetup(): Promise<SetupSnapshot> {
  let status: SetupStatus | null = null;
  let telemetryOn: boolean | null = null;
  let autostartOn: boolean | null = null;
  try { status = await invoke<SetupStatus>('get_setup_status'); } catch { /* unknown */ }
  try { telemetryOn = (await invoke<{ enabled: boolean }>('get_telemetry_status')).enabled; } catch { /* unknown */ }
  try { autostartOn = await isAutostartEnabled(); } catch { /* unknown */ }
  return { status, telemetryOn, autostartOn };
}

/** Ordered first-run steps. The stack step needs the engine running AND both ports answering. */
export function setupSteps(s: SetupSnapshot): SetupStep[] {
  const st = s.status;
  return [
    { id: 'engine', ok: st ? st.engineFound : null, required: true, title: 'setup.engine', hint: 'setup.engine.hint' },
    { id: 'stack', ok: st ? st.engineRunning && st.prometheusUp && st.otlpUp : null, required: true, title: 'onb.stack', hint: 'onb.stack.hint' },
    { id: 'telemetry', ok: s.telemetryOn, required: true, title: 'setup.telemetry', hint: 'setup.telemetry.hint' },
    { id: 'claude', ok: st ? st.claudeDirExists : null, required: true, title: 'setup.claudeDir', hint: 'setup.claudeDir.hint' },
    { id: 'autostart', ok: s.autostartOn, required: false, title: 'setup.autostart', hint: 'setup.autostart.hint' },
  ];
}

export const missingRequired = (s: SetupSnapshot) => setupSteps(s).filter((x) => x.required && x.ok === false);

export function onboardingSkipped(): boolean {
  try { return localStorage.getItem(SKIP_KEY) === '1'; } catch { return false; }
}
export function skipOnboarding() {
  try { localStorage.setItem(SKIP_KEY, '1'); } catch { /* ignore */ }
}

/** True when the first-run wizard should open by itself: a required step fails and the user has not skipped it. */
export async function shouldAutoOnboard(): Promise<boolean> {
  if (onboardingSkipped()) return false;
  return missingRequired(await loadSetup()).length > 0;
}

/** Each fixer returns '' on success or a message to show. */
export async function fixStack(composeDir: string | null, errorText: (code: string) => string): Promise<string> {
  try {
    const r = await invoke<{ state: string; code: string; detail: string }>('ensure_monitoring_stack', { composeDir: composeDir || null });
    return r.state === 'error' ? `${errorText(r.code)}${r.detail ? ` (${r.detail})` : ''}` : '';
  } catch (e) { return String(e); }
}
export async function fixTelemetry(): Promise<string> {
  try { await invoke('enable_claude_telemetry'); return ''; } catch (e) { return String(e); }
}
export async function fixAutostart(): Promise<string> {
  try { await enableAutostart(); return ''; } catch (e) { return String(e); }
}
