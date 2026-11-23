import { useRef, type KeyboardEvent } from "react";
import { Check } from "lucide-react";
import type { MiniTimerStyle } from "../types";
import { formatClock, type MessageKey, type Translator } from "../i18n";
import { WidgetArtwork } from "./WidgetArtwork";
import { elapsedFraction } from "../lib/progress";
import "../widgets.css";

export const widgetStyles: MiniTimerStyle[] = ["ring", "tomato", "capsule"];
const labels: Record<MiniTimerStyle, MessageKey> = { ring: "widgetRing", tomato: "widgetTomato", capsule: "widgetCapsule" };
const descriptions: Record<MiniTimerStyle, MessageKey> = { ring: "widgetRingHelp", tomato: "widgetTomatoHelp", capsule: "widgetCapsuleHelp" };

export function WidgetStylePicker({ value, disabled = false, t, onChange, remainingMs, durationMs, compact = false }: {
  value: MiniTimerStyle; disabled?: boolean; t: Translator; onChange: (value: MiniTimerStyle) => void;
  remainingMs?: number; durationMs?: number; compact?: boolean;
}) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const progress = remainingMs === undefined ? 0 : elapsedFraction({ remainingMs, durationMs: durationMs ?? remainingMs });
  function navigate(event: KeyboardEvent<HTMLDivElement>) {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(event.key) || disabled) return;
    event.preventDefault();
    const current = widgetStyles.indexOf(value);
    const next = event.key === "Home" ? 0 : event.key === "End" ? widgetStyles.length - 1
      : (current + (["ArrowRight", "ArrowDown"].includes(event.key) ? 1 : -1) + widgetStyles.length) % widgetStyles.length;
    onChange(widgetStyles[next]);
    buttons.current[next]?.focus({ preventScroll: true });
  }
  return <div className={"widget-style-picker " + (compact ? "widget-style-picker-compact" : "")} role="radiogroup" aria-label={t("widgetStyle")} onKeyDown={navigate}>
    {widgetStyles.map((variant, index) => <button type="button" key={variant} ref={button => { buttons.current[index] = button; }}
      role="radio" aria-checked={value === variant} aria-label={t(labels[variant]) + " · " + t(descriptions[variant])}
      tabIndex={value === variant ? 0 : -1} disabled={disabled} className="widget-style-option"
      data-testid={"widget-style-" + variant} onClick={() => onChange(variant)}>
      <span className="widget-preview-scene" aria-hidden="true"><span className={"widget-style-preview widget-style-preview-" + variant}>
        <WidgetArtwork variant={variant} progress={progress} />
        {remainingMs !== undefined && <span className={"widget-preview-clock " + (remainingMs >= 6_000_000 ? "widget-preview-clock-long" : "")}>{formatClock(remainingMs)}</span>}
      </span></span>
      <span className="widget-style-name">{t(labels[variant])}</span>
      {value === variant && <Check className="widget-style-check" size={13} aria-hidden="true" />}
    </button>)}
  </div>;
}
