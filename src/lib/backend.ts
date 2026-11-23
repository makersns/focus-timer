import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AndroidTimerStatus, FocusBackend, NotificationStatus, NotificationTestResult, Preferences, RuntimeInfo, Snapshot, TimerAction, WidgetRequest } from "../types";
import { PreviewEngine, type PreviewTransition } from "./preview-engine";
import { SnapshotGate } from "./snapshot-gate";

const DESTINATIONS = {
  author: "https://sugar.makersns.com",
  github: "https://github.com/makersns/focus-timer",
  license: "https://github.com/makersns/focus-timer/blob/main/LICENSE",
} as const;

class NativeBackend implements FocusBackend {
  readonly native = true;
  async getRuntimeInfo(): Promise<RuntimeInfo> { return invoke<RuntimeInfo>("get_runtime_info"); }
  private readonly gate = new SnapshotGate();
  async getSnapshot(): Promise<Snapshot> {
    return this.gate.accept(await invoke<Snapshot>("get_snapshot"));
  }
  async dispatch(action: TimerAction): Promise<Snapshot> {
    return this.gate.accept(await invoke<Snapshot>("timer_action", { action }));
  }
  async updatePreferences(preferences: Preferences): Promise<Snapshot> {
    return this.gate.accept(await invoke<Snapshot>("update_preferences", { preferences }));
  }
  async clearHistory(): Promise<Snapshot> {
    return this.gate.accept(await invoke<Snapshot>("clear_history"));
  }
  async subscribe(listener: (snapshot: Snapshot) => void): Promise<() => void> {
    return listen<Snapshot>("focus://snapshot", ({ payload }) => {
      const latest = this.gate.accept(payload);
      if (latest === payload) listener(latest);
    });
  }
  async subscribeNotificationErrors(listener: () => void): Promise<() => void> {
    return listen<string>("focus://notification-error", () => listener());
  }
  async setMiniTimerVisible(visible: boolean): Promise<Snapshot> {
    return this.gate.accept(await invoke<Snapshot>("set_mini_timer_visible", { visible }));
  }
  async showMainWindow(): Promise<void> { await invoke("show_main_window"); }
  async startDragging(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const window = getCurrentWindow();
    if (window.label === "mini") {
      await invoke("start_widget_drag");
    } else {
      await window.startDragging();
    }
  }
  async minimizeWindow(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().minimize();
  }
  async toggleMaximizeWindow(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().toggleMaximize();
  }
  async closeWindow(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  }
  async hideTrayPopover(): Promise<void> { await invoke("hide_tray_popover"); }
  async quit(): Promise<void> { await invoke("quit_app"); }
  async requestHomeScreenWidget(): Promise<WidgetRequest> {
    return invoke<WidgetRequest>("request_home_screen_widget");
  }
  async getAndroidTimerStatus(): Promise<AndroidTimerStatus> {
    return invoke<AndroidTimerStatus>("get_android_timer_status");
  }
  async requestExactAlarmPermission(): Promise<WidgetRequest> {
    return invoke<WidgetRequest>("request_exact_alarm_permission");
  }
  async getNotificationStatus(sound: boolean): Promise<NotificationStatus> {
    return invoke<NotificationStatus>("get_notification_status", { sound });
  }
  async requestNotifications(sound: boolean): Promise<NotificationStatus> {
    return invoke<NotificationStatus>("request_notification_permission", { sound });
  }
  async openNotificationSettings(sound: boolean): Promise<void> {
    await invoke("open_notification_settings", { sound });
  }
  async testNotification(sound: boolean): Promise<NotificationTestResult> {
    return invoke<NotificationTestResult>("test_notification", { sound });
  }
  async openExternal(target: keyof typeof DESTINATIONS): Promise<void> {
    await invoke("open_external", { target });
  }
}

const PREVIEW_KEY = "focus.preview.v1";
class BrowserPreviewBackend implements FocusBackend {
  readonly native = false;
  async getRuntimeInfo(): Promise<RuntimeInfo> {
    return { platform: "web", desktopWidget: false, homeScreenWidget: false, tray: false };
  }
  private readonly engine: PreviewEngine;
  private readonly listeners = new Set<(snapshot: Snapshot) => void>();
  private interval: ReturnType<typeof setInterval> | null = null;
  private storage: Storage | null = null;
  private blockedSave = false;

  constructor() {
    let raw: string | null = null;
    try {
      this.storage = window.localStorage;
      raw = this.storage.getItem(PREVIEW_KEY);
    } catch { this.blockedSave = true; }
    try {
      this.engine = new PreviewEngine(Date.now(), raw ?? undefined);
    } catch {
      // Preserve malformed data before starting a fresh in-memory preview.
      try {
        this.storage?.setItem(PREVIEW_KEY + ".corrupt." + Date.now(), raw ?? "");
      } catch { this.blockedSave = true; }
      this.engine = new PreviewEngine(Date.now());
      this.engine.setPersistenceError(true);
    }
    if (!this.storage || this.blockedSave) this.engine.setPersistenceError(true);
    window.addEventListener("pagehide", () => {
      if (this.engine.snapshot().timer.status === "running")
        this.deliver(this.engine.dispatch({ type: "pause" }, Date.now()));
    });
  }
  private deliver(transition: PreviewTransition): Snapshot {
    if (transition.persist) {
      if (this.storage && !this.blockedSave) {
        try {
          this.storage.setItem(PREVIEW_KEY, this.engine.serialize());
          this.engine.setPersistenceError(false);
        } catch { this.engine.setPersistenceError(true); }
      } else this.engine.setPersistenceError(true);
    }
    const snapshot = this.engine.snapshot();
    for (const listener of this.listeners) listener(snapshot);
    return snapshot;
  }
  async getSnapshot(): Promise<Snapshot> {
    return this.deliver(this.engine.tick(Date.now()));
  }
  async dispatch(action: TimerAction): Promise<Snapshot> {
    return this.deliver(this.engine.dispatch(action, Date.now()));
  }
  async updatePreferences(preferences: Preferences): Promise<Snapshot> {
    return this.deliver(this.engine.updatePreferences(preferences, Date.now()));
  }
  async clearHistory(): Promise<Snapshot> {
    return this.deliver(this.engine.clearHistory(Date.now()));
  }
  async subscribe(listener: (snapshot: Snapshot) => void): Promise<() => void> {
    this.listeners.add(listener);
    if (!this.interval) {
      this.interval = setInterval(() => {
        if (this.engine.snapshot().timer.status === "running")
          this.deliver(this.engine.tick(Date.now()));
      }, 250);
    }
    return () => {
      this.listeners.delete(listener);
      if (!this.listeners.size && this.interval) {
        clearInterval(this.interval); this.interval = null;
      }
    };
  }
  async subscribeNotificationErrors(_listener: () => void): Promise<() => void> { return () => {}; }
  async setMiniTimerVisible(_visible: boolean): Promise<Snapshot> { throw new Error("desktop_only"); }
  async showMainWindow(): Promise<void> { throw new Error("desktop_only"); }
  async startDragging(): Promise<void> { throw new Error("desktop_only"); }
  async minimizeWindow(): Promise<void> { throw new Error("desktop_only"); }
  async toggleMaximizeWindow(): Promise<void> { throw new Error("desktop_only"); }
  async closeWindow(): Promise<void> { throw new Error("desktop_only"); }
  async hideTrayPopover(): Promise<void> { throw new Error("desktop_only"); }
  async quit(): Promise<void> { throw new Error("desktop_only"); }
  async requestHomeScreenWidget(): Promise<WidgetRequest> { return { supported: false, requested: false }; }
  async getAndroidTimerStatus(): Promise<AndroidTimerStatus> {
    return { exactAlarmAllowed: false, canRequestExactAlarm: false, notificationsAllowed: false, widgetCount: 0 };
  }
  async requestExactAlarmPermission(): Promise<WidgetRequest> { return { supported: false, requested: false }; }
  async getNotificationStatus(_sound: boolean): Promise<NotificationStatus> {
    return { permission: "unsupported", canRequest: false, notificationsAllowed: false, channelEnabled: null, soundAllowed: null, settingsAvailable: false };
  }
  async requestNotifications(sound: boolean): Promise<NotificationStatus> { return this.getNotificationStatus(sound); }
  async openNotificationSettings(_sound: boolean): Promise<void> { throw new Error("desktop_only"); }
  async testNotification(sound: boolean): Promise<NotificationTestResult> {
    return { submitted: false, soundRequested: false, status: await this.getNotificationStatus(sound) };
  }
  async openExternal(target: keyof typeof DESTINATIONS): Promise<void> {
    const destination = DESTINATIONS[target];
    if (!destination) throw new Error("invalid_destination");
    window.open(destination, "_blank", "noopener,noreferrer");
  }
}

export const backend: FocusBackend = isTauri() ? new NativeBackend() : new BrowserPreviewBackend();
