# ステップ 5: 公開面とファイル構成

## 目的

- `zintl_ui_desktop` のフラットな再エクスポートを、用途別のモジュールに分けます。
- テスト用の検査 API を `App` から切り離します。
- 1000 行を超えるファイルを分割し、重複したヘルパーをまとめます。

## 現状

### 公開面

`ui/zintl-ui-desktop/src/lib.rs` はクレート直下に約 150 個の名前を出しています。次のものが同列に並んでいます。

- アプリを書く人が使うもの: `Window`、`Button`、`Store`、`View`、`list!` など
- カスタムビューを書く人だけが使うもの: `NativeView`、`Attachments`、`Element`、`ElementFactory`、`RenderNode`、`ViewKind`、`Event`、`EventKind`
- 特定機能のもの: メニュー関連 15 個、Canvas 関連 20 個超、アクセシビリティ関連

### 検査 API

- `App::render()`、`App::render_tree()`、`RenderedNode` が公開されています。用途はテストでのツリー検査です。
- `App` の `#[cfg(test)]` メソッド(`dispatch_root_event`、`dispatch_at`、`render_roots`、`update_text_store`)は desktop 内のテスト専用で、外部クレートのテストからは使えません。
- native のテストは別のヘルパー(`composer()`、`root_id()`)を持っています。
- `tests/ui` と `examples/notes` は `render_tree` も `RenderNode` も参照していません(grep で確認)。

### ファイル

| ファイル | 行数 | 中身 |
|---|---|---|
| `zintl-ui-desktop/src/lib.rs` | 1277 | マクロ、`Window`、`Text`、`Button`、`TextField`、スタック、`Spacer`、`App`、テスト約 500 行 |
| `zintl-ui-desktop/src/widget_tests.rs` | 2294 | 全ウィジェットのテスト |
| `zintl-ui-native/src/controls.rs` | 1397 | `Switch` から `DatePicker` まで 11 種 |
| `zintl-ui-native/src/menu.rs` | 1068 | メニュー、ショートカット、ローカライザ、標準メニュー |

## 仕様(提案)

### 公開モジュール

ステップ 2 の後、ビューは native にあり、desktop は再エクスポートの窓口です。

| パス | 内容 |
|---|---|
| クレート直下 | `App`、`AppError`、全ビュー型とそのスタイル enum、`View`、`Store`、`Context`、`list!`、共通モディファイアのトレイト(`Layout` `Attach` `Control`)、`Rect` `Size` `Color`、レイアウト enum(`Axis` `CrossAxisAlignment` `MainAxisDistribution`) |
| `menu` | `MainMenu` `Menu` `MenuItem` `Shortcut` `ShortcutKey` `Modifier` `StandardAction` `StandardMenu` `StandardText` `MenuLocalizer` と両ローカライザ |
| `canvas` | `Canvas2D` `Canvas2DHandler` `CanvasEvent` `CanvasEventContext` `CanvasId` `CanvasSetup` `Frame` `Redraw` `RedrawPolicy` `Invalidator` `Cursor`、ポインタ・キー・スクロール・拡大のイベント型、`wgpu` フィーチャ時の `WgpuCanvas` 一式 |
| `graphics` | 既存の `zintl_ui_native::graphics` をそのまま |
| `accessibility` | `AccessibilityRole` `AnnouncementPriority` |
| `dialog` | `Alert` `AlertButton` `AlertStyle` `FileDialog` `ColorPanel` `Popover` `PopoverEdge` |
| `render` | カスタムビュー作者向け: `NativeView` `Modifiers` `modifiers!` `Element` `ElementFactory` `IntoElement` `KeyedElement` `ListFactory` `RenderNode` `ViewKind` `Event` `EventKind` `LayoutStyle` `LayoutDimension` `ChildSizing` |
| `thread` | `Signal` `StoreSender` |
| `testing` | 下記のハーネス |

`use zintl_ui_desktop::*;` だけで通常のアプリが書ける状態は保ちます。
`examples/canvas.rs` などは `use zintl_ui_desktop::canvas::*;` を追加します。

### 検査用ハーネス

`App` から検査 API を外し、`testing::Harness` にまとめます。置き場所は native(ビューと同じクレート)で、desktop が再エクスポートします。

```rust
/// ヘッドレスバックエンドにビューをマウントし、ツリーの検査とイベント送出を行う。
pub struct Harness { .. }

impl Harness {
    pub fn new<E: IntoElement<Output = RenderNode>>(root: E) -> Self;

    /// 最初のルート要素のツリー。
    pub fn tree(&self) -> RenderedNode;
    /// メインメニューや複数ウィンドウを含む全ルート。
    pub fn roots(&self) -> Vec<RenderedNode>;
    /// ビュー ID で最初に見つかったノード。
    pub fn find(&self, id: &str) -> Option<RenderedNode>;

    /// ルートからの子インデックス列で指したノードへイベントを送る。
    pub fn dispatch(&mut self, path: &[usize], event: Event) -> bool;
    /// ビュー ID で指したノードへイベントを送る。
    pub fn dispatch_to(&mut self, id: &str, event: Event) -> bool;

    pub fn get<T: Clone + 'static>(&mut self, store: Store<T>) -> T;
    /// Store を更新し、再描画まで行う。
    pub fn update<T: 'static>(&mut self, store: Store<T>, update: impl FnOnce(&mut T));
    /// View::init の外で Store を作る。
    pub fn store<T: 'static>(&mut self, value: T) -> Store<T>;
}
```

- `RenderedNode` は `testing` に移します。
- `App::render`、`App::render_tree`、`App` の `#[cfg(test)]` メソッド、native テストの `composer()` / `root_id()` を削除し、すべて `Harness` に置き換えます。
- `App` は常に実バックエンドを使うようにでき、`DesktopBackend` の `cfg(test)` 分岐が不要になります。
- `testing` をフィーチャフラグの裏に置くかは未決事項 1。

### ファイル分割

ステップ 2 後の native を想定した配置です。

| 新ファイル | 中身 |
|---|---|
| `text.rs` | `Text` |
| `text_field.rs` | `TextField`(既存) |
| `text_editor.rs` | `TextEditor`(`text_field.rs` から分離) |
| `button.rs` | `Button` `ButtonStyle` `ButtonRole` |
| `stack.rs` | `HStack` `VStack` `Spacer` `ZStack` と内部の `Container` |
| `grid.rs` | `Grid` `GridCell` |
| `disclosure.rs` | `DisclosureGroup` |
| `toggle.rs` | `Toggle` `ToggleStyle` と内部の `Switch` |
| `picker.rs` | `Picker` `PickerStyle` と内部の `PopUpButton` `SegmentedControl` |
| `slider.rs` `combo_box.rs` `search_field.rs` `color_well.rs` | 各 1 型 |
| `number_field.rs` | `NumberField` `IntegerField` `IntegerValue` |
| `date_picker.rs` | `DatePicker` `OptionalDatePicker` |
| `menu/` | `mod.rs`(`Menu` `MenuItem`)、`shortcut.rs`、`main_menu.rs`、`localizer.rs` |
| `testing.rs` | `Harness` `RenderedNode` |

desktop 側は `lib.rs`(再エクスポート)と `app.rs`(`App`、バックエンド選択)だけになります。

テストは対象の型と同じファイルの `#[cfg(test)] mod tests` に移し、`widget_tests.rs` を解体します。

### 重複の整理

| 重複 | 対応 |
|---|---|
| `Stack::native_element` と `Container::element` | native の `Container` に一本化(ステップ 2 と同時) |
| `impl View for X { render → native_element }` の定型。native の `control!`、desktop の `native_view!`、手書き約 15 か所 | `native_view!` 1 つに統一し、`modifiers!` と併せて全型で使う |
| `NativeView` の `attachments` / `attachments_mut` 手書き(desktop に約 10 か所) | `modifiers!()` に置き換え |
| `type Change<T> = dyn for<'a> Fn(&mut Context<'a>, T)` が desktop と native の両方に定義 | native の 1 か所に |

`impl<V: NativeView> View for V` のブランケット実装は採りません。
`DisclosureGroup` や `SplitView` のように `init` を持つ型が自前の `View` 実装を必要とし、衝突するためです。

## 手順

1. `Harness` を追加し、既存テストを載せ替える(ステップ 2a より前に行うと、テスト移動が 1 回で済む)。
2. `App` から検査 API を削除する。
3. ファイル分割(コードの変更を伴わない移動だけの PR にする)。
4. 公開モジュールを作り、クレート直下の再エクスポートを絞る。examples と下流クレートの `use` を直す。

## テスト

- 既存テストが `Harness` 上ですべて通ること。
- `Harness::find` / `dispatch_to` が、入れ子のビュー ID を正しく引けること。
- 全 example が新しい `use` でビルドできること(`cargo check --examples`、`wgpu` フィーチャ付きも)。

## 未決事項

1. `testing` モジュールを `testing` フィーチャの裏に置くか。`HeadlessBackend` は `backend-core` に常時入っているので、置かなくてもビルド時間への影響は小さいです。
2. `zintl-ui-test` クレート(中核用の `TestBackend`)との関係。役割が違う(中核の差分操作の検証 vs ウィジェットの検査)ので併存させる想定ですが、名前が紛らわしければ `zintl-ui-test` を `zintl-ui-core-test` に改名します。
3. モジュール名 `render` は `View::render` と紛らわしい可能性があります。代案は `custom` または `raw`。
