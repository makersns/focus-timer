use super::{NotificationStatus, Permission};
use notify_rust::{Hint, Notification, NotificationHandle};
use std::{collections::VecDeque, sync::Mutex};
use tauri::AppHandle;

pub struct Backend {
    // GNOME may dismiss a notification when its originating connection closes.
    handles: Mutex<VecDeque<NotificationHandle>>,
}
impl Backend {
    pub fn new(_: &AppHandle) -> Self {
        Self {
            handles: Mutex::new(VecDeque::new()),
        }
    }
    pub fn status(&self, _: bool) -> Result<NotificationStatus, String> {
        // The freedesktop protocol has no portable permission/settings API.
        Ok(NotificationStatus {
            permission: Permission::Unknown,
            can_request: false,
            notifications_allowed: None,
            channel_enabled: None,
            sound_allowed: None,
            settings_available: false,
        })
    }
    pub fn request_permission(&self, sound: bool) -> Result<NotificationStatus, String> {
        self.status(sound)
    }
    pub fn open_settings(&self, _: bool) -> Result<(), String> {
        Err("notification_settings_unavailable".into())
    }
    pub fn submit(&self, _: &str, title: &str, body: &str, sound: bool) -> Result<(), String> {
        let mut notice = Notification::new();
        notice
            .appname("Focus")
            .summary(title)
            .body(body)
            .auto_icon()
            .hint(Hint::SuppressSound(!sound));
        if sound {
            notice.sound_name("message-new-instant");
        }
        let handle = notice.show().map_err(|_| "notification_submit_failed")?;
        let mut handles = self
            .handles
            .lock()
            .map_err(|_| "notification_service_unavailable")?;
        handles.push_back(handle);
        while handles.len() > 16 {
            handles.pop_front();
        }
        Ok(())
    }
}
