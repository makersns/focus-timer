# Android app and home-screen widget

Focus for Android uses the same Rust timer engine and validated local store as the desktop app. The Android Activity, home-screen widget, foreground service, and alarm receiver all enter one native runtime through JNI. The WebView does not own a second timer.

## Supported build

- Android 7.0 (API 24) or later; the published APK contains `arm64-v8a`.
- Compile and target SDK: 36. Build Tools: 36.1.0.
- Android NDK: 29.0.14206865, including native 16 KB page support.
- JDK 21. Gradle wrapper: 9.6.1. Android Gradle Plugin: 9.3.1. Kotlin: 2.2.10.
- Node.js and Rust versions match the root project and CI configuration.

The source under `src-tauri/gen/android` is maintained application code. **Do not run `tauri android init` over it.** Clone the repository and use the scripts below.

## Add and use the widget

Open Focus, then choose **Add home-screen widget** in Settings. On launchers that support pinned widgets, Android asks where to place it. You can also long-press the home screen, open the launcher’s widget picker, and choose Focus. A successful pin request means the launcher showed the request; the user still controls whether a widget is added.

The resizable widget shows a large countdown, the current focus or break mode, the session intention, a progress line, and a pause/resume button. Tap the time or title to open the app. Multiple Focus widgets share the same session. The widget follows the app’s language and light/dark/system appearance preferences; Simplified Chinese, Traditional Chinese, English, Japanese, and Korean are included.

A native Android `Chronometer` renders seconds in the launcher. Focus does not send a widget broadcast every second. The progress line refreshes at coarse checkpoints and on state changes. One scheduled expiry alarm stops the countdown at zero and settles the current round once.

## Background behavior and permissions

Starting or resuming a timer starts a foreground service with a quiet ongoing notification and a pause action. The service stops when the timer pauses, ends, or completes. The ongoing notification is part of Android’s background execution model; optional completion alerts are a separate preference and are off by default.

On Android 13 and later, Focus declares `USE_EXACT_ALARM` for its core timer function. On Android 12/12L, the app checks the special **Alarms & reminders** permission before starting. If permission is unavailable, the app shows a settings action instead of starting a timer that cannot reliably finish in the background. Revoking access pauses an active timer when the app next receives control.

The running deadline uses Android’s monotonic `elapsedRealtime` clock. Changing the device clock does not extend or shorten an active round. If Android reclaims the process, the saved snapshot, boot count, and elapsed-time checkpoint let a new process continue the same session during the same boot, including time spent asleep. A late expiry settles only that round and never starts more sessions automatically.

After a device reboot, an unknown boot identity, or an explicit system-level stop detected by Android’s exit history, Focus restores the last checkpoint paused. Open the app to continue. Force-stopping an app also removes its alarms until the app is opened again. Android versions and manufacturer battery controls can delay background delivery; Focus never requests an unrestricted battery exemption automatically.

The timer, preferences, and bounded session statistics are stored atomically in app-private storage. Android backup and device-transfer copying of this timer state are disabled so another device cannot accidentally continue a saved running deadline. There is no account, analytics, or network synchronization.

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
