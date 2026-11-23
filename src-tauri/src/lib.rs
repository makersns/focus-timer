mod storage;
mod timer;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod desktop;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use desktop::run;

#[cfg(target_os = "android")]
mod mobile;

#[cfg(target_os = "android")]
#[tauri::mobile_entry_point]
pub fn run() {
    mobile::run();
}
