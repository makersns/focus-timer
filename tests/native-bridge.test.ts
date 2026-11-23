import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Snapshot } from "../src/types";
import { PreviewEngine } from "../src/lib/preview-engine";

const native = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: native.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: native.listen }));

beforeEach(() => {
  vi.resetModules();
  native.invoke.mockReset();
  native.listen.mockReset();
});

describe("native bridge with concurrent surfaces", () => {
  it("keeps the newer shared event when an older command finishes late, and drops stale events", async () => {
    let receive: ((event: { payload: Snapshot }) => void) | undefined;
    let finishOld: ((snapshot: Snapshot) => void) | undefined;
    const unsubscribe = vi.fn();
    native.listen.mockImplementation((_name: string, handler: typeof receive) => {
      receive = handler;
      return Promise.resolve(unsubscribe);
    });
    native.invoke.mockReturnValue(new Promise<Snapshot>(resolve => { finishOld = resolve; }));
    const { backend } = await import("../src/lib/backend");
    const listener = vi.fn();
    const stop = await backend.subscribe(listener);
    const pending = backend.getSnapshot();
    const initial = new PreviewEngine(1_700_000_000_000).snapshot();
    const older = { ...initial, revision: 10 };
    const newer = { ...initial, revision: 11, timer: { ...initial.timer, intent: "Shared native round" } };
    receive!({ payload: newer });
    finishOld!(older);
    expect(await pending).toBe(newer);
    receive!({ payload: older });
    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenLastCalledWith(newer);

    // A late action response must also preserve the already rendered revision.
    native.invoke.mockResolvedValue(older);
    expect(await backend.dispatch({ type: "pause" })).toBe(newer);
    stop();
    expect(unsubscribe).toHaveBeenCalledOnce();
  });
});
