# UI ライブラリ リファクタリング計画

`ui/` 配下、主に `zintl-ui-desktop` と `zintl-ui-native` の API を揃えるための計画でした。
クロスプラットフォーム対応の評価は対象外です。

ステップ 1〜8 はすべて完了しました。内訳は、共通モディファイア、ラッパー層の廃止、値とコールバック、選択と ID、公開面、利用者定義ビュー、ビューの状態、細かい不揃いです。
使い方は `ui/zintl-ui-desktop/README.md` と `ui/zintl-ui-desktop/examples/custom_view.rs` を参照してください。
後で拾う価値のある課題は `TODO.md` に移しています。

## 見送ったこと

再開するときの前提として、採らなかった案とその理由を残します。

| 項目 | 理由 |
|---|---|
| `Event` の統合(選択系 3 つ、テキスト系 3 つ) | サイドバーとツールバーのイベントは宛先がウィンドウです。統合には送信元を区別するフィールドが要り、両バックエンドのイベント送出を書き換えることになります。効果に対して変更が大きいため見送りました。 |
| 実測ベースの文字幅 API | 実測はすでにレイアウト時に行われています(`LayoutNative::measure_wrapping`)。広げるなら、ラベルとボタンの幅もレイアウト時に測る方向にします(`TODO.md`)。 |
| `Toggle` / `Picker` / `TabView` の非制御モード | ステップ 3 で決めた「離散的なコントロールは常に `value` を表示する」規則の例外になるためです。また、`init` を持つ型が増えます。状態を変えるには `bind` します。 |
| アイコンボタンのラベル欠落の警告 | `Button::icon` がラベルを必須の引数にしたので不要です。 |
| `TextEditor` のプレーンテキストバインド | 貼り付けた書式を落とす設定をバックエンドに足す必要があり、別の機能になります(`TODO.md`)。 |
| `system_image` の `Symbol` 型化 | クロスプラットフォーム評価の範囲です。そのときは `Button::icon`、`Button::system_image`、`SidebarItem::system_image`、`ToolbarItem::button` を同時に変えます。 |
| `MainAxisDistribution::SpaceAround` / `SpaceEvenly` | 使う箇所がありません。 |
| `Grid` の縞をレイアウト後の行位置に敷く案 | レイアウト結果を Canvas に渡す仕組みが要るためです。`Grid` は `GridRow` で行単位に受け取ります。 |

## 現状の層構造

```
zintl-ui                 View / Store / Local / Element / Composer(描画先に依存しない中核)
zintl-ui-layout          LayoutStyle と taffy によるレイアウト
zintl-ui-graphics        Canvas 用の 2D 描画型
zintl-ui-backend-core    RenderNode / ViewKind / Event / Context / Palette / HeadlessBackend
zintl-ui-native          すべての宣言的ビュー(RenderNode を組み立てる、AppKit 寄りの語彙)
zintl-ui-desktop         App ランナーと、native のビューを領域別モジュールで再公開する入口
zintl-ui-appkit-backend  AppKit バックエンド
zintl-ui-winui3-backend  WinUI 3 バックエンド
zintl-ui-test            zintl-ui 中核用のテストバックエンド
```
