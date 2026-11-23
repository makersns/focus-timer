# Contributing to Focus

Thanks for helping make a small, thoughtful desktop and Android app.

## Start here

Use Node.js 24 or newer, a current stable Rust toolchain, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform. Run npm ci, then npm run tauri -- dev. Use npm run dev for the browser preview when native integration is not needed.

Please keep a change focused and explain the user-visible problem it solves. Include a screenshot for visual changes, and explain which platforms you tested. A successful cross-compile is not the same as running on that hardware.

## Architecture

| Location | Responsibility |
| --- | --- |
| src/App.tsx, src/components/ | Timer view, preferences, confirmations, and records |
| src/i18n/ | Five-language copy, locale resolution, and formatting |
| src/types.ts | Exact camelCase IPC snapshot/action contract |
| src/lib/backend.ts | Native command/event bridge; isolated browser preview |
| src/lib/preview-engine.ts | Browser-only timer model for development |
| src-tauri/src/timer.rs | Authoritative timer, records, daily totals, and validation |
| src-tauri/src/storage.rs | Bounded state loading, corrupt-file recovery, atomic persistence |
| src-tauri/src/lib.rs | Platform entry points |
| src-tauri/src/desktop.rs | Desktop lifecycle, tray/popover, shared floating window and notifications |
| src-tauri/src/mobile.rs | Shared Android Rust runtime and JNI bridge |
| src-tauri/gen/android/ | Maintained native AppWidget, foreground service, alarms and Android resources |
| .github/workflows/ | Platform verification and release builds |

The packaged app uses the Rust engine. Keep preview semantics aligned, but never move authoritative timekeeping into a render loop. Snapshots carry a revision so an older IPC response cannot overwrite a newer event.

## Behavior that must remain true

- Paused time does not count; late ticks reconcile a deadline instead of counting intervals.
- A completion is recorded once, including after sleep.
- Desktop explicit quit saves a paused state; unexpected desktop restart restores a paused checkpoint.
- Android same-boot recovery reconciles the current authorized round; reboot or missing metadata pauses. Persist timing and boot calibration atomically.
- All surfaces share one engine. Timer ticks never show, focus, reposition or change the stacking order of windows.
- Android widgets use system Chronometer rendering and explicit transition updates, not per-second broadcasts.
- Changing theme or language cannot replace a custom duration or interrupt a timer.
- Completed and early-ended focus are distinct. Breaks do not add focus minutes.
- Closing hides only when a tray was successfully created.
- No account, network analytics, remote content, or automatic next round is required.
- Keep notification permission opt-in. Never add broader file, shell, or network capabilities without a concrete feature reason.

## Checks

~~~bash
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri -- build
~~~

Exercise start/pause/resume, completion, mode-change confirmation, preferences during a running timer, restore, and an empty history. Check both themes and a narrow window. Preserve reduced-motion behavior and readable focus indicators. Add regression coverage for actual state or persistence bugs; avoid tests that only repeat markup.

## Translation

Messages live in src/i18n/index.ts. Every locale must contain the same keys and interpolation tokens. Tests catch missing keys and unexpanded placeholders. Keep copy short enough for desktop controls, use natural phrasing, and retain the distinction between a completed session and an early ending.

Native desktop tray and notification text lives in src-tauri/src/desktop.rs. Android widget/service text is in the native Android resources. Update all surfaces when adding or changing a language; a language switch must preserve the timer.

## Assets and privacy

public/app-icon.svg is the editable app icon. Run npm run icons to generate Tauri assets. The generated native icons are committed so a clean checkout can build without an image toolchain.

Use real screenshots. Do not include personal intentions, history, identifiers, credentials, local app data, or build caches in a pull request. Test with disposable local data.

## Release

A release must pass frontend checks, Rust tests, native Windows UI smoke tests, real platform CI builds, and Android widget/background checks. See [Android development and signing](docs/ANDROID.md). Do not reinitialize the maintained Android project or commit SDK paths, Gradle caches, keystores or signing passwords. Keep installer checksums with the assets. Explain signing/notarization status accurately. Release notes should describe user-visible behavior and known platform limitations, without claiming untested hardware was verified.
