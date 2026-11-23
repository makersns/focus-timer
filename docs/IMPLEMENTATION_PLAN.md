# Focus by Sugar — implementation and release plan

Updated: 2026-11-23. Status: implementation and verification in progress.

## Product and approved design

Focus is an independent Tauri 2 focus timer for macOS, Windows, Linux and Android. Source is intended for **makersns/focus-timer** under MIT. Version: **0.1.0**; application identifier: **com.makersns.focus**.

The approved interface and asset inventory are kept in [design/SPECIFICATION.md](design/SPECIFICATION.md), [design/approved-focus-ui.png](design/approved-focus-ui.png), and [design/assets.json](design/assets.json). They supersede the earlier dual-column prototype. Main desktop window: 820 × 840, minimum 640 × 640; mini window: 360 × 176. Use a continuous warm surface, a centered single-column timer, the native macOS traffic lights, a menu-bar countdown and a matching floating component.

Support system/light/dark appearance and Simplified Chinese, Traditional Chinese, English, Japanese and Korean. Android adapts this hierarchy to mobile safe areas and provides an actual launcher AppWidget.

## Scope and behavior

- Focus, short break and long break; defaults 25/5/15 minutes. Per-round and default durations are distinct, integer minutes 1–180.
- Start, pause, resume, end, reset, and an explicit next-phase action. Never automatically start a second round.
- Ask before ending an active/paused round due to a mode or duration change. The core independently rejects an active switch.
- Optional intention, maximum 120 characters; no account, cloud sync, network analytics or fabricated statistics.
- Record completed and nonzero early-ended focus truthfully; completed-session count excludes early endings. Completed breaks do not add focus time.
- Store 500 history records and 366 days of local totals. Clearing history preserves the running timer.
- All native views and widgets share one authoritative Rust timer. UI rendering is not the clock source.
- Desktop timer continues while the main window is hidden. Explicit Quit pauses and saves; unexpected desktop restart restores a paused checkpoint.
- A missing desktop tray must not strand a hidden application. Completion never forces a window to the foreground.
- Mini timer visibility and pin preferences persist. Closing it only hides it.
- Android same-boot process recovery may continue a user-started timer using atomic boot/elapsed-time metadata. Device reboot or unverifiable metadata restores paused.
- Android Chronometer draws the countdown; an exact alarm finalizes the current round; a running foreground service follows Android's timer lifecycle. No per-second widget broadcasts.
- Completion notifications are optional and default off. The Android ongoing service notification is a separate OS requirement. Request user-granted permissions only at a relevant explicit action.
- Language, theme and unrelated preference changes never reset time.
- Accessible labels, state announcements (not second-by-second speech), keyboard handling and reduced-motion preferences remain part of the UI.

## Shared protocol

See `src/types.ts` for the exact camelCase JSON contract. The core emits revisioned snapshots on `focus://snapshot`. Every WebView rejects snapshots older than its current revision.

Commands:

| Command | Result / purpose |
| --- | --- |
| get_snapshot | Current reconciled Snapshot |
| timer_action | Apply TimerAction and return Snapshot |
| update_preferences | Save validated preferences and return Snapshot |
| clear_history | Clear records without resetting the timer |
| open_external | Fixed author / GitHub / license destinations |
| get_runtime_info | Native platform and actual surface capabilities |
| set_mini_timer_visible | Explicit desktop component visibility |
| show_main_window | Open main window following a user action |
| hide_tray_popover | Dismiss the menu-bar panel |
| quit_app | Save a paused desktop round before exiting |
| request_home_screen_widget | Request Android launcher confirmation |
| get_android_timer_status | Exact alarm / notification / widget status |
| request_exact_alarm_permission | Open the relevant Android system setting |

Native does not mean desktop: the UI uses RuntimeInfo capabilities. A system request result distinguishes supported/requested from completion by the user. API 31/32 start/resume may return `exact_alarm_permission_required` before changing timer state.

Timer modes: focus / shortBreak / longBreak. States: idle / running / paused / completed. Actions: start, pause, resume, reset, end, switchMode, setDuration, setIntent.

The Rust Engine owns transitions, reconciliation, completion identity, history and daily aggregation. The runtime drops engine locks before OS UI/notification calls, serializes writes and uses atomic replacement. Persistence failures are visible but never silently reset a timer. Preserve corrupt files and reject unsafe paths or future schemas.

Use deadlines and monotonic safeguards against clock rewind. Reconcile natural completion at its actual deadline, not a delayed wake-up time. Settle each run once. Pause/resume excludes paused time. Completion and explicit transitions persist immediately; desktop active checkpoints occur at most once every 30 seconds.

Android extends persistence with a single atomic envelope for snapshot, boot identity and elapsed-time calibration. It does not introduce a second Kotlin timer or independent SharedPreferences copy of the deadline. Older version-1 preferences receive safe defaults for miniTimerEnabled=false and miniTimerAlwaysOnTop=true.

## Work sequence and release gates

1. **Materials:** preserve the approved board, original SVG icon, visual specification and asset hashes.
2. **Implementation:** update the React surfaces, desktop shell, Android native widget and shared runtime. Maintain platform-specific permissions and correct lifecycle behavior.
3. **Core verification:** pause/resume, delayed tick, clock changes, exact expiry, duplicate completion, invalid/corrupt storage, interrupted writes, compatibility, Android same-boot recovery/reboot, preferences and true statistics.
4. **UI verification:** compare real screenshots to the approved board; inspect default/compact/mobile sizes, all five languages, dark mode, states, dialogs, accessibility and keyboard interaction.
5. **Desktop integration:** run the packaged macOS application; verify menu-bar time, main/mini/popover synchronization, hiding/reopening, saved pin/visibility and no focus changes on ticks. Windows CI must run the actual native application and component smoke tests as well as building it.
6. **Android integration:** use an isolated emulator; add the actual AppWidget, start/pause/resume from both surfaces, background the app, verify deadline completion and process recovery, and inspect launcher screenshots.
7. **Packaging:** verify macOS universal binary and DMG, Windows installer, Linux packages and Android APK with checksums. Distinguish locally executed tests, CI tests and builds from tests on physical hardware.
8. **Publication:** review tracked files for private data, generated caches and placeholder content; commit/push source; publish only verified artifacts and accurate multilingual release notes.
9. **Portfolio:** update the existing Focus entry with real screenshots, exact platform capabilities and working release links, preserving other projects.
10. **Cleanup:** remove only task-owned temporary branches/files and retain needed signing material, release provenance and diagnostic evidence.

## Documentation and reproducibility

The README and release notes must describe actual implemented behavior and build requirements. Product screenshots go under `docs/images/`; the design board is clearly labeled as a concept reference. Signing keys, local SDK paths, emulator images, caches and private app state must never be tracked.

Dependencies remain locked. Use official registries and supported toolchains. Keep heavy builds in a task-specific workspace when local disk is limited. Do not modify unrelated projects or their build environments.

## Official references

- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri system tray](https://v2.tauri.app/learn/system-tray/)
- [Tauri mobile plugins](https://v2.tauri.app/develop/plugins/develop-mobile/)
- [Tauri WebDriver testing](https://v2.tauri.app/develop/tests/webdriver/)
- [Tauri GitHub distribution](https://v2.tauri.app/distribute/pipelines/github/)
- [Android AppWidget](https://developer.android.com/develop/ui/views/appwidgets)
- [Android alarms](https://developer.android.com/develop/background-work/services/alarms)
- [Android foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types)
