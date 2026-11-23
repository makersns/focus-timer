import { useCallback, useEffect, useRef, type KeyboardEvent } from "react";
import { Check, Coffee, Copy, LoaderCircle, Pause, PictureInPicture2, Play, Power, X } from "lucide-react";
import { backend } from "../lib/backend";
import type { FocusBackend } from "../types";
import { formatClock } from "../i18n";
import { performPrimaryAction, primaryLabel, statusLabel, useFocusClient } from "./useFocusClient";

export default function TrayPopover({ api = backend }: { api?: FocusBackend }) {
  const { snapshot, runtime, accept, run, busy, loadFailed, retry, feedback, setFeedback, announcement, t } = useFocusClient(api, "tray");
  const menu = useRef<HTMLDivElement>(null);
  const primary = useRef<HTMLButtonElement>(null);
  const supported = runtime?.tray === true;
  const hide = useCallback(() => { if (supported) void api.hideTrayPopover().catch(() => {}); }, [api, supported]);

  useEffect(() => {
    const handler = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); hide(); }
    };
    const focusPrimary = () => { requestAnimationFrame(() => primary.current?.focus({ preventScroll: true })); };
    document.addEventListener("keydown", handler);
    window.addEventListener("focus", focusPrimary);
    if (supported && snapshot) focusPrimary();
    return () => { document.removeEventListener("keydown", handler); window.removeEventListener("focus", focusPrimary); };
  }, [hide, supported, snapshot !== null]);

  function toggleTimer() {
    if (!supported) return;
    void run(async () => {
      const current = await api.getSnapshot();
      accept(current);
      accept(await performPrimaryAction(api, current, accept));
      await api.hideTrayPopover();
    });
  }
  function toggleMini() {
    if (!supported || !runtime?.desktopWidget) return;
    void run(async () => {
      const current = await api.getSnapshot();
      accept(await api.setMiniTimerVisible(!current.preferences.miniTimerEnabled));
      await api.hideTrayPopover();
    });
  }
  function openMain() {
    if (!supported) return;
    void run(async () => { await api.showMainWindow(); await api.hideTrayPopover(); });
  }
  function quit() { if (supported) void run(() => api.quit()); }
  function navigateMenu(event: KeyboardEvent<HTMLDivElement>) {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    const buttons = Array.from(menu.current?.querySelectorAll<HTMLButtonElement>("button:not([disabled])") ?? []);
    if (!buttons.length) return;
    event.preventDefault();
    const index = buttons.findIndex(button => button === document.activeElement);
    const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
    buttons[next]?.focus();
  }

  if (!snapshot) return <section className="tray-shell auxiliary-loading"><strong>Focus</strong><p role="status">{t(loadFailed ? "loadError" : "loading")}</p>{loadFailed ? <button type="button" className="text-button" onClick={retry}>{t("retry")}</button> : <LoaderCircle className="spinner" size={20} />}</section>;

  const timer = snapshot.timer;
  const seconds = Math.max(0, Math.ceil(timer.remainingMs / 1000));
  const PrimaryIcon = busy ? LoaderCircle : timer.status === "running" ? Pause : timer.status === "completed" && timer.mode === "focus" ? Coffee : Play;
  return <section className="tray-shell" aria-label="Focus" data-status={timer.status}>
    <header className="tray-summary"><p>{t(statusLabel(snapshot))}</p><time data-testid="tray-timer-clock" role="timer" aria-live="off" aria-label={t("remaining", { minutes: Math.floor(seconds / 60), seconds: seconds % 60 })}>{formatClock(timer.remainingMs)}</time></header>
    <div className="tray-menu" ref={menu} role="menu" onKeyDown={navigateMenu}>
      <button ref={primary} type="button" role="menuitem" data-testid="tray-timer-toggle" disabled={busy || !supported} onClick={toggleTimer}><PrimaryIcon size={18} className={busy ? "spinner" : ""} /><span>{t(primaryLabel(snapshot))}</span></button>
      <button type="button" role="menuitemcheckbox" data-testid="tray-mini-toggle" aria-checked={snapshot.preferences.miniTimerEnabled} disabled={busy || !runtime?.desktopWidget || !supported} onClick={toggleMini}><PictureInPicture2 size={18} /><span>{t("showMiniTimer")}</span>{snapshot.preferences.miniTimerEnabled && <Check size={17} className="tray-checked" />}</button>
      <button type="button" role="menuitem" data-testid="tray-open-main" disabled={busy || !supported} onClick={openMain}><Copy size={18} /><span>{t("openMainWindow")}</span></button>
      <div role="separator" className="tray-divider" />
      <button type="button" role="menuitem" data-testid="tray-quit" disabled={busy || !supported} onClick={quit}><Power size={18} /><span>{t("quitApp")}</span></button>
    </div>
    {runtime && !supported && <p className="tray-preview">{t("miniPreview")}</p>}
    {feedback && <div className="tray-feedback" role="alert"><span>{t(feedback.key)}</span><button type="button" className="mini-icon-button" aria-label={t("dismiss")} onClick={() => setFeedback(null)}><X size={14} /></button></div>}
    <p className="sr-only" role="status" aria-live="polite">{announcement}</p>
  </section>;
}
