# Changelog

## 0.1.0

First independent desktop and Android release of Focus by Sugar.

### Added

- Tauri 2 app for macOS, Windows, Linux, and Android with an original app icon.
- Focus, short breaks, and long breaks; customizable 1–180 minute rounds.
- Native deadline-based timer, pause/resume, tray controls, and opt-in notifications.
- Seamless single-column interface with a flat, closed dial and elapsed progress that grows as the countdown runs.
- Live macOS menu-bar clock and an anchored native control panel that also works in another app's full-screen Space.
- Ring, Tomato and Capsule floating mini timers on macOS/Windows/Linux, sharing the same round and saved appearance/pin preferences.
- Nonactivating macOS mini panel that stays above other apps' full-screen Spaces when pinned, without bringing the main window forward.
- Native Android AppWidget with three styles, foreground timing, exact completion alarms and same-boot recovery.
- Explicit notification-permission status, test actions and system-settings guidance; optional sound through the single completion notification.
- Paused desktop/reboot recovery; atomic local saves with corrupt-file preservation.
- Completed and early-ended focus records, daily totals, and recent-session history.
- Light, dark, and system themes with five interface languages.
- Keyboard controls, confirmation dialogs, reduced-motion support, and responsive layout.
- Automated timer/storage/locale tests, five-platform source CI, native desktop/emulator acceptance, platform manifests and release checksums.
- Five-language documentation and real native screenshots, with public release provenance and verification scope.

### Fixed

- macOS menu controls anchor to the actual status item and stay usable over another application's full-screen Space.
- Desktop mini interaction preserves timer identity and position across hiding, pinning and style changes.
- Android widget text colors refresh correctly when recycled views switch themes.
- Windows notification objects retain a compatible process-lifetime MTA reference across worker and STA lifetimes.

### Release scope

Desktop and Android. No accounts, cloud synchronization, automatic next rounds, iOS package, or automatic updater. First-release packages are not Apple-notarized or signed with a Windows publisher certificate.

See the [bilingual release notes](docs/releases/v0.1.0.md) for downloads and platform details.
