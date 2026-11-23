# Approved Focus interface

Approved on 2026-11-23. This specification and [approved-focus-ui.png](approved-focus-ui.png) define the implementation target. The board contains product concepts, not application screenshots. Release screenshots must be captured from the working application.

## Assets and identity

- Product: **Focus · 专注计时**; author: **Sugar**.
- Author destination: https://sugar.makersns.com
- Source destination: https://github.com/makersns/focus-timer
- Editable application icon: `public/app-icon.svg`, 1024 × 1024 view box. Original broken peach circle, vertical focus stroke, sage accent, warm tile.
- Compact brand mark: `public/brand-mark.svg`.
- Native icon files are generated from the SVG with `npm run icons`; the original vector remains the source of truth.
- Interface line icons use Lucide. Do not add stock photos, decorative bitmap backgrounds, network fonts, or unrelated illustrations.
- Keep the approved board in documentation, not in the runtime bundle. Build the interface from semantic controls, CSS and SVG geometry.

## Visual tokens

| Role | Light | Dark |
| --- | --- | --- |
| Main surface | #FCF8F5 | #36362D |
| Primary text | #373830 | #F8F8F3 |
| Secondary text | #868381 | #C4C6BB |
| Peach accent | #D8986F | #E3A27B |
| Progress track | #F3ECE7 | #67695B |
| Fine divider | #E8DFD8 | #525448 |

Use the platform system sans-serif family, including the native CJK fallback. Time uses tabular numerals. Shadows are soft, neutral and restrained; avoid glow, glass overlays, gradients across the page, or visible card-within-card framing. Preserve the ring's rounded caps.

## Main window

Default desktop size: **820 × 840 CSS pixels**; minimum **640 × 640**. Android and browser preview also fit a 320-pixel content width. The approved board's main panel is about 810 × 828. Keep its proportions at the reference size and compress vertical gaps/ring size at smaller heights.

1. A continuous warm surface extends to the top edge. There is no detached gray title strip.
2. A roughly 70-pixel header has centered **Focus**, statistics, mini-timer and settings line icons on the right. macOS retains real system traffic lights on the left. Reserve at least 84 pixels for their hit areas.
3. Windows/Linux window controls share the same surface. Android uses status/navigation insets and no desktop window controls.
4. The mode selector is centered, about 368 × 44 pixels, with Focus / Short break / Long break. Active mode has the peach fill.
5. The optional intention is centered underneath, editable without a permanent bordered input card. Do not prefill a fictional user task.
6. The timer ring is about 352 pixels in diameter with a 16-pixel stroke. It shows elapsed progress, the large remaining clock, a concise state label and one quiet guidance sentence.
7. Reference clock typography is about 82 pixels, weight 650. State text is about 23 pixels and guidance about 18 pixels. Long durations scale down gracefully.
8. One primary pill button, about 230 × 52 pixels, starts/pauses/resumes the real timer. An unobtrusive End round action follows. Destructive mode/reset changes retain confirmation.
9. Duration presets form one horizontal row: 15 / 25 / 50 / Custom. Selected duration reflects the current round, not a changed default.
10. Footer: local saving status on the left; **by Sugar** author link on the right.

There is no persistent sidebar, quote block, dashboard card or permanent statistics strip. Statistics/history/settings remain real functions accessed from the header. Idle, running, paused and completed states retain the same visual hierarchy. No automatic next-round start.

## Desktop mini timer

Default **360 × 176**, minimum **320 × 160**, following the approved board's approximately 2:1 proportions.

- No native title strip or traffic-light row.
- Header: Focus · current mode; pin, open main window, hide controls.
- Large remaining time at left; peach start/pause/resume button at right.
- Intention below the clock; thin elapsed progress track at the bottom.
- Drag only from title/empty header space; buttons never initiate dragging.
- Pin/unpin updates only on explicit action or changed preference.
- Hiding the component changes its visibility preference; it never ends or pauses a round.
- Windows/macOS/Linux share the same authoritative Rust state with the main window.
- Light and warm charcoal variants share geometry. Save visibility and pin preferences.

## Tray / menu-bar panel

Popover baseline **280 × 276** (may grow to 292 pixels if required by actual localized text).

- macOS menu bar shows the remaining clock directly, updating without opening a window.
- The panel has a small state label and a prominent clock, followed by pause/resume/start, show/hide mini timer with checked state, open main window, a divider, and quit.
- Opening or hiding the panel is an explicit interaction. A timer tick must never show, focus, move or reorder any window.
- Escape/outside click dismisses the panel. Native fallback menus remain usable if a tray platform cannot support the custom panel.
- Completion informs the user through enabled notifications and existing surfaces; never steal focus.

## Android home-screen widget

This is a real Android **AppWidget**, not a React window or a fake preview. Use the approved mini-timer hierarchy and colors with Android-safe dimensions.

- Remaining clock, current mode, intention, pause/resume/start control, thin progress bar.
- Tapping the content opens the application.
- The launcher owns moving/resizing/removing the widget; omit desktop pin/close controls.
- Request pinning through the launcher when supported. A successful request is not proof of addition; wording asks the user to confirm on their home screen.
- Widget, foreground service, alarm and application share one Rust timer and one atomic persisted state.
- RemoteViews Chronometer handles visible second changes; avoid per-second widget broadcasts.
- A one-shot exact alarm settles the current round and stops the countdown at zero even when the process is not active.
- User-started active timing uses the appropriate Android foreground service. Optional completion alerts remain separate from the system's ongoing running-timer notification.
- Same-boot recovery can continue an authorized active timer using persisted elapsed-time calibration. Device reboot or missing/invalid boot metadata restores a paused round.
- API 31/32 exposes a clear precise-timing permission action when necessary. Start/resume never silently promises accurate background timing without that permission.

## Runtime capabilities

`get_runtime_info` returns:

```ts
interface RuntimeInfo {
  platform: "macos" | "windows" | "linux" | "android" | "web";
  desktopWidget: boolean;
  homeScreenWidget: boolean;
  tray: boolean;
}
```

Native does not imply desktop. UI actions are gated by these capabilities.

Additional commands: `set_mini_timer_visible`, `show_main_window`, `hide_tray_popover`, `quit_app`, `request_home_screen_widget`, `get_android_timer_status`, `request_exact_alarm_permission`. Main/mini/tray are views of the same engine. Auxiliary routes use `?view=mini` / `?view=tray`; the frontend also accepts the equivalent `window` parameter.

System request results contain `supported` and `requested`, neither of which means a user has completed an OS confirmation. Android timer status contains `exactAlarmAllowed`, `canRequestExactAlarm`, `notificationsAllowed` and `widgetCount`.

## Review and evidence

Compare working screenshots to the approved board at the default dimensions, then inspect a shorter desktop window, 320-pixel mobile width, long translations, dark appearance and large timers. Test keyboard/focus behavior, motion preferences, real state transitions and native component synchronization. Test Android launcher addition, background completion, process recovery, and reboot separately. Store release screenshots under `docs/images/`; label concept imagery as design references.
