import type { TimerState } from "../types";

/** Digits count down; the colored ring grows with elapsed time on every surface. */
export function elapsedFraction(timer: Pick<TimerState, "durationMs" | "remainingMs">): number {
  return Math.max(0, Math.min(1, (timer.durationMs - timer.remainingMs) / Math.max(1, timer.durationMs)));
}
