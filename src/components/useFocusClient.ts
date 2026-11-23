import { useCallback, useEffect, useRef, useState } from "react";
import type { AndroidTimerStatus, FocusBackend, RuntimeInfo, Snapshot } from "../types";
import { resolveLocale, translate, type MessageKey, type Translator } from "../i18n";

export type Feedback = { kind: "error" | "info"; key: MessageKey };
export const isActive = (snapshot: Snapshot) => snapshot.timer.status === "running" || snapshot.timer.status === "paused";
export const isDesktop = (runtime: RuntimeInfo | null) => runtime !== null && ["macos", "windows", "linux"].includes(runtime.platform);
export function primaryLabel(snapshot: Snapshot): MessageKey {
  const { timer, cycleCompleted } = snapshot;
  if (timer.status === "running") return "pause";
  if (timer.status === "paused") return "resume";
  if (timer.status === "completed") return timer.mode === "focus" ? (cycleCompleted >= 4 ? "startLongBreak" : "startShortBreak") : "nextFocus";
  return timer.mode === "focus" ? "startFocus" : "startBreak";
}
export function statusLabel(snapshot: Snapshot): MessageKey {
  if (snapshot.timer.status === "running") return snapshot.timer.mode === "focus" ? "inFocus" : "inBreak";
  return ({ idle: "readyLabel", paused: "pausedLabel", completed: "completedLabel" } as const)[snapshot.timer.status];
}
export async function performPrimaryAction(api: FocusBackend, current: Snapshot, accept: (snapshot: Snapshot) => void): Promise<Snapshot> {
  if (current.timer.status === "running") return api.dispatch({ type: "pause" });
  if (current.timer.status === "paused") return api.dispatch({ type: "resume" });
  if (current.timer.status === "completed") {
    const mode = current.timer.mode === "focus" ? (current.cycleCompleted >= 4 ? "longBreak" : "shortBreak") : "focus";
    accept(await api.dispatch({ type: "switchMode", mode }));
  }
  return api.dispatch({ type: "start" });
}
function operationError(error: unknown): MessageKey {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes("exact_alarm_permission_required")) return "exactAlarmPermissionRequired";
  if (message.includes("timer_active")) return "timerActive";
  if (/invalid_duration|duration/.test(message)) return "invalidDuration";
  return "operationError";
}

/** Every surface consumes the same authoritative snapshots. This hook never advances a clock. */
export function useFocusClient(api: FocusBackend, surface: "main" | "mini" | "tray") {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const latest = useRef<Snapshot | null>(null);
  const mounted = useRef(true);
  const queue = useRef<Promise<void>>(Promise.resolve());
  const [runtime, setRuntime] = useState<RuntimeInfo | null>(null);
  const [androidStatus, setAndroidStatus] = useState<AndroidTimerStatus | null>(null);
  const [pending, setPending] = useState(0);
  const [attempt, setAttempt] = useState(0);
  const [loadFailed, setLoadFailed] = useState(false);
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const [announcementKey, setAnnouncementKey] = useState<MessageKey | null>(null);
  const previousStatus = useRef<string | null>(null);
  const locale = resolveLocale(snapshot?.preferences.language ?? "system");
  const t: Translator = useCallback((key, params) => translate(locale, key, params), [locale]);

  const accept = useCallback((next: Snapshot) => {
    if (!mounted.current || (latest.current && next.revision < latest.current.revision)) return;
    latest.current = next;
    setSnapshot(next);
    setLoadFailed(false);
  }, []);
  const run = useCallback((work: () => Promise<Snapshot | void>, blocking = true) => {
    if (blocking) { setPending(value => value + 1); setFeedback(null); }
    const job = queue.current.then(async () => {
      try {
        const result = await work();
        if (result) accept(result);
      } catch (error) {
        if (mounted.current) setFeedback({ kind: "error", key: operationError(error) });
      } finally {
        if (blocking && mounted.current) setPending(value => Math.max(0, value - 1));
      }
    });
    queue.current = job.catch(() => {});
    return job;
  }, [accept]);

  useEffect(() => {
    mounted.current = true;
    let cancelled = false;
    let unsubscribe: (() => void) | undefined;
    setLoadFailed(false);
    const connect = async () => {
      try {
        const listener = await api.subscribe(value => { if (!cancelled) accept(value); });
        if (cancelled) { listener(); return; }
        unsubscribe = listener;
        const value = await api.getSnapshot();
        if (!cancelled) accept(value);
      } catch {
        if (!cancelled) setLoadFailed(true);
      }
    };
    const inspectRuntime = async () => {
      try {
        const info = await api.getRuntimeInfo();
        if (!cancelled) setRuntime(info);
      } catch {
        if (!cancelled) setFeedback({ kind: "error", key: "operationError" });
      }
    };
    void Promise.all([connect(), inspectRuntime()]);
    return () => { cancelled = true; mounted.current = false; unsubscribe?.(); };
  }, [api, accept, attempt]);

  const refreshAndroidStatus = useCallback(async () => {
    if (runtime?.platform !== "android") return;
    const status = await api.getAndroidTimerStatus();
    if (mounted.current) setAndroidStatus(status);
  }, [api, runtime?.platform]);
  useEffect(() => {
    if (runtime?.platform !== "android") return;
    const refresh = () => {
      if (document.visibilityState !== "hidden") void refreshAndroidStatus().catch(() => {});
    };
    refresh();
    document.addEventListener("visibilitychange", refresh);
    window.addEventListener("focus", refresh);
    return () => {
      document.removeEventListener("visibilitychange", refresh);
      window.removeEventListener("focus", refresh);
    };
  }, [runtime?.platform, refreshAndroidStatus]);

  useEffect(() => {
    if (feedback?.kind !== "info") return;
    const timeout = window.setTimeout(() => setFeedback(current => current === feedback ? null : current), 5500);
    return () => window.clearTimeout(timeout);
  }, [feedback]);
  useEffect(() => {
    document.documentElement.lang = locale;
    document.documentElement.dataset.surface = surface;
    const preference = snapshot?.preferences.theme ?? "system";
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const theme = preference === "system" ? (query.matches ? "dark" : "light") : preference;
      document.documentElement.dataset.theme = theme;
      document.documentElement.style.colorScheme = theme;
    };
    apply();
    query.addEventListener("change", apply);
    return () => query.removeEventListener("change", apply);
  }, [locale, surface, snapshot?.preferences.theme]);
  useEffect(() => {
    if (!snapshot) return;
    const status = snapshot.timer.status;
    const previous = previousStatus.current;
    previousStatus.current = status;
    if (previous === null || previous === status) return;
    const key: MessageKey = status === "completed" ? "completeAnnouncement" : status === "paused" ? "pauseAnnouncement" : status === "running" ? (previous === "paused" ? "resumeAnnouncement" : "startAnnouncement") : "resetAnnouncement";
    setAnnouncementKey(key);
  }, [snapshot]);

  return {
    snapshot, latest, runtime, androidStatus, refreshAndroidStatus,
    accept, run, busy: pending > 0, loadFailed, retry: () => setAttempt(value => value + 1),
    feedback, setFeedback, announcement: announcementKey ? t(announcementKey) : "", locale, t,
  };
}
