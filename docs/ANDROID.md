# Android app and home-screen widget

Focus for Android uses the same Rust timer engine and validated local store as the desktop app. The Android Activity, home-screen widget, foreground service, and alarm receiver all enter one native runtime through JNI. The WebView does not own a second timer.

## Supported build

- Android 7.0 (API 24) or later; release APKs target `arm64-v8a`.
- Compile and target SDK: 36. Build Tools: 36.1.0.
- Android NDK: 29.0.14206865, including native 16 KB page support.
- JDK 21. Gradle wrapper: 9.6.1. Android Gradle Plugin: 9.3.1. Kotlin: 2.2.10.
- Node.js and Rust versions match the root project and CI configuration.

The source under `src-tauri/gen/android` is maintained application code. **Do not run `tauri android init` over it.** Clone the repository and use the scripts below.

## Add and use the widget

Open Focus, then choose **Add widget** in the widget shelf or the home-screen widget action in Settings. On launchers that support pinned widgets, Android asks where to place it. You can also long-press the home screen, open the launcher’s widget picker, and choose Focus. A successful pin request means the launcher showed the request; the user still controls whether a widget is added.

Choose **Ring**, **Tomato**, or **Capsule** in the app’s widget shelf or Settings. The same saved style applies to real Android home-screen widgets. Ring uses a flat light/dark disc with no bevel or halo, Tomato keeps its original three-leaf crown, and Capsule places a small ring beside the clock. The artwork is translated from the original vectors under `public/widgets`; timer digits and buttons remain native views.

Tap the clock to open the app, or use the pause/continue/next-round button directly. Multiple Focus widgets share the same session. The session intention remains available in the accessible description. At compact sizes the round skins omit the visible status line to preserve a readable clock and a minimum 48 dp action target. The clock fits durations through 180 minutes without ellipsis. Simplified Chinese, Traditional Chinese, English, Japanese, Korean, and light/dark/system appearance are supported.

A native Android `Chronometer` renders seconds in the launcher. Focus does not send a widget broadcast every second. The closed neutral track stays visible. Its colored arc represents **elapsed time**, increasing from zero at idle/reset to a full closed ring at completion; the digits count down. There is no colored dot at zero. The arc refreshes once per elapsed minute and on state changes, while the native clock still updates every second. A paused arc stays fixed. One scheduled expiry alarm stops the countdown at zero and settles the current round once.

## Background behavior and permissions

Starting or resuming a timer starts a foreground service with a quiet ongoing notification and a pause action. The service stops when the timer pauses, ends, or completes. The ongoing notification is part of Android’s background execution model; optional completion alerts are a separate preference and are off by default.

On Android 13 and later, Focus declares `USE_EXACT_ALARM` for its core timer function. On Android 12/12L, the app checks the special **Alarms & reminders** permission before starting. If permission is unavailable, the app shows a settings action instead of starting a timer that cannot reliably finish in the background. Revoking access pauses an active timer when the app next receives control.

The running deadline uses Android’s monotonic `elapsedRealtime` clock. Changing the device clock does not extend or shorten an active round. If Android reclaims the process, the saved snapshot, boot count, and elapsed-time checkpoint let a new process continue the same session during the same boot, including time spent asleep. A late expiry settles only that round and never starts more sessions automatically.

After a device reboot, an unknown boot identity, or an explicit system-level stop detected by Android’s exit history, Focus restores the last checkpoint paused. Open the app to continue. Force-stopping an app also removes its alarms until the app is opened again. Android versions and manufacturer battery controls can delay background delivery; Focus never requests an unrestricted battery exemption automatically.

The timer, preferences, and bounded session statistics are stored atomically in app-private storage. Android backup and device-transfer copying of this timer state are disabled so another device cannot accidentally continue a saved running deadline. There is no account, analytics, or network synchronization.

## Completion reminders

Completion notifications are off by default. The sound option is on by default but only applies when completion notifications are enabled. Android owns the sound; Focus does not play a second beep from the WebView or bypass silent mode, notification volume, or Do Not Disturb.

Open **Settings → Reminders** to see the current notification permission and the selected channel's availability. On Android 13 and later, the system permission prompt appears only after an explicit reminder action; reading the status or opening the app never requests permission. A declined or system-blocked notification provides an action to open Android's app or channel settings. Returning to Focus refreshes the displayed status.

**Test reminder** sends one real system notification using the sound choice currently shown in Settings. It does not save the draft, change the timer, or add history. A successful submission means Android accepted the request; the OS still controls whether a banner appears and whether a sound is audible.

The existing `focus_completion` channel is retained so upgrades preserve the user's channel choices. A separate `focus_completion_silent` channel handles quiet reminders, with each silent notification also explicitly silenced. Turning sound on again never overrides a channel the user muted in Android settings. The ongoing running-timer notification remains quiet and separate from these completion channels.

## Development and tests

Install the documented SDK/NDK and JDK with Android Studio or your normal SDK tooling. Review SDK licenses yourself before accepting any new terms. The build script checks installed components and does not run a license-acceptance command.

Set standard environment variables as needed:

```sh
export JAVA_HOME=/path/to/jdk-21
export ANDROID_HOME=/path/to/Android/sdk
export NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"
export ANDROID_NDK_HOME="$NDK_HOME"
export GRADLE_USER_HOME=/path/to/focus-gradle-cache
export CARGO_TARGET_DIR=/path/to/focus-cargo-target
npm ci
node scripts/android-build.mjs --debug --test
```

The macOS helper can also use the ignored `.data/native-build.json` dedicated build volume. Machine-specific paths and SDK `local.properties` are never committed. CI uses the same versions and standard environment variables.

The debug APK is reported by the script and normally appears under `src-tauri/gen/android/app/build/outputs/apk/universal/debug/`. It is for testing only. JVM test reports remain under `src-tauri/gen/android/app/build/reports/tests/`; a build summary is written to the ignored `native-evidence/android/` folder. Rust unit tests cover same-boot recovery, sleep expiry, reboot pause, duplicate completion, legacy migration, corrupt state, and stale saves.

## Verified Android behavior

The Android integration was exercised on a dedicated Pixel 7 emulator running API 36 with a 16 KB page-size system image. This is emulator evidence, not a claim of testing on every handset or Android version.

Visual acceptance used WebView 133.0.6943.137 and the emulator's host GLES backend with guest Vulkan disabled. A controlled comparison using the same APK and data reproduced stale native surface content after returning to the app on the tested SwiftShader path; DOM and WebView captures remained correct. Host-rendered native screenshots passed cold start and three returns in the same process. No product rendering setting or CSS workaround was changed. See [Android's emulator graphics troubleshooting](https://developer.android.com/studio/run/emulator-troubleshooting).

- Install and cold start open the actual Tauri Activity and Rust-backed application. The system status and gesture areas remain clear of the main controls.
- Android's pin confirmation adds a real launcher widget. The recommended size is 4 × 2 cells; launcher grids and user resizing can change its final dimensions.
- Ring, Tomato, and Capsule were checked at the launcher's 4 × 2 and reduced 3 × 2 sizes with a full `3:00:00` clock. Capsule was also checked with the system font size set to 130 percent. These are actual launcher dimensions; other launchers can use different cell sizes.
- Changing skin, language, or theme while paused preserves the session ID and exact remaining time. All five app languages were exercised in native widgets.
- Switching a forced theme back to the system theme was checked in both directions. Separate root layouts prevent Android from retaining stale forced text colors when it reuses a widget view.
- Starting in the app, pausing in the widget, and resuming in the widget operate on the same session ID. A paused countdown remains unchanged.
- Killing only the running app process lets Android restart its foreground service without an Activity. The restored native timer keeps the same session and deadline during the same boot.
- A real one-minute focus round completes once: one completed history record, one daily completed session, and exactly 60,000 milliseconds of focus time. The widget stays at a static `00:00` after expiry.
- The completed widget's next action starts the appropriate break, matching the main app's primary action.
- Notification permission was tested from an unrequested state through allowance, denial, and OS-settings recovery. The test action posts a real system notification without changing the saved timer, preferences, or history.
- Quiet reminders, an OS-muted channel, a blocked channel, and Do Not Disturb were checked against Android's actual notification records. One background completion after process recovery produced one history record and one completion notification. The emulator evidence verifies system policy and submission, not acoustic playback on a physical handset.

The source also includes Rust recovery and validation tests, plus JVM tests for countdown rounding, bounded progress, widget aspect ratios and 48 dp targets across supported sizes, notification permission states, system mute/Do Not Disturb decisions, and duplicate reminder protection. The arm64 release-variant Android lint check was also run with no errors; generated-template warnings and non-blocking resource/style recommendations remain. Release installers are separately signed and checked from the recorded source commit; debug APKs are never published as downloads.

## Release signing

Create a dedicated Focus release key once. Keep it outside the repository and outside temporary build volumes:

```sh
node scripts/android-create-signing-key.mjs --directory /private/persistent/focus-signing
```

This creates a PKCS#12 keystore and private JSON configuration, with private directory/file permissions on Unix. The command prints paths only. Back up both files securely before publishing; future APK updates need the same key. Never add them to Git, upload them as release assets, or reuse another application’s signing key.

After reviewing, testing, and committing the source, build from a clean working tree:

```sh
export FOCUS_ANDROID_SIGNING_CONFIG=/private/persistent/focus-signing/focus-signing.json
node scripts/android-build.mjs --release
```

The script builds release code, aligns native libraries for 16 KB pages, signs using environment-supplied credentials, and verifies the APK certificate and ZIP alignment. It refuses dirty source, debug-signing fallback, keys inside the repository, and replacement of an existing release artifact.

Release output is `Focus_VERSION_android-arm64.apk`, alongside a SHA-256 checksum and a manifest containing the exact source commit, target, version, APK size/hash, and public certificate SHA-256. The private key and passwords are never included. The root release verification script can include this Android manifest with the desktop installers.

## Architecture references

- [Tauri mobile plugin development and JNI](https://v2.tauri.app/develop/plugins/develop-mobile/)
- [Android app widgets](https://developer.android.com/develop/ui/views/appwidgets/overview)
- [RemoteViews Chronometer API](https://developer.android.com/reference/android/widget/RemoteViews)
- [Android alarms and exact-alarm access](https://developer.android.com/develop/background-work/services/alarms)
- [Foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types)
- [Foreground service background start restrictions](https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start)

- [Android notification runtime permission](https://developer.android.com/develop/ui/views/notifications/notification-permission)
- [Android notification channels and user settings](https://developer.android.com/develop/ui/views/notifications/channels)
- [NotificationCompat silent notifications](https://developer.android.com/reference/androidx/core/app/NotificationCompat.Builder#setSilent(boolean))
