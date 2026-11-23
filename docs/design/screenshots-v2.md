# Focus v2 screenshot plan

This is a capture and publication handoff, not evidence that an untested platform scenario passed. Screenshots must come from the current running application or the actual Android launcher. The approved design boards describe intended appearance; they are not product screenshots.

## Exact website gallery order

The first item is the sole website cover. The other ten become the shared project gallery in exactly this order. All nine macOS v4 originals were visually reviewed by root and copied into `docs/images` without changing their bytes. Four original Android screenshots are also present. The two additional Android style images remain outside this eleven-image website selection.

The dimensions below are capture targets or verified device dimensions, not invented final PNG metadata. Desktop point sizes are logical window/content sizes. On a 2× display an exact-bounds capture has twice those pixel dimensions, but system frame, shadow and contextual captures may be larger. `prepare.mjs` reads the actual PNG width and height; do not resize or rewrite PNGs merely to fit this table.

| Order | File under `docs/images/` | Manifest role | Logical capture target | Expected/actual pixel handling |
| --- | --- | --- | --- | --- |
| 01 | `focus-light.png` | `cover` | 820 × 840 pt content target | 1640 × 1680 at 2×, before any native frame/shadow |
| 02 | `focus-dark.png` | `desktopDark` | 820 × 840 pt content target | 1640 × 1680 at 2×, before any native frame/shadow |
| 03 | `focus-settings.png` | `settings` | 820 × 840 pt host; capture complete visible settings | Use the actual PNG dimensions; 1640 × 1680 if full host at 2× |
| 04 | `focus-language-menu.png` | `settings` | 820 × 840 pt host; complete custom menu in view | Use the actual PNG dimensions; 1640 × 1680 if full host at 2× |
| 05 | `focus-mini.png` | `desktopMini` | 260 × 260 pt | 520 × 520 at 2× for exact native bounds |
| 06 | `focus-mini-dark.png` | `desktopMini` | 260 × 260 pt | 520 × 520 at 2× for exact native bounds |
| 07 | `focus-mini-tomato.png` | `desktopMini` | 260 × 280 pt | 520 × 560 at 2× for exact native bounds |
| 08 | `focus-mini-capsule.png` | `desktopMini` | 340 × 112 pt | 680 × 224 at 2× for exact native bounds |
| 09 | `focus-menu-bar.png` | `menuBar` | 264 × 252 pt popover web content, plus native chrome, arrow and menu-bar context | 528 × 504 at 2× is inner content only; record the larger actual contextual PNG size |
| 10 | `focus-android.png` | `androidMain` | Device screenshot; do not convert pixels to an invented dp size | 1080 × 2400, Pixel 7 / API 36 arm64 emulator |
| 11 | `focus-android-widget.png` | `androidWidget` | Actual launcher placement; tested 3 × 2 cell bounds 699 × 588 px | 1080 × 2400, Pixel 7 / API 36 arm64 emulator |

This is one cover plus ten gallery images, the current preparation helper's maximum of eleven. The main light image maps to `cover`; it is not repeated in `gallery`.

### What each image must show

- **01 — `focus-light.png`:** Light main window; full integrated header, flat closed dial, in-ring primary control and all three style choices.
- **02 — `focus-dark.png`:** Same complete main window in dark appearance; actual countdown and real session state.
- **03 — `focus-settings.png`:** Native reminder settings: observed permission state, sound choice and test action. No browser-only unsupported state presented as native evidence.
- **04 — `focus-language-menu.png`:** Custom language menu with the five supported languages and its surrounding setting; no clipped options.
- **05 — `focus-mini.png`:** Light Ring mini timer, flat face and fully closed neutral track, with the current real countdown.
- **06 — `focus-mini-dark.png`:** Dark Ring mini timer; no outer shadow, bevel or artificial progress gap.
- **07 — `focus-mini-tomato.png`:** Tomato mini timer with original leaf crown and contrasting pale elapsed arc. New file.
- **08 — `focus-mini-capsule.png`:** Capsule mini timer at its real compact dimensions, preserving countdown and controls. New file.
- **09 — `focus-menu-bar.png`:** Actual menu-bar item and correctly anchored native popover. Keep enough menu-bar context to show the relationship. New file.
- **10 — `focus-android.png`:** The updated main UI running in the real Android application; preserve the full device screenshot.
- **11 — `focus-android-widget.png`:** Real Android launcher Ring AppWidget, native countdown and controls. Record that this is an emulator, not a physical-device photograph.

Android currently uses one global `miniTimerStyle` across the app and all of its widgets. Three different styles cannot be photographed together as three independently styled live widgets. The Android agent has delivered `focus-android-widget-tomato.png` and `focus-android-widget-capsule.png`, each a separate unmodified 1080 × 2400 launcher screenshot. They are additional repository/README evidence, outside the eleven-image website list above. Do not silently append them as a twelfth and thirteenth website image. If a later editorial decision uses all three Android images on the website, explicitly replace two other gallery slots while retaining the cover, a `desktopMini`, and an `androidWidget`.

## Delivered Android originals

These are native `adb screencap` PNGs from the dedicated Pixel 7 / API 36 arm64 emulator, each 1080 × 2400 pixels with no scaling or composite editing. The main screenshot is the real 25:00 idle UI after a warm return. The three widget screenshots separately show the same real one-minute session paused at 27,980 ms, displayed as 00:28. They are not three independently styled widgets in one simultaneous scene.

| File | Bytes | SHA-256 |
| --- | --- | --- |
| `focus-android.png` | 223057 | `c32e13b2776180778f4b036ecca36d95a5aadbac9ddd97abf06365253f1e6b81` |
| `focus-android-widget.png` | 1316635 | `6bd503630db79daaffecfef29009754dbbb2ec9826beffd5cd09e5c4e69ec4b9` |
| `focus-android-widget-tomato.png` | 1347653 | `cf95f8e593499f6d8738efd0d73a9339ead9d4fa1d59e8530094185ee08c8ee4` |
| `focus-android-widget-capsule.png` | 1284995 | `8349d59ec044652a5b30084e84f04d7b35f390e2ec92846fc09c7391af300256` |

Cold launch and three ordinary Home/return cycles were visually clean with the host GLES renderer on Apple M2 Pro, using Android WebView 133.0.6943.137. No CSS or application lifecycle workaround was added. `display-recovery.json` preserves the renderer comparison; this is emulator coverage, not physical-device testing.

## Five-language caption and alt-text mapping

The numbered rows map exactly to the filenames above. These strings are ready for README alt text and visible labels. The current website schema has project-level `cover: string` and `gallery: string[]`, with no per-image captions. Its existing gallery renderer generates localized generic alt text from project title and image number. Do not add caption objects, translated galleries or extra fields to the publication payload and expect them to render. No website schema or interface change is required for this handoff.

| Image | 简体中文 | 繁體中文 | English |
| --- | --- | --- | --- |
| 01 | Focus 浅色主窗口：平面圆环与三款组件选择 | Focus 淺色主畫面：平面圓環與三款元件選擇 | Focus in light appearance, with a flat dial and three widget styles |
| 02 | Focus 深色主窗口：倒计时与已投入时间进度 | Focus 深色主畫面：倒數與已投入時間進度 | Focus in dark appearance, showing remaining time and elapsed progress |
| 03 | 提醒设置：通知权限、系统声音与测试提醒 | 提醒設定：通知權限、系統聲音與測試提醒 | Reminder settings with permission status, system sound and a test notification |
| 04 | 五种界面语言的自定义选择菜单 | 五種介面語言的自訂選擇選單 | The custom picker for five interface languages |
| 05 | 浅色圆环悬浮计时：平面外观与闭合轨道 | 淺色圓環懸浮計時：平面外觀與閉合軌道 | The light Ring mini timer, with a flat face and continuous track |
| 06 | 深色圆环悬浮计时：同一轮计时的另一种外观 | 深色圓環懸浮計時：同一輪計時的另一種外觀 | The dark Ring mini timer, displaying the same round |
| 07 | 番茄悬浮计时：暖色外形与清楚的进度描边 | 番茄懸浮計時：暖色外形與清楚的進度描邊 | The Tomato mini timer, with warm color and a clear progress outline |
| 08 | 胶囊悬浮计时：紧凑横向布局与快捷操作 | 膠囊懸浮計時：精巧橫向配置與快捷操作 | The Capsule mini timer, with a compact horizontal layout and quick controls |
| 09 | macOS 菜单栏动态计时与原生快捷面板 | macOS 選單列動態計時與原生快捷面板 | Live macOS menu-bar time and its native control panel |
| 10 | Android 客户端的专注计时主界面 | Android 應用程式的專注計時主畫面 | The focus timer running in the Android application |
| 11 | Android 主屏幕上的原生圆环计时小组件 | Android 主畫面上的原生圓環計時小工具 | The native Ring timer widget on the Android home screen |

| Image | 日本語 | 한국어 |
| --- | --- | --- |
| 01 | フラットなリングと3つのスタイルを備えた Focus のライト画面 | 평면 링과 세 가지 위젯 스타일을 갖춘 Focus 밝은 메인 화면 |
| 02 | 残り時間と経過時間の進捗を示す Focus のダーク画面 | 남은 시간과 경과 진행을 보여 주는 Focus 어두운 메인 화면 |
| 03 | 通知の許可状態、システム通知音、テスト通知の設定 | 알림 권한 상태, 시스템 알림음과 테스트 알림 설정 |
| 04 | 5つの表示言語を選ぶカスタムメニュー | 다섯 가지 화면 언어를 선택하는 맞춤 메뉴 |
| 05 | フラットな面と閉じた軌道を持つライトのリング・ミニタイマー | 평면 디자인과 닫힌 트랙의 밝은 링 미니 타이머 |
| 06 | 同じセッションを表示するダークのリング・ミニタイマー | 같은 세션을 보여 주는 어두운 링 미니 타이머 |
| 07 | 暖かな色と見やすい進捗の輪郭を備えたトマト・ミニタイマー | 따뜻한 색과 선명한 진행 테두리의 토마토 미니 타이머 |
| 08 | 省スペースな横長レイアウトと操作ボタンを持つカプセル・ミニタイマー | 간결한 가로 배치와 빠른 조작의 캡슐 미니 타이머 |
| 09 | macOS メニューバーの残り時間とネイティブ操作パネル | macOS 메뉴 막대의 실시간 타이머와 네이티브 제어 패널 |
| 10 | Android アプリで動く集中タイマーのメイン画面 | Android 앱에서 실행되는 집중 타이머 메인 화면 |
| 11 | Android ホーム画面のネイティブなリング・タイマーウィジェット | Android 홈 화면의 네이티브 링 타이머 위젯 |

For the two additional Android style captures, use the same native-widget wording as image 11 and replace only the style name: 圆环 → 番茄 / 胶囊; 圓環 → 番茄 / 膠囊; Ring → Tomato / Capsule; リング → トマト / カプセル; 링 → 토마토 / 캡슐. Describe them as separate screenshots, not simultaneous appearances.

## Capture checks and actual QA status

- Digits count down while colored progress represents elapsed time and grows from zero to full. Zero has no foreground dot, pause freezes the arc, and the neutral track has no gap. Tomato uses the approved pale contrast color; the main dial and Ring use orange.
- Interactive synchronization checks use the same real round across main and mini surfaces. Exported screenshots may record different real sessions, which must be described in their capture notes. Changing style does not reset a round. Do not insert invented completion history or fake timing.
- Preserve the native mini-window transparent gutter. Do not stretch the ring, tomato or capsule to match another style's dimensions.
- Review legibility, complete labels, clipped edges, private content and unnecessary empty canvas before copying captures into the repository.
- Mac and Windows mini timers are floating application windows. Android uses a real AppWidget. Do not label the macOS surface as a WidgetKit system widget.
- A notification being submitted is not proof it appeared or sounded. Captions must not imply that mute or Do Not Disturb was bypassed.
- A still screenshot is not proof of full-screen Spaces behavior. Record the interactive test and its evidence separately from the gallery.

Root reported that the native macOS menu popover passed true full-screen placement, Escape dismissal, outside-click dismissal, second-click dismissal and opening the main window through IPC. The separate Mini NSPanel requirement has now passed on the isolated `com.makersns.focus.qa.v3` app (binary fingerprint prefix `160ae553`): all three pinned styles appeared above another app's true full-screen Space while Focus QA stayed inactive and its main window stayed offscreen. Real first-click pause was checked on Ring and Capsule; native dragging, hide/reopen with the same advancing round, stable top-left on style changes and unpin to native level 0 were also checked without leaving the Space. These observations used macOS 15.5 and one built-in display; mixed-DPI behavior has geometry unit coverage only.

Keep the exact `.data/final-macos-install-20261124/` evidence filenames in capture notes: `v3-ring-fullscreen.json`, `v3-tomato-fullscreen.json`, `v3-capsule-fullscreen.json`, `v3-widget-first-click.json`, `v3-widget-drag.json`, `v3-widget-hide.json`, `v3-widget-reopen.json` and `v3-widget-unpin.json`. They are local evidence, not public image links. The v4 QA app and normal app builds now include the corrected first-run notification guidance and have passed with 37 frontend and 54 native tests. Use the updated QA app for final screenshots; the v3 native behavior evidence remains identified by its actual build. See [verification scope](../VERIFICATION.md).

Android capture dimensions and launcher placement above were reported by the Android implementation/QA owner: dedicated Pixel 7 / API 36 arm64 emulator, 1080 × 2400 pixels, tested 3 × 2 launcher placement with 699 × 588-pixel root bounds (approximately 266 × 224 dp). The code's minimum geometry is 220 × 132 dp. Do not present these emulator results as physical-device testing.

Notification capture readiness: the real macOS authorization dialog, successful Allow action, refreshed granted state and visible test notifications with sound requested both on/off have passed on the isolated v3 QA app. The source receipts are `v3-native-notification-prompt.json`, `v3-notification-granted-refreshed.json`, `v3-notification-test-sound.json` and `v3-notification-test-silent.json` in the same local evidence folder. Paused timer/history/preferences stayed unchanged through the tests. No physical audibility result is claimed. The separate natural one-minute completion has now passed on v4, with one completed 60,000 ms record, stable zero/history after eight further seconds and an actual OS completion notification (`v4-natural-completion.json/png`). The post-completion Mini action to start a short break passed without changing history (`v4-next-phase.json`). Capture Settings from v4 so the image includes the corrected prompt-versus-denied guidance.

## Manifest entries and provenance

The existing private preparation directory is `sugar-personal-studio/.data/focus-project-update-20261123/`. This repository document records the handoff contract; it does not execute a production write.

Populate a fresh run's `release-manifest.json.images` in the table order. The supported item shape is:

```json
{
  "file": "docs/images/focus-mini-tomato.png",
  "role": "desktopMini",
  "kind": "native-screenshot",
  "appVersion": "<releaseTag without its initial v>",
  "capturedAppCommit": "<actual 40-character capture baseline commit>",
  "captureNotes": "<QA build identity, real source changes, evidence path and review result>",
  "sha256": "<actual 64-character SHA-256 of this PNG>"
}
```

The placeholder values above intentionally are not a publishable manifest. Replace them with observed values. The other two new entries use `docs/images/focus-mini-capsule.png` / `desktopMini` and `docs/images/focus-menu-bar.png` / `menuBar`, with the same required metadata fields. The language-menu image uses role `settings`, just like the main settings capture. Multiple `desktopMini` or `settings` roles are allowed; filenames and file hashes must remain unique.

For every item:

1. `file` must resolve to a real original PNG inside this repository's `docs/images`. Preserve the source bytes through upload and public read-back.
2. `appVersion` must match the final release manifest and `releaseTag` without its leading `v`. Record the actual QA application's version, not an assumed one.
3. `capturedAppCommit` is the actual committed baseline at capture time. The helper verifies it is an ancestor of the final release commit. It may differ from the release commit because screenshots and reviewed source changes can be committed later. Never substitute the final commit for an earlier real capture baseline.
4. `captureNotes` must identify the independent QA app/bundle ID or emulator, the build/source fingerprint evidence, relevant uncommitted changes at capture, the corresponding native evidence path and root's review result. A commit-ancestor check alone does not prove the screenshot depicts the final source.
5. `sha256` comes from the final reviewed PNG bytes. Width, height and size are derived by `prepare.mjs`; they do not need to be invented or added to the input manifest.
6. Keep `screenshotsReviewed: false` until every image and its provenance have been reviewed. When a surface still needs a required behavior fix, capture again from the corrected QA build.
7. Media validation permits PNGs up to 8 MiB, dimensions up to 16000 pixels per side and at most 50,000,000 total pixels. The sizes in this plan are comfortably below the pixel limits; check actual byte sizes.

The publish script assigns `next.cover = uploaded[0].url`, `next.coverKind = "real"`, and `next.gallery = uploaded.slice(1).map(item => item.url)`. Gallery order therefore follows the manifest. There are no invented `/media/` URLs in the source file. Keep source `gallery: []` and do not place gallery or cover fields inside translations.

## Completed README integration

All five READMEs now use the reviewed native files with localized alt text. Each displays nine unique product screenshots: the light main cover, dark main/settings pair, Ring/Tomato/Capsule table, dark Ring/Android widget table and the native macOS menu-bar image. The original download, project, source and license links are preserved. The language-menu and Android-main images remain website gallery entries; the two extra Android style captures remain separate repository evidence outside the eleven-image website selection.

| README | Image prefix | Style-section heading | Surface-section heading |
| --- | --- | --- | --- |
| `README.md` | `docs/images/` | `Three looks. The same timer.` | `The timer stays where you need it` |
| `README.zh-CN.md` | `docs/images/` | `三种样式，同一计时` | `把计时放在顺手的位置` |
| `docs/README.zh-TW.md` | `images/` | `三種樣式，同一計時` | `隨時看得到` |
| `docs/README.ja.md` | `images/` | `3つのかたち、同じタイマー` | `必要な場所に、残り時間を` |
| `docs/README.ko.md` | `images/` | `세 가지 모습, 같은 타이머` | `필요한 곳에서 시간을 확인하세요` |

The platform paragraph in each README includes the verified pinned-mode behavior in its own language:

| Locale | Full-screen Mini wording |
| --- | --- |
| 简体中文 | 在 macOS 中，将小组件置顶后，也能在其他应用的全屏空间上方查看计时。 |
| 繁體中文 | 在 macOS 中，將小工具置頂後，也能在其他應用程式的全螢幕空間上方查看計時。 |
| English | On macOS, pin the mini timer to keep it visible above another app in a full-screen Space. |
| 日本語 | macOS ではミニタイマーを最前面に固定すると、別のアプリのフルスクリーンスペースでも表示できます。 |
| 한국어 | macOS에서 미니 타이머를 항상 위에 표시하도록 고정하면 다른 앱의 전체 화면 공간 위에서도 시간을 확인할 수 있습니다. |

The five portfolio descriptions, desktop-widget body paragraphs and corresponding highlight also include this behavior. The normal installed macOS v4 app independently passed the full-screen check with unchanged user state; the local receipts are `normal-v4-install/installation.json` and `v4-installed-fullscreen.json/png`. Single-display and mixed-DPI scope remains as described in the verification document.

## Publication sequence and exact refresh points

The reviewed `docs/portfolio-content.json` SHA-256 at this handoff is:

```text
bc21328b3775379c210662f9babbbd70c15d4473d8f9df3417837b5f1d86dbd7
```

Earlier helper records may still contain the superseded `a30bcc…` or `7c9bdd…` hashes. A new run must use the current reviewed source hash above, recomputed if any localized content changes. Do not update a sealed run's prepared outputs to work around the guard.

1. Finish required native behavior QA and capture from the corresponding independent QA build. Record observed metadata and image hashes. Only then set `screenshotsReviewed` after review.
2. Refresh the five READMEs as described above and retain all five portfolio locales. The source document's `id` / `sourceAvailable` metadata remains for helper compatibility; the public normalization whitelist removes these fields.
3. Run the existing `prepare.mjs --validate-content` mode for local five-language normalization/review checks. This mode is explicitly not publishable and does not upload media or write production data.
4. Publish and verify the final stable GitHub Release, exact commit, four-platform downloads and `SHA256SUMS`. Screenshots alone or a local build do not satisfy the release guard.
5. Create a fresh run through the existing read-only Focus snapshot flow. Its current version and published version populate the manifest; do not reuse the earlier observed version zero.
6. Populate the manifest with the exact eleven images, current content hash, all five reviewed locales, actual release metadata and verified downloads. Preparation checks local and public release commits, asset size/checksums, image bytes, capture-baseline ancestry and the website's existing content validators.
7. Review the resulting preflight, payload and prepared content before invoking the existing publication entry point. The publisher preserves project identity/order/style, uploads and reads back original images, then performs the existing save/publish operations for Focus within its guarded transaction.
8. If any prepared source, publisher, backup or payload changes, use a new run. Release verification older than 24 hours must be refreshed. A failure or uncertain connection at commit time requires read-only reconciliation, not a blind retry.
9. After publication, run the existing public GET-only verifier for all five languages, release/source links, real shared gallery and exact public PNG bytes, then review the live pages and image viewer. A publication command returning does not replace that review.

No website database, media, helper source, frontend, native source or publication state is modified by this document update.
