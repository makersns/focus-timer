# Verification and release gates

This document records the verification scope for Focus 0.1.0 and the checks required before installers are published. It distinguishes native UI observations from build and static checks.

## Automated checks

The [CI workflow](https://github.com/makersns/focus-timer/actions/workflows/ci.yml) runs on Ubuntu 22.04, Windows Server 2022, macOS 15 on Apple Silicon, and macOS 15 on Intel. An additional Ubuntu job builds the Android ARM64 application.

The current suites contain 25 frontend tests, 45 desktop Rust tests, and 2 Kotlin presentation tests. They cover timer transitions, record accounting, clock changes, persisted-state validation and recovery, preference migration, translation completeness, out-of-order native responses, and native next-phase behavior. TypeScript, production frontend builds, Rust formatting, and Clippy with warnings denied are required gates.

Android release lint has also been run against the maintained native project. The checked result has no errors and no NewApi, MissingPermission, foreground-service, or PendingIntent errors. Remaining warnings concern toolchain update suggestions, generated/template resources, optional newer-platform widget metadata, and layout/style recommendations. No lint baseline or error suppression was added to obtain this result.

## Native desktop observations

### macOS

A real macOS 15.5 Apple Silicon session was used for native window checks. Main windows were checked at 820 × 840 and 640 × 640 logical pixels; the mini timer was checked at 360 × 176.

Observed checks include:

- The title area shares the page background and uses real system traffic lights.
- Main-window, mini-timer, and menu-bar text show the same round and remaining time. Pausing from the mini timer updates all three; resuming from the main window restores the same round.
- Closing the mini timer leaves the timer running. Reopening it shows the same round.
- Pinned and unpinned mini windows retain their native levels. A separate foreground guard kept focus throughout each six-second observation while the timer updated.
- Light and dark surfaces, custom language selection, draft cancellation, and Escape behavior were exercised in WKWebView. The dark auxiliary-window transparency regression was corrected and rechecked in real windows.
- Normal macOS application termination saves a paused state with no running deadline, preserving the round identity and history. Both Tauri ExitRequested and the native macOS Exit path use the same idempotent save operation.

An isolated QA bundle identifier was used for public screenshots and destructive test cases. The normal application state was backed up and verified unchanged after those checks. Screenshot clocks are from actual running timers with neutral demonstration text.

The machine's menu-bar status item was reported outside the visible screen bounds. Consequently, a real click and blur cycle for the native menu-bar popover could not be completed in that environment. The menu-bar text synchronization and positioning logic have been checked; this document does not count the unavailable click sequence as a passed GUI test.

### Windows

The mandatory Windows UI job launches the real Tauri/WebView2 application through a matching WebDriver. Its seven checks cover startup, shared main/mini actions, pinning, hiding/reopening the mini timer, and continued tray operation after closing the main window.

Pinned and unpinned tests each observed approximately six seconds of timer updates while a separate native window held the foreground. The probe identifies the mini timer by process and exact native title; small internal Tao/single-instance windows are deliberately excluded. No timer-driven focus theft or stacking changes were observed in the passing run.

### Linux

Linux has native Rust test, Clippy, and packaging coverage on Ubuntu 22.04. Tray display and window-stacking behavior still depend on the user's desktop environment and compositor; a Linux GUI session was not part of the local desktop observations above.

## Android observations

The Android UI and AppWidget were exercised in a dedicated ARM64 API 36 emulator using 16 KB memory pages. This is emulator coverage, not a claim of testing every physical device or Android version.

Observed checks include:

- Cold launch, correct system-bar safe areas, and no unsolicited notification permission request.
- The actual launcher pin confirmation and a 4 × 2 home-screen widget.
- Starting in the application, pausing in the widget, stable time while paused, and resuming the same round from the widget.
- Same-boot process reclamation: the foreground service restarted without an Activity, loaded the Rust bridge, and retained the original round and deadline.
- A real one-minute completion remained at 00:00 after completion and produced exactly one history record and one completed session. The next-phase widget action started the appropriate break.
- Five language variants and light/dark/system appearance updates without changing the round or paused remainder.
- Device reboot restored the same round as paused and marked it as recovered.

The release APK must additionally be installed afresh after signing and checked for cold launch and app/widget controls. This matters because release shrinking and signing differ from debug builds. The private publisher key is never stored in Git or included in release assets.

## UI preview checks

The custom language menu was exercised in a browser at 820 × 840, 640 × 640, 320 × 740, and 320 × 568. Checks included focus on opening; Arrow, Home, End, typeahead, Enter/Space, Escape, and Tab; outside dismissal; cancellation of unsaved choices; all five language choices; persistence after reload; upward opening; and manual scrolling in a height-constrained list. The preview uses separate browser data and is not evidence of native operating-system integration.

## Installer publication

Publish only artifacts tied to the final clean source commit. The release collector and verifier check package versions, target architectures, byte sizes, SHA-256 hashes, source commit agreement, and the Android release-certificate metadata. The final release must include macOS universal DMG, Windows x64 installer, Linux x64 AppImage/DEB/RPM, Android ARM64 APK, and a combined SHA256SUMS.txt.

The macOS app uses an ad-hoc signature and is not Apple-notarized; Windows has no publisher certificate in this release. CI artifacts, isolated QA bundles, and debug-signed Android APKs are not substitutes for the published installer set.

See [contributing](../CONTRIBUTING.md), [Android behavior and build instructions](ANDROID.md), and the [release workflow](https://github.com/makersns/focus-timer/actions/workflows/release.yml) for reproducible commands and gates.
