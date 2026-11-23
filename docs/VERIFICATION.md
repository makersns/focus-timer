# Verification and release gates

This document records the verification scope for Focus 0.1.0. It separates current local checks, native UI observations, historical results and the gates that still need to pass before final installers are published.

**Release validation remains open.** The previous `f20…` installer generation predates the new widget family, notification permission handling and macOS native popover/panel changes. Those older installers and their passing checks are historical evidence, not evidence for the current release candidate. The final clean source commit, its required CI results and its actual installer set must agree.

## Current verification matrix

| Area | Status at this checkpoint | Scope and limit |
| --- | --- | --- |
| Frontend | Passed locally: 37 tests, strict TypeScript and production build | Includes three rendered permission-guidance regressions. Both current local macOS app builds passed after that correction. Local builds do not replace final source/CI/package provenance checks. |
| macOS Rust | Passed locally: 54 tests and Clippy with warnings denied | Includes the three new native-panel tests. Unit tests do not establish visibility or interaction in another application's full-screen Space. |
| Android JVM and lint | Passed locally: 11 JVM tests and ARM64 release lint | Native presentation/widget logic and release static analysis. This does not replace installing the final signed APK. |
| Current main-window web layout | Passed at 820 × 840 and 640 × 620 | No page overflow in the observed layouts. The 640 × 620 browser case is an extra layout stress check below the native window's 640 × 640 minimum; it is not a native window-size claim. |
| Flat, closed ring | Passed in the current web UI inspection | The neutral track is continuous and the ring SVG filter is `none`. Digits count down while colored progress grows with elapsed time. |
| macOS native menu popover | Passed in real GUI checks | Correct placement in a true full-screen Space; Escape, outside click and second-click dismissal; opening the main window through IPC. The normal app's timer/history were unchanged. |
| macOS Mini over another app's full-screen Space | Passed in real GUI checks on the v3 QA app | Pinned Ring, Tomato and Capsule were visible over a separate full-screen application without activating Focus or exposing its main window. First-click actions, native dragging, hide/reopen and unpin behavior were exercised; scope and receipts are below. |
| macOS native notification UI | Passed: actual authorization and both test-notification modes | The OS permission prompt appeared only after an explicit action; grant/refreshed status and real notification UI were observed. Test requests preserved the entire paused timer/history/preferences state. Sound-request flags were verified, not physical audibility. |
| macOS natural completion notification | Passed on the local v4 QA app | A real one-minute run completed while Focus was inactive and its main window offscreen, with Mini above another full-screen app. Exactly one 60,000 ms completed record was added; timer/history stayed stable for another eight seconds, preferences and run identity were preserved, and the OS completion notification was observed. |
| Normal macOS app upgrade | Passed on the installed local v4 app | Existing timer, preferences, five session records, daily totals and cycle count were unchanged. The installed Tomato Mini also remained visible above another app's full-screen Space without activating Focus or changing user state. |
| Android notification behavior | Passed in the earlier dedicated native QA run: 22 cases | Permission denial/retry/settings return, test notification, sound/channel/mute behavior and headless expiry were checked. This is not a substitute for final signed-package acceptance. |
| Windows and Linux for the current source commit | Pending required CI and package gates | Earlier Windows GUI and Linux build results are retained below as history; they do not certify the revised current source. |
| Current three-style Android release package | Pending final package acceptance | JVM/lint results and earlier native timer/AppWidget evidence are available; fresh signed APK install, launcher controls and release provenance still require the final receipt. |
| Final release | Pending | Current-commit CI, exact source/tag agreement, all installer checks and final published checksums must pass. No older package is substituted. |

The [CI workflow](https://github.com/makersns/focus-timer/actions/workflows/ci.yml) has jobs for Ubuntu 22.04, Windows Server 2022, Apple Silicon and Intel macOS 15, plus an Ubuntu Android ARM64 build. A workflow definition or an older successful run is not a passing result for the final source commit.

## Current UI and reminder behavior

The main window uses an integrated title area, a flat closed dial and the primary control inside the dial. Desktop minis have three styles: Ring at 260 × 260, Tomato at 260 × 280 and Capsule at 340 × 112 logical pixels. macOS and Windows minis are floating application windows; the macOS surface is not a WidgetKit system widget. Android uses a real launcher AppWidget.

Progress represents elapsed time. A reset or idle round has no colored foreground dot; progress grows as the countdown decreases, freezes on pause and fills on completion. The neutral track stays fully closed. The main dial and Ring have no outer shadow or bevel; Tomato uses a contrasting pale arc against its colored face.

Completion notifications start disabled. Permission is requested only through an explicit enable/test action, not automatically at launch. Settings reports the platform's actual status, including an unknown or unsupported state where appropriate, and provides a system-settings action or guidance. The sound option belongs to the system notification; there is no second independent player. The operating system controls mute, volume and Do Not Disturb. A successful request is not described as proof that the user saw or heard the notification. Testing a notification must not change the timer or session history.

### Current macOS menu evidence

The local, ignored QA folder is `.data/final-macos-install-20261124/`. Its relevant receipts are:

- `v2-menu-fullscreen.json`: real menu-item/popover placement in a true full-screen Space.
- `v2-menu-dismissal.json`: Escape, outside-click and second-click dismissal.
- `v2-menu-open-main.json`: opening the main window through the native/webview action.

These are local evidence filenames, not public download links. The normal application timer/history were checked unchanged during this menu QA. The separate Mini NSPanel requirement was subsequently verified on the v3 QA app as described below.

### Current macOS Mini evidence

An isolated `com.makersns.focus.qa.v3` app, with a binary fingerprint beginning `160ae553`, was exercised on macOS 15.5 with one built-in display. Root reviewed actual screenshots of all three native mini windows above a separate application's true full-screen Space: Ring 260 × 260, Tomato 260 × 280 and Capsule 340 × 112 logical pixels. Focus QA remained inactive, its main window was not onscreen, and the full-screen application remained active.

| Interaction | Observed result | Local evidence in the same QA folder |
| --- | --- | --- |
| Full-screen visibility | All three pinned styles were visible above the independent full-screen app. Switching styles preserved the top-left position. | `v3-ring-fullscreen.json`, `v3-tomato-fullscreen.json`, `v3-capsule-fullscreen.json` |
| First mouse click | A real first click paused the Ring and Capsule timers while the QA app was inactive. This was not a scripted DOM action. | `v3-widget-first-click.json` |
| Dragging | A real 180 × 120 drag changed the native panel frame by the observed displacement. | `v3-widget-drag.json` |
| Hiding | Hiding left the same round running and preserved its identity/history. | `v3-widget-hide.json` |
| Reopening | Showing the mini from the menu restored the same round, with the countdown already advanced. | `v3-widget-reopen.json` |
| Unpinning | The native window level returned to 0 without modifying the timer or switching away from the full-screen Space. | `v3-widget-unpin.json` |

This is actual single-display coverage. Mixed-DPI placement has geometry unit tests but has not been exercised on physical mixed-DPI displays. The later notification-guidance correction changes neither Mini styling nor the native panel implementation. Its local app builds have passed; final source/CI/package provenance checks remain required.

### Current macOS notification evidence

The isolated `com.makersns.focus.qa.v3` app did not show an OS permission prompt on first launch. Choosing Allow notifications opened the real macOS authorization dialog, and the operating-system Allow action succeeded. When the user decision exceeded the native 120-second wait, the UI exposed a retry; refreshing then accurately showed that system notifications were allowed.

Both explicit test modes reached actual macOS notification UI with the expected test body: the sound-enabled request reported `soundRequested: true`, and the silent request reported `soundRequested: false`. The complete paused timer, history and preferences JSON was unchanged before and after either test. Saving notification/sound enablement changed only preferences, leaving the timer and history unchanged.

The receipts in `.data/final-macos-install-20261124/` are `v3-native-notification-prompt.json`, `v3-notification-granted-refreshed.json`, `v3-notification-test-sound.json` and `v3-notification-test-silent.json`. These establish actual OS authorization and visible notification delivery, plus the requested sound configuration. No physical speaker/listening channel was available, so audibility is not claimed.

The v4 QA app incorporates the corrected first-run guidance. Its local build, the normal app build, 37 frontend tests and 54 macOS native tests passed. Final screenshots use that updated QA app; the v3 native observations keep their actual build identity.

### Current macOS completion and installed-app evidence

The v4 QA app completed a real one-minute focus round set and started through the UI. During the run, Focus QA was inactive, the main window was offscreen and Mini remained above a separate full-screen application. At zero, exactly one `completed` history entry with 60,000 ms elapsed time was added. After another eight seconds, the timer remained at zero and the history was unchanged; preferences and run identity were preserved. The actual macOS notification UI displayed the completion body. This confirms visible delivery, not physical audibility.

Local receipts are `v4-natural-completion.json` and `v4-natural-completion.png`. The subsequent “start short break” Mini action also passed through the real IPC path without changing history, recorded in `v4-next-phase.json`.

The normal installed app was upgraded to the local v4 build with binary fingerprint prefix `74b1bbf6`. Its existing five records, timer, preferences, daily totals and cycle count matched before and after the upgrade. The normal app's Tomato Mini was also observed over another app's true full-screen Space with the main window offscreen and Focus inactive; the full user state stayed unchanged. Receipts are `normal-v4-install/installation.json`, `v4-installed-fullscreen.json` and `v4-installed-fullscreen.png` in the same local evidence folder. The isolated QA instance was exited after acceptance.

This is acceptance of the local installed macOS app. The final clean-source CI and cross-platform distribution artifacts still require their own source/version/checksum verification.

## Historical verification

The following results preserve earlier verification scope and provenance. They apply to the earlier implementation/package generation, including the old `f20…` baseline, and are not counted as completion of current-release gates.

### Earlier automated suites

Earlier verification recorded 25 frontend tests, 45 desktop Rust tests and 2 Kotlin presentation tests. Those suites covered timer transitions, record accounting, clock changes, persisted-state validation and recovery, preference migration, translation completeness, out-of-order native responses and native next-phase behavior. They have since grown to the current local counts in the matrix above.

Earlier Android release lint recorded no errors and no NewApi, MissingPermission, foreground-service or PendingIntent errors. Its remaining warnings concerned toolchain suggestions, generated/template resources, optional newer-platform widget metadata and layout/style recommendations. No lint baseline or error suppression was added to obtain that result.

### Earlier macOS observations

A macOS 15.5 Apple Silicon session checked main windows at 820 × 840 and 640 × 640 logical pixels and the previous rectangular mini timer at 360 × 176. That mini size belongs to the old design and does not describe the new Ring, Tomato or Capsule windows.

Observed earlier checks included:

- The title area shared the page background and retained real system traffic lights.
- Main, mini and menu-bar text showed the same round. Mini pause and main-window resume synchronized the same timer.
- Hiding the mini did not pause the timer; reopening showed the same round.
- Pinned and unpinned windows retained their native levels while a separate foreground guard held focus during six-second observations.
- Light/dark surfaces, the custom language picker, draft cancellation and Escape behavior worked in WKWebView. A dark auxiliary-window transparency regression was fixed and rechecked.
- Normal application termination saved a paused state with no running deadline and preserved round identity/history. Tauri ExitRequested and the native macOS Exit path used the same idempotent save operation.

An isolated QA bundle was used for public screenshots and destructive test cases, with normal application state backed up and checked unchanged. Screenshot clocks came from actual timers.

The earlier machine session reported a status item outside visible screen bounds, preventing real popover click/blur acceptance then. That historical gap is superseded by the current native NSPopover GUI results above; it was never a passed check in the earlier evidence.

### Earlier Windows observations

The Windows UI job launched the real Tauri/WebView2 application through a matching WebDriver. Its seven passing checks covered startup, shared main/mini actions, pinning, hiding/reopening and continued tray operation after closing the main window.

Pinned and unpinned tests each observed approximately six seconds of updates while a separate native window held the foreground. The probe selected the mini by process and exact native title, excluding small internal Tao/single-instance windows. No timer-driven focus theft or stacking changes were observed in that earlier passing run. The revised source still needs its own required Windows run.

### Earlier Linux coverage

Ubuntu 22.04 provided Rust tests, Clippy and package-build coverage. No local Linux GUI observation was recorded. Tray availability and window stacking remain dependent on desktop environment and compositor.

### Earlier Android observations

A dedicated ARM64 API 36 emulator with 16 KB memory pages exercised the app and AppWidget. This is emulator coverage, not a claim about every physical device or Android version.

Earlier observations included:

- Cold launch, correct system-bar safe areas and no unsolicited notification permission request.
- Real launcher pin confirmation and a 4 × 2 home-screen widget.
- Starting in the app, pausing in the widget, stable paused time and resuming the same round.
- Same-boot process reclamation: foreground service restart without an Activity, Rust bridge loading and preservation of the original round/deadline.
- A real one-minute completion remaining at 00:00, with exactly one history record and one completed session; the next-phase action started the appropriate break.
- Five languages and light/dark/system appearance changes without modifying the round or paused remainder.
- Device reboot restoring the same round as paused and marked recovered.

The separate notification QA receipt, `notification-qa.json`, records the 22-case native permission/reminder run noted in the current matrix. A final signed APK still needs fresh installation, cold launch and app/widget controls because shrinking and signing differ from a debug build. The private signing key is never stored in Git or release assets.

### Earlier browser interaction checks

The custom language menu was exercised at 820 × 840, 640 × 640, 320 × 740 and 320 × 568. Checks covered opening focus; Arrow, Home, End, typeahead, Enter/Space, Escape and Tab; outside dismissal; draft cancellation; all five language choices; persistence after reload; upward opening; and scrolling in a constrained list.

Those earlier interaction results remain useful history. The current main-layout observations in the matrix have their own dimensions and scope. Browser preview data is separate and provides no proof of native operating-system behavior.

## Installer publication gates

Publish only artifacts tied to the final clean source commit and its passing required CI. The release collector and verifier must agree on package versions, target architectures, byte sizes, SHA-256 hashes, source commit and Android release-certificate metadata.

The final set must include a macOS universal DMG, Windows x64 installer, Linux x64 AppImage/DEB/RPM, Android ARM64 APK and combined `SHA256SUMS.txt`. The current local test results do not mark that final set as built, accepted or published.

The macOS package uses an ad-hoc signature and is not Apple-notarized; Windows has no publisher certificate in this release. Earlier `f20…` installers, arbitrary CI artifacts, isolated QA bundles and debug-signed APKs cannot stand in for the final release set.

See [contributing](../CONTRIBUTING.md), [Android behavior and build instructions](ANDROID.md) and the [release workflow](https://github.com/makersns/focus-timer/actions/workflows/release.yml) for the required commands and gates.
