//! One native notification owner; the operating system owns notification audio.
//! Submission is not a promise of presentation: Focus/Do Not Disturb, sound
//! settings, and notification-server policy remain in control.
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;

// Keep one wire vocabulary across platforms, even when an OS cannot produce
// every state (Windows has no prompt; freedesktop exposes no permission query).
#[cfg_attr(
    any(target_os = "windows", target_os = "linux"),
    allow(
        dead_code,
        reason = "Shared notification wire states vary by platform."
    )
)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Prompt,
    Granted,
    Denied,
    Unknown,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationStatus {
    pub permission: Permission,
    pub can_request: bool,
    pub notifications_allowed: Option<bool>,
    pub channel_enabled: Option<bool>,
    pub sound_allowed: Option<bool>,
    pub settings_available: bool,
}

impl NotificationStatus {
    fn blocked(&self) -> bool {
        matches!(
            self.permission,
            Permission::Prompt | Permission::Denied | Permission::Unsupported
        ) || self.notifications_allowed == Some(false)
            || self.channel_enabled == Some(false)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTestResult {
    pub submitted: bool,
    pub sound_requested: bool,
    pub status: NotificationStatus,
}

#[derive(Default)]
struct CompletionLedger {
    ids: VecDeque<String>,
}
impl CompletionLedger {
    fn claim(&mut self, id: &str) -> bool {
        if self.ids.iter().any(|known| known == id) {
            return false;
        }
        self.ids.push_back(id.to_owned());
        while self.ids.len() > 64 {
            self.ids.pop_front();
        }
        true
    }
}

pub struct NotificationService {
    backend: platform::Backend,
    completed: Mutex<CompletionLedger>,
}

impl NotificationService {
    /// Called once from Tauri setup. On macOS this also retains the foreground
    /// notification delegate for the application's lifetime.
    pub fn new(app: &AppHandle) -> Self {
        Self {
            backend: platform::Backend::new(app),
            completed: Mutex::new(CompletionLedger::default()),
        }
    }

    pub fn status(&self, sound: bool) -> Result<NotificationStatus, String> {
        self.backend.status(sound)
    }

    pub fn request_permission(&self, sound: bool) -> Result<NotificationStatus, String> {
        self.backend.request_permission(sound)
    }

    pub fn open_settings(&self, sound: bool) -> Result<(), String> {
        self.backend.open_settings(sound)
    }

    fn submit(&self, id: &str, body: &str, sound: bool) -> Result<NotificationTestResult, String> {
        let status = self.status(sound)?;
        if status.blocked() {
            return Ok(NotificationTestResult {
                submitted: false,
                sound_requested: false,
                status,
            });
        }
        self.backend.submit(id, "Focus", body, sound)?;
        Ok(NotificationTestResult {
            submitted: true,
            sound_requested: sound,
            status,
        })
    }

    /// Explicit user preview; it never reads or changes the timer or its history.
    pub fn test(&self, sound: bool, body: &str) -> Result<NotificationTestResult, String> {
        self.submit(&Uuid::new_v4().to_string(), body, sound)
    }

    pub fn completed(self: &Arc<Self>, app: AppHandle, run_id: &str, body: &str, sound: bool) {
        let claimed = self
            .completed
            .lock()
            .map(|mut ledger| ledger.claim(run_id))
            .unwrap_or(false);
        if !claimed {
            return;
        }
        let service = Arc::clone(self);
        let id = run_id.to_owned();
        let body = body.to_owned();
        // Do not block the timer, a webview command, or the native event loop
        // while an OS notification service answers.
        tauri::async_runtime::spawn_blocking(move || {
            if let Err(code) = service.submit(&id, &body, sound) {
                eprintln!("Focus: completion notification submission failed ({code}).");
                let _ = app.emit("focus://notification-error", code);
            }
        });
    }
}

#[cfg(any(target_os = "macos", test))]
fn mac_status(
    authorization: isize,
    sound: isize,
    alerts: isize,
    center: isize,
) -> NotificationStatus {
    let permission = match authorization {
        0 => Permission::Prompt,
        1 => Permission::Denied,
        2..=4 => Permission::Granted,
        _ => Permission::Unknown,
    };
    let allowed = match permission {
        Permission::Prompt | Permission::Denied => Some(false),
        Permission::Granted => Some(true),
        _ => None,
    };
    // A user may disable banners while retaining Notification Center entries.
    // Both presentation routes disabled is a truthful blocked result.
    let presentation = match (alerts, center) {
        (2, _) | (_, 2) => Some(true),
        (1, 1) => Some(false),
        _ => None,
    };
    NotificationStatus {
        permission,
        can_request: permission == Permission::Prompt,
        notifications_allowed: allowed,
        channel_enabled: presentation,
        sound_allowed: match sound {
            2 => Some(true),
            1 => Some(false),
            _ => None,
        },
        settings_available: true,
    }
}

#[cfg(any(target_os = "windows", test))]
fn windows_status(setting: i32) -> NotificationStatus {
    let (permission, allowed) = match setting {
        0 => (Permission::Granted, Some(true)),
        1..=4 => (Permission::Denied, Some(false)),
        _ => (Permission::Unknown, None),
    };
    NotificationStatus {
        permission,
        can_request: false,
        notifications_allowed: allowed,
        channel_enabled: None,
        sound_allowed: None,
        settings_available: true,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod platform {
    use super::*;
    pub struct Backend;
    impl Backend {
        pub fn new(_: &AppHandle) -> Self {
            Self
        }
        pub fn status(&self, _: bool) -> Result<NotificationStatus, String> {
            Ok(NotificationStatus {
                permission: Permission::Unsupported,
                can_request: false,
                notifications_allowed: Some(false),
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
        pub fn submit(&self, _: &str, _: &str, _: &str, _: bool) -> Result<(), String> {
            Err("notifications_unsupported".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_claims_are_idempotent_across_interleaved_runs_and_bounded() {
        let mut ledger = CompletionLedger::default();
        assert!(ledger.claim("a"));
        assert!(ledger.claim("b"));
        assert!(!ledger.claim("a"));
        assert!(!ledger.claim("b"));
        for i in 0..100 {
            ledger.claim(&i.to_string());
        }
        assert_eq!(ledger.ids.len(), 64);
        assert!(!ledger.claim("99"));
    }

    #[test]
    fn mac_permission_and_presentation_do_not_claim_unavailable_alerts() {
        let prompt = mac_status(0, 0, 0, 0);
        assert!(prompt.can_request && prompt.blocked());
        assert_eq!(prompt.permission, Permission::Prompt);
        assert!(mac_status(1, 1, 1, 1).blocked());
        let silent = mac_status(2, 1, 2, 2);
        assert!(!silent.blocked());
        assert_eq!(silent.sound_allowed, Some(false));
        assert!(!mac_status(2, 2, 1, 2).blocked());
        assert!(mac_status(2, 2, 1, 1).blocked());
        assert_eq!(mac_status(99, 0, 0, 0).permission, Permission::Unknown);
    }

    #[test]
    fn windows_policy_user_and_app_blocks_are_reported_without_fake_prompts() {
        assert!(!windows_status(0).blocked());
        for setting in 1..=4 {
            let status = windows_status(setting);
            assert!(status.blocked());
            assert_eq!(status.permission, Permission::Denied);
            assert!(!status.can_request);
        }
        let unknown = windows_status(99);
        assert_eq!(unknown.permission, Permission::Unknown);
        assert_eq!(unknown.notifications_allowed, None);
        assert!(!unknown.can_request);
    }
}
