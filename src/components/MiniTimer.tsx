import { useCallback, useEffect, useRef, type PointerEvent } from "react";
import { AlertCircle, Coffee, LoaderCircle, Pause, Pin, PinOff, Play, X } from "lucide-react";
import { backend } from "../lib/backend";
import { elapsedFraction } from "../lib/progress";
import type { FocusBackend } from "../types";
import { formatClock, modeKey } from "../i18n";
import { performPrimaryAction, primaryLabel, statusLabel, useFocusClient } from "./useFocusClient";
import { WidgetArtwork } from "./WidgetArtwork";
import "../widgets.css";

export default function MiniTimer({ api = backend }: { api?: FocusBackend }) {
  const { snapshot, runtime, accept, run, busy, loadFailed, retry, feedback, setFeedback, announcement, t } = useFocusClient(api, "mini");
  const primaryButton = useRef<HTMLButtonElement>(null);
  const supported = runtime?.desktopWidget === true;
  const toggleTimer = useCallback(() => {
    if (!supported) return;
    void run(async () => {
      const current = await api.getSnapshot();
      accept(current);
      return performPrimaryAction(api, current, accept);
    });
  }, [api, accept, run, supported]);
  const hide = useCallback(() => {
    if (supported) void run(() => api.setMiniTimerVisible(false));
  }, [api, run, supported]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.isComposing || event.repeat || event.metaKey || event.ctrlKey || event.altKey || busy) return;
      if (event.key === "Escape" && supported) { event.preventDefault(); hide(); return; }
      if (event.code !== "Space" || !snapshot) return;
      if (event.target instanceof HTMLElement && event.target.closest("button,input,select,textarea,a")) return;
      event.preventDefault();
      toggleTimer();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [busy, hide, snapshot, supported, toggleTimer]);

  function togglePin() {
    if (!supported) return;
    void run(async () => {
      const current = await api.getSnapshot();
      accept(current);
      return api.updatePreferences({ ...current.preferences, miniTimerAlwaysOnTop: !current.preferences.miniTimerAlwaysOnTop });
    });
  }
  function drag(event: PointerEvent<HTMLDivElement>) {
    if (!supported || event.button !== 0) return;
    event.preventDefault();
    void api.startDragging().catch(() => {});
  }
  function openMain() { if (supported) void run(() => api.showMainWindow(), false); }

  if (!snapshot) return <section className="mini-shell auxiliary-loading">
    <div className="mini-header"><strong>Focus</strong><button type="button" className="mini-icon-button" disabled={!supported} aria-label={t("closeMiniTimer")} onClick={hide}><X size={16} /></button></div>
    <p role="status">{t(loadFailed ? "loadError" : "loading")}</p>
    {loadFailed ? <button type="button" className="text-button" onClick={retry}>{t("retry")}</button> : <LoaderCircle className="spinner" size={20} />}
  </section>;

  const timer = snapshot.timer;
  const variant = snapshot.preferences.miniTimerStyle;
  const seconds = Math.max(0, Math.ceil(timer.remainingMs / 1000));
  const progress = elapsedFraction(timer);
  const intent = timer.mode === "focus" ? timer.intent.trim() : t("breakIntention");
  const PrimaryIcon = busy ? LoaderCircle : timer.status === "running" ? Pause : timer.status === "completed" && timer.mode === "focus" ? Coffee : Play;

  return <section className={"focus-widget focus-widget-" + variant} data-style={variant} data-status={timer.status} data-mode={timer.mode} aria-label={t("miniTimer")}>
    <WidgetArtwork variant={variant} progress={progress} />
    <div className="widget-drag-surface" onPointerDown={drag} title={supported ? t("dragWindow") : t("desktopOnly")} aria-hidden="true" />
    <header className="widget-actions">
      {snapshot.persistenceError && <button type="button" className="widget-icon-button mini-save-error" aria-label={t("persistenceError")} title={t("persistenceError")} onClick={openMain} disabled={!supported}><AlertCircle size={15} /></button>}
      <button type="button" className="widget-icon-button" data-testid="mini-pin" aria-pressed={snapshot.preferences.miniTimerAlwaysOnTop} disabled={busy || !supported} title={t(snapshot.preferences.miniTimerAlwaysOnTop ? "unpinMiniTimer" : "pinMiniTimer")} aria-label={t(snapshot.preferences.miniTimerAlwaysOnTop ? "unpinMiniTimer" : "pinMiniTimer")} onClick={togglePin}>{snapshot.preferences.miniTimerAlwaysOnTop ? <Pin size={15} /> : <PinOff size={15} />}</button>
      <button type="button" className="widget-icon-button" data-testid="mini-close" disabled={busy || !supported} title={t("closeMiniTimer")} aria-label={t("closeMiniTimer")} onClick={hide}><X size={17} /></button>
    </header>
    <button type="button" className="widget-open" data-testid="mini-expand" disabled={!supported} title={t("openMainWindow") + (intent ? " · " + intent : "")} aria-label={t("remaining", { minutes: Math.floor(seconds / 60), seconds: seconds % 60 }) + " · " + t("openMainWindow")} aria-describedby="mini-intention-help" onClick={openMain}>
      <time data-testid="mini-timer-clock" className={timer.durationMs >= 6_000_000 ? "long-duration" : undefined} role="timer" aria-live="off" aria-label={t("remaining", { minutes: Math.floor(seconds / 60), seconds: seconds % 60 })}>{formatClock(timer.remainingMs)}</time>
      <span className="widget-state">{!supported && runtime ? t("miniPreview") : t(statusLabel(snapshot))}</span>
    </button>
    <button ref={primaryButton} type="button" className="widget-timer-button" data-testid="mini-timer-toggle" disabled={busy || !supported} onClick={toggleTimer} title={t(primaryLabel(snapshot))} aria-label={t(primaryLabel(snapshot))}><PrimaryIcon className={busy ? "spinner" : ""} size={20} fill={PrimaryIcon === Play || PrimaryIcon === Pause ? "currentColor" : "none"} /></button>
    {feedback && <div className="widget-feedback" role="alert"><span>{t(feedback.key)}</span><button type="button" className="widget-icon-button" aria-label={t("dismiss")} onClick={() => setFeedback(null)}><X size={14} /></button></div>}
    <p className="sr-only" id="mini-intention-help">{t(modeKey(timer.mode))}{intent ? " · " + intent : ""}</p>
    <p className="sr-only" role="status" aria-live="polite">{announcement}</p>
  </section>;
}
