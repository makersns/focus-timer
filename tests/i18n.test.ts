import { describe, expect, it } from "vitest";
import { formatClock, formatDuration, locales, messages, resolveLocale, translate, type MessageKey } from "../src/i18n";

describe("five-language UI", () => {
  it("has the same non-empty messages and interpolation parameters in every language", () => {
    const keys = Object.keys(messages.en).sort();
    const placeholders = (value: string) => [...value.matchAll(/\{(\w+)\}/g)].map(match => match[1]).sort();
    expect(locales).toHaveLength(5);
    for (const locale of locales) {
      expect(Object.keys(messages[locale]).sort()).toEqual(keys);
      for (const key of keys as MessageKey[]) {
        expect(messages[locale][key].trim().length).toBeGreaterThan(0);
        expect(placeholders(messages[locale][key])).toEqual(placeholders(messages.en[key]));
      }
    }
  });
  it("substitutes translated values and keeps no raw template placeholders", () => {
    for (const locale of locales) {
      expect(translate(locale, "minutes", { value: 25 })).toContain("25");
      expect(translate(locale, "remaining", { minutes: 4, seconds: 9 })).not.toMatch(/[{}]/);
      expect(translate(locale, "cycle", { value: 3 })).toContain("3");
    }
  });
  it("resolves system variants and honors an explicit language", () => {
    expect(resolveLocale("system", "zh-Hant-HK")).toBe("zh-TW");
    expect(resolveLocale("system", "zh-SG")).toBe("zh-CN");
    expect(resolveLocale("system", "ja-JP")).toBe("ja");
    expect(resolveLocale("system", "ko-KR")).toBe("ko");
    expect(resolveLocale("system", "de-DE")).toBe("en");
    expect(resolveLocale("zh-CN", "en-US")).toBe("zh-CN");
  });
  it("rounds the live countdown up and never displays a negative clock", () => {
    expect(formatClock(1)).toBe("00:01");
    expect(formatClock(60_001)).toBe("01:01");
    expect(formatClock(25 * 60_000)).toBe("25:00");
    expect(formatClock(-1)).toBe("00:00");
    expect(formatClock(180 * 60_000)).toBe("180:00");
  });
  it("does not present partial seconds as an invented whole focus minute", () => {
    const t = (key: MessageKey, params?: Record<string, string | number>) => translate("en", key, params);
    expect(formatDuration(20_000, t)).toBe(messages.en.lessThanMinute);
    expect(formatDuration(119_000, t)).toBe("1 min");
    expect(formatDuration(0, t)).toBe("0 min");
  });
});

describe("English singular and plural forms", () => {
  it("uses natural session and remaining-time grammar", () => {
    expect(translate("en", "totalSessions", { value: 0 })).toBe("0 sessions");
    expect(translate("en", "totalSessions", { value: 1 })).toBe("1 session");
    expect(translate("en", "totalSessions", { value: 2 })).toBe("2 sessions");
    expect(translate("en", "remaining", { minutes: 1, seconds: 1 })).toBe("1 minute and 1 second remaining");
    expect(translate("en", "remaining", { minutes: 0, seconds: 1 })).toBe("0 minutes and 1 second remaining");
    expect(translate("en", "remaining", { minutes: 1, seconds: 0 })).toBe("1 minute and 0 seconds remaining");
  });
});
