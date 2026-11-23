use super::{mac_status, NotificationStatus, Permission};
use block2::{DynBlock, RcBlock};
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{Bool, ProtocolObject},
    AnyThread, DefinedClass,
};
use objc2_foundation::{NSBundle, NSError, NSObject, NSObjectProtocol, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotification,
    UNNotificationDefaultActionIdentifier, UNNotificationPresentationOptions,
    UNNotificationRequest, UNNotificationResponse, UNNotificationSettings, UNNotificationSound,
    UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};
use std::{cell::RefCell, ptr::NonNull, sync::mpsc, time::Duration};
use tauri::{AppHandle, Manager};

define_class!(
    // NSObject has no subclassing obligations. AppHandle is thread-safe; the
    // delegate never mutates shared Objective-C state from a callback.
    #[unsafe(super = NSObject)]
    #[thread_kind = AnyThread]
    #[name = "FocusCompletionNotificationDelegate"]
    #[ivars = AppHandle]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}
    unsafe impl UNUserNotificationCenterDelegate for Delegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            notification: &UNNotification,
            completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            let mut options =
                UNNotificationPresentationOptions::Banner | UNNotificationPresentationOptions::List;
            // This is the same OS notification's sound, never an extra player.
            if notification.request().content().sound().is_some() {
                options |= UNNotificationPresentationOptions::Sound;
            }
            completion.call((options,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_respond(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion: &DynBlock<dyn Fn()>,
        ) {
            // Dismissing a notification must not open a hidden window.
            if &*response.actionIdentifier() == unsafe { UNNotificationDefaultActionIdentifier } {
                let app = self.ivars().clone();
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Some(main) = handle.get_webview_window("main") {
                        let _ = main.unminimize();
                        let _ = main.show();
                        let _ = main.set_focus();
                    }
                });
            }
            completion.call(());
        }
    }
);
impl Delegate {
    fn new(app: AppHandle) -> Retained<Self> {
        let this = Self::alloc().set_ivars(app);
        // NSObject's init has this exact signature.
        unsafe { msg_send![super(this), init] }
    }
}
thread_local! {
    // UNUserNotificationCenter.delegate is weak. Retain it on the setup/main
    // thread for the whole app lifetime, independent of webview lifecycles.
    static DELEGATE: RefCell<Option<Retained<Delegate>>> = const { RefCell::new(None) };
}

pub struct Backend {
    available: bool,
}
impl Backend {
    pub fn new(app: &AppHandle) -> Self {
        // UNUserNotificationCenter can raise an exception for an unbundled dev
        // binary. Do not impersonate Terminal or another app's permissions.
        let available = NSBundle::mainBundle()
            .bundleIdentifier()
            .is_some_and(|identifier| identifier.to_string() == app.config().identifier);
        if available {
            let delegate = Delegate::new(app.clone());
            UNUserNotificationCenter::currentNotificationCenter()
                .setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
            DELEGATE.with(|slot| *slot.borrow_mut() = Some(delegate));
        }
        Self { available }
    }

    pub fn status(&self, _: bool) -> Result<NotificationStatus, String> {
        if !self.available {
            return Ok(NotificationStatus {
                permission: Permission::Unsupported,
                can_request: false,
                notifications_allowed: Some(false),
                channel_enabled: None,
                sound_allowed: None,
                settings_available: false,
            });
        }
        let (send, receive) = mpsc::sync_channel(1);
        let block = RcBlock::new(move |settings: NonNull<UNNotificationSettings>| {
            // Apple supplies a valid settings object for the callback duration.
            let settings = unsafe { settings.as_ref() };
            let status = mac_status(
                settings.authorizationStatus().0,
                settings.soundSetting().0,
                settings.alertSetting().0,
                settings.notificationCenterSetting().0,
            );
            let _ = send.try_send(status);
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .getNotificationSettingsWithCompletionHandler(&block);
        receive
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "notification_status_failed".into())
    }

    pub fn request_permission(&self, sound: bool) -> Result<NotificationStatus, String> {
        let current = self.status(sound)?;
        if !current.can_request {
            return Ok(current);
        }
        let (send, receive) = mpsc::sync_channel(1);
        let block = RcBlock::new(move |_granted: Bool, error: *mut NSError| {
            let _ = send.try_send(error.is_null());
        });
        // Always request the optional sound capability alongside alerts, so
        // toggling the user's sound preference later never needs a new prompt.
        UNUserNotificationCenter::currentNotificationCenter()
            .requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
                &block,
            );
        match receive.recv_timeout(Duration::from_secs(120)) {
            Ok(true) => self.status(sound),
            _ => Err("notification_permission_failed".into()),
        }
    }

    pub fn open_settings(&self, _: bool) -> Result<(), String> {
        // Fixed system destination; never accepts a URL or shell command.
        open::that("x-apple.systempreferences:com.apple.Notifications-Settings.extension")
            .map_err(|_| "notification_settings_failed".into())
    }

    pub fn submit(&self, id: &str, title: &str, body: &str, sound: bool) -> Result<(), String> {
        if !self.available {
            return Err("notifications_unsupported".into());
        }
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(title));
        content.setBody(&NSString::from_str(body));
        let notification_sound = sound.then(UNNotificationSound::defaultSound);
        content.setSound(notification_sound.as_deref());
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&format!("focus.{id}")),
            &content,
            None,
        );
        let (send, receive) = mpsc::sync_channel(1);
        let block = RcBlock::new(move |error: *mut NSError| {
            let _ = send.try_send(error.is_null());
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .addNotificationRequest_withCompletionHandler(&request, Some(&block));
        match receive.recv_timeout(Duration::from_secs(5)) {
            Ok(true) => Ok(()),
            _ => Err("notification_submit_failed".into()),
        }
    }
}
