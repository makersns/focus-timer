import type { DailyStat, Preferences, SessionRecord, Snapshot, TimerAction, TimerMode } from "../types";

export const DEFAULT_PREFERENCES: Preferences = {
  durations: { focus: 25, shortBreak: 5, longBreak: 15 },
  language: "system", theme: "system", notificationsEnabled: false, closeToTray: true,
  miniTimerEnabled: false, miniTimerAlwaysOnTop: true,
};
const MODES: TimerMode[] = ["focus", "shortBreak", "longBreak"];
const LIMIT_MS = 180 * 60_000;
const MAX_DATE = 8_640_000_000_000_000;
const clone = <T>(value: T): T => structuredClone(value);
const record = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);
const integer = (value: unknown, min: number, max: number): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= min && value <= max;
const timestamp = (value: unknown) => integer(value, 0, MAX_DATE);
const shortString = (value: unknown, max: number) =>
  typeof value === "string" && [...value].length <= max;

export function validPreferences(value: unknown): value is Preferences {
  if (!record(value) || !record(value.durations)) return false;
  return MODES.every(mode => integer((value.durations as Record<string, unknown>)[mode], 1, 180))
    && ["system", "zh-CN", "zh-TW", "en", "ja", "ko"].includes(String(value.language))
    && ["system", "light", "dark"].includes(String(value.theme))
    && typeof value.notificationsEnabled === "boolean" && typeof value.closeToTray === "boolean"
    && typeof value.miniTimerEnabled === "boolean" && typeof value.miniTimerAlwaysOnTop === "boolean";
}
function validHistory(value: unknown): value is SessionRecord {
  if (!record(value)) return false;
  return shortString(value.id, 128) && (value.id as string).length > 0
    && MODES.includes(value.mode as TimerMode) && shortString(value.intent, 120)
    && timestamp(value.startedAt) && timestamp(value.finishedAt)
    && integer(value.durationMs, 60_000, LIMIT_MS)
    && integer(value.elapsedMs, 0, value.durationMs as number)
    && ["completed", "ended"].includes(String(value.outcome))
    && (value.outcome !== "completed" || value.elapsedMs === value.durationMs);
}
function validDaily(value: unknown): value is DailyStat {
  if (!record(value)) return false;
  return typeof value.date === "string" && /^\d{4}-\d{2}-\d{2}$/.test(value.date)
    && integer(value.focusMs, 0, Number.MAX_SAFE_INTEGER)
    && integer(value.completedSessions, 0, Number.MAX_SAFE_INTEGER)
    && integer(value.endedSessions, 0, Number.MAX_SAFE_INTEGER);
}
export function decodePreview(raw: string): Snapshot {
  if (raw.length > 2_000_000) throw new Error("invalid_storage");
  const value: unknown = JSON.parse(raw);
  // New desktop-view preferences are additive within schema v1.
  if (record(value) && record(value.preferences)) {
    if (!("miniTimerEnabled" in value.preferences)) value.preferences.miniTimerEnabled = false;
    if (!("miniTimerAlwaysOnTop" in value.preferences)) value.preferences.miniTimerAlwaysOnTop = true;
  }
  if (!record(value) || value.version !== 1 || !record(value.timer) || !validPreferences(value.preferences))
    throw new Error("invalid_storage");
  const timer = value.timer;
  if (!integer(value.revision, 0, Number.MAX_SAFE_INTEGER) ||
    !MODES.includes(timer.mode as TimerMode) ||
    !["idle", "running", "paused", "completed"].includes(String(timer.status)) ||
    !integer(timer.durationMs, 60_000, LIMIT_MS) ||
    !integer(timer.remainingMs, 0, timer.durationMs as number) ||
    !(timer.deadlineAt === null || timestamp(timer.deadlineAt)) ||
    !(timer.startedAt === null || timestamp(timer.startedAt)) ||
    !timestamp(timer.updatedAt) || !shortString(timer.intent, 120) ||
    !(timer.runId === null || shortString(timer.runId, 128)) ||
    !Array.isArray(value.history) || value.history.length > 500 ||
    !value.history.every(validHistory) ||
    !Array.isArray(value.dailyStats) || value.dailyStats.length > 366 ||
    !value.dailyStats.every(validDaily) ||
    !integer(value.cycleCompleted, 0, 4) ||
    typeof value.persistenceError !== "boolean" || typeof value.restoredPaused !== "boolean")
    throw new Error("invalid_storage");
  if ((timer.status === "running" || timer.status === "paused") &&
    (timer.startedAt === null || !timer.runId || (timer.status === "running" && timer.deadlineAt === null)))
    throw new Error("invalid_storage");
  if (timer.status === "completed" && timer.remainingMs !== 0) throw new Error("invalid_storage");
  if (new Set(value.history.map(item => (item as SessionRecord).id)).size !== value.history.length ||
      new Set(value.dailyStats.map(item => (item as DailyStat).date)).size !== value.dailyStats.length)
    throw new Error("invalid_storage");
  return clone(value as unknown as Snapshot);
}
function localDate(now: number): string {
  const date = new Date(now);
  return [date.getFullYear(), String(date.getMonth() + 1).padStart(2, "0"),
    String(date.getDate()).padStart(2, "0")].join("-");
}
function initial(now: number): Snapshot {
  return {
    version: 1, revision: 0,
    timer: { mode: "focus", status: "idle", durationMs: 25 * 60_000,
      remainingMs: 25 * 60_000, deadlineAt: null, startedAt: null,
      updatedAt: now, intent: "", runId: null },
    preferences: clone(DEFAULT_PREFERENCES), history: [], dailyStats: [],
    cycleCompleted: 0, persistenceError: false, restoredPaused: false,
  };
}
export interface PreviewTransition { snapshot: Snapshot; persist: boolean; completed: SessionRecord | null }

/** Development/browser preview only. Packaged apps always use the Rust engine. */
export class PreviewEngine {
  private state: Snapshot;
  private checkpointAt: number;
  constructor(now: number, raw?: string, private readonly id: () => string = () => crypto.randomUUID()) {
    this.state = raw ? decodePreview(raw) : initial(now);
    this.checkpointAt = now;
    this.state.persistenceError = false;
    this.state.restoredPaused = false;
    if (this.state.timer.status === "running") {
      this.state.timer.status = "paused";
      this.state.timer.deadlineAt = null;
      this.state.timer.updatedAt = now;
      this.state.restoredPaused = true;
      this.state.revision += 1;
    }
  }
  snapshot(): Snapshot { return clone(this.state); }
  serialize(): string { return JSON.stringify(this.state); }
  setPersistenceError(value: boolean): void {
    if (this.state.persistenceError !== value) { this.state.persistenceError = value; this.state.revision++; }
  }
  private changed(now: number): void { this.state.timer.updatedAt = now; this.state.revision++; }
  private transition(persist: boolean, completed: SessionRecord | null = null): PreviewTransition {
    return { snapshot: this.snapshot(), persist, completed };
  }
  private record(now: number, outcome: SessionRecord["outcome"]): SessionRecord | null {
    const t = this.state.timer;
    const elapsedMs = outcome === "completed" ? t.durationMs : t.durationMs - t.remainingMs;
    if ((outcome === "ended" && t.mode !== "focus") || !t.runId || t.startedAt === null || elapsedMs <= 0 ||
      this.state.history.some(item => item.id === t.runId)) return null;
    const entry: SessionRecord = {
      id: t.runId, mode: t.mode, intent: t.intent, startedAt: t.startedAt,
      finishedAt: Math.max(t.startedAt, now), durationMs: t.durationMs, elapsedMs, outcome,
    };
    this.state.history.unshift(entry);
    this.state.history = this.state.history.slice(0, 500);
    if (t.mode === "focus") {
      const date = localDate(entry.finishedAt);
      let day = this.state.dailyStats.find(item => item.date === date);
      if (!day) {
        day = { date, focusMs: 0, completedSessions: 0, endedSessions: 0 };
        this.state.dailyStats.push(day);
      }
      day.focusMs += elapsedMs;
      if (outcome === "completed") {
        day.completedSessions++;
        this.state.cycleCompleted = Math.min(4, this.state.cycleCompleted + 1);
      } else day.endedSessions++;
      this.state.dailyStats.sort((a, b) => b.date.localeCompare(a.date));
      this.state.dailyStats = this.state.dailyStats.slice(0, 366);
    } else if (t.mode === "longBreak" && outcome === "completed") this.state.cycleCompleted = 0;
    return entry;
  }
  tick(now: number): PreviewTransition {
    const t = this.state.timer;
    if (t.status !== "running" || t.deadlineAt === null) return this.transition(false);
    if (now < t.updatedAt) {
      const rewind = t.updatedAt - now;
      t.deadlineAt = now + t.remainingMs;
      if (t.startedAt !== null) t.startedAt = Math.max(0, t.startedAt - rewind);
    }
    const next = Math.min(t.remainingMs, Math.max(0, t.deadlineAt - now));
    if (next === t.remainingMs && now === t.updatedAt) return this.transition(false);
    t.remainingMs = next;
    this.changed(now);
    if (next === 0) {
      const finishedAt = t.deadlineAt;
      t.status = "completed"; t.deadlineAt = null;
      const completed = this.record(finishedAt, "completed");
      this.checkpointAt = now;
      return this.transition(true, completed);
    }
    const persist = now - this.checkpointAt >= 30_000 || now < this.checkpointAt;
    if (persist) this.checkpointAt = now;
    return this.transition(persist);
  }
  dispatch(action: TimerAction, now: number): PreviewTransition {
    const tick = this.tick(now);
    const t = this.state.timer;
    const active = t.status === "running" || t.status === "paused";
    switch (action.type) {
      case "start":
        if (active) throw new Error("timer_active");
        t.status = "running"; t.remainingMs = t.durationMs; t.deadlineAt = now + t.durationMs;
        t.startedAt = now; t.runId = this.id(); this.state.restoredPaused = false;
        break;
      case "pause":
        if (t.status !== "running") return tick;
        t.status = "paused"; t.deadlineAt = null;
        break;
      case "resume":
        if (t.status !== "paused") return tick;
        t.status = "running"; t.deadlineAt = now + t.remainingMs; this.state.restoredPaused = false;
        break;
      case "end":
      case "reset":
        if (active) this.record(now, "ended");
        t.status = "idle"; t.remainingMs = t.durationMs; t.deadlineAt = null;
        t.startedAt = null; t.runId = null; this.state.restoredPaused = false;
        break;
      case "switchMode":
        if (active) throw new Error("timer_active");
        if (!MODES.includes(action.mode)) throw new Error("invalid_mode");
        t.mode = action.mode; t.durationMs = this.state.preferences.durations[action.mode] * 60_000;
        t.remainingMs = t.durationMs; t.status = "idle"; t.deadlineAt = null;
        t.startedAt = null; t.runId = null; this.state.restoredPaused = false;
        break;
      case "setDuration":
        if (active) throw new Error("timer_active");
        if (!integer(action.minutes, 1, 180)) throw new Error("invalid_duration");
        t.durationMs = action.minutes * 60_000; t.remainingMs = t.durationMs;
        t.status = "idle"; t.deadlineAt = null; t.startedAt = null; t.runId = null;
        this.state.restoredPaused = false;
        break;
      case "setIntent":
        if (!shortString(action.intent, 120)) throw new Error("invalid_intent");
        t.intent = action.intent;
        break;
    }
    this.changed(now); this.checkpointAt = now;
    return this.transition(true, tick.completed);
  }
  updatePreferences(preferences: Preferences, now: number): PreviewTransition {
    if (!validPreferences(preferences)) throw new Error("invalid_preferences");
    const tick = this.tick(now);
    const previousDuration = this.state.preferences.durations[this.state.timer.mode];
    this.state.preferences = clone(preferences);
    const t = this.state.timer;
    // A language/theme change must never replace a custom round duration.
    if ((t.status === "idle" || t.status === "completed") &&
        previousDuration !== preferences.durations[t.mode]) {
      t.durationMs = preferences.durations[t.mode] * 60_000;
      if (t.status === "idle") t.remainingMs = t.durationMs;
    }
    this.changed(now); this.checkpointAt = now;
    return this.transition(true, tick.completed);
  }
  clearHistory(now: number): PreviewTransition {
    const tick = this.tick(now);
    this.state.history = []; this.state.dailyStats = [];
    this.changed(now); this.checkpointAt = now;
    return this.transition(true, tick.completed);
  }
}
