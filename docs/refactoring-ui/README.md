# UI ライブラリ リファクタリング計画

`ui/` 配下、主に `zintl-ui-desktop` と `zintl-ui-native` の API を揃えるための計画でした。
クロスプラットフォーム対応の評価は対象外です。

ステップ 1〜8(共通モディファイア、ラッパー層の廃止、値とコールバック、選択と ID、公開面、利用者定義ビュー、ビューの状態、細かい不揃い)はすべて完了しました。
使い方は `ui/zintl-ui-desktop/README.md` と `ui/zintl-ui-desktop/examples/custom_view.rs` を参照してください。

## 決定事項

各ステップで未決だった点の結論です。見送ったものは、再開するときの前提として残します。

### 利用者定義ビュー(ステップ 6)

- **`Event` の統合は行わない。** サイドバーとツールバーのイベントはウィンドウ宛てで、統合には送信元を区別するフィールドが要ります。効果に対して変更が大きいためです。
- **`Palette` に `separator` と `alternating_row` を追加した。** AppKit は `NSColor.separatorColor` と `alternatingContentBackgroundColors[1]` を外観ごとに解決します。WinUI は従来どおり `Palette::LIGHT` の固定値です。
- **実測ベースの文字幅 API は用意しない。** 見積もりは `TextEstimate` に一本化しました。AppKit はコントロールとラベルの最小サイズを実測で引き上げるため、見積もりは下限として働きます。必要になったら `Context` 経由で問い合わせ、ヘッドレスでは `TextEstimate` に戻す形にします。
- 外部クレートは `ViewKind` を追加できません。拡張手段は合成、`Canvas2D`、`WgpuCanvas` です(`render` モジュールの doc に記載)。

### ビューの状態(ステップ 7)

- **`Toggle` / `Picker` / `TabView` に非制御モードは与えない。** 値を表示する制御コンポーネントのままにします(ステップ 3 の規則 4)。内部状態が欲しい合成ビューは `Local<T>` で自作できます。
- **名前は `Local<T>`。** `zintl_ui::store` に置き、`zintl-ui-desktop` の直下から再公開しています。
- **要確認事項の結果**: 旧 `SplitView` はドラッグ中に親が再描画されると掴み位置を失っていました。掴み位置を `Divider` ハンドラに移して修正し、回帰テスト `split_view_drag_survives_a_rebuild_of_its_parent` を追加しました。

### 細かい不揃い(ステップ 8)

- **`Window` の既定サイズは 640 × 480、パネルは 320 × 400。** 位置を省略すると画面中央に開きます。
- **ラベルの無いアイコンボタンは警告しない。** 親にラベルがある場合などで誤検出しやすく、出力先も決められないためです。`Button::icon` の doc で `accessibility_label` を案内します。
- **`TextEditor` にプレーンテキストのバインドは足さない。** `AttributedText::from(&str)` と `AttributedText::text` で変換します。
- **`system_image` は SF Symbols 名の `String` のまま。** クロスプラットフォーム評価を始めるときに、`Button`、`SidebarItem`、`ToolbarItem` を同時に `Symbol` 型へ置き換えます。
- `Grid` の縞は `Grid::rows` と `GridRow` で作ったグリッドだけに付きます(案 A)。`Grid::new` は従来どおり子を順に流し込み、`GridCell` で列を結合します。
- ヘルプボタンは `ButtonStyle::Help` をやめ、`Button::help()` にしました。

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
