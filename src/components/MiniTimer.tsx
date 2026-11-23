import { useCallback, useEffect, useRef, type PointerEvent } from "react";
import { AlertCircle, Coffee, LoaderCircle, Maximize, Pause, Pin, PinOff, Play, X } from "lucide-react";
import { backend } from "../lib/backend";
import type { FocusBackend } from "../types";
import { formatClock, modeKey } from "../i18n";
import { performPrimaryAction, primaryLabel, statusLabel, useFocusClient } from "./useFocusClient";

export default function MiniTimer({ api = backend }: { api?: FocusBackend }) {
  const { snapshot, runtime, accept, run, busy, loadFailed, retry, feedback, setFeedback, announcement, locale, t } = useFocusClient(api, "mini");
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
  const seconds = Math.max(0, Math.ceil(timer.remainingMs / 1000));
  const progress = Math.min(100, Math.max(0, (timer.durationMs - timer.remainingMs) / Math.max(1, timer.durationMs) * 100));
  const intent = timer.mode === "focus" ? timer.intent.trim() : t("breakIntention");
  const title = locale === "en" && timer.mode === "focus" ? "Focus" : "Focus · " + t(modeKey(timer.mode));
  const PrimaryIcon = busy ? LoaderCircle : timer.status === "running" ? Pause : timer.status === "completed" && timer.mode === "focus" ? Coffee : Play;

  return <section className="mini-shell" data-status={timer.status} data-mode={timer.mode} aria-label={t("miniTimer")}>
    <header className="mini-header">
      <div className="mini-drag-area" onPointerDown={drag} title={supported ? t("dragWindow") : t("desktopOnly")}><strong>{title}</strong></div>
      <div className="mini-header-actions">
        {snapshot.persistenceError && <button type="button" className="mini-icon-button mini-save-error" aria-label={t("persistenceError")} title={t("persistenceError")} onClick={openMain} disabled={!supported}><AlertCircle size={15} /></button>}
        <button type="button" className="mini-icon-button" data-testid="mini-pin" aria-pressed={snapshot.preferences.miniTimerAlwaysOnTop} disabled={busy || !supported} title={t(snapshot.preferences.miniTimerAlwaysOnTop ? "unpinMiniTimer" : "pinMiniTimer")} aria-label={t(snapshot.preferences.miniTimerAlwaysOnTop ? "unpinMiniTimer" : "pinMiniTimer")} onClick={togglePin}>{snapshot.preferences.miniTimerAlwaysOnTop ? <Pin size={16} /> : <PinOff size={16} />}</button>
        <button type="button" className="mini-icon-button" data-testid="mini-expand" disabled={!supported} title={t("openMainWindow")} aria-label={t("openMainWindow")} onClick={openMain}><Maximize size={16} /></button>
        <button type="button" className="mini-icon-button" data-testid="mini-close" disabled={busy || !supported} title={t("closeMiniTimer")} aria-label={t("closeMiniTimer")} onClick={hide}><X size={18} /></button>
      </div>
    </header>
    <div className="mini-clock-row">
      <time data-testid="mini-timer-clock" className={timer.durationMs >= 6_000_000 ? "long-duration" : undefined} role="timer" aria-live="off" aria-label={t("remaining", { minutes: Math.floor(seconds / 60), seconds: seconds % 60 })}>{formatClock(timer.remainingMs)}</time>
      <button ref={primaryButton} type="button" className="mini-timer-button" data-testid="mini-timer-toggle" disabled={busy || !supported} onClick={toggleTimer} title={t(primaryLabel(snapshot))} aria-label={t(primaryLabel(snapshot))}><PrimaryIcon className={busy ? "spinner" : ""} size={22} fill={PrimaryIcon === Play || PrimaryIcon === Pause ? "currentColor" : "none"} /></button>
    </div>
    <p className="mini-intention" title={intent}>{!supported && runtime ? t("miniPreview") : <>{timer.status !== "running" && <span className="mini-state">{t(statusLabel(snapshot))}{intent ? " · " : ""}</span>}{intent || (timer.status === "running" ? t(statusLabel(snapshot)) : "")}</>}</p>
    <div className="mini-progress" role="progressbar" aria-label={t("progress")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(progress)}><span style={{ width: progress + "%" }} /></div>
    {feedback && <div className="mini-feedback" role="alert"><span>{t(feedback.key)}</span><button type="button" className="mini-icon-button" aria-label={t("dismiss")} onClick={() => setFeedback(null)}><X size={14} /></button></div>}
    <p className="sr-only" role="status" aria-live="polite">{announcement}</p>
  </section>;
}
