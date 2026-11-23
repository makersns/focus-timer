//! Android owns one headless-safe runtime. Activity/plugin/service/widget/alarm
//! commands all enter it through JNI; the WebView never owns a second timer.
use crate::storage::{AndroidClock, Store};
use crate::timer::{Engine, Preferences, Snapshot, TimerAction, TimerStatus};
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jlong, jstring};
use jni::JNIEnv;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{
    plugin::{Builder as PluginBuilder, PluginHandle, TauriPlugin},
    AppHandle, Emitter, Manager, Runtime,
};

static RUNTIME: OnceLock<Mutex<Option<Arc<MobileRuntime>>>> = OnceLock::new();
static APP: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();

struct MobileState {
    engine: Engine,
    logical_anchor: i64,
    elapsed_anchor: i64,
    needs_initial_save: bool,
}
struct MobileRuntime {
    directory: PathBuf,
    operation: Mutex<()>,
    state: Mutex<MobileState>,
    store: Mutex<Store>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
enum Request {
    Snapshot,
    Tick,
    Action {
        action: TimerAction,
        #[serde(default, rename = "expectedRunId")]
        expected_run_id: Option<String>,
    },
    Preferences {
        preferences: Preferences,
    },
    Next {
        #[serde(rename = "expectedRunId")]
        expected_run_id: String,
    },
    ClearHistory,
    Alarm {
        #[serde(rename = "runId")]
        run_id: String,
    },
}

impl MobileRuntime {
    fn new(directory: PathBuf, wall_now: i64, clock: AndroidClock, force_pause: bool) -> Self {
        let mut store = Store::new(directory.clone());
        let (mut engine, logical_now) = match store.load_android(wall_now, clock, force_pause) {
            Ok(loaded) => {
                let mut engine = loaded.engine;
                if loaded.recovered_corrupt.is_some() {
                    engine.set_persistence_error(true);
                }
                (engine, loaded.logical_now)
            }
            Err(_) => {
                let mut engine = Engine::new(wall_now);
                engine.set_persistence_error(true);
                (engine, wall_now)
            }
        };
        // Keep initial recovery status until the first durable write succeeds.
        if engine.serialize().is_err() {
            engine.set_persistence_error(true);
        }
        Self {
            directory,
            operation: Mutex::new(()),
            state: Mutex::new(MobileState {
                engine,
                logical_anchor: logical_now,
                elapsed_anchor: clock.elapsed_realtime_ms,
                needs_initial_save: true,
            }),
            store: Mutex::new(store),
        }
    }

    fn perform(
        &self,
        request: Request,
        wall_now: i64,
        clock: AndroidClock,
    ) -> Result<Value, String> {
        clock.validate()?;
        let _operation = self.operation.lock().map_err(|_| "timer_unavailable")?;
        let (transition, persist, valid) = {
            let mut state = self.state.lock().map_err(|_| "timer_unavailable")?;
            let now = if state.engine.snapshot().timer.status == TimerStatus::Running {
                state
                    .logical_anchor
                    .checked_add(
                        clock
                            .elapsed_realtime_ms
                            .saturating_sub(state.elapsed_anchor)
                            .max(0),
                    )
                    .ok_or("invalid_android_clock")?
            } else {
                wall_now
            };
            let transition = match request {
                Request::Snapshot | Request::Tick => state.engine.tick(now),
                Request::Action {
                    action,
                    expected_run_id,
                } => {
                    if let Some(expected) = expected_run_id {
                        let actual = state
                            .engine
                            .snapshot()
                            .timer
                            .run_id
                            .unwrap_or_else(|| "idle".into());
                        if expected != actual {
                            return Err("stale_widget_action".into());
                        }
                    }
                    state.engine.dispatch(action, now)?
                }
                Request::Next { expected_run_id } => {
                    state.engine.start_next_completed(&expected_run_id, now)?
                }
                Request::Preferences { preferences } => {
                    state.engine.update_preferences(preferences, now)?
                }
                Request::ClearHistory => state.engine.clear_history(now),
                Request::Alarm { run_id } => {
                    // An old alarm cannot act on a newly started round.
                    if state.engine.snapshot().timer.run_id.as_deref() != Some(run_id.as_str()) {
                        return Err("stale_alarm".into());
                    }
                    state.engine.tick(now)
                }
            };
            state.logical_anchor = now;
            state.elapsed_anchor = clock.elapsed_realtime_ms;
            let persist = transition.persist || state.needs_initial_save;
            state.needs_initial_save = false;
            let valid = if persist {
                state.engine.serialize().map(|_| ())
            } else {
                Ok(())
            };
            (transition, persist, valid)
        };
        // Filesystem work happens after releasing the engine lock.
        if persist {
            let result = valid.and_then(|()| {
                self.store
                    .lock()
                    .map_err(|_| String::from("storage_unavailable"))?
                    .save_android(&transition.snapshot, clock)
            });
            self.state
                .lock()
                .map_err(|_| "timer_unavailable")?
                .engine
                .set_persistence_error(result.is_err());
        }
        let snapshot = self
            .state
            .lock()
            .map_err(|_| "timer_unavailable")?
            .engine
            .snapshot();
        let app = APP
            .get()
            .and_then(|slot| slot.lock().ok())
            .and_then(|slot| slot.clone());
        if let Some(app) = app {
            if transition.changed || snapshot.revision != transition.snapshot.revision {
                let _ = app.emit("focus://snapshot", &snapshot);
            }
        }
        let deadline_elapsed = if snapshot.timer.status == TimerStatus::Running {
            Some(
                clock
                    .elapsed_realtime_ms
                    .saturating_add(snapshot.timer.remaining_ms),
            )
        } else {
            None
        };
        Ok(json!({
            "ok": true, "snapshot": snapshot, "completed": transition.completed,
            "deadlineElapsedMs": deadline_elapsed,
        }))
    }
}

fn execute_native(
    directory: String,
    payload: String,
    wall_now: i64,
    elapsed: i64,
    boot: i64,
    force_pause: bool,
) -> Result<Value, String> {
    if payload.len() > 16_384 {
        return Err("invalid_request".into());
    }
    let request: Request = serde_json::from_str(&payload).map_err(|_| "invalid_request")?;
    let clock = AndroidClock {
        boot_count: boot,
        elapsed_realtime_ms: elapsed,
    };
    clock.validate()?;
    let directory = PathBuf::from(directory);
    if !directory.is_absolute() {
        return Err("invalid_state_directory".into());
    }
    let runtime = {
        let mut slot = RUNTIME
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| "timer_unavailable")?;
        if slot.is_none() {
            *slot = Some(Arc::new(MobileRuntime::new(
                directory.clone(),
                wall_now,
                clock,
                force_pause,
            )));
        }
        let runtime = slot.as_ref().ok_or("timer_unavailable")?;
        if runtime.directory != directory {
            return Err("state_directory_changed".into());
        }
        runtime.clone()
    };
    runtime.perform(request, wall_now, clock)
}

/// Only the application's fixed Kotlin bridge calls this symbol.
#[no_mangle]
pub extern "system" fn Java_com_makersns_focus_FocusNative_dispatchNative(
    mut env: JNIEnv<'_>,
    _class: JClass<'_>,
    directory: JString<'_>,
    payload: JString<'_>,
    wall_now: jlong,
    elapsed: jlong,
    boot: jlong,
    force_pause: jboolean,
) -> jstring {
    let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let directory: String = env
            .get_string(&directory)
            .map_err(|_| "invalid_request")?
            .into();
        let payload: String = env
            .get_string(&payload)
            .map_err(|_| "invalid_request")?
            .into();
        execute_native(
            directory,
            payload,
            wall_now,
            elapsed,
            boot,
            force_pause != 0,
        )
    }));
    let value = match response {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => json!({ "ok": false, "error": error }),
        Err(_) => json!({ "ok": false, "error": "timer_unavailable" }),
    };
    env.new_string(value.to_string())
        .map(|value| value.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

struct Bridge<R: Runtime>(PluginHandle<R>);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new("focus-mobile")
        .setup(|app, api| {
            let handle = api.register_android_plugin("com.makersns.focus", "FocusPlugin")?;
            app.manage(Bridge(handle));
            Ok(())
        })
        .build()
}

fn call(app: &AppHandle, command: &str, payload: Value) -> Result<Value, String> {
    app.state::<Bridge<tauri::Wry>>()
        .0
        .run_mobile_plugin(command, payload)
        .map_err(|error| match error {
            tauri::plugin::mobile::PluginInvokeError::InvokeRejected(error) => error
                .code
                .or(error.message)
                .unwrap_or_else(|| "android_operation_failed".into()),
            _ => "android_operation_failed".into(),
        })
}
fn timer_call(app: &AppHandle, request: Value) -> Result<Snapshot, String> {
    let value = call(app, "execute", json!({ "request": request.to_string() }))?;
    serde_json::from_value(value).map_err(|_| "invalid_android_response".into())
}

#[tauri::command]
async fn get_snapshot(app: AppHandle) -> Result<Snapshot, String> {
    timer_call(&app, json!({ "type": "snapshot" }))
}
#[tauri::command]
async fn timer_action(app: AppHandle, action: TimerAction) -> Result<Snapshot, String> {
    timer_call(&app, json!({ "type": "action", "action": action }))
}
#[tauri::command]
async fn update_preferences(app: AppHandle, preferences: Preferences) -> Result<Snapshot, String> {
    timer_call(
        &app,
        json!({ "type": "preferences", "preferences": preferences }),
    )
}
#[tauri::command]
async fn clear_history(app: AppHandle) -> Result<Snapshot, String> {
    timer_call(&app, json!({ "type": "clearHistory" }))
}
#[tauri::command]
async fn open_external(app: AppHandle, target: String) -> Result<(), String> {
    if !["author", "github", "license"].contains(&target.as_str()) {
        return Err("invalid_external_target".into());
    }
    call(&app, "openExternal", json!({ "target": target }))?;
    Ok(())
}
#[tauri::command]
fn get_runtime_info() -> Value {
    json!({ "platform": "android", "desktopWidget": false, "homeScreenWidget": true, "tray": false })
}
#[tauri::command]
async fn request_home_screen_widget(app: AppHandle) -> Result<Value, String> {
    call(&app, "requestHomeScreenWidget", json!({}))
}
#[tauri::command]
async fn get_android_timer_status(app: AppHandle) -> Result<Value, String> {
    call(&app, "getTimerStatus", json!({}))
}
#[tauri::command]
async fn request_exact_alarm_permission(app: AppHandle) -> Result<Value, String> {
    call(&app, "requestExactAlarmPermission", json!({}))
}
#[tauri::command]
async fn get_notification_status(app: AppHandle, sound: bool) -> Result<Value, String> {
    call(&app, "getNotificationStatus", json!({ "sound": sound }))
}
#[tauri::command]
async fn request_notification_permission(app: AppHandle, sound: bool) -> Result<Value, String> {
    call(
        &app,
        "requestNotificationPermission",
        json!({ "sound": sound }),
    )
}
#[tauri::command]
async fn open_notification_settings(app: AppHandle, sound: bool) -> Result<(), String> {
    call(&app, "openNotificationSettings", json!({ "sound": sound }))?;
    Ok(())
}
#[tauri::command]
async fn test_notification(app: AppHandle, sound: bool) -> Result<Value, String> {
    call(&app, "testNotification", json!({ "sound": sound }))
}
#[tauri::command]
async fn show_main_window(app: AppHandle) -> Result<(), String> {
    call(&app, "openApp", json!({}))?;
    Ok(())
}
#[tauri::command]
fn hide_tray_popover() {}
#[tauri::command]
fn set_mini_timer_visible(visible: bool) -> Result<(), String> {
    let _ = visible;
    Err("desktop_widget_unavailable".into())
}
#[tauri::command]
fn quit_app() -> Result<(), String> {
    Err("desktop_action_unavailable".into())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            timer_action,
            update_preferences,
            clear_history,
            open_external,
            get_runtime_info,
            request_home_screen_widget,
            get_android_timer_status,
            request_exact_alarm_permission,
            get_notification_status,
            request_notification_permission,
            open_notification_settings,
            test_notification,
            show_main_window,
            hide_tray_popover,
            set_mini_timer_visible,
            quit_app,
        ])
        .setup(|app| {
            *APP.get_or_init(|| Mutex::new(None))
                .lock()
                .map_err(|_| std::io::Error::other("timer_unavailable"))? =
                Some(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Focus could not initialize its Android window");
}
