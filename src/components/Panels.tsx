import { useId, useState, type FormEvent } from "react";
import { ArrowUpRight, BarChart3, Check, CircleCheck, Coffee, Leaf, Monitor, Moon, Sun, Trash2 } from "lucide-react";
import type { AndroidTimerStatus, Locale, Preferences, RuntimeInfo, Snapshot, TimerMode } from "../types";
import { formatDuration, localDateKey, localeLabels, locales, modeKey, type Translator } from "../i18n";
import { Modal } from "./Modal";
import { SelectMenu } from "./SelectMenu";

const modes: TimerMode[] = ["focus", "shortBreak", "longBreak"];
export function SettingsPanel({ preferences, runtime, androidStatus, busy, t, onClose, onSave, onExternal, onAddWidget, onRequestExactTiming }: {
  preferences: Preferences; runtime: RuntimeInfo | null; androidStatus: AndroidTimerStatus | null; busy: boolean; t: Translator; onClose: () => void;
  onSave: (preferences: Preferences) => void; onExternal: (target: "author" | "github" | "license") => void;
  onAddWidget: () => void; onRequestExactTiming: () => void;
}) {
  const [draft, setDraft] = useState(() => structuredClone(preferences));
  const [miniChanged, setMiniChanged] = useState(false);
  const [durations, setDurations] = useState<Record<TimerMode, string>>(() => Object.fromEntries(modes.map(mode => [mode, String(preferences.durations[mode])])) as Record<TimerMode, string>);
  const [invalid, setInvalid] = useState(false);
  const id = useId();
  function submit(event: FormEvent) {
    event.preventDefault();
    const valid = modes.every(mode => /^\d+$/.test(durations[mode]) && Number(durations[mode]) >= 1 && Number(durations[mode]) <= 180);
    if (!valid) { setInvalid(true); return; }
    onSave({ ...draft, miniTimerEnabled: miniChanged ? draft.miniTimerEnabled : preferences.miniTimerEnabled, miniTimerAlwaysOnTop: preferences.miniTimerAlwaysOnTop, durations: Object.fromEntries(modes.map(mode => [mode, Number(durations[mode])])) as Record<TimerMode, number> });
  }
  return <Modal title={t("preferencesTitle")} description={t("preferencesSubtitle")} closeLabel={t("close")} onClose={onClose} className="settings-modal">
    <form onSubmit={submit} className="settings-form">
      <div className="modal-body">
        <fieldset className="settings-section" disabled={busy}>
          <legend>{t("defaultDurations")}</legend><p className="field-help">{t("defaultDurationsHelp")}</p>
          <div className="duration-fields">{modes.map(mode => <label className="duration-field" key={mode}>
            <span>{t(modeKey(mode))}</span><div className="number-field"><input aria-label={t(modeKey(mode)) + " · " + t("minuteInput")} type="text" inputMode="numeric" pattern="[0-9]*" maxLength={3} value={durations[mode]} aria-invalid={invalid || undefined} aria-describedby={invalid ? id + "-duration-error" : undefined} onChange={event => { setInvalid(false); setDurations(previous => ({ ...previous, [mode]: event.target.value })); }} /><span>{t("minutes", { value: "" }).trim()}</span></div>
          </label>)}</div>
          {invalid && <p className="field-error" role="alert" id={id + "-duration-error"}>{t("invalidDuration")}</p>}
        </fieldset>
        <fieldset className="settings-section" disabled={busy}><legend>{t("appearance")}</legend>
          <div className="theme-options" role="group" aria-label={t("appearance")}>{([
            ["system", Monitor, "themeSystem"], ["light", Sun, "themeLight"], ["dark", Moon, "themeDark"]
          ] as const).map(([value, Icon, label]) => <button type="button" key={value} aria-pressed={draft.theme === value} onClick={() => setDraft(previous => ({ ...previous, theme: value }))}><Icon size={18} /><span>{t(label)}</span>{draft.theme === value && <Check size={13} className="theme-check" />}</button>)}</div>
          <div className="select-field"><label id={id + "-language-label"} htmlFor={id + "-language"}>{t("language")}</label><SelectMenu<Preferences["language"]> id={id + "-language"} labelId={id + "-language-label"} value={draft.language} disabled={busy} options={[{ value: "system", label: t("systemLanguage") }, ...locales.map(locale => ({ value: locale, label: localeLabels[locale], lang: locale }))]} onChange={language => setDraft(previous => ({ ...previous, language }))} /></div>
        </fieldset>
        <fieldset className="settings-section setting-switches" disabled={busy}>
          <label className="setting-switch"><span><strong>{t("notifications")}</strong><small>{t("notificationsHelp")}</small></span><input type="checkbox" role="switch" checked={draft.notificationsEnabled} onChange={event => setDraft(previous => ({ ...previous, notificationsEnabled: event.target.checked }))} /></label>
          {runtime?.tray && <label className="setting-switch"><span><strong>{t("closeToTray")}</strong><small>{t("closeToTrayHelp")}</small></span><input type="checkbox" role="switch" checked={draft.closeToTray} onChange={event => setDraft(previous => ({ ...previous, closeToTray: event.target.checked }))} /></label>}
        </fieldset>
        <fieldset className="settings-section setting-switches" disabled={busy}>
          <legend>{t("widgetSettings")}</legend>
          {!runtime ? <p className="field-help">{t("checkingDevice")}</p> : runtime.platform === "android" ? <>
            <div className="widget-setting"><div><strong>{t("homeScreenWidget")}</strong><p>{t("homeScreenWidgetHelp")}</p>{androidStatus && <small>{t("homeWidgetCount", { value: androidStatus.widgetCount })}</small>}</div><button type="button" className="button secondary" data-testid="android-widget-add" disabled={!runtime.homeScreenWidget} onClick={onAddWidget}>{t("addHomeScreenWidget")}</button></div>
            {!runtime.homeScreenWidget && <p className="field-help">{t("homeWidgetManual")}</p>}
            <div className="android-timing-setting"><strong>{t("preciseTimer")}</strong>{androidStatus ? androidStatus.exactAlarmAllowed ? <p className="status-positive"><Check size={14} />{t("preciseTimerReady")}</p> : <><p>{t("preciseTimerNeedsPermission")}</p>{androidStatus.canRequestExactAlarm ? <button type="button" className="button secondary" data-testid="android-exact-alarm" onClick={onRequestExactTiming}>{t("allowPreciseTimer")}</button> : <p className="field-help">{t("exactAlarmUnavailable")}</p>}</> : <p className="field-help">{t("checkingDevice")}</p>}</div>
          </> : <label className={"setting-switch " + (!runtime.desktopWidget ? "setting-unavailable" : "")}><span><strong>{t("miniTimer")}</strong><small>{t("miniTimerHelp")}{!runtime.desktopWidget && <span className="desktop-only-note">{t("desktopOnly")}</span>}</small></span><input type="checkbox" role="switch" data-testid="mini-setting" disabled={!runtime.desktopWidget} checked={miniChanged ? draft.miniTimerEnabled : preferences.miniTimerEnabled} onChange={event => { setMiniChanged(true); setDraft(previous => ({ ...previous, miniTimerEnabled: event.target.checked })); }} /></label>}
        </fieldset>
        <section className="about-focus"><img src="/brand-mark.svg" alt="" width="42" height="42" /><div><h3>{t("about")}</h3><p>{t("aboutText")}</p></div><p className="privacy-note">{t("privacy")}</p><div className="about-links"><button type="button" onClick={() => onExternal("author")}>Sugar <ArrowUpRight size={14} /></button><button type="button" onClick={() => onExternal("github")}>{t("github")} <ArrowUpRight size={14} /></button><button type="button" onClick={() => onExternal("license")}>{t("license")} <ArrowUpRight size={14} /></button></div></section>
      </div>
      <footer className="modal-footer"><button type="button" className="button secondary" onClick={onClose}>{t("cancel")}</button><button type="submit" className="button primary" disabled={busy}>{busy ? t("pending") : t("save")}</button></footer>
    </form>
  </Modal>;
}

export function getStatistics(snapshot: Snapshot, now = new Date()) {
  const today = localDateKey(now);
  const daily = new Map(snapshot.dailyStats.map(day => [day.date, { ...day }]));
  const current = daily.get(today) ?? { date: today, focusMs: 0, completedSessions: 0, endedSessions: 0 };
  if (snapshot.timer.mode === "focus" && (snapshot.timer.status === "running" || snapshot.timer.status === "paused")) current.focusMs += Math.max(0, snapshot.timer.durationMs - snapshot.timer.remainingMs);
  daily.set(today, current);
  const days = Array.from({ length: 7 }, (_, index) => {
    const date = new Date(now); date.setHours(12, 0, 0, 0); date.setDate(date.getDate() - (6 - index));
    const key = localDateKey(date);
    return { date, key, ms: daily.get(key)?.focusMs ?? 0 };
  });
  return { today: current, days, weekMs: days.reduce((sum, day) => sum + day.ms, 0) };
}
export function StatisticsPanel({ snapshot, locale, busy, t, onClose, onClear }: {
  snapshot: Snapshot; locale: Locale; busy: boolean; t: Translator; onClose: () => void; onClear: () => void;
}) {
  const [limit, setLimit] = useState(30);
  const statistics = getStatistics(snapshot);
  const maximum = Math.max(...statistics.days.map(day => day.ms), 60_000);
  const history = [...snapshot.history].sort((a, b) => b.finishedAt - a.finishedAt);
  const dateFormat = new Intl.DateTimeFormat(locale, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
  const weekdayFormat = new Intl.DateTimeFormat(locale, { weekday: "short" });
  return <Modal title={t("statsTitle")} description={t("statsSubtitle")} closeLabel={t("close")} onClose={onClose} className="statistics-modal">
    <div className="modal-body">
      <div className="stat-cards">
        <div><span>{t("todayFocus")}</span><strong>{formatDuration(statistics.today.focusMs, t)}</strong></div>
        <div><span>{t("completedTimers")}</span><strong>{t("totalSessions", { value: statistics.today.completedSessions })}</strong></div>
        <div><span>{t("weekFocus")}</span><strong>{formatDuration(statistics.weekMs, t)}</strong></div>
      </div>
      <section className="week-chart" aria-labelledby="week-chart-heading"><h3 id="week-chart-heading">{t("lastSevenDays")}</h3>
        <ol>{statistics.days.map((day, index) => <li key={day.key} className={index === 6 ? "chart-today" : ""} aria-label={new Intl.DateTimeFormat(locale, { month: "short", day: "numeric" }).format(day.date) + ": " + formatDuration(day.ms, t)}>
          <span className="chart-value" aria-hidden="true">{Math.floor(day.ms / 60_000)}</span><div className="bar-track" aria-hidden="true"><div className="bar-fill" style={{ height: day.ms > 0 ? Math.max(3, day.ms / maximum * 100) + "%" : "0%" }} /></div><span className="chart-label" aria-hidden="true">{index === 6 ? t("today") : weekdayFormat.format(day.date)}</span>
        </li>)}</ol>
      </section>
      <section className="history-section"><div className="section-heading"><h3>{t("history")}</h3><span>{history.length}</span></div>
        {history.length === 0 ? <div className="empty-history"><span className="empty-history-icon"><BarChart3 size={28} strokeWidth={1.5} /></span><h4>{t("emptyTitle")}</h4><p>{t("emptyBody")}</p></div> :
          <ul className="history-list">{history.slice(0, limit).map(record => {
            const Icon = record.mode === "focus" ? CircleCheck : record.mode === "shortBreak" ? Coffee : Leaf;
            return <li key={record.id}><span className={"history-icon " + (record.mode === "focus" ? "" : "history-break")}><Icon size={19} /></span><div className="history-copy"><strong>{record.mode === "focus" ? record.intent.trim() || t("focus") : t(modeKey(record.mode))}</strong><span><time dateTime={new Date(record.finishedAt).toISOString()}>{dateFormat.format(record.finishedAt)}</time><span className="history-divider">·</span>{t(record.outcome === "completed" ? "completedOutcome" : "endedOutcome")}</span></div><span className="history-duration">{formatDuration(record.elapsedMs, t)}</span></li>;
          })}</ul>}
        {history.length > limit && <button type="button" className="text-button load-more" onClick={() => setLimit(previous => previous + 30)}>{t("loadMore")}</button>}
      </section>
    </div>
    <footer className="modal-footer stats-footer"><p>{t("privacy")}</p><button type="button" className="text-button danger" disabled={busy || (snapshot.history.length === 0 && snapshot.dailyStats.length === 0)} onClick={onClear}><Trash2 size={16} />{t("clearHistory")}</button></footer>
  </Modal>;
}

export function DurationPanel({ initialMinutes, mode, busy, t, onClose, onApply }: {
  initialMinutes: number; mode: TimerMode; busy: boolean; t: Translator; onClose: () => void; onApply: (minutes: number) => void;
}) {
  const [value, setValue] = useState(String(initialMinutes));
  const [invalid, setInvalid] = useState(false);
  const suggestions = { focus: [15, 25, 50], shortBreak: [3, 5, 10], longBreak: [15, 20, 30] }[mode];
  return <Modal title={t("customTitle")} description={t("customHelp")} closeLabel={t("close")} onClose={onClose} className="compact-modal">
    <form onSubmit={event => { event.preventDefault(); if (!/^\d+$/.test(value) || Number(value) < 1 || Number(value) > 180) { setInvalid(true); return; } onApply(Number(value)); }}>
      <div className="modal-body"><div className="custom-presets" role="group" aria-label={t("presets")}>{suggestions.map(minutes => <button type="button" key={minutes} aria-pressed={Number(value) === minutes} disabled={busy} onClick={() => { setValue(String(minutes)); setInvalid(false); }}>{t("minutes", { value: minutes })}</button>)}</div><label className="custom-duration-input"><span className="sr-only">{t("minuteInput")}</span><input data-autofocus type="text" inputMode="numeric" pattern="[0-9]*" maxLength={3} value={value} aria-invalid={invalid || undefined} onChange={event => { setValue(event.target.value); setInvalid(false); }} /><span>{t("minutes", { value: "" }).trim()}</span></label>{invalid && <p className="field-error" role="alert">{t("invalidDuration")}</p>}</div>
      <footer className="modal-footer"><button type="button" className="button secondary" onClick={onClose}>{t("cancel")}</button><button type="submit" className="button primary" disabled={busy}>{t("apply")}</button></footer>
    </form>
  </Modal>;
}
