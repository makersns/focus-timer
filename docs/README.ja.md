# Focus by Sugar

**ひとつのことに、少しの時間を。** [Sugar](https://sugar.makersns.com) が作る、デスクトップと Android 向けの穏やかな集中タイマーです。

[English](../README.md) · [简体中文](../README.zh-CN.md) · [繁體中文](README.zh-TW.md) · 日本語 · [한국어](README.ko.md)

![Focus の明るいテーマ](images/focus-light.png)

## 小さな一歩から

取り組みたいことをひとつ決め、時間を選んで開始します。画面の中心はタイマーと次の操作。休憩を取るタイミングも、次のセッションを始めるタイミングも、自分で選べます。

- 集中・短い休憩・長い休憩。標準は 25 / 5 / 15 分、1〜180 分に変更できます。
- macOS のメニューバーに残り時間を表示。デスクトップのミニタイマーは移動と最前面表示に対応し、メイン画面と同期します。
- Android のホーム画面ウィジェットから、そのまま開始・一時停止・再開できます。
- デスクトップの終了時は一時停止して保存。Android は同じ起動中のプロセス再生成から実行中のタイマーを復元し、端末の再起動後は一時停止します。
- 完了したセッションと途中で終了した集中時間を区別。休憩時間を集中時間に加算しません。
- ライト・ダーク・システム設定に対応。簡体字中国語、繁体字中国語、英語、日本語、韓国語を選べます。
- アカウント、広告、アクセス解析はありません。通知は任意で、次のタイマーは自動的に始まりません。

## 必要な場所に、残り時間を

macOS / Windows は軽量なフローティングウィンドウ、Android はネイティブ AppWidget を使用します。ミニタイマーを隠しても計時は続き、毎秒の更新でほかのアプリのフォーカスを奪うことはありません。

| デスクトップミニタイマー | Android ホーム画面ウィジェット |
| --- | --- |
| <img src="images/focus-mini.png" width="360" alt="Focus のデスクトップミニタイマー" /><br /><br /><img src="images/focus-mini-dark.png" width="360" alt="Focus のダークテーマのデスクトップミニタイマー" /> | <img src="images/focus-android-widget.png" width="220" alt="Focus の Android ホーム画面ウィジェット" /> |

## ダウンロード

[GitHub Releases](https://github.com/makersns/focus-timer/releases/latest) から macOS ユニバーサル DMG、Windows x64 インストーラー、Linux x64 AppImage / DEB / RPM、Android arm64 の署名済み APK をダウンロードできます。SHA-256 のチェックサムも同梱しています。

初回リリースは Apple の公証と Windows の発行元証明書による署名を行っていません。OS が開発元を確認できない旨を表示する場合があります。Linux のトレイ機能はデスクトップ環境に依存します。

## 記録について

直近 500 件のセッションと、最大 366 日付分の日別合計を端末に保存します。途中終了した集中時間も保存されますが、完了数には含めません。記録を消去しても実行中のタイマーは止まりません。

デスクトップでは操作時と最長 30 秒間隔のチェックポイントで保存します。突然終了した場合は、最後に保存した時点以降の進行が失われる可能性があります。Android はシステムの Chronometer、フォアグラウンドサービス、終了時刻のアラームを使い、起動情報と経過時間を同じ状態ファイルで管理します。記録をサーバーへ送信することはありません。

## 開発

Node.js 24 以降、Rust、[Tauri 2 のプラットフォーム依存環境](https://v2.tauri.app/start/prerequisites/) を用意してください。

~~~bash
git clone https://github.com/makersns/focus-timer.git
cd focus-timer
npm ci
npm run tauri -- dev
~~~

~~~bash
npm test
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run tauri -- build
~~~

構成と翻訳の手順は [Contributing](../CONTRIBUTING.md) にあります。ブラウザーでの UI プレビューは npm run dev で起動できます。トレイ、フローティングウィンドウ、Android ウィジェット、通知はインストール版の機能です。Android の開発と署名方法は [Android ガイド](ANDROID.md) を参照してください。

気に入ったら、Star で応援していただけるとうれしいです。不具合の報告や翻訳の改善も歓迎します。

[MIT ライセンス](../LICENSE) · [作者のサイト](https://sugar.makersns.com) · [更新履歴](../CHANGELOG.md)
