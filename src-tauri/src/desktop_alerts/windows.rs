use super::{windows_status, NotificationStatus};
use ::windows::{
    core::{IInspectable, HSTRING},
    Data::Xml::Dom::XmlDocument,
    Foundation::TypedEventHandler,
    Win32::{
        Foundation::RPC_E_CHANGED_MODE,
        System::{
            Com::CoIncrementMTAUsage,
            WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED, RO_INIT_SINGLETHREADED},
        },
    },
    UI::Notifications::{
        ToastFailedEventArgs, ToastNotification, ToastNotificationManager, ToastNotifier,
    },
};
use std::{
    collections::VecDeque,
    sync::{Mutex, OnceLock},
};
use tauri::{AppHandle, Emitter, Manager};

pub struct Backend {
    app: AppHandle,
    app_id: String,
    // WinRT owns each registered delegate; keeping its toast alive also keeps
    // callbacks available after Show returns. Older registrations are detached.
    toasts: Mutex<VecDeque<RetainedToast>>,
}

fn keep_process_mta_alive() -> Result<(), String> {
    // windows-core 0.62 caches agile activation factories for the entire process.
    // Submitted toasts also outlive the worker apartment that created them. Keep
    // one process-wide MTA usage reference so the last worker's RoUninitialize
    // cannot unload resources still referenced by those factories or toasts.
    //
    // The opaque cookie is intentionally retained until process termination.
    // Do not CoDecrementMTAUsage in Backend/command/toast cleanup: windows-core's
    // static factory caches remain alive. This is one bounded process resource;
    // each thread's separate RoInitialize reference is still balanced below.
    // CoIncrementMTAUsage cookies may be retained across threads; the address is
    // stored as an opaque integer and is never dereferenced.
    static COOKIE: OnceLock<Result<usize, String>> = OnceLock::new();
    COOKIE
        .get_or_init(|| {
            unsafe { CoIncrementMTAUsage() }
                .map(|cookie| cookie.0 as usize)
                .map_err(|_| "notification_service_unavailable".to_owned())
        })
        .as_ref()
        .map(|_| ())
        .map_err(Clone::clone)
}

struct WinRtApartment;
impl WinRtApartment {
    fn enter() -> Result<Self, String> {
        keep_process_mta_alive()?;
        // Worker threads normally use MTA. If the caller already has an STA
        // apartment, initialize WinRT in that same apartment rather than changing it.
        let result = match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
            Err(error) if error.code() == RPC_E_CHANGED_MODE => unsafe {
                RoInitialize(RO_INIT_SINGLETHREADED)
            },
            result => result,
        };
        result
            .map(|()| Self)
            .map_err(|_| "notification_service_unavailable".to_owned())
    }
}
impl Drop for WinRtApartment {
    fn drop(&mut self) {
        // Both S_OK and S_FALSE were successful and require one matching call.
        unsafe { RoUninitialize() };
    }
}

struct RetainedToast {
    notification: ToastNotification,
    activated: i64,
    failed: i64,
}
impl RetainedToast {
    fn new(notification: ToastNotification, app: &AppHandle) -> Result<Self, String> {
        let activation_app = app.clone();
        let activated = notification
            .Activated(&TypedEventHandler::<ToastNotification, IInspectable>::new(
                move |_, _| {
                    let handle = activation_app.clone();
                    let _ = activation_app.run_on_main_thread(move || {
                        if let Some(window) = handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    });
                    Ok(())
                },
            ))
            .map_err(|_| "notification_service_unavailable".to_owned())?;

        let failure_app = app.clone();
        let failed = match notification.Failed(&TypedEventHandler::<
            ToastNotification,
            ToastFailedEventArgs,
        >::new(move |_, _| {
            eprintln!("Focus: the system rejected a submitted completion notification.");
            let _ = failure_app.emit("focus://notification-error", "notification_submit_failed");
            Ok(())
        })) {
            Ok(token) => token,
            Err(_) => {
                let _ = notification.RemoveActivated(activated);
                return Err("notification_service_unavailable".to_owned());
            }
        };
        Ok(Self {
            notification,
            activated,
            failed,
        })
    }
}
impl Drop for RetainedToast {
    fn drop(&mut self) {
        if let Ok(_apartment) = WinRtApartment::enter() {
            let _ = self.notification.RemoveActivated(self.activated);
            let _ = self.notification.RemoveFailed(self.failed);
        }
    }
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

impl Backend {
    pub fn new(app: &AppHandle) -> Self {
        Self {
            app: app.clone(),
            app_id: app.config().identifier.clone(),
            toasts: Mutex::new(VecDeque::new()),
        }
    }
    fn notifier(&self) -> Result<ToastNotifier, String> {
        // The NSIS installer registers this same AppUserModelID.
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(&self.app_id))
            .map_err(|_| "notification_service_unavailable".to_owned())
    }
    pub fn status(&self, _: bool) -> Result<NotificationStatus, String> {
        let _apartment = WinRtApartment::enter()?;
        let setting = self
            .notifier()?
            .Setting()
            .map_err(|_| "notification_status_failed")?;
        Ok(windows_status(setting.0))
    }
    pub fn request_permission(&self, sound: bool) -> Result<NotificationStatus, String> {
        self.status(sound) // Desktop Windows has no app permission prompt.
    }
    pub fn open_settings(&self, _: bool) -> Result<(), String> {
        open::that("ms-settings:notifications").map_err(|_| "notification_settings_failed".into())
    }
    pub fn submit(&self, id: &str, title: &str, body: &str, sound: bool) -> Result<(), String> {
        let _apartment = WinRtApartment::enter()?;
        let audio = if sound {
            "<audio src=\"ms-winsoundevent:Notification.Default\"/>"
        } else {
            "<audio silent=\"true\"/>"
        };
        let xml = format!(
            "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual>{audio}</toast>",
            escaped(title), escaped(body),
        );
        let document = XmlDocument::new().map_err(|_| "notification_submit_failed")?;
        document
            .LoadXml(&HSTRING::from(xml))
            .map_err(|_| "notification_submit_failed")?;
        let notification = ToastNotification::CreateToastNotification(&document)
            .map_err(|_| "notification_submit_failed")?;
        let tag: String = id
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .take(16)
            .collect();
        notification
            .SetTag(&HSTRING::from(tag))
            .map_err(|_| "notification_submit_failed")?;
        notification
            .SetGroup(&HSTRING::from("FocusTimer"))
            .map_err(|_| "notification_submit_failed")?;

        // These callbacks activate an already running process. We do not claim
        // activation after Quit, which would require a registered COM activator.
        let toast = RetainedToast::new(notification, &self.app)?;
        let mut toasts = self
            .toasts
            .lock()
            .map_err(|_| "notification_service_unavailable")?;
        self.notifier()?
            .Show(&toast.notification)
            .map_err(|_| "notification_submit_failed")?;
        toasts.push_back(toast);
        while toasts.len() > 16 {
            toasts.pop_front();
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::{
        escaped, RoInitialize, ToastNotification, WinRtApartment, XmlDocument, HSTRING,
        RO_INIT_SINGLETHREADED,
    };

    #[test]
    fn notification_text_cannot_create_toast_markup() {
        assert_eq!(escaped("a<&\"'"), "a&lt;&amp;&quot;&apos;");
    }

    #[test]
    fn winrt_toasts_survive_fresh_sta_and_fresh_apartment_cycles() {
        // Exercise the real factories and retain a toast after its creating
        // thread's balanced apartment guards have exited. Each next apartment
        // must be able to use both the old toast and the cached factories.
        // No notification is shown and no OS permission is changed.
        let mut retained: Option<ToastNotification> = None;
        for existing_sta in [false, true, false] {
            let previous = retained.take();
            retained = Some(
                std::thread::spawn(move || {
                    let _existing = if existing_sta {
                        unsafe { RoInitialize(RO_INIT_SINGLETHREADED) }.unwrap();
                        Some(WinRtApartment)
                    } else {
                        None
                    };
                    let _apartment = WinRtApartment::enter().unwrap();
                    if let Some(previous) = previous {
                        assert!(previous.Content().is_ok());
                    }
                    let document = XmlDocument::new().unwrap();
                    document
                        .LoadXml(&HSTRING::from(
                            "<toast><visual><binding template=\"ToastGeneric\"><text>Focus</text></binding></visual><audio silent=\"true\"/></toast>",
                        ))
                        .unwrap();
                    let toast = ToastNotification::CreateToastNotification(&document).unwrap();
                    assert!(toast.Content().is_ok());
                    toast
                })
                .join()
                .unwrap(),
            );
        }
        let _apartment = WinRtApartment::enter().unwrap();
        assert!(retained.as_ref().unwrap().Content().is_ok());
        drop(retained);
    }
}
