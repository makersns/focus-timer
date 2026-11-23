import { describe, expect, it } from "vitest";
import { PreviewEngine } from "../src/lib/preview-engine";
import { elapsedFraction } from "../src/lib/progress";

describe("elapsed ring with a countdown timer", () => {
  it("grows as digits count down, freezes while paused, fills on completion, and clears on reset", () => {
    const start = 1_700_000_000_000;
    const engine = new PreviewEngine(start);
    engine.dispatch({ type: "setDuration", minutes: 1 }, start);
    expect(elapsedFraction(engine.snapshot().timer)).toBe(0);
    engine.dispatch({ type: "start" }, start);
    engine.tick(start + 15_000);
    expect(engine.snapshot().timer.remainingMs).toBe(45_000);
    expect(elapsedFraction(engine.snapshot().timer)).toBe(.25);
    engine.dispatch({ type: "pause" }, start + 15_000);
    engine.tick(start + 300_000);
    expect(engine.snapshot().timer.remainingMs).toBe(45_000);
    expect(elapsedFraction(engine.snapshot().timer)).toBe(.25);
    engine.dispatch({ type: "resume" }, start + 300_000);
    engine.tick(start + 345_000);
    expect(engine.snapshot().timer.remainingMs).toBe(0);
    expect(elapsedFraction(engine.snapshot().timer)).toBe(1);
    engine.dispatch({ type: "reset" }, start + 346_000);
    expect(engine.snapshot().timer.remainingMs).toBe(60_000);
    expect(elapsedFraction(engine.snapshot().timer)).toBe(0);
    expect(engine.snapshot().history).toHaveLength(1);
  });
});
