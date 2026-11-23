import { useCallback, useEffect, useRef, useState, type PointerEvent } from "react";
import { ChartNoAxesColumnIncreasing, Coffee, LoaderCircle, Maximize2, Minus, Pause, PictureInPicture2, Play, RotateCcw, Settings, X } from "lucide-react";
import { backend } from "./lib/backend";
import type { FocusBackend, Preferences, Snapshot, TimerAction, TimerMode } from "./types";
import { formatClock, modeKey, type MessageKey } from "./i18n";
import { Modal } from "./components/Modal";
import { DurationPanel, SettingsPanel, StatisticsPanel } from "./components/Panels";
import { isActive, isDesktop, performPrimaryAction, primaryLabel, statusLabel, useFocusClient } from "./components/useFocusClient";

type Panel = "settings" | "statistics" | "duration" | null;
type Confirmation = { kind: "mode"; mode: TimerMode } | { kind: "duration"; minutes: number } | { kind: "reset" } | { kind: "end" } | { kind: "clear" };
const modes: TimerMode[] = ["focus", "shortBreak", "longBreak"];
const presets: Record<TimerMode, number[]> = { focus: [15, 25, 50], shortBreak: [3, 5, 10], longBreak: [15, 20, 30] };

export default function App({ api = backend }: { api?: FocusBackend }) {
  const { snapshot, latest, runtime, androidStatus, refreshAndroidStatus, accept, run, busy, loadFailed, retry, feedback, setFeedback, announcement, locale, t } = useFocusClient(api, "main");
  const [panel, setPanel] = useState<Panel>(null);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [restoredDismissed, setRestoredDismissed] = useState(false);
  const [intentDraft, setIntentDraft] = useState("");
  const intentValue = useRef("");
  const intentDirty = useRef(false);
  const ready = snapshot !== null;

  useEffect(() => {
    if (!intentDirty.current) {
      const value = snapshot?.timer.intent ?? "";
      intentValue.current = value;
      setIntentDraft(value);
    }
  }, [snapshot?.timer.intent]);
  useEffect(() => {
    if (!ready || !intentDirty.current) return;
    const timeout = window.setTimeout(() => {
      const value = intentDraft;
      void run(async () => {
        if (latest.current?.timer.intent === value) {
          if (intentValue.current === value) intentDirty.current = false;
          return;
        }
        const result = await api.dispatch({ type: "setIntent", intent: value });
        if (intentValue.current === value) intentDirty.current = false;
        return result;
      }, false);
    }, 450);
    return () => window.clearTimeout(timeout);
  }, [intentDraft, ready, api, run, latest]);

  const flushIntent = useCallback(async (): Promise<Snapshot | null> => {
    const current = latest.current;
    if (!current || !intentDirty.current) return current;
    const value = intentValue.current;
    if (current.timer.intent === value) { intentDirty.current = false; return current; }
    const result = await api.dispatch({ type: "setIntent", intent: value });
    if (intentValue.current === value) intentDirty.current = false;
    accept(result);
    return result;
  }, [api, accept, latest]);
  const dispatch = useCallback((action: TimerAction) => { void run(() => api.dispatch(action)); }, [api, run]);
  const mainAction = useCallback(() => {
    void run(async () => {
      const current = await flushIntent();
      if (current) return performPrimaryAction(api, current, accept);
    });
  }, [api, run, accept, flushIntent]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.isComposing || event.repeat || event.altKey) return;
      if ((event.metaKey || event.ctrlKey) && event.key === ",") {
        event.preventDefault();
        if (!confirmation) setPanel("settings");
        return;
      }
      if (event.metaKey || event.ctrlKey || event.shiftKey || event.code !== "Space" || busy || panel || confirmation || !ready) return;
      if (event.target instanceof HTMLElement && event.target.closest('input,textarea,select,button,a,[contenteditable="true"],dialog,[role="textbox"]')) return;
      event.preventDefault();
      mainAction();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [mainAction, busy, panel, confirmation, ready]);

  function changeIntent(value: string) { intentDirty.current = true; intentValue.current = value; setIntentDraft(value); }
  function changeMode(mode: TimerMode) {
    const current = latest.current;
    if (!current || current.timer.mode === mode) return;
    if (isActive(current)) setConfirmation({ kind: "mode", mode });
    else dispatch({ type: "switchMode", mode });
  }
  function changeDuration(minutes: number) {
    setPanel(null);
    const current = latest.current;
    if (!current || minutes * 60_000 === current.timer.durationMs) return;
    if (isActive(current)) setConfirmation({ kind: "duration", minutes });
    else dispatch({ type: "setDuration", minutes });
  }
  function executeConfirmation() {
    const choice = confirmation;
    if (!choice) return;
    setConfirmation(null);
    void run(async () => {
      if (choice.kind === "clear") return api.clearHistory();
      await flushIntent();
      if (choice.kind === "reset") return api.dispatch({ type: "reset" });
      if (choice.kind === "end") return api.dispatch({ type: "end" });
      if (latest.current && isActive(latest.current)) accept(await api.dispatch({ type: "end" }));
      if (choice.kind === "mode") return api.dispatch({ type: "switchMode", mode: choice.mode });
      return api.dispatch({ type: "setDuration", minutes: choice.minutes });
    });
  }
  function savePreferences(preferences: Preferences) {
    void run(async () => {
      let denied = false;
      const next = structuredClone(preferences);
      if (next.notificationsEnabled && !latest.current?.preferences.notificationsEnabled) {
        try { next.notificationsEnabled = await api.requestNotifications(); }
        catch { next.notificationsEnabled = false; }
        denied = !next.notificationsEnabled;
      }
      const result = await api.updatePreferences(next);
      setPanel(null);
      setFeedback({ kind: "info", key: denied ? "notificationError" : "preferencesSaved" });
      return result;
    });
  }
  function openExternal(target: "author" | "github" | "license") { void run(() => api.openExternal(target), false); }
  function requestHomeWidget() {
    if (runtime?.platform !== "android" || !runtime.homeScreenWidget) return;
    void run(async () => {
      const result = await api.requestHomeScreenWidget();
      setFeedback({ kind: "info", key: result.supported && result.requested ? "homeWidgetRequested" : "homeWidgetManual" });
      await refreshAndroidStatus().catch(() => {});
    });
  }
  function requestExactTiming() {
    if (runtime?.platform !== "android") return;
    void run(async () => {
      const result = await api.requestExactAlarmPermission();
      setFeedback({ kind: "info", key: result.supported && result.requested ? "exactAlarmRequested" : "exactAlarmUnavailable" });
      await refreshAndroidStatus().catch(() => {});
    });
  }
  function toggleWidget() {
    if (runtime?.platform === "android") { requestHomeWidget(); return; }
    if (!runtime?.desktopWidget) return;
    void run(async () => {
      const current = await api.getSnapshot();
      accept(current);
      return api.setMiniTimerVisible(!current.preferences.miniTimerEnabled);
    });
  }
  function dragHeader(event: PointerEvent<HTMLElement>) {
    if (!isDesktop(runtime) || event.button !== 0 || event.detail > 1) return;
    if (event.target instanceof Element && event.target.closest("button,input,select,textarea,a")) return;
    event.preventDefault();
    void api.startDragging().catch(() => {});
  }

  if (!snapshot) return <div className="app-shell loading-shell">
    <img src="/brand-mark.svg" width="60" height="60" alt="" /><h1>Focus</h1><p role="status">{t(loadFailed ? "loadError" : "loading")}</p>
    {loadFailed ? <button type="button" className="button primary" onClick={retry}>{t("retry")}</button> : <LoaderCircle className="spinner" size={22} aria-hidden="true" />}
  </div>;

  const timer = snapshot.timer;
  const active = isActive(snapshot);
  const isBreak = timer.mode !== "focus";
  const seconds = Math.max(0, Math.ceil(timer.remainingMs / 1000));
  const progress = Math.min(1, Math.max(0, (timer.durationMs - timer.remainingMs) / timer.durationMs));
  const circumference = 2 * Math.PI * 168;
  const guidance: MessageKey = isBreak ? ({ idle: "breakReady", running: "breakRunning", paused: "breakPaused", completed: "breakCompleted" } as const)[timer.status] : ({ idle: "ready", running: "running", paused: "paused", completed: "completed" } as const)[timer.status];
  const PrimaryIcon = busy ? LoaderCircle : timer.status === "running" ? Pause : timer.status === "completed" && !isBreak ? Coffee : Play;
  const currentMinutes = Math.round(timer.durationMs / 60_000);
  const confirmTitle: MessageKey | null = confirmation ? ({ mode: "modeConfirmTitle", duration: "durationConfirmTitle", reset: "resetConfirmTitle", end: "endConfirmTitle", clear: "clearTitle" } as const)[confirmation.kind] : null;
  const confirmBody: MessageKey | null = confirmation && isBreak && confirmation.kind !== "clear" ? "breakChangeConfirmBody" : confirmation ? ({ mode: "modeConfirmBody", duration: "durationConfirmBody", reset: "resetConfirmBody", end: "endConfirmBody", clear: "clearBody" } as const)[confirmation.kind] : null;
  const confirmButton: MessageKey = confirmation?.kind === "mode" ? "endAndSwitch" : confirmation?.kind === "clear" ? "clearConfirm" : confirmation?.kind === "end" ? "confirmEnd" : "endAndReset";
  const widgetSupported = runtime?.platform === "android" ? runtime.homeScreenWidget : runtime?.desktopWidget;
  const widgetTitle = runtime?.platform === "android" ? t(widgetSupported ? "addHomeScreenWidget" : "homeWidgetManual") : !widgetSupported ? t("desktopOnly") : t(snapshot.preferences.miniTimerEnabled ? "hideMiniTimer" : "showMiniTimer");
  const customWindowControls = runtime?.platform === "windows" || runtime?.platform === "linux";

  return <div className={"app-shell platform-" + (runtime?.platform ?? "loading")} data-status={timer.status} data-mode={timer.mode}>
    <header className="app-header" onPointerDown={dragHeader}>
      <h1>Focus</h1>
      <div className="header-actions">
        <nav className="app-toolbar" aria-label={t("settings")}>
          <button type="button" className="toolbar-button" data-testid="statistics-open" onClick={() => setPanel("statistics")} title={t("statistics")} aria-label={t("statistics")}><ChartNoAxesColumnIncreasing size={23} /></button>
          {runtime && <button type="button" className="toolbar-button" data-testid="mini-toggle" disabled={busy || !widgetSupported} aria-pressed={runtime.desktopWidget ? snapshot.preferences.miniTimerEnabled : undefined} onClick={toggleWidget} title={widgetTitle} aria-label={widgetTitle}><PictureInPicture2 size={23} /></button>}
          <button type="button" className="toolbar-button" data-testid="settings-open" onClick={() => setPanel("settings")} title={t("settings")} aria-label={t("settings")}><Settings size={23} /></button>
        </nav>
        {customWindowControls && <div className="window-controls">
          <button type="button" data-testid="window-minimize" aria-label={t("minimizeWindow")} title={t("minimizeWindow")} onClick={() => { void run(() => api.minimizeWindow(), false); }}><Minus size={16} /></button>
          <button type="button" data-testid="window-maximize" aria-label={t("maximizeWindow")} title={t("maximizeWindow")} onClick={() => { void run(() => api.toggleMaximizeWindow(), false); }}><Maximize2 size={14} /></button>
          <button type="button" className="window-close" data-testid="window-close" aria-label={t("closeWindow")} title={t("closeWindow")} onClick={() => { void run(() => api.closeWindow(), false); }}><X size={17} /></button>
        </div>}
      </div>
    </header>

    <div className="feedback-stack">
      {snapshot.persistenceError && <div className="notice notice-error" role="alert"><span>{t("persistenceError")}</span></div>}
      {snapshot.restoredPaused && timer.status === "paused" && !restoredDismissed && <div className="notice notice-info"><span>{t("restoredPaused")}</span><button type="button" className="icon-button" aria-label={t("dismiss")} onClick={() => setRestoredDismissed(true)}><X size={17} /></button></div>}
      {feedback && <div className={"notice notice-" + feedback.kind} role={feedback.kind === "error" ? "alert" : "status"}><span>{t(feedback.key)}{feedback.key === "exactAlarmPermissionRequired" && runtime?.platform === "android" && <button type="button" className="notice-action" disabled={busy} onClick={requestExactTiming}>{t("allowPreciseTimer")}</button>}</span><button type="button" className="icon-button" aria-label={t("dismiss")} onClick={() => setFeedback(null)}><X size={17} /></button></div>}
    </div>

    <main className="focus-layout">
      <section className="timer-stage" aria-label={t(modeKey(timer.mode))}>
        <div className="mode-switch" role="group" aria-label={t("modeGroup")}>{modes.map(mode => <button type="button" key={mode} aria-pressed={timer.mode === mode} disabled={busy} onClick={() => changeMode(mode)}>{t(modeKey(mode))}</button>)}</div>
        <div className="intention-line">{isBreak ? <p>{t("breakIntention")}</p> : <label><span className="sr-only">{t("intention")}</span><input data-testid="focus-intention" disabled={busy} maxLength={120} value={intentDraft} onChange={event => changeIntent(event.target.value)} placeholder={t("intention")} aria-describedby="intention-help" /><span className="sr-only" id="intention-help">{t("intentionHelp")}</span></label>}</div>
        <div className="timer-dial">
          <svg viewBox="0 0 360 360" aria-hidden="true"><circle className="ring-track" cx="180" cy="180" r="168" /><circle className="ring-progress" cx="180" cy="180" r="168" strokeDasharray={circumference} strokeDashoffset={circumference * (1 - progress)} /></svg>
          <div className="timer-face">
            <time data-testid="main-timer-clock" className={timer.durationMs >= 6_000_000 ? "long-duration" : undefined} role="timer" aria-live="off" aria-label={t("remaining", { minutes: Math.floor(seconds / 60), seconds: seconds % 60 })}>{formatClock(timer.remainingMs)}</time>
            <p className="timer-state">{t(statusLabel(snapshot))}</p>
            <p className="timer-guidance">{t(guidance)}</p>
          </div>
        </div>
        <div className="timer-controls">
          <button type="button" className="button primary timer-primary" data-testid="main-timer-toggle" disabled={busy} onClick={mainAction}><PrimaryIcon size={22} className={busy ? "spinner" : ""} fill={PrimaryIcon === Play || PrimaryIcon === Pause ? "currentColor" : "none"} /><span>{t(primaryLabel(snapshot))}</span></button>
          <div className="secondary-timer-actions">
            {active && <button type="button" className="text-button end-round" data-testid="main-timer-end" disabled={busy} onClick={() => setConfirmation({ kind: "end" })}>{t("end")}</button>}
            {timer.status === "paused" && <button type="button" className="text-button reset-control" disabled={busy} onClick={() => setConfirmation({ kind: "reset" })} title={t("resetRound")}><RotateCcw size={14} />{t("reset")}</button>}
            {timer.status === "completed" && <button type="button" className="text-button end-round" disabled={busy} onClick={() => dispatch({ type: "start" })}>{t("anotherRound")}</button>}
          </div>
        </div>
        <div className="duration-row" role="group" aria-label={t("presets")}>
          {presets[timer.mode].map(minutes => <button type="button" key={minutes} disabled={busy} aria-pressed={currentMinutes === minutes} onClick={() => changeDuration(minutes)}>{t("minutes", { value: minutes })}</button>)}
          <button type="button" aria-pressed={!presets[timer.mode].includes(currentMinutes)} disabled={busy} onClick={() => setPanel("duration")} title={t("minutes", { value: currentMinutes })}>{!presets[timer.mode].includes(currentMinutes) ? t("minutes", { value: currentMinutes }) : t("custom")}</button>
        </div>
        <span className="sr-only">{t("cycle", { value: Math.min(4, snapshot.cycleCompleted) })}</span>
      </section>
    </main>
    <footer className="app-footer"><span className={snapshot.persistenceError ? "save-failed" : ""}>{t(snapshot.persistenceError ? "localSaveFailed" : runtime?.platform === "web" ? "browserPreviewShort" : "localSaved")}</span><button type="button" onClick={() => openExternal("author")} aria-label={t("bySugar")}>by Sugar</button></footer>
    <p className="sr-only" role="status" aria-live="polite" aria-atomic="true">{announcement}</p>

    {panel === "settings" && <SettingsPanel preferences={snapshot.preferences} runtime={runtime} androidStatus={androidStatus} busy={busy} t={t} onClose={() => setPanel(null)} onSave={savePreferences} onExternal={openExternal} onAddWidget={requestHomeWidget} onRequestExactTiming={requestExactTiming} />}
    {panel === "statistics" && <StatisticsPanel snapshot={snapshot} locale={locale} busy={busy} t={t} onClose={() => setPanel(null)} onClear={() => setConfirmation({ kind: "clear" })} />}
    {panel === "duration" && <DurationPanel initialMinutes={currentMinutes} mode={timer.mode} busy={busy} t={t} onClose={() => setPanel(null)} onApply={changeDuration} />}
    {confirmation && confirmTitle && confirmBody && <Modal title={t(confirmTitle)} description={t(confirmBody, confirmation.kind === "duration" ? { value: confirmation.minutes } : {})} closeLabel={t("close")} onClose={() => setConfirmation(null)} className="compact-modal confirmation-modal"><footer className="modal-footer"><button type="button" className="button secondary" data-autofocus onClick={() => setConfirmation(null)}>{t("cancel")}</button><button type="button" className={"button " + (confirmation.kind === "clear" ? "danger-button" : "primary")} disabled={busy} onClick={executeConfirmation}>{t(confirmButton)}</button></footer></Modal>}
  </div>;
}
