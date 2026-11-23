export type TimerMode = "focus" | "shortBreak" | "longBreak";
export type TimerStatus = "idle" | "running" | "paused" | "completed";
export type Locale = "zh-CN" | "zh-TW" | "en" | "ja" | "ko";
export type Language = "system" | Locale;
export type Theme = "system" | "light" | "dark";

export interface Preferences {
  durations: Record<TimerMode, number>;
  language: Language;
  theme: Theme;
  notificationsEnabled: boolean;
  closeToTray: boolean;
  miniTimerEnabled: boolean;
  miniTimerAlwaysOnTop: boolean;
}
export interface TimerState {
  mode: TimerMode;
  status: TimerStatus;
  durationMs: number;
  remainingMs: number;
  deadlineAt: number | null;
  startedAt: number | null;
  updatedAt: number;
  intent: string;
  runId: string | null;
}
export interface SessionRecord {
  id: string;
  mode: TimerMode;
  intent: string;
  startedAt: number;
  finishedAt: number;
  durationMs: number;
  elapsedMs: number;
  outcome: "completed" | "ended";
}
export interface DailyStat {
  date: string;
  focusMs: number;
  completedSessions: number;
  endedSessions: number;
}
export interface Snapshot {
  version: 1;
  revision: number;
  timer: TimerState;
  preferences: Preferences;
  history: SessionRecord[];
  dailyStats: DailyStat[];
  cycleCompleted: number;
  persistenceError: boolean;
  restoredPaused: boolean;
}
export type TimerAction =
  | { type: "start" | "pause" | "resume" | "reset" | "end" }
  | { type: "switchMode"; mode: TimerMode }
  | { type: "setDuration"; minutes: number }
  | { type: "setIntent"; intent: string };
export type Platform = "macos" | "windows" | "linux" | "android" | "web";
export interface RuntimeInfo {
  platform: Platform;
  desktopWidget: boolean;
  homeScreenWidget: boolean;
  tray: boolean;
}
export interface AndroidTimerStatus {
  exactAlarmAllowed: boolean;
  canRequestExactAlarm: boolean;
  notificationsAllowed: boolean;
  widgetCount: number;
}
export interface WidgetRequest {
  supported: boolean;
  requested: boolean;
}
export interface FocusBackend {
  readonly native: boolean;
  getRuntimeInfo(): Promise<RuntimeInfo>;
  getSnapshot(): Promise<Snapshot>;
  dispatch(action: TimerAction): Promise<Snapshot>;
  updatePreferences(preferences: Preferences): Promise<Snapshot>;
  clearHistory(): Promise<Snapshot>;
  subscribe(listener: (snapshot: Snapshot) => void): Promise<() => void>;
  requestNotifications(): Promise<boolean>;
  setMiniTimerVisible(visible: boolean): Promise<Snapshot>;
  showMainWindow(): Promise<void>;
  startDragging(): Promise<void>;
  minimizeWindow(): Promise<void>;
  toggleMaximizeWindow(): Promise<void>;
  closeWindow(): Promise<void>;
  hideTrayPopover(): Promise<void>;
  quit(): Promise<void>;
  requestHomeScreenWidget(): Promise<WidgetRequest>;
  getAndroidTimerStatus(): Promise<AndroidTimerStatus>;
  requestExactAlarmPermission(): Promise<WidgetRequest>;
  openExternal(target: "author" | "github" | "license"): Promise<void>;
}
