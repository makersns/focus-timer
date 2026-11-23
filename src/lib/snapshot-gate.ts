import type { Snapshot } from "../types";

/** Commands and events can cross in flight. Never render an older native revision. */
export class SnapshotGate {
  private current: Snapshot | null = null;
  accept(next: Snapshot): Snapshot {
    if (!this.current || next.revision >= this.current.revision) this.current = next;
    return this.current;
  }
  get latest(): Snapshot | null { return this.current; }
}
