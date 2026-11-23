use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::desktop_alerts::{NotificationService, NotificationStatus, NotificationTestResult};
use crate::storage::Store;
use crate::timer::{
    Engine, MiniTimerStyle, Preferences, Snapshot, TimerAction, TimerMode, TimerStatus, Transition,
};
use serde::{Deserialize, Serialize};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, RunEvent, Theme, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

#[cfg(target_os = "macos")]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};

const SNAPSHOT_EVENT: &str = "focus://snapshot";

struct AppState {
    alerts: Arc<NotificationService>,
    engine: Mutex<Engine>,
    // The engine is never held while a store write or any native UI call runs.
    store: Mutex<Option<Store>>,
    persistence_report_revision: AtomicU64,
    tray: Mutex<Option<TrayControls>>,
    tray_summary: Mutex<String>,
    tray_ready: AtomicBool,
    mini_ready: AtomicBool,
    native_preferences: Mutex<Option<NativePreferences>>,
    shutting_down: AtomicBool,
    system_locale: String,
}

#[derive(Clone)]
struct TrayControls {
    show: MenuItem<tauri::Wry>,
    toggle: MenuItem<tauri::Wry>,
    status: MenuItem<tauri::Wry>,
    mini: CheckMenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

/// A completed operation contains everything required for I/O after unlocking.
struct Pending {
    transition: Transition,
    serialized: Option<Result<String, String>>,
}

fn report_persistence(engine: &Mutex<Engine>, latest: &AtomicU64, revision: u64, failed: bool) {
    if let Ok(mut engine) = engine.lock() {
        // A late successful old write cannot hide a newer failed checkpoint.
        if revision >= latest.fetch_max(revision, Ordering::AcqRel) {
            engine.set_persistence_error(failed);
        }
    }
}

fn persist(app: &AppHandle, revision: u64, serialized: Result<String, String>) {
    let state = app.state::<AppState>();
    // Keep result reporting in the same order as writes. No caller obtains the
    // store lock while holding the engine lock, so this cannot invert a lock.
    let store = state.store.lock();
    let failed = match store {
        Ok(mut guard) => {
            let result = match (guard.as_mut(), serialized) {
                (Some(store), Ok(raw)) => store.save(revision, &raw),
                (_, Err(error)) => Err(error),
                (None, _) => Err("storage_unavailable".to_owned()),
            };
            report_persistence(
                &state.engine,
                &state.persistence_report_revision,
                revision,
                result.is_err(),
            );
            result.is_err()
        }
        Err(_) => {
            report_persistence(
                &state.engine,
                &state.persistence_report_revision,
                revision,
                true,
            );
            true
        }
    };
    if failed {
        // Never log session intentions, state JSON, or private filesystem paths.
        eprintln!("Focus: local state could not be saved; the timer remains available.");
    }
}

fn apply<F>(app: &AppHandle, operation: F) -> Result<Snapshot, String>
where
    F: FnOnce(&mut Engine, i64) -> Result<Transition, String>,
{
    let state = app.state::<AppState>();
    let pending = {
        let mut engine = state.engine.lock().map_err(|_| "timer_unavailable")?;
        if state.shutting_down.load(Ordering::Acquire) {
            return Err("app_closing".to_owned());
        }
        let transition = operation(&mut engine, now_ms())?;
        let serialized = transition.persist.then(|| engine.serialize());
        Pending {
            transition,
            serialized,
        }
    };

    if let Some(serialized) = pending.serialized {
        persist(app, pending.transition.snapshot.revision, serialized);
    }

    let snapshot = state
        .engine
        .lock()
        .map_err(|_| "timer_unavailable")?
        .snapshot();

    if pending.transition.changed || snapshot.revision != pending.transition.snapshot.revision {
        // Frontends reject out-of-order revisions. A concurrently completed
        // command may legitimately make this snapshot newer than our operation.
        let _ = app.emit(SNAPSHOT_EVENT, &snapshot);
        refresh_tray(app);
    }

    if let Some(completed) = pending.transition.completed {
        if snapshot.preferences.notifications_enabled {
            let language = locale(&snapshot.preferences.language, &state.system_locale);
            let text = words(language);
            let body = match completed.mode {
                TimerMode::Focus => text.focus_complete,
                TimerMode::ShortBreak | TimerMode::LongBreak => text.break_complete,
            };
            state.alerts.completed(
                app.clone(),
                &completed.id,
                body,
                snapshot.preferences.sound_enabled,
            );
        }
    }
    Ok(snapshot)
}

#[tauri::command]
async fn get_notification_status(
    app: AppHandle,
    sound: bool,
) -> Result<NotificationStatus, String> {
    let service = Arc::clone(&app.state::<AppState>().alerts);
    tauri::async_runtime::spawn_blocking(move || service.status(sound))
        .await
        .map_err(|_| "notification_service_unavailable".to_owned())?
}

#[tauri::command]
async fn request_notification_permission(
    app: AppHandle,
    sound: bool,
) -> Result<NotificationStatus, String> {
    let service = Arc::clone(&app.state::<AppState>().alerts);
    tauri::async_runtime::spawn_blocking(move || service.request_permission(sound))
        .await
        .map_err(|_| "notification_service_unavailable".to_owned())?
}

#[tauri::command]
async fn open_notification_settings(app: AppHandle, sound: bool) -> Result<(), String> {
    let service = Arc::clone(&app.state::<AppState>().alerts);
    tauri::async_runtime::spawn_blocking(move || service.open_settings(sound))
        .await
        .map_err(|_| "notification_service_unavailable".to_owned())?
}

#[tauri::command]
async fn test_notification(app: AppHandle, sound: bool) -> Result<NotificationTestResult, String> {
    let state = app.state::<AppState>();
    let preference = state
        .engine
        .lock()
        .map_err(|_| "timer_unavailable")?
        .snapshot()
        .preferences
        .language;
    let body = match locale(&preference, &state.system_locale) {
        "zh-CN" => "这是一条测试提醒。计时结束时，Focus 会在这里提醒你。",
        "zh-TW" => "這是一則測試提醒。計時結束時，Focus 會在這裡提醒你。",
        "ja" => "テスト通知です。タイマーが終了すると、ここでお知らせします。",
        "ko" => "테스트 알림입니다. 타이머가 끝나면 여기에서 알려드려요.",
        _ => "A test reminder. Focus will notify you here when your timer finishes.",
    };
    let service = Arc::clone(&state.alerts);
    tauri::async_runtime::spawn_blocking(move || service.test(sound, body))
        .await
        .map_err(|_| "notification_service_unavailable".to_owned())?
}

#[tauri::command]
fn get_snapshot(app: AppHandle) -> Result<Snapshot, String> {
    apply(&app, |engine, now| Ok(engine.tick(now)))
}

#[tauri::command]
fn timer_action(app: AppHandle, action: TimerAction) -> Result<Snapshot, String> {
    apply(&app, |engine, now| engine.dispatch(action, now))
}

#[tauri::command]
fn update_preferences(app: AppHandle, preferences: Preferences) -> Result<Snapshot, String> {
    if preferences.mini_timer_enabled && !app.state::<AppState>().mini_ready.load(Ordering::Acquire)
    {
        return Err("desktop_widget_unavailable".to_owned());
    }
    let snapshot = apply(&app, |engine, now| {
        engine.update_preferences(preferences, now)
    })?;
    schedule_native_preferences(&app, false);
    Ok(snapshot)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeInfo {
    platform: &'static str,
    desktop_widget: bool,
    home_screen_widget: bool,
    tray: bool,
}

#[derive(Serialize)]
struct WidgetRequest {
    supported: bool,
    requested: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AndroidTimerStatus {
    exact_alarm_allowed: bool,
    can_request_exact_alarm: bool,
    notifications_allowed: bool,
    widget_count: u32,
}

#[tauri::command]
fn get_runtime_info(app: AppHandle) -> RuntimeInfo {
    RuntimeInfo {
        platform: std::env::consts::OS,
        desktop_widget: app.state::<AppState>().mini_ready.load(Ordering::Acquire),
        home_screen_widget: false,
        tray: app.state::<AppState>().tray_ready.load(Ordering::Acquire),
    }
}

#[tauri::command]
fn request_home_screen_widget() -> WidgetRequest {
    WidgetRequest {
        supported: false,
        requested: false,
    }
}

#[tauri::command]
fn get_android_timer_status() -> AndroidTimerStatus {
    AndroidTimerStatus {
        exact_alarm_allowed: false,
        can_request_exact_alarm: false,
        notifications_allowed: false,
        widget_count: 0,
    }
}

#[tauri::command]
fn request_exact_alarm_permission() -> WidgetRequest {
    WidgetRequest {
        supported: false,
        requested: false,
    }
}

fn set_mini_visible(app: &AppHandle, visible: bool) -> Result<Snapshot, String> {
    if !app.state::<AppState>().mini_ready.load(Ordering::Acquire) {
        return Err("desktop_widget_unavailable".to_owned());
    }
    let snapshot = apply(app, |engine, now| {
        let mut preferences = engine.snapshot().preferences;
        preferences.mini_timer_enabled = visible;
        engine.update_preferences(preferences, now)
    })?;
    // This is an explicit user action. No timer tick calls window synchronization.
    schedule_native_preferences(app, visible);
    Ok(snapshot)
}

#[tauri::command]
fn set_mini_timer_visible(app: AppHandle, visible: bool) -> Result<Snapshot, String> {
    set_mini_visible(&app, visible)
}

#[cfg(target_os = "macos")]
fn close_native_widget(app: &AppHandle) {
    let _ = set_mini_visible(app, false);
}

#[tauri::command]
fn start_widget_drag(app: AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    if window.label() != "mini" || !app.state::<AppState>().mini_ready.load(Ordering::Acquire) {
        return Err("desktop_widget_unavailable".to_owned());
    }
    #[cfg(target_os = "macos")]
    {
        crate::macos_widget::drag(&app)
    }
    #[cfg(not(target_os = "macos"))]
    {
        window
            .start_dragging()
            .map_err(|_| "widget_drag_unavailable".to_owned())
    }
}

#[tauri::command]
fn show_main_window(app: AppHandle) {
    hide_popover(&app);
    show_main(&app);
}

fn hide_popover(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    crate::macos_popover::hide(app);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[tauri::command]
fn hide_tray_popover(app: AppHandle) {
    hide_popover(&app);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    prepare_exit(&app);
    app.exit(0);
}

/// Only these preferences can affect a native window's visibility or z-order.
/// Keeping them separate from Snapshot prevents ticking from raising a window.
#[derive(Clone, Debug, Eq, PartialEq)]
struct NativePreferences {
    theme: String,
    mini_enabled: bool,
    mini_on_top: bool,
    mini_style: MiniTimerStyle,
}

impl From<&Preferences> for NativePreferences {
    fn from(preferences: &Preferences) -> Self {
        Self {
            theme: preferences.theme.clone(),
            mini_enabled: preferences.mini_timer_enabled,
            mini_on_top: preferences.mini_timer_always_on_top,
            mini_style: preferences.mini_timer_style,
        }
    }
}

fn selected_theme(preference: &str) -> Option<Theme> {
    match preference {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        _ => None,
    }
}

fn schedule_native_preferences(app: &AppHandle, explicit_show: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || sync_native_preferences(&handle, explicit_show));
}

fn mini_size(style: MiniTimerStyle) -> (f64, f64) {
    match style {
        MiniTimerStyle::Ring => (260.0, 260.0),
        MiniTimerStyle::Tomato => (260.0, 280.0),
        MiniTimerStyle::Capsule => (340.0, 112.0),
    }
}

#[cfg(any(not(target_os = "macos"), test))]
fn clamp_widget_origin(
    origin: (f64, f64),
    size: (f64, f64),
    work: (f64, f64, f64, f64),
) -> (f64, f64) {
    let min_x = work.0 + 12.0;
    let min_y = work.1 + 12.0;
    let max_x = (work.0 + work.2 - size.0 - 12.0).max(min_x);
    let max_y = (work.1 + work.3 - size.1 - 12.0).max(min_y);
    (origin.0.clamp(min_x, max_x), origin.1.clamp(min_y, max_y))
}

#[cfg(not(target_os = "macos"))]
fn resize_mini(mini: &tauri::WebviewWindow, style: MiniTimerStyle) {
    let size = mini_size(style);
    let previous_scale = mini.scale_factor().unwrap_or(1.0);
    let previous_origin = mini.outer_position().ok().map(|point| {
        (
            point.x as f64 / previous_scale,
            point.y as f64 / previous_scale,
        )
    });
    let monitor = mini
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| mini.primary_monitor().ok().flatten());
    let _ = mini.set_size(tauri::LogicalSize::new(size.0, size.1));
    if let Some(monitor) = monitor {
        let scale = monitor.scale_factor();
        let work = monitor.work_area();
        let logical_work = (
            work.position.x as f64 / scale,
            work.position.y as f64 / scale,
            work.size.width as f64 / scale,
            work.size.height as f64 / scale,
        );
        let origin = previous_origin.unwrap_or((
            logical_work.0 + logical_work.2 - size.0 - 24.0,
            logical_work.1 + 24.0,
        ));
        let (x, y) = clamp_widget_origin(origin, size, logical_work);
        // Logical coordinates avoid interpreting a new display's physical pixels
        // with the scale factor of the window's previous display.
        let _ = mini.set_position(tauri::LogicalPosition::new(x, y));
    }
}

fn sync_native_preferences(app: &AppHandle, explicit_show: bool) {
    let state = app.state::<AppState>();
    let Ok(engine) = state.engine.lock() else {
        return;
    };
    let next = NativePreferences::from(&engine.snapshot().preferences);
    drop(engine);
    let Ok(mut applied) = state.native_preferences.lock() else {
        return;
    };
    let previous = applied.replace(next.clone());
    drop(applied);
    let theme_changed = previous.as_ref().is_none_or(|old| old.theme != next.theme);
    if theme_changed {
        for window in app.webview_windows().values() {
            let _ = window.set_theme(selected_theme(&next.theme));
        }
        #[cfg(target_os = "macos")]
        crate::macos_popover::set_theme(app, selected_theme(&next.theme));
    }
    if state.mini_ready.load(Ordering::Acquire) {
        #[cfg(target_os = "macos")]
        if let Err(error) = crate::macos_widget::sync(
            selected_theme(&next.theme),
            mini_size(next.mini_style),
            next.mini_on_top,
            next.mini_enabled,
            explicit_show,
        ) {
            eprintln!("Focus: native widget unavailable ({error}).");
        }
        #[cfg(not(target_os = "macos"))]
        if let Some(mini) = app.get_webview_window("mini") {
            if previous
                .as_ref()
                .is_none_or(|old| old.mini_style != next.mini_style)
            {
                resize_mini(&mini, next.mini_style);
            }
            if previous
                .as_ref()
                .is_none_or(|old| old.mini_on_top != next.mini_on_top)
            {
                let _ = mini.set_always_on_top(next.mini_on_top);
            }
            if explicit_show
                || previous
                    .as_ref()
                    .is_none_or(|old| old.mini_enabled != next.mini_enabled)
            {
                if next.mini_enabled {
                    let _ = mini.show();
                } else {
                    let _ = mini.hide();
                }
            }
        }
    }
    // Deliberately no set_focus here: even startup restoration leaves the
    // user's current application focused. Only Open Focus focuses the main UI.
}

fn setup_auxiliary_windows(app: &AppHandle) -> tauri::Result<()> {
    // Tauri warns that creating a webview in a synchronous command or event
    // handler can deadlock on Windows. Create each hidden surface once in setup.
    let mini =
        WebviewWindowBuilder::new(app, "mini", WebviewUrl::App("index.html?view=mini".into()))
            .title("Focus — Mini")
            .accept_first_mouse(true)
            .inner_size(260.0, 260.0)
            .min_inner_size(100.0, 80.0)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .decorations(false)
            .transparent(true)
            .background_color(tauri::window::Color(0, 0, 0, 0))
            .shadow(false)
            .no_redirection_bitmap(true)
            .skip_taskbar(true)
            .focused(false)
            .visible(false)
            .build()?;

    #[cfg(target_os = "macos")]
    let mini_ready = match crate::macos_widget::initialize(&mini, close_native_widget) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("Focus: native widget could not initialize ({error}).");
            false
        }
    };
    #[cfg(not(target_os = "macos"))]
    let mini_ready = {
        let _ = mini;
        true
    };
    app.state::<AppState>()
        .mini_ready
        .store(mini_ready, Ordering::Release);

    #[cfg(target_os = "macos")]
    WebviewWindowBuilder::new(app, "tray", WebviewUrl::App("index.html?view=tray".into()))
        .title("Focus — Menu")
        .inner_size(264.0, 252.0)
        .initialization_script("document.addEventListener('DOMContentLoaded', () => { document.documentElement.dataset.nativePopover = 'true'; });")
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .transparent(true)
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .shadow(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .focused(false)
        .visible(false)
        .build()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn toggle_popover(app: &AppHandle) {
    let theme = app
        .state::<AppState>()
        .engine
        .lock()
        .ok()
        .and_then(|engine| selected_theme(&engine.snapshot().preferences.theme));
    if let Err(error) = crate::macos_popover::toggle(app, theme) {
        eprintln!("Focus: menu popover unavailable ({error}).");
        show_main(app);
    }
}

#[tauri::command]
fn clear_history(app: AppHandle) -> Result<Snapshot, String> {
    apply(&app, |engine, now| Ok(engine.clear_history(now)))
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ExternalTarget {
    Author,
    Github,
    License,
}

impl ExternalTarget {
    fn url(&self) -> &'static str {
        match self {
            Self::Author => "https://sugar.makersns.com/",
            Self::Github => "https://github.com/makersns/focus-timer",
            Self::License => "https://github.com/makersns/focus-timer/blob/main/LICENSE",
        }
    }
}

#[tauri::command]
fn open_external(target: ExternalTarget) -> Result<(), String> {
    open::that(target.url()).map_err(|_| "open_external_failed".to_owned())
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_timer_transition(engine: &mut Engine, now: i64) -> Result<Transition, String> {
    let snapshot = engine.snapshot();
    let action = match snapshot.timer.status {
        TimerStatus::Running => TimerAction::Pause,
        TimerStatus::Paused => TimerAction::Resume,
        TimerStatus::Idle => TimerAction::Start,
        TimerStatus::Completed => {
            return engine
                .start_next_completed(snapshot.timer.run_id.as_deref().unwrap_or_default(), now);
        }
    };
    engine.dispatch(action, now)
}

fn toggle_timer(app: &AppHandle) {
    let _ = apply(app, toggle_timer_transition);
}

/// Explicit exit is a pause, never a silently completed session.
fn prepare_exit(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    if state.shutting_down.swap(true, Ordering::AcqRel) {
        return;
    }
    let pending = state.engine.lock().ok().map(|mut engine| {
        if engine.snapshot().timer.status == TimerStatus::Running {
            // Pause first reconciles a deadline that has already completed.
            let _ = engine.dispatch(TimerAction::Pause, now_ms());
        }
        (engine.snapshot().revision, engine.serialize())
    });
    if let Some((revision, serialized)) = pending {
        persist(app, revision, serialized);
    }
}

#[derive(Clone, Copy)]
struct Words {
    show: &'static str,
    mini: &'static str,
    start: &'static str,
    next: &'static str,
    pause: &'static str,
    resume: &'static str,
    quit: &'static str,
    focus: &'static str,
    short_break: &'static str,
    long_break: &'static str,
    focus_complete: &'static str,
    break_complete: &'static str,
}

fn locale(preferred: &str, system: &str) -> &'static str {
    let selected = if preferred == "system" {
        system
    } else {
        preferred
    };
    let normalized = selected.to_ascii_lowercase().replace('_', "-");
    if normalized.starts_with("zh") {
        if normalized.contains("hant")
            || normalized.starts_with("zh-tw")
            || normalized.starts_with("zh-hk")
            || normalized.starts_with("zh-mo")
        {
            "zh-TW"
        } else {
            "zh-CN"
        }
    } else if normalized.starts_with("ja") {
        "ja"
    } else if normalized.starts_with("ko") {
        "ko"
    } else {
        "en"
    }
}

fn words(language: &str) -> Words {
    match language {
        "zh-CN" => Words {
            mini: "显示桌面小组件",
            show: "打开 Focus",
            start: "开始",
            next: "下一阶段",
            pause: "暂停",
            resume: "继续",
            quit: "暂停并退出",
            focus: "专注",
            short_break: "短休息",
            long_break: "长休息",
            focus_complete: "这一段专注完成了。休息一下，再继续吧。",
            break_complete: "休息结束了。准备好后，再开始一段专注。",
        },
        "zh-TW" => Words {
            mini: "顯示桌面小工具",
            show: "開啟 Focus",
            start: "開始",
            next: "下一階段",
            pause: "暫停",
            resume: "繼續",
            quit: "暫停並結束",
            focus: "專注",
            short_break: "短休息",
            long_break: "長休息",
            focus_complete: "這一段專注完成了。休息一下，再繼續吧。",
            break_complete: "休息結束了。準備好後，再開始一段專注。",
        },
        "ja" => Words {
            mini: "ミニタイマーを表示",
            show: "Focus を開く",
            start: "開始",
            next: "次のセッション",
            pause: "一時停止",
            resume: "再開",
            quit: "一時停止して終了",
            focus: "集中",
            short_break: "短い休憩",
            long_break: "長い休憩",
            focus_complete: "集中の時間が終わりました。ひと息つきましょう。",
            break_complete: "休憩が終わりました。準備ができたら、また始めましょう。",
        },
        "ko" => Words {
            mini: "미니 타이머 표시",
            show: "Focus 열기",
            start: "시작",
            next: "다음 단계",
            pause: "일시 정지",
            resume: "계속",
            quit: "일시 정지 후 종료",
            focus: "집중",
            short_break: "짧은 휴식",
            long_break: "긴 휴식",
            focus_complete: "집중 시간이 끝났어요. 잠시 쉬어 가세요.",
            break_complete: "휴식이 끝났어요. 준비되면 다시 시작하세요.",
        },
        _ => Words {
            mini: "Show mini timer",
            show: "Open Focus",
            start: "Start",
            next: "Next phase",
            pause: "Pause",
            resume: "Resume",
            quit: "Pause and quit",
            focus: "Focus",
            short_break: "Short break",
            long_break: "Long break",
            focus_complete: "Your focus session is complete. Take a gentle break.",
            break_complete: "Your break is over. Begin again when you are ready.",
        },
    }
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let snapshot = state
        .engine
        .lock()
        .map_err(|_| tauri::Error::Io(std::io::Error::other("timer_unavailable")))?
        .snapshot();
    let text = words(locale(&snapshot.preferences.language, &state.system_locale));
    let controls = TrayControls {
        show: MenuItem::with_id(app, "focus.show", text.show, true, None::<&str>)?,
        toggle: MenuItem::with_id(app, "focus.toggle", text.start, true, None::<&str>)?,
        status: MenuItem::with_id(app, "focus.status", "Focus", false, None::<&str>)?,
        mini: CheckMenuItem::with_id(
            app,
            "focus.mini",
            text.mini,
            app.get_webview_window("mini").is_some(),
            snapshot.preferences.mini_timer_enabled,
            None::<&str>,
        )?,
        quit: MenuItem::with_id(app, "focus.quit", text.quit, true, None::<&str>)?,
    };
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &controls.status,
            &controls.show,
            &controls.toggle,
            &controls.mini,
            &separator,
            &controls.quit,
        ],
    )?;
    #[cfg(not(target_os = "macos"))]
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::Io(std::io::Error::other("tray_icon_unavailable")))?;
    #[cfg(target_os = "macos")]
    let icon = menu_bar_icon(menu_bar_progress(&snapshot));
    TrayIconBuilder::with_id("focus")
        .icon(icon)
        .tooltip("Focus")
        .menu(&menu)
        .show_menu_on_left_click(!cfg!(target_os = "macos"))
        .on_menu_event(|app, event| match event.id().as_ref() {
            "focus.show" => show_main(app),
            "focus.toggle" => toggle_timer(app),
            "focus.mini" => {
                let enabled = app
                    .state::<AppState>()
                    .engine
                    .lock()
                    .map(|engine| engine.snapshot().preferences.mini_timer_enabled)
                    .unwrap_or(false);
                let _ = set_mini_visible(app, !enabled);
            }
            "focus.quit" => {
                prepare_exit(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            #[cfg(target_os = "macos")]
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_popover(tray.app_handle());
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (tray, event);
        })
        .build(app)?;
    if let Ok(mut tray) = state.tray.lock() {
        *tray = Some(controls);
    }
    state.tray_ready.store(true, Ordering::Release);
    refresh_tray(app);
    Ok(())
}

fn refresh_tray(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        // Read at execution time, not when the clock or a command queues work.
        // A delayed running snapshot must never overwrite a newer paused state.
        let snapshot = {
            let state = handle.state::<AppState>();
            let snapshot = state.engine.lock().ok().map(|engine| engine.snapshot());
            snapshot
        };
        if let Some(snapshot) = snapshot {
            render_tray(&handle, &snapshot);
        }
    });
}

fn render_tray(app: &AppHandle, snapshot: &Snapshot) {
    let state = app.state::<AppState>();
    let language = locale(&snapshot.preferences.language, &state.system_locale);
    let text = words(language);
    let mode = match snapshot.timer.mode {
        TimerMode::Focus => text.focus,
        TimerMode::ShortBreak => text.short_break,
        TimerMode::LongBreak => text.long_break,
    };
    let seconds = (snapshot.timer.remaining_ms.max(0) + 999) / 1000;
    let summary = format!("{} · {:02}:{:02}", mode, seconds / 60, seconds % 60);
    let action = match snapshot.timer.status {
        TimerStatus::Running => text.pause,
        TimerStatus::Paused => text.resume,
        TimerStatus::Idle => text.start,
        TimerStatus::Completed => text.next,
    };
    let key = format!(
        "{}|{}|{}|{}",
        language, action, summary, snapshot.preferences.mini_timer_enabled
    );
    if let Ok(mut previous) = state.tray_summary.lock() {
        if *previous == key {
            return;
        }
        *previous = key;
    }
    // Menu operations can dispatch to the main thread. Clone handles and drop
    // all application mutexes before making those calls.
    let controls = state.tray.lock().ok().and_then(|tray| tray.clone());
    if let Some(controls) = controls {
        let _ = controls.status.set_text(&summary);
        let _ = controls.show.set_text(text.show);
        let _ = controls.toggle.set_text(action);
        let _ = controls.quit.set_text(text.quit);
        let _ = controls.mini.set_text(text.mini);
        let _ = controls
            .mini
            .set_checked(snapshot.preferences.mini_timer_enabled);
    }
    if let Some(tray) = app.tray_by_id("focus") {
        let _ = tray.set_tooltip(Some(format!("Focus — {}", summary)));
        #[cfg(target_os = "macos")]
        {
            let _ = tray.set_title(Some(tray_title(snapshot)));
            static LAST_RING_PROGRESS: AtomicU64 = AtomicU64::new(u64::MAX);
            let progress = menu_bar_progress(snapshot);
            if LAST_RING_PROGRESS.swap(progress as u64, Ordering::AcqRel) != progress as u64 {
                let _ = tray.set_icon(Some(menu_bar_icon(progress)));
            }
        }
    }
}

#[cfg(any(target_os = "macos", test))]
fn tray_title(snapshot: &Snapshot) -> String {
    let seconds = (snapshot.timer.remaining_ms.max(0) + 999) / 1000;
    let prefix = match snapshot.timer.status {
        TimerStatus::Paused => "⏸ ",
        TimerStatus::Completed => "✓ ",
        TimerStatus::Idle | TimerStatus::Running => "",
    };
    format!("{prefix}{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(any(target_os = "macos", test))]
fn menu_bar_progress(snapshot: &Snapshot) -> u16 {
    let timer = &snapshot.timer;
    if timer.duration_ms <= 0 {
        return 0;
    }
    if timer.status == TimerStatus::Completed {
        return 360;
    }
    // Digits count down; the orange ring fills as time is completed.
    let elapsed = timer.duration_ms - timer.remaining_ms.clamp(0, timer.duration_ms);
    ((elapsed as i128 * 360 + timer.duration_ms as i128 / 2) / timer.duration_ms as i128) as u16
}

#[cfg(any(target_os = "macos", test))]
fn menu_bar_icon_pixels(progress: u16) -> Vec<u8> {
    use std::f64::consts::TAU;

    let size = 32;
    let radius = 10.5;
    let sweep = progress.min(360) as f64 / 360.0 * TAU;
    let end = (sweep.sin() * radius, -sweep.cos() * radius);
    let orange = [233.0, 158.0, 114.0];
    // A neutral track stays visible on both light and dark menu bars.
    let track = [145.0, 142.0, 134.0];
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 + 0.5 - 16.0;
            let dy = y as f64 + 0.5 - 16.0;
            let distance = dx.hypot(dy);
            let ring_distance = (distance - radius).abs();
            let track_coverage = (2.0 - ring_distance).clamp(0.0, 1.0);
            let angle = dx.atan2(-dy).rem_euclid(TAU);
            let arc_distance = if progress >= 360 || angle <= sweep {
                ring_distance
            } else {
                dx.hypot(dy + radius).min((dx - end.0).hypot(dy - end.1))
            };
            let active = if progress == 0 {
                0.0
            } else {
                (2.0 - arc_distance).clamp(0.0, 1.0)
            };
            let base = track_coverage * 0.7 * (1.0 - active);
            let alpha = active + base;
            let index = ((y * size + x) * 4) as usize;
            if alpha > 0.0 {
                for channel in 0..3 {
                    pixels[index + channel] =
                        ((orange[channel] * active + track[channel] * base) / alpha).round() as u8;
                }
                pixels[index + 3] = (alpha * 255.0).round() as u8;
            }
        }
    }
    pixels
}

#[cfg(target_os = "macos")]
fn menu_bar_icon(progress: u16) -> tauri::image::Image<'static> {
    tauri::image::Image::new_owned(menu_bar_icon_pixels(progress), 32, 32)
}

pub fn run() {
    let application = tauri::Builder::default()
        // Register first so a second launch cannot touch the state file.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main(app)
        }))
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_notification_status,
            request_notification_permission,
            open_notification_settings,
            test_notification,
            timer_action,
            update_preferences,
            clear_history,
            open_external,
            get_runtime_info,
            request_home_screen_widget,
            get_android_timer_status,
            request_exact_alarm_permission,
            set_mini_timer_visible,
            start_widget_drag,
            show_main_window,
            hide_tray_popover,
            quit_app
        ])
        .setup(|app| {
            let mut engine = Engine::new(now_ms());
            let mut store = app.path().app_data_dir().ok().map(Store::new);
            match store.as_mut().map(|store| store.load(now_ms())) {
                Some(Ok(loaded)) => {
                    engine = loaded.engine;
                    if loaded.recovered_corrupt.is_some() {
                        engine.set_persistence_error(true);
                    }
                }
                _ => engine.set_persistence_error(true),
            }
            app.manage(AppState {
                alerts: Arc::new(NotificationService::new(app.handle())),
                engine: Mutex::new(engine),
                store: Mutex::new(store),
                persistence_report_revision: AtomicU64::new(0),
                tray: Mutex::new(None),
                tray_summary: Mutex::new(String::new()),
                tray_ready: AtomicBool::new(false),
                mini_ready: AtomicBool::new(false),
                native_preferences: Mutex::new(None),
                shutting_down: AtomicBool::new(false),
                system_locale: sys_locale::get_locale().unwrap_or_else(|| "en".to_owned()),
            });

            if setup_auxiliary_windows(app.handle()).is_err() {
                eprintln!("Focus: an auxiliary timer surface could not be created.");
            }
            // A missing tray panel must not prevent theme or mini restoration.
            sync_native_preferences(app.handle(), false);

            if setup_tray(app.handle()).is_err() {
                // Closing quits normally if a desktop cannot supply a tray.
                eprintln!(
                    "Focus: system tray unavailable; closing the window will pause and quit."
                );
            }

            let handle = app.handle().clone();
            thread::Builder::new()
                .name("focus-clock".to_owned())
                .spawn(move || {
                    while !handle
                        .state::<AppState>()
                        .shutting_down
                        .load(Ordering::Acquire)
                    {
                        let _ = apply(&handle, |engine, now| Ok(engine.tick(now)));
                        thread::sleep(Duration::from_secs(1));
                    }
                })?;
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            let Some(state) = app.try_state::<AppState>() else {
                return;
            };
            if window.label() == "tray" {
                // Its content is hosted by NSPopover. The hidden Tauri window's
                // focus events do not describe the actual menu's visibility.
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    hide_popover(app);
                }
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                if state.shutting_down.load(Ordering::Acquire) {
                    return;
                }
                api.prevent_close();
                if window.label() == "mini" {
                    // Closing a widget changes visibility only; the run remains shared.
                    let _ = set_mini_visible(app, false);
                    return;
                }
                let close_to_tray = state
                    .engine
                    .lock()
                    .ok()
                    .map(|engine| engine.snapshot().preferences.close_to_tray)
                    .unwrap_or(false);
                if close_to_tray && state.tray_ready.load(Ordering::Acquire) {
                    let _ = window.hide();
                } else {
                    prepare_exit(app);
                    app.exit(0);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Focus could not initialize its desktop window");

    application.run(|app, event| match event {
        // macOS native Quit/AppleEvents can deliver Exit directly, without an
        // ExitRequested event. The idempotent checkpoint covers both paths.
        RunEvent::ExitRequested { .. } | RunEvent::Exit => prepare_exit(app),
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => show_main(app),
        _ => {}
    });
}

#[cfg(test)]
mod tests {
    use super::{
        clamp_widget_origin, locale, menu_bar_icon_pixels, menu_bar_progress, mini_size,
        report_persistence, toggle_timer_transition, tray_title, AtomicU64, Engine, ExternalTarget,
        MiniTimerStyle, Mutex, NativePreferences, TimerAction, TimerMode, TimerStatus,
    };

    #[test]
    fn native_menu_primary_action_advances_completed_phase() {
        let now = 1_800_000_000_000;
        let mut engine = Engine::new(now);
        engine
            .dispatch(TimerAction::SetDuration { minutes: 1 }, now)
            .unwrap();
        toggle_timer_transition(&mut engine, now).unwrap();
        let focus_id = engine.snapshot().timer.run_id;
        engine.tick(now + 60_000);
        let next = toggle_timer_transition(&mut engine, now + 60_001).unwrap();
        assert_eq!(next.snapshot.timer.mode, TimerMode::ShortBreak);
        assert_eq!(next.snapshot.timer.status, TimerStatus::Running);
        assert_ne!(next.snapshot.timer.run_id, focus_id);
        assert_eq!(next.snapshot.history.len(), 1);
        assert!(next.persist);
        assert!(next.completed.is_none());
        engine.tick(now + 360_001);
        let next = toggle_timer_transition(&mut engine, now + 360_002).unwrap();
        assert_eq!(next.snapshot.timer.mode, TimerMode::Focus);
        assert_eq!(next.snapshot.timer.status, TimerStatus::Running);
        assert_eq!(next.snapshot.history.len(), 2);
    }

    #[test]
    fn ticking_never_changes_native_window_preferences() {
        let mut engine = Engine::new(1_800_000_000_000);
        engine
            .dispatch(TimerAction::Start, 1_800_000_000_000)
            .unwrap();
        let before = NativePreferences::from(&engine.snapshot().preferences);
        engine.tick(1_800_000_001_000);
        let after = NativePreferences::from(&engine.snapshot().preferences);
        assert_eq!(before, after);
        assert!(!after.mini_enabled);
    }

    #[test]
    fn menu_bar_countdown_distinguishes_running_and_paused() {
        let mut engine = Engine::new(1_800_000_000_000);
        assert_eq!(tray_title(&engine.snapshot()), "25:00");
        engine
            .dispatch(TimerAction::Start, 1_800_000_000_000)
            .unwrap();
        engine.tick(1_800_000_001_001);
        assert_eq!(tray_title(&engine.snapshot()), "24:59");
        engine
            .dispatch(TimerAction::Pause, 1_800_000_001_001)
            .unwrap();
        assert_eq!(tray_title(&engine.snapshot()), "⏸ 24:59");
    }

    #[test]
    fn resizing_widgets_preserves_visibility_on_negative_origin_displays() {
        let work = (-1440.0, 25.0, 1440.0, 875.0);
        let origin = clamp_widget_origin((-80.0, 790.0), mini_size(MiniTimerStyle::Tomato), work);
        assert_eq!(origin, (-272.0, 608.0));
        assert_eq!(
            clamp_widget_origin((-1000.0, 100.0), mini_size(MiniTimerStyle::Capsule), work),
            (-1000.0, 100.0)
        );
        assert_eq!(mini_size(MiniTimerStyle::Ring), (260.0, 260.0));
    }

    #[test]
    fn menu_bar_ring_fills_with_elapsed_time_and_stops_when_paused() {
        let now = 1_800_000_000_000;
        let mut engine = Engine::new(now);
        assert_eq!(menu_bar_progress(&engine.snapshot()), 0);
        engine.dispatch(TimerAction::Start, now).unwrap();
        engine.tick(now + 375_000);
        assert_eq!(menu_bar_progress(&engine.snapshot()), 90);
        engine.dispatch(TimerAction::Pause, now + 375_000).unwrap();
        engine.tick(now + 390_000);
        assert_eq!(menu_bar_progress(&engine.snapshot()), 90);
        engine.dispatch(TimerAction::Resume, now + 400_000).unwrap();
        engine.tick(now + 1_900_000);
        assert_eq!(menu_bar_progress(&engine.snapshot()), 360);
        assert_eq!(engine.snapshot().history.len(), 1);
    }

    #[test]
    fn menu_bar_ring_has_a_visible_empty_track_and_distinct_progress() {
        let full = menu_bar_icon_pixels(360);
        let half = menu_bar_icon_pixels(180);
        let empty = menu_bar_icon_pixels(0);
        let orange_count = |pixels: &[u8]| {
            pixels
                .chunks_exact(4)
                .filter(|pixel| pixel[0] > 200 && pixel[3] > 220)
                .count()
        };
        assert_eq!(full.len(), 32 * 32 * 4);
        assert_eq!(orange_count(&empty), 0);
        assert!(orange_count(&full) > 100);
        assert!((orange_count(&half) as f64 / orange_count(&full) as f64 - 0.5).abs() < 0.15);
        assert!(empty.chunks_exact(4).any(|pixel| pixel[3] > 150));
        assert_eq!(&empty[..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn old_success_cannot_hide_newer_unsaved_state() {
        let engine = Mutex::new(Engine::new(1_800_000_000_000));
        let latest = AtomicU64::new(0);
        report_persistence(&engine, &latest, 12, true);
        report_persistence(&engine, &latest, 11, false);
        assert!(engine.lock().unwrap().snapshot().persistence_error);
        report_persistence(&engine, &latest, 13, false);
        assert!(!engine.lock().unwrap().snapshot().persistence_error);
    }

    #[test]
    fn external_targets_cannot_open_arbitrary_urls_or_commands() {
        for forbidden in [
            "https://example.com",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "shell",
        ] {
            assert!(serde_json::from_str::<ExternalTarget>(
                &serde_json::to_string(forbidden).unwrap()
            )
            .is_err());
        }
        assert_eq!(
            serde_json::from_str::<ExternalTarget>("\"author\"")
                .unwrap()
                .url(),
            "https://sugar.makersns.com/"
        );
    }

    #[test]
    fn native_language_matches_supported_locale_fallbacks() {
        assert_eq!(locale("system", "zh_Hant_HK"), "zh-TW");
        assert_eq!(locale("system", "zh_CN"), "zh-CN");
        assert_eq!(locale("system", "ja-JP"), "ja");
        assert_eq!(locale("ko", "en-US"), "ko");
        assert_eq!(locale("system", "de-DE"), "en");
    }
}
