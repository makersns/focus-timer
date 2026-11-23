import type { MiniTimerStyle } from "../types";

/** Original decorative vectors. Clock text and every control remain real accessible elements. */
export function WidgetArtwork({ variant, progress = 0 }: { variant: MiniTimerStyle; progress?: number }) {
  const fraction = Math.max(0, Math.min(1, progress));
  const tomato = variant === "tomato";
  const viewBox = tomato ? "0 0 240 260" : variant === "capsule" ? "0 0 320 92" : "0 0 240 240";
  const shape = tomato
    ? <path d="M120 67C176 62 210 96 213 140C219 195 181 233 120 233C59 233 22 196 27 140C29 97 64 64 120 67Z" pathLength={100} />
    : variant === "capsule" ? <circle cx="47" cy="46" r="28" pathLength={100} />
    : <circle cx="120" cy="120" r="107" pathLength={100} />;
  return <div className={"widget-artwork widget-artwork-" + variant} aria-hidden="true">
    <img className="widget-background widget-background-light" src={"/widgets/" + variant + "-light.svg"} alt="" draggable={false} />
    <img className="widget-background widget-background-dark" src={"/widgets/" + variant + "-dark.svg"} alt="" draggable={false} />
    <svg viewBox={viewBox} className={"widget-progress-art " + (tomato ? "" : "widget-progress-circle")}>
      <g className="widget-ring-track">{shape}</g>
      <g className="widget-ring-value" strokeDasharray={fraction >= 1 ? undefined : 100 * fraction + " 100"} opacity={fraction > 0 ? 1 : 0}>{shape}</g>
    </svg>
  </div>;
}
