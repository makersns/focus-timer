import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { NotificationStatus } from "../src/types";
import { canSubmitNotification, NotificationSettings, type useNotificationAccess } from "../src/components/NotificationSettings";
import { DEFAULT_PREFERENCES } from "../src/lib/preview-engine";

const allowed: NotificationStatus = {
  permission: "granted", canRequest: false, notificationsAllowed: true,
  channelEnabled: true, soundAllowed: true, settingsAvailable: true,
};

describe("notification permission decisions", () => {
  it("never treats a known OS or category block as permission to send", () => {
    expect(canSubmitNotification(allowed)).toBe(true);
    for (const blocked of [
      { ...allowed, permission: "denied" as const },
      { ...allowed, permission: "prompt" as const, canRequest: true },
      { ...allowed, permission: "unsupported" as const },
      { ...allowed, notificationsAllowed: false },
      { ...allowed, channelEnabled: false },
    ]) expect(canSubmitNotification(blocked)).toBe(false);
  });
  it("can request delivery when a platform cannot report status, without inventing a grant", () => {
    const unknown: NotificationStatus = {
      permission: "unknown", canRequest: false, notificationsAllowed: null,
      channelEnabled: null, soundAllowed: null, settingsAvailable: false,
    };
    expect(canSubmitNotification(unknown)).toBe(true);
    expect(unknown.permission).toBe("unknown");
    expect(canSubmitNotification({ ...unknown, notificationsAllowed: false })).toBe(false);
  });
  it("allows a silent notification without overriding the system sound setting", () => {
    const silent = { ...allowed, soundAllowed: false };
    expect(canSubmitNotification(silent)).toBe(true);
    expect(silent.soundAllowed).toBe(false);
  });
});

function reminderMarkup(status: NotificationStatus, enabled: boolean) {
  const action = async () => { throw new Error("Rendering must not request permission or change settings"); };
  const access: ReturnType<typeof useNotificationAccess> = {
    status, loading: false, action: null, notice: null, busy: false,
    refresh: action, prepareToEnable: action, requestPermission: action, openSettings: action, test: action,
  };
  return renderToStaticMarkup(createElement(NotificationSettings, {
    sectionRef: null, draft: { ...DEFAULT_PREFERENCES, notificationsEnabled: enabled },
    disabled: false, access, t: key => key, onChange: () => {},
  }));
}

describe("notification settings guidance", () => {
  it("treats first-run prompt flags as undecided and offers permission only after opting in", () => {
    const prompt: NotificationStatus = {
      ...allowed, permission: "prompt", canRequest: true,
      notificationsAllowed: false, soundAllowed: false,
    };
    for (const enabled of [false, true]) {
      const markup = reminderMarkup(prompt, enabled);
      expect(markup).toContain("notificationStatusPrompt");
      expect(markup).not.toContain("notification-blocked");
      expect(markup).not.toContain('data-testid="notification-settings"');
      expect(markup).not.toContain("notificationSoundMuted");
      expect(markup.includes('data-testid="notification-request"')).toBe(enabled);
    }
  });
  it("offers system settings for an actual denial or blocked notification channel", () => {
    const denied = reminderMarkup({ ...allowed, permission: "denied", notificationsAllowed: false, soundAllowed: false }, true);
    expect(denied).toContain("notificationStatusDenied");
    expect(denied).toContain("notification-blocked");
    expect(denied).toContain('data-testid="notification-settings"');
    expect(denied).not.toContain("notificationSoundMuted");
    const channelBlocked = reminderMarkup({ ...allowed, channelEnabled: false }, true);
    expect(channelBlocked).toContain("notificationChannelBlocked");
    expect(channelBlocked).toContain('data-testid="notification-settings"');
  });
  it("shows a system-sound hint only after permission is granted and reminders are enabled", () => {
    const muted: NotificationStatus = { ...allowed, soundAllowed: false };
    const granted = reminderMarkup(muted, true);
    expect(granted).toContain("notificationStatusGranted");
    expect(granted).toContain("notificationSoundMuted");
    expect(granted).toContain('data-testid="notification-settings"');
    expect(granted).not.toContain("notification-blocked");
    for (const markup of [
      reminderMarkup(muted, false),
      reminderMarkup({ ...muted, permission: "unknown", notificationsAllowed: null }, true),
    ]) {
      expect(markup).not.toContain("notificationSoundMuted");
      expect(markup).not.toContain('data-testid="notification-settings"');
    }
  });
});
