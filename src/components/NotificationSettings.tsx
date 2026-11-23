import { useCallback, useEffect, useRef, useState, type Ref } from "react";
import { Bell, Check, ExternalLink, LoaderCircle } from "lucide-react";
import type { FocusBackend, NotificationStatus, Preferences } from "../types";
import type { MessageKey, Translator } from "../i18n";

export type NotificationApi = Pick<FocusBackend, "getNotificationStatus" | "requestNotifications" | "openNotificationSettings" | "testNotification">;
type Action = "permission" | "settings" | "test";
type Notice = { key: MessageKey; error: boolean };

/** An unknown platform permission may still deliver; a known OS or channel block must win. */
export function canSubmitNotification(status: NotificationStatus): boolean {
  return !["prompt", "denied", "unsupported"].includes(status.permission)
    && status.notificationsAllowed !== false && status.channelEnabled !== false;
}

export function useNotificationAccess(api: NotificationApi, sound: boolean) {
  const [status, setStatus] = useState<NotificationStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [action, setAction] = useState<Action | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const mounted = useRef(true);
  const activeAction = useRef<Action | null>(null);
  const readVersion = useRef(0);

  const refresh = useCallback(async () => {
    if (activeAction.current) return;
    const version = ++readVersion.current;
    setLoading(true);
    try {
      const next = await api.getNotificationStatus(sound);
      if (mounted.current && version === readVersion.current && !activeAction.current) {
        setStatus(next);
        setNotice(previous => previous?.key === "notificationPermissionUnavailable" ? null : previous);
      }
    } catch {
      if (mounted.current && version === readVersion.current)
        setNotice({ key: "notificationPermissionUnavailable", error: true });
    } finally {
      if (mounted.current && version === readVersion.current) setLoading(false);
    }
  }, [api, sound]);

  useEffect(() => {
    mounted.current = true;
    setStatus(null);
    setNotice(null);
    void refresh();
    const onReturn = () => { if (document.visibilityState !== "hidden") void refresh(); };
    window.addEventListener("focus", onReturn);
    document.addEventListener("visibilitychange", onReturn);
    return () => {
      mounted.current = false;
      ++readVersion.current;
      window.removeEventListener("focus", onReturn);
      document.removeEventListener("visibilitychange", onReturn);
    };
  }, [refresh]);

  const perform = async (kind: Action, work: () => Promise<boolean>): Promise<boolean> => {
    if (activeAction.current) return false;
    activeAction.current = kind;
    ++readVersion.current;
    setLoading(false);
    setAction(kind);
    setNotice(null);
    try { return await work(); }
    catch {
      if (mounted.current) setNotice({
        key: kind === "test" ? "notificationTestFailed" : kind === "settings" ? "notificationSettingsUnavailable" : "notificationPermissionUnavailable",
        error: true,
      });
      return false;
    } finally {
      activeAction.current = null;
      if (mounted.current) setAction(null);
    }
  };

  const prepareToEnable = () => perform("permission", async () => {
    let next = await api.getNotificationStatus(sound);
    if (next.canRequest) next = await api.requestNotifications(sound);
    if (!mounted.current) return false;
    setStatus(next);
    const allowed = canSubmitNotification(next);
    if (!allowed) setNotice({ key: "notificationPermissionRequired", error: true });
    return allowed;
  });
  const requestPermission = () => perform("permission", async () => {
    const next = await api.requestNotifications(sound);
    if (!mounted.current) return false;
    setStatus(next);
    const allowed = canSubmitNotification(next);
    if (!allowed) setNotice({ key: "notificationPermissionRequired", error: true });
    return allowed;
  });
  const openSettings = () => perform("settings", async () => {
    await api.openNotificationSettings(sound);
    return mounted.current;
  });
  const test = () => perform("test", async () => {
    let next = await api.getNotificationStatus(sound);
    if (next.canRequest) next = await api.requestNotifications(sound);
    if (!mounted.current) return false;
    setStatus(next);
    if (!canSubmitNotification(next)) {
      setNotice({ key: "notificationPermissionRequired", error: true });
      return false;
    }
    const result = await api.testNotification(sound);
    if (!mounted.current) return false;
    setStatus(result.status);
    setNotice({ key: result.submitted ? "notificationTestRequested" : canSubmitNotification(result.status) ? "notificationTestFailed" : "notificationPermissionRequired", error: !result.submitted });
    return result.submitted;
  });
  return { status, loading, action, notice, busy: action !== null, refresh, prepareToEnable, requestPermission, openSettings, test };
}

export function NotificationSettings({ sectionRef, draft, disabled, access, t, onChange }: {
  sectionRef: Ref<HTMLFieldSetElement>; draft: Preferences; disabled: boolean; access: ReturnType<typeof useNotificationAccess>; t: Translator;
  onChange: (change: Pick<Preferences, "notificationsEnabled"> | Pick<Preferences, "soundEnabled">) => void;
}) {
  const { status, loading, notice, action } = access;
  const supported = status?.permission !== "unsupported";
  // Before the first permission decision, OS delivery/sound flags may both be false.
  const blocked = supported && status?.permission !== "prompt"
    && (status?.permission === "denied" || status?.notificationsAllowed === false || status?.channelEnabled === false);
  const soundMuted = status?.permission === "granted" && status.soundAllowed === false;
  const statusKey: MessageKey = !status ? loading ? "notificationStatusLoading" : "notificationStatusUnknown"
    : status.permission === "unsupported" ? "notificationStatusUnsupported"
    : status.permission === "prompt" ? "notificationStatusPrompt"
    : status.permission === "denied" || status.notificationsAllowed === false ? "notificationStatusDenied"
    : status.channelEnabled === false ? "notificationChannelBlocked"
    : status.permission === "granted" ? "notificationStatusGranted" : "notificationStatusUnknown";
  return <fieldset ref={sectionRef} className="settings-section notification-settings" disabled={disabled || access.busy}>
    <legend>{t("reminders")}</legend>
    <label className="setting-switch"><span><strong>{t("notifications")}</strong><small>{t("notificationsHelp")}</small></span><input type="checkbox" role="switch" data-testid="notification-setting" checked={draft.notificationsEnabled} disabled={!supported} onChange={event => onChange({ notificationsEnabled: event.target.checked })} /></label>
    <label className={"setting-switch notification-sound " + (!draft.notificationsEnabled || !supported ? "setting-unavailable" : "")}><span><strong>{t("notificationSound")}</strong><small>{t("notificationSoundHelp")}</small></span><input type="checkbox" role="switch" data-testid="notification-sound-setting" checked={draft.soundEnabled} disabled={!draft.notificationsEnabled || !supported} onChange={event => onChange({ soundEnabled: event.target.checked })} /></label>
    <div className={"notification-access " + (blocked ? "notification-blocked" : status?.permission === "granted" ? "notification-allowed" : "")}>
      <p data-testid="notification-status" role="status">{!status && loading ? <LoaderCircle size={14} className="spinner" /> : status?.permission === "granted" && !blocked ? <Check size={14} /> : <Bell size={14} />}<span>{t(statusKey)}</span></p>
      {draft.notificationsEnabled && status?.canRequest ? <button type="button" className="text-button notification-access-action" data-testid="notification-request" onClick={() => { void access.requestPermission(); }}>{t("notificationRequest")}</button>
        : status?.settingsAvailable && (blocked || draft.notificationsEnabled && draft.soundEnabled && soundMuted) ? <button type="button" className="text-button notification-access-action" data-testid="notification-settings" onClick={() => { void access.openSettings(); }}>{t("notificationSettings")}<ExternalLink size={12} /></button> : null}
      {draft.notificationsEnabled && draft.soundEnabled && soundMuted && !blocked && <p className="notification-sound-note">{t("notificationSoundMuted")}</p>}
      {supported && blocked && !status?.settingsAvailable && <p className="notification-sound-note">{t("notificationSettingsUnavailable")}</p>}
    </div>
    <div className="notification-test-row"><button type="button" className="button secondary" data-testid="notification-test" title={!draft.notificationsEnabled ? t("notificationTestUnavailable") : undefined} disabled={!draft.notificationsEnabled || !supported || loading} onClick={() => { void access.test(); }}>{action === "test" ? <LoaderCircle size={15} className="spinner" /> : <Bell size={15} />}{t(action === "test" ? "notificationTesting" : "notificationTest")}</button><p>{t("notificationSystemControls")}</p></div>
    {notice && <p className={"notification-result " + (notice.error ? "field-error" : "notification-result-info")} role={notice.error ? "alert" : "status"} data-testid="notification-result">{t(notice.key)}{notice.key === "notificationPermissionUnavailable" && <button type="button" className="text-button" onClick={() => { void access.refresh(); }}>{t("retry")}</button>}</p>}
  </fieldset>;
}
