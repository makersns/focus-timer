# Focus UI v2 — approved widget family and main window

## Design source and scope

The user approved the three-style widget board (ring, tomato, capsule), then asked for the main window to use the same visual language. The implementation uses original SVG geometry and CSS, not crops of either design image. All times, progress, status and cycle information come from the shared native timer snapshot. Design-board sample times and example intentions must never become seeded application data.

The existing Focus application icon remains unchanged. The tomato leaves belong to the tomato widget only.

## Asset inventory

| Asset | Purpose | View box |
| --- | --- | --- |
| `public/widgets/ring-light.svg`, `ring-dark.svg` | Flat light / dark disc | 240 × 240 |
| `public/widgets/tomato-light.svg`, `tomato-dark.svg` | Original tomato body and three leaves | 240 × 260 |
| `public/widgets/capsule-light.svg`, `capsule-dark.svg` | Compact rounded body | 320 × 92 |
| `src/components/WidgetArtwork.tsx` | Shared live progress geometry and decorative background | Style-specific |
| `src/components/WidgetStylePicker.tsx` | Accessible three-choice gallery used in the main window and settings | Responsive |
| `src/widgets.css` | Widget family styles, gallery and isolated mini-window layout | Logical CSS pixels |

SVG backgrounds contain no timer digits or control icons. App content remains selectable and accessible native WebView text/buttons. Android may translate these exact shapes into native vector/bitmap backgrounds; its countdown stays a native Chronometer and its buttons stay RemoteViews actions.

## Widget dimensions and safe areas

| Style | Native desktop window | Visible artwork area | Main digits | Primary button |
| --- | --- | --- | --- | --- |
| Ring | 260 × 260 | 240 × 240 at (10, 10) | 44 px; 38 px for 100–180 minutes | 40 px circle |
| Tomato | 260 × 280 | 240 × 260 at (10, 10) | 44 px; 38 px for 100–180 minutes | 40 px circle |
| Capsule | 340 × 112 | 320 × 92 at (10, 10) | 38 px; 33 px for 100–180 minutes | 38 px circle |

The ten-pixel transparent gutter keeps the edge clear. The ring has no shadow, bevel or glow; tomato and capsule may retain a restrained shadow. The native window uses no title bar or opaque background. Native resize must follow the saved style while preserving visibility, pin state, the shared timer, and user control of focus.

- Ring: flat solid round body without a bevel or outer halo; thin peach elapsed-time ring; centered clock, status and a round action.
- Tomato: softly rounded coral body; original three-leaf crown; cream elapsed-time ring; centered clock/status/action.
- Capsule: small ring at left, clock/status in the center, round action at right. Pin/close controls appear on hover and keyboard focus without shifting the clock.
- Light and dark versions share geometry. Dark tomato uses a warm dark brown body and the same muted leaves.
- Keep long clocks on one line. No ellipsis on time. Intent is available in an accessible description or title, not a crowded extra line.
- Pin and close are small visual controls with at least 28 × 28 px hit areas. They must be keyboard-accessible even when visually discreet.
- The clock opens the main window. Dedicated drag area excludes buttons and follows the existing native startDragging path.
- Space starts/pauses/continues when no control is focused; Escape hides the mini window.
- Completed focus starts the appropriate break; completed break starts focus. No new timer engine exists in the widget.

## Progress behavior — latest explicit user correction

The user superseded the sample arc amounts shown in the design boards: the digits count down, while the colored arc grows with elapsed time: `(durationMs - remainingMs) / durationMs`. Idle and reset show zero colored arc; a paused timer freezes its arc; completion shows a fully closed colored ring. Every background track is a continuous closed shape. There is no fixed decorative gap or visible foreground dot at zero. Main window, native status indicator, desktop widgets, thumbnails and Android use this same meaning.

The tomato track is now a closed organic outline in viewBox 240 × 260: `M120 67 C176 62 210 96 213 140 C219 195 181 233 120 233 C59 233 22 196 27 140 C29 97 64 64 120 67 Z`.

## Flat ring refinement — latest explicit user correction

The user requested the original flat treatment for ring mode. Remove the main dial’s gradient, border halo and shadow. Ring widget and ring thumbnails use a solid flat disc and no filter/drop-shadow; their neutral track remains fully closed. The tomato and capsule retain their approved shapes and gentle depth. Tomato uses the contrasting cream/light-peach arc on its coral body, with the same elapsed-time meaning; orange-on-orange would hide useful progress.

## Palette

| Token | Light | Dark |
| --- | --- | --- |
| Page | #FCF8F5 | #302D27 |
| Porcelain | #FFFDF9 → #F7F0E8 | #46443A → #302F28 |
| Ink | #34332D | #FFF9EE |
| Peach ring / primary | #E7A06E → #D98754 | #F0AF7F → #E69A64 |
| Tomato | #EF9575 → #DB7659 | #5A493E → #3D3630 |
| Tomato ink / arc | #FFF7E9 / #F6CEB0 | #FFF7EA / #E9AC82 |
| Leaves | #A7B775, #93A965, #B5C18A | Same hues, restrained saturation |

Any retained tomato/capsule gradients are restrained and directional. Rings remain flat. No noise texture, external bitmap, animated background, large glow, or separate window title strip is needed. Reduced-motion preferences disable transition effects.

## Shared preference

`miniTimerStyle: "ring" | "tomato" | "capsule"`

Missing v1 values migrate to `ring`. A present invalid value must be rejected instead of silently replacing a saved preference. This additive migration changes appearance only; notification preferences, timer identity, remaining time, history, language and theme remain intact.

The main-window gallery saves immediately through the existing serialized native preference command using a fresh snapshot. Settings changes remain a draft until Save. Both update the same preference, so desktop and Android surfaces stay coherent.

## Main window

Use the existing 820 × 840 logical default size and integrated title bar. At that size:

1. A calm 64–70 px header with centered Focus title and only statistics/settings actions at right. macOS traffic lights retain their native safe area.
2. A restrained, softly recessed three-mode segmented control.
3. A compact intention label and bordered text field. Keep the user's actual text; show a placeholder when empty.
4. A dominant 340–360 px flat dial with a fine peach ring, large tabular clock, one status line, and the round primary action inside the dial.
5. Quiet end/reset actions below the dial. Four cycle dots and the actual completed-session count. A small duration button opens the existing duration editor with presets and custom entry.
6. A compact widget shelf: style gallery plus show/hide control, separated by a hairline. Android keeps its real Add widget action.
7. A small local-save indicator and by Sugar footer.

Do not remove the existing confirmation flows for ending/changing an active timer, history access, settings, persistence errors, keyboard shortcuts or accessibility announcements.

At narrow widths, the widget shelf wraps into a label/action row and a three-column gallery. The intention field and localized copy may wrap; the clock may not. A short viewport may scroll vertically rather than clipping controls. Test 320 × 568, ordinary Android portrait, 640 × 640 and 820 × 840; test all five locales and 180-minute clocks in light/dark themes.

## Acceptance evidence

Implementation acceptance requires real native or browser screenshots compared against the approved boards, not merely a successful compile. Verify all three shapes, light/dark palettes, pin/close/drag/open controls, shared running/paused/completed state, style persistence/relaunch and actual native resizing. Verify Android's smallest supported widget bounds and native countdown independently. Test screenshots must preserve actual times and user data.
