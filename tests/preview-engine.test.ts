import { describe, expect, it } from "vitest";
import { PreviewEngine, decodePreview, DEFAULT_PREFERENCES } from "../src/lib/preview-engine";
import { SnapshotGate } from "../src/lib/snapshot-gate";

const T0 = new Date("2026-11-20T12:00:00Z").getTime();
function engine(minutes = 1) {
  let sequence = 0;
  const e = new PreviewEngine(T0, undefined, () => "test-" + (++sequence));
  e.dispatch({ type: "setDuration", minutes }, T0);
  return e;
}
function started(minutes = 1) {
  const e = engine(minutes); e.dispatch({ type: "start" }, T0); return e;
}

describe("browser preview obeys native timer behavior", () => {
  it("uses the deadline after delayed ticks and completes exactly one round", () => {
    const e = started();
    const result = e.tick(T0 + 3_600_000);
    expect(result.completed?.finishedAt).toBe(T0 + 60_000);
    expect(result.snapshot.timer.status).toBe("completed");
    expect(result.snapshot.history).toHaveLength(1);
    expect(result.snapshot.dailyStats[0].focusMs).toBe(60_000);
    e.tick(T0 + 7_200_000);
    expect(e.snapshot().history).toHaveLength(1);
    expect(e.snapshot().cycleCompleted).toBe(1);
  });
  it("excludes paused time from the remaining time and early-ended total", () => {
    const e = started();
    e.dispatch({ type: "pause" }, T0 + 10_000);
    expect(e.snapshot().timer.remainingMs).toBe(50_000);
    e.tick(T0 + 3_600_000);
    expect(e.snapshot().timer.remainingMs).toBe(50_000);
    e.dispatch({ type: "resume" }, T0 + 3_600_000);
    e.dispatch({ type: "end" }, T0 + 3_610_000);
    expect(e.snapshot().history[0].elapsedMs).toBe(20_000);
    expect(e.snapshot().dailyStats[0]).toMatchObject({ completedSessions: 0, endedSessions: 1 });
  });
  it("does not increase remaining time when the wall clock goes backwards", () => {
    const e = started();
    e.tick(T0 + 20_000);
    e.tick(T0 - 10_000);
    expect(e.snapshot().timer.remainingMs).toBe(40_000);
    e.tick(T0);
    expect(e.snapshot().timer.remainingMs).toBe(30_000);
  });
  it("completes on an exact-expiry pause and does not record a second partial round", () => {
    const e = started();
    e.dispatch({ type: "pause" }, T0 + 60_000);
    expect(e.snapshot().timer.status).toBe("completed");
    e.dispatch({ type: "reset" }, T0 + 60_001);
    expect(e.snapshot().history).toHaveLength(1);
    expect(e.snapshot().history[0].outcome).toBe("completed");
  });
  it("rejects switching active modes and invalid durations without replacing the timer", () => {
    const e = started();
    expect(() => e.dispatch({ type: "switchMode", mode: "shortBreak" }, T0)).toThrow("timer_active");
    e.dispatch({ type: "pause" }, T0 + 10);
    expect(() => e.dispatch({ type: "setDuration", minutes: 3 }, T0 + 10)).toThrow("timer_active");
    e.dispatch({ type: "end" }, T0 + 10);
    for (const minutes of [0, -1, 1.2, 181, Number.NaN, Number.POSITIVE_INFINITY])
      expect(() => e.dispatch({ type: "setDuration", minutes }, T0 + 10)).toThrow("invalid_duration");
  });
  it("keeps custom and active rounds when unrelated preferences change", () => {
    const e = engine(17);
    const preferences = structuredClone(DEFAULT_PREFERENCES);
    preferences.language = "ja"; preferences.theme = "dark";
    e.updatePreferences(preferences, T0);
    expect(e.snapshot().timer.durationMs).toBe(17 * 60_000);
    e.dispatch({ type: "start" }, T0);
    preferences.durations.focus = 50;
    e.updatePreferences(preferences, T0 + 10_000);
    expect(e.snapshot().timer.durationMs).toBe(17 * 60_000);
    expect(e.snapshot().timer.remainingMs).toBe(17 * 60_000 - 10_000);
  });
  it("keeps the completion screen when default duration changes", () => {
    const e = started(); e.tick(T0 + 60_000);
    const preferences = structuredClone(DEFAULT_PREFERENCES);
    preferences.durations.focus = 30;
    e.updatePreferences(preferences, T0 + 60_001);
    expect(e.snapshot().timer).toMatchObject({ status: "completed", remainingMs: 0, durationMs: 30 * 60_000 });
    e.dispatch({ type: "start" }, T0 + 60_002);
    expect(e.snapshot().timer.remainingMs).toBe(30 * 60_000);
  });
  it("restores a running checkpoint as paused without counting shutdown time", () => {
    const e = started(); e.tick(T0 + 15_000);
    const restored = new PreviewEngine(T0 + 8_000_000, e.serialize());
    expect(restored.snapshot().timer).toMatchObject({ status: "paused", remainingMs: 45_000, deadlineAt: null });
    expect(restored.snapshot().restoredPaused).toBe(true);
    restored.dispatch({ type: "resume" }, T0 + 8_000_000);
    expect(restored.snapshot().restoredPaused).toBe(false);
  });
  it("does not save every tick, but checkpoints at thirty seconds", () => {
    const e = started(25);
    expect(e.tick(T0 + 1_000).persist).toBe(false);
    expect(e.tick(T0 + 29_999).persist).toBe(false);
    expect(e.tick(T0 + 30_000).persist).toBe(true);
    expect(e.tick(T0 + 31_000).persist).toBe(false);
  });
  it("records a completed break separately and omits an early-ended break", () => {
    const e = engine();
    e.dispatch({ type: "switchMode", mode: "shortBreak" }, T0);
    e.dispatch({ type: "setDuration", minutes: 1 }, T0);
    e.dispatch({ type: "start" }, T0);
    e.dispatch({ type: "end" }, T0 + 30_000);
    expect(e.snapshot().history).toHaveLength(0);
    e.dispatch({ type: "start" }, T0 + 30_000);
    e.tick(T0 + 90_000);
    expect(e.snapshot().history[0].mode).toBe("shortBreak");
    expect(e.snapshot().dailyStats).toHaveLength(0);
    expect(e.snapshot().cycleCompleted).toBe(0);
  });
  it("clears history without interrupting the current round or its cycle", () => {
    const e = started(); e.tick(T0 + 60_000);
    e.dispatch({ type: "start" }, T0 + 70_000);
    e.clearHistory(T0 + 80_000);
    expect(e.snapshot().history).toHaveLength(0);
    expect(e.snapshot().dailyStats).toHaveLength(0);
    expect(e.snapshot().timer.status).toBe("running");
    expect(e.snapshot().timer.remainingMs).toBe(50_000);
    expect(e.snapshot().cycleCompleted).toBe(1);
  });
  it("accepts optional unicode intentions and rejects overlong content", () => {
    const e = engine();
    e.dispatch({ type: "setIntent", intent: "🍑".repeat(120) }, T0);
    expect(() => e.dispatch({ type: "setIntent", intent: "🍑".repeat(121) }, T0)).toThrow("invalid_intent");
    e.dispatch({ type: "setIntent", intent: "" }, T0);
    e.dispatch({ type: "start" }, T0);
    expect(e.snapshot().timer.status).toBe("running");
  });
  it("bounds recent history while keeping truthful daily totals", () => {
    const e = engine();
    for (let i = 0; i < 501; i++) {
      const now = T0 + i * 60_000;
      e.dispatch({ type: "start" }, now); e.tick(now + 60_000);
    }
    expect(e.snapshot().history).toHaveLength(500);
    expect(e.snapshot().dailyStats.reduce((sum, day) => sum + day.completedSessions, 0)).toBe(501);
  });
});

describe("storage validation and asynchronous native snapshots", () => {
  it("rejects corrupt, unsupported, and inconsistent persisted state", () => {
    expect(() => decodePreview("{")).toThrow();
    const valid = engine().snapshot();
    for (const invalid of [
      { ...valid, version: 99 },
      { ...valid, timer: { ...valid.timer, remainingMs: -1 } },
      { ...valid, timer: { ...valid.timer, status: "running", startedAt: null } },
      { ...valid, preferences: { ...valid.preferences, durations: { focus: 0, shortBreak: 5, longBreak: 15 } } },
      { ...valid, dailyStats: [{ date: "2026-11-20", focusMs: -1, completedSessions: 0, endedSessions: 0 }] },
    ]) expect(() => decodePreview(JSON.stringify(invalid))).toThrow("invalid_storage");
  });
  it("rejects older command results after a newer event has arrived", () => {
    const gate = new SnapshotGate();
    const initial = engine().snapshot();
    const newer = { ...initial, revision: 12 };
    const older = { ...initial, revision: 11 };
    expect(gate.accept(newer)).toBe(newer);
    expect(gate.accept(older)).toBe(newer);
    expect(gate.latest?.revision).toBe(12);
  });
  it("returns copies so rendering cannot mutate authoritative state", () => {
    const e = started();
    const snapshot = e.snapshot(); snapshot.timer.remainingMs = 0;
    snapshot.preferences.durations.focus = 99;
    expect(e.snapshot().timer.remainingMs).toBe(60_000);
    expect(e.snapshot().preferences.durations.focus).toBe(25);
  });
});

describe("version-1 mini timer preference migration", () => {
  it("adds safe defaults without changing an existing paused custom round or language", () => {
    const e = engine(17);
    const preferences = structuredClone(DEFAULT_PREFERENCES);
    preferences.language = "ja";
    preferences.theme = "dark";
    e.updatePreferences(preferences, T0);
    e.dispatch({ type: "setIntent", intent: "A saved intention" }, T0);
    e.dispatch({ type: "start" }, T0);
    e.dispatch({ type: "pause" }, T0 + 9_000);
    const before = e.snapshot();
    const legacy = JSON.parse(e.serialize());
    delete legacy.preferences.miniTimerEnabled;
    delete legacy.preferences.miniTimerAlwaysOnTop;
    const restored = new PreviewEngine(T0 + 86_400_000, JSON.stringify(legacy)).snapshot();
    expect(restored.preferences).toMatchObject({
      language: "ja", theme: "dark", miniTimerEnabled: false, miniTimerAlwaysOnTop: true,
    });
    expect(restored.timer).toMatchObject({
      status: "paused", durationMs: 17 * 60_000, remainingMs: before.timer.remainingMs,
      intent: "A saved intention", runId: before.timer.runId,
    });
    expect(restored.history).toEqual(before.history);
  });
  it("rejects malformed new preference values instead of treating them as defaults", () => {
    const snapshot = engine().snapshot();
    for (const [key, value] of [
      ["miniTimerEnabled", "true"], ["miniTimerAlwaysOnTop", 1],
    ] as const) {
      expect(() => decodePreview(JSON.stringify({
        ...snapshot, preferences: { ...snapshot.preferences, [key]: value },
      }))).toThrow("invalid_storage");
    }
  });
});

describe("version-1 reminder preferences", () => {
  it("keeps existing audible notifications and a paused round when upgrading old data", () => {
    const e = started();
    e.tick(T0 + 60_000);
    e.dispatch({ type: "start" }, T0 + 70_000);
    e.dispatch({ type: "pause" }, T0 + 80_000);
    const before = e.snapshot();
    const legacy = JSON.parse(e.serialize());
    legacy.preferences.notificationsEnabled = true;
    delete legacy.preferences.soundEnabled;
    const restored = new PreviewEngine(T0 + 86_400_000, JSON.stringify(legacy)).snapshot();
    expect(restored.preferences).toMatchObject({ notificationsEnabled: true, soundEnabled: true });
    expect(restored.timer).toEqual(before.timer);
    expect(restored.history).toEqual(before.history);
    expect(restored.dailyStats).toEqual(before.dailyStats);
  });
  it("preserves an explicit silent reminder and rejects malformed sound preferences", () => {
    const e = started();
    const preferences = { ...e.snapshot().preferences, notificationsEnabled: true, soundEnabled: false };
    e.updatePreferences(preferences, T0 + 10_000);
    expect(e.snapshot().timer.remainingMs).toBe(50_000);
    expect(decodePreview(e.serialize()).preferences.soundEnabled).toBe(false);
    for (const soundEnabled of ["false", 0, null]) {
      expect(() => decodePreview(JSON.stringify({
        ...e.snapshot(), preferences: { ...preferences, soundEnabled },
      }))).toThrow("invalid_storage");
    }
  });
});

describe("widget style migration and live changes", () => {
  it("migrates an older paused timer to ring without changing time, history or reminders", () => {
    const e = started(); e.tick(T0 + 60_000);
    e.dispatch({ type: "start" }, T0 + 70_000); e.dispatch({ type: "pause" }, T0 + 80_000);
    const before = e.snapshot();
    const old = JSON.parse(e.serialize()); delete old.preferences.miniTimerStyle;
    const after = new PreviewEngine(T0 + 90_000, JSON.stringify(old)).snapshot();
    expect(after.preferences.miniTimerStyle).toBe("ring");
    expect(after.timer).toEqual(before.timer);
    expect(after.history).toEqual(before.history);
    expect(after.preferences.notificationsEnabled).toBe(before.preferences.notificationsEnabled);
    expect(after.preferences.soundEnabled).toBe(before.preferences.soundEnabled);
  });
  it("switches all three styles during a round without changing its identity or deadline", () => {
    const e = started(180);
    const before = e.snapshot();
    for (const miniTimerStyle of ["ring", "tomato", "capsule"] as const) {
      e.updatePreferences({ ...e.snapshot().preferences, miniTimerStyle }, T0 + 1_000);
      expect(e.snapshot().preferences.miniTimerStyle).toBe(miniTimerStyle);
      expect(e.snapshot().timer).toMatchObject({ runId: before.timer.runId, deadlineAt: before.timer.deadlineAt, status: "running", remainingMs: 180 * 60_000 - 1_000 });
      expect(e.snapshot().history).toEqual(before.history);
      expect(decodePreview(e.serialize()).preferences.miniTimerStyle).toBe(miniTimerStyle);
    }
  });
  it("rejects a present invalid style instead of silently resetting it", () => {
    for (const miniTimerStyle of ["classic", "", 1, null]) expect(() => decodePreview(JSON.stringify({
      ...engine().snapshot(), preferences: { ...DEFAULT_PREFERENCES, miniTimerStyle },
    }))).toThrow("invalid_storage");
  });
});
