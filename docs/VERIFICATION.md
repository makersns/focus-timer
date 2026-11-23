# Focus 0.1.0 — verification record

Published on **23 November 2026 (UTC)**. [Download the stable release](https://github.com/makersns/focus-timer/releases/tag/v0.1.0). This record separates final-source package checks from the earlier native interaction sessions whose build identities are retained below.

## Source and automated checks

The release source is `6ace789bfe63db7dde3449236d7cdde9ca591d0c`, tagged `v0.1.0`. The [main CI run](https://github.com/makersns/focus-timer/actions/runs/45241402661) passed all five required jobs: Ubuntu 22.04, Windows Server 2022, Apple Silicon macOS 15, Intel macOS 15, and Android ARM64. The initial [tag workflow](https://github.com/makersns/focus-timer/actions/runs/45243141787) encountered GitHub-hosted macOS ARM capacity constraints: GitHub cancelled that job before any test step ran, while the other four check jobs passed. Its packaging jobs correctly did not start. The successful five-job main CI already tested the exact same source. The replacement [Windows/Linux packaging run](https://github.com/makersns/focus-timer/actions/runs/45247245331) therefore verifies that canonical CI run, all five successful jobs and the unchanged release tag before building. It still uses the standard native tests and build/collector scripts. The macOS universal and signed Android packages were built locally from that same clean source, with separate platform acceptance receipts. The product source and version tag were not changed to work around the hosted-runner cancellation.

| Area | Verified result | Practical scope |
| --- | --- | --- |
| Frontend | 37 tests, strict TypeScript and production build passed | Timer presentation, preference/localization behavior and the three rendered notification-permission guidance regressions. |
| macOS Rust | 54 tests and warnings-denied Clippy passed | Includes native panel behavior and geometry tests. Actual full-screen visibility was also checked separately below. |
| Windows Rust | 52 tests passed, with no ignored or filtered tests; Clippy passed | Includes the real WinRT fresh-thread → STA → fresh-thread lifecycle regression. |
| Linux Rust | Required native tests and Clippy passed | Automated Linux coverage; no Linux desktop GUI session was observed. |
| Android | ARM64 build, 11 local JVM tests and release lint passed | Lint had no errors. The final signed APK was also installed and exercised, as described below. |
| Windows GUI | 7 real Tauri/WebView2 scenarios passed | 110 operating-system foreground/z-order observations across pinned and unpinned checks, plus actual native screenshots. |
| Main web layout | Checked at 820 × 840 and 640 × 620 | No page overflow in these observations. The second size is an extra browser stress case below the native 640 × 640 window minimum. |

The first Windows run for source `15b0c385…` terminated with `STATUS_ACCESS_VIOLATION` during the native test run. Review identified a mismatch between temporary apartment lifetimes and retained WinRT factories/objects; the failed run did not provide a native stack identifying the exact crashing instruction. The fix retains a process-lifetime MTA usage reference alongside the Windows runtime's process-lifetime activation-factory cache. Per-thread initialization is still balanced. The strengthened test retains and uses notification objects across fresh worker and STA lifetimes; it passes on the final source. No test was disabled to obtain the passing result.

## Interface and timer behavior

Digits show time remaining. The colored arc shows elapsed time: it starts empty, grows during a round, freezes when paused, fills at completion and clears on reset. The neutral track is continuous and closed; the Ring style uses a complete circle. The main dial and Ring mini have no outer shadow, bevel or glow. Tomato uses a contrasting pale arc on its colored face.

Desktop minis share the application's native timer. Ring is 260 × 260, Tomato 260 × 280, and Capsule 340 × 112 logical pixels. Choosing a different style does not restart the round. Desktop minis are floating application windows; the macOS mini is not a WidgetKit system widget. Android uses an actual launcher AppWidget.

Completion notifications start disabled. Permission is requested only after an explicit enable or test action. Settings reports the platform's actual status and provides a system-settings action or appropriate guidance. Testing a notification must leave the timer and history unchanged. Sound belongs to the single system notification; there is no independent second audio player. Operating-system mute, volume, notification-channel and Do Not Disturb policy still apply.

## macOS native acceptance

Actual GUI checks used macOS 15.5 on Apple Silicon with one built-in display. A separate, owned test application entered a true macOS full-screen Space. This was a full-screen application test, not a maximized-window approximation.

### Menu-bar controls

The retained native popover was observed beneath its actual status-bar item while the other application was full screen. Escape, outside click, a second status-item click, and the action to open the main window all worked. The normal application's timer and history were checked unchanged.

The implementation is in [macos_popover.rs](../src-tauri/src/macos_popover.rs). Local receipts are `v2-menu-fullscreen.json`, `v2-menu-dismissal.json` and `v2-menu-open-main.json` under the ignored `.data/final-macos-install-20261124/` directory.

### Mini timers above another application's full-screen Space

The mini uses a retained, nonactivating native NSPanel, with the Tauri webview rehosted as its content. Pinning sets its floating level and Space/full-screen collection behavior. Showing the panel does not activate the Focus main window. Unpinning returns its native level to 0.

| Interaction | Actual observation |
| --- | --- |
| Three styles over full screen | Pinned Ring, Tomato and Capsule were visible while the independent full-screen application remained active and Focus's main window stayed offscreen. |
| First click | A real first mouse click paused Ring and Capsule while Focus was inactive. |
| Dragging | A native drag moved the panel by the observed 180 × 120 displacement. |
| Style changes | Switching styles preserved the top-left position and the same timer. |
| Hide and reopen | The round continued, retained its identity/history, and showed its advanced countdown when reopened. |
| Unpin | The window returned to level 0 without altering the round or switching away from the full-screen Space. |

The implementation is in [macos_widget.rs](../src-tauri/src/macos_widget.rs). Receipts include `v3-ring-fullscreen.json`, `v3-tomato-fullscreen.json`, `v3-capsule-fullscreen.json`, `v3-widget-first-click.json`, `v3-widget-drag.json`, `v3-widget-hide.json`, `v3-widget-reopen.json` and `v3-widget-unpin.json`.

These detailed interaction observations were made on the isolated v3 QA build. The later v4 change corrected notification guidance without changing the panel implementation. The final, normally installed S2 application was separately checked above a true full-screen Space, as described below.

### Notifications, completion and the installed application

The QA application did not request notification permission on first launch. An explicit action opened the real macOS authorization dialog. Allow succeeded, and a status refresh accurately reflected the grant. A deliberately slow response exceeded the native 120-second wait; the interface exposed a retry and subsequently recovered the actual OS state.

Both sound-enabled and silent test requests produced real macOS notification UI. The respective request flags were verified, and the entire paused timer/history/preferences state was unchanged by either test. Saving notification preferences changed preferences only.

A real one-minute v4 QA round then ran while Focus was inactive, its main window was offscreen and its mini remained above another full-screen application. It added exactly one completed record with 60,000 ms elapsed time. The timer remained at zero and history stayed unchanged for another eight seconds. The actual OS completion notification was observed. The mini's “start short break” action started the next phase through native IPC without changing that history.

Relevant QA receipts include `v3-native-notification-prompt.json`, `v3-notification-granted-refreshed.json`, `v3-notification-test-sound.json`, `v3-notification-test-silent.json`, `v4-natural-completion.json` and `v4-next-phase.json`. The v4 QA binary starts with SHA-256 `8e847f86`; its 74 desktop build inputs were matched to the final UI source before commit. The final source's subsequent change affects Windows notification lifetime handling only. These QA builds are not substituted as public installers.

### Final universal app installed and checked

The clean-source universal application from the final release build was installed as the normal `com.makersns.focus` app. Its executable contains both `arm64` and `x86_64` and passed deep, strict signature verification. Its executable SHA-256 is:

`2f5cda04c3e8034e22b1194700aa71736d5b4e93fd9e4ad80f930f52df22d31f`

Installation began after the existing round was already completed. All timer business fields, existing history, preferences, daily totals and cycle count were preserved. The old app's graceful exit persisted its existing restore revision and timestamp metadata; those expected lifecycle changes are recorded explicitly. The installer did not rewrite the state file to make before/after bytes match. From that post-exit checkpoint through final launch and full-screen acceptance, the complete disk state remained equal.

Using the existing pinned Tomato preference, the final app's 260 × 280 native mini was visible above a separate application's true full-screen Space. That application stayed active, Focus stayed inactive and the main Focus window stayed offscreen. An actual 3024 × 1964 native screenshot was visually reviewed: the mini and its completed-state controls were legible. No user preference changed during the test. The owned test app was closed, the original desktop and main view were restored, and the final normal Focus app remained running.

The ignored `normal-S2-install/` receipts have these SHA-256 values:

| Receipt | SHA-256 |
| --- | --- |
| `installation.json` | `d9800a91ebd3921371b74436f83e66d6a917a839e2e69185ab062bda60cc0f8c` |
| `fullscreen.json` | `5c4fd76becf443d054ea4bcbb5e38240cbbb431419a864c5c8faf6a34d792332` |
| `fullscreen.png` | `ffa17df6158720b542d78a6ca7ae603fbaa4c096c17ae19297687dea554ecdbe` |

The earlier normal-v4 installation receipts remain historical. Both pre-upgrade app backups and the private evidence are retained.

## Windows native acceptance

The final main CI launched the real Tauri/WebView2 application through its matching WebDriver. All seven scenarios passed: startup; main/mini shared-round controls; pinning; unpinning; hiding and reopening the mini; continued tray operation after closing the main window; and state persistence.

A separate native foreground guard stayed active while the probe observed 55 updates for each of the pinned and unpinned cases. No timer-driven foreground theft or unintended z-order change was observed. The probe identifies the mini by application process and exact native title rather than confusing it with internal Tao or single-instance helper windows.

Five actual native screenshots were visually reviewed. The light running and paused main windows and the dark main window rendered without clipping; both light and dark Ring minis had complete controls and transparent corner pixels. This does not claim that the duration picker was tested open or that separate Tomato/Capsule Windows mini windows were visually inspected.

The private final-source evidence is under `evidence/main-ci-45241402661/windows/` in the final artifact directory. `native-smoke.json` has SHA-256 `6cf38e41fc68d49f503428d38596b7b4932814363f79b31af3ed281fe78bb89f`; `visual-review.json` has SHA-256 `35a24615032892a91e93c5a4196b0dc1c1ce3aec6a3726b3a500630acf78ed46`. This run validates real WinRT factories and desktop window behavior; it does not establish that a Windows toast visibly appeared on a user's interactive desktop.

## Android signed-package acceptance

The final APK was built and signed from clean source `6ace789bfe63db7dde3449236d7cdde9ca591d0c`. It is non-debuggable, 7,460,171 bytes, with SHA-256:

`948ba19a67f7c5f36bddbaff21553091e650d610b444fdd78b1510616ca0e784`

Its release certificate is:

`e79f8d4875a66c7d5a59ac4540735d5c3349044a982f065149f465421ac6837e`

Signature verification and 16 KiB ZIP alignment passed. The actual final APK was installed as an update. Cold launch, R8/JNI/native integration, ordinary warm return, retained preferences/history and the real launcher AppWidget passed.

The same complete signed APK had just passed 13 recorded acceptance checks from source `15b0c385…`: actual notification authorization, a real test notification without a state change, launcher pinning, shared pause/resume, task removal and a new process retaining the same round, natural one-minute completion, stable zero after expiry, and the next short break. Completion added one history record and exactly 60,000 ms / one completed session to daily totals; a single silent OS completion notification was observed.

This evidence reuse is explicit: the only source changes from that build to the final source are the Windows notification module and a Windows-target Cargo feature. The two APK files are byte-identical, and all 735 archive payloads match. The final-source APK was nevertheless installed and checked again.

The earlier dedicated native widget/reminder checks also covered all three widget styles, light/dark/system transitions, pause/resume without changing the round, and 22 notification-permission/channel/recovery cases. A recycled RemoteViews text-color problem was fixed and checked in both theme directions. These earlier observations keep their own build identity; they are not presented as additional final-source full-suite runs.

The environment was a dedicated ARM64 API 36 emulator with 16 KiB pages and WebView 133.0.6943.137. A warm-return paint artifact observed with SwiftShader was investigated by comparing the same APK/state under the host Apple M2 Pro GLES renderer; cold and repeated warm native captures were clean on the host renderer. No workaround was added to the application's CSS or lifecycle code for that emulator artifact.

The final receipt is the ignored `native-evidence/android/final-6ace-release-qa.json`, SHA-256 `6de2d20396f598ce7bc93862fb92a4912cd2af8d14abeeea1283a3f08bfc22a8`. The referenced behavioral receipt is `final-15b-release-qa.json`, SHA-256 `ceae7c95d24e3fd13bdc8f3b4147bff0d1a9e9bf2e9200ec6eb82f2b5a322fe8`. The dedicated emulator was shut down normally after acceptance, with its evidence, state and signing keys preserved.

## Final distribution verification

[Focus v0.1.0](https://github.com/makersns/focus-timer/releases/tag/v0.1.0) was published as a stable, non-draft release at **2026-11-23 19:31:05 UTC**. The annotated tag object is `f59625c41f2029619fe957c35096292235a3c5af`, peeling to the unchanged release source `6ace789bfe63db7dde3449236d7cdde9ca591d0c`.

| Distribution | Build origin | Final checks |
| --- | --- | --- |
| macOS universal DMG | Local clean tagged source | 54 native tests; frontend typecheck/build; both architectures; deep/strict ad-hoc signature; DMG CRC; read-only mount and exact copied executable; normal final-app installation and full-screen acceptance. |
| Windows x64 installer | Verified-source GitHub packaging run | Standard build scripts with native tests enabled; source/tag/provenance checks; package manifest and checksum. The separate five-platform main CI also contains the actual Windows GUI checks described above. |
| Linux x64 AppImage, DEB and RPM | Verified-source GitHub packaging run | Standard native tests/build and collector; source/tag/provenance checks; package manifests and checksums. |
| Android ARM64 APK | Local clean tagged source, project release key | Non-debuggable APK; release signature/certificate and 16 KiB alignment; final APK update installation and native runtime acceptance. |

The unchanged `release-verify.mjs --include-android` passed for the complete six-installer set. Each platform manifest records the same release source. All ten installer/manifest entries in the combined checksum file independently passed SHA-256 comparison. The checksum file itself is 995 bytes with SHA-256 `f9504f46b5fa081bb11a6a0a935b324123ea3f660a41c235653bab39c83acdfa`.

Exactly eleven assets were uploaded: six installers, four platform manifests and `SHA256SUMS.txt`. Their remote sizes and GitHub asset digests were matched against the reviewed local files before publication. After publication, the unauthenticated public release API and the publicly downloadable checksum file were independently verified. Full filenames, bytes, hashes, URLs and build origins are recorded in [the public provenance record](releases/v0.1.0-provenance.json).

The final collection receipt has SHA-256 `582f3c801ef4f13c55b61c87a67dfbb79b68e46272d72ee65980b5d407706049`. The local macOS distribution receipt has SHA-256 `ed5ecb2eba02feec834b1e09cf5c02d0a976980135cd934eddf7ef87f46798ce`. The public publication receipt has SHA-256 `2aa66d34e2fe9e82bdc9450697a9f3c2dbe848bae1ba24f6e6f5ae08f422ea94`; these hashes identify retained private audit records without exposing local paths or user data.

macOS is ad-hoc signed and not Apple-notarized; Windows has no publisher signing certificate. Android uses the established release certificate listed above. The [bilingual release notes](releases/v0.1.0.md) include installation links and platform limitations.

## Limits and reproducibility

- macOS GUI acceptance used one built-in display on macOS 15.5. Mixed-DPI geometry has unit coverage, but no physical multi-display GUI session was observed.
- Windows coverage includes the actual desktop application and native WinRT object lifetimes; visible Windows toast delivery was not separately observed.
- Linux has required native tests, Clippy and package builds. Desktop-environment/compositor-specific GUI and tray behavior was not exercised locally.
- Android runtime acceptance used the documented emulator, not a physical handset or every supported Android version.
- Notification permissions, visible macOS/Android delivery and requested sound/channel settings were checked. No physical listening channel was available, so audible playback is not claimed.
- Public screenshots are actual native captures with their real build/capture provenance recorded in [the screenshot inventory](design/screenshots-v2.md). They are not substitutes for platform runtime tests.

Earlier `f20…` installers and the earlier rectangular mini design are superseded. Their retained private evidence is historical, and those binaries are excluded from the final release set.

See [contributing](../CONTRIBUTING.md), [Android behavior and signing](ANDROID.md), and the [release workflow](https://github.com/makersns/focus-timer/actions/workflows/release.yml) for the build and verification commands. Local evidence filenames above identify private development receipts; public release assets and CI runs are linked separately.
