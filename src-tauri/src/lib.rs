mod storage;
mod timer;

#[cfg(target_os = "macos")]
mod macos_popover;
#[cfg(target_os = "macos")]
mod macos_widget;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod desktop;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod desktop_alerts;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use desktop::run;

#[cfg(target_os = "android")]
mod mobile;

#[cfg(target_os = "android")]
#[tauri::mobile_entry_point]
pub fn run() {
    mobile::run();
}
