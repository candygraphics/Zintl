# ステップ 1: 共通モディファイア(レイアウト・`id`・`enabled`)

## 目的

サイズ指定、`id`、`enabled` を型ごとの個別メソッドから、全ビュー共通のトレイトへ移します。
利用者が「この型で `grow` は使えるか」を調べなくて済むようにします。

## 現状

`RenderNode::View` は `layout`、`id`、`enabled` を全ビュー共通のフィールドとして持っています
(`ui/zintl-ui-backend-core/src/lib.rs`)。しかしビルダーは型ごとに別実装です。

| 型 | 使えるサイズ指定 |
|---|---|
| `HStack` / `VStack` | `fill_width` `width` `height` `minimum_size` `padding` `grow` |
| `ZStack` | `fill_width` `grow` |
| `Grid` | `fill_width` |
| `Text` | `minimum_size` `fill_width` |
| `Button` | `minimum_size` |
| `Slider` / `SearchField` | `minimum_size` `fill_width` |
| `ScrollView` | `minimum_size` `height` |
| `Table` | `minimum_size` `height` |
| `Canvas2D` | `size` `minimum_size` `fill_width` `grow` |
| `ProgressIndicator` | `size` |
| `Toggle` `Picker` `BarChart` `SplitView` `DisclosureGroup` | なし |

- `id: Option<String>` フィールドと `pub fn id` が各型に重複しています。
- `enabled` は desktop の `TextField`、`SearchField`、`TabView` にありません。native の `TextField` は `enabled: true` を固定で出力しています。
- 手本になる仕組みはすでにあります。`Attach` トレイト(`ui/zintl-ui-native/src/attach.rs`)は `NativeView::native_element` の中で、`base_element` が返したノードの `help` と `accessibility` を後から上書きしています。

## 仕様(提案)

### 1. `Attachments` を `Modifiers` に拡張する

```rust
/// 全 NativeView が持つ共通設定。
#[derive(Clone, Default)]
pub struct Modifiers {
    id: Option<String>,
    enabled: Option<bool>,
    layout: LayoutOverrides,
    // 既存の Attachments の中身
    menu: Option<Menu>,
    popovers: Vec<Popover>,
    title_bar_interactive: bool,
    help: Option<String>,
    accessibility: Accessibility,
}

/// 明示された項目だけを持ち、ビュー既定の LayoutStyle に重ねる。
#[derive(Clone, Copy, Default)]
pub struct LayoutOverrides {
    width: Option<LayoutDimension>,
    height: Option<LayoutDimension>,
    minimum_size: Option<Size>,
    grow: Option<f32>,
}
```

`NativeView` のアクセサは `modifiers()` / `modifiers_mut()` に改名します。
`attachments!()` マクロは `modifiers!()` に改名し、クレート外にも公開します(ステップ 6 で使用)。

### 2. トレイト

| トレイト | 実装対象 | メソッド |
|---|---|---|
| `Layout` | 全 `NativeView`(ブランケット実装) | `width(f32)` `height(f32)` `size(Size)` `fill_width()` `fill_height()` `minimum_size(Size)` `grow()` |
| `Attach` | 全 `NativeView`(既存) | 既存のもの + `id(impl Into<String>)` |
| `Control` | 入力を受けるビューだけ(型ごとに明示実装) | `enabled(bool)` |

`Control` をブランケット実装にしない理由: `Text` やスタックに `enabled` を付けても、子孫を無効化する仕組みが今のバックエンドに無いためです(未決事項 1)。

### 3. 適用規則

`native_element` で `base_element` の結果に次の順で重ねます。

1. `id`: `Modifiers.id` が `Some` ならノードの `id` を置き換える。
2. `enabled`: `Some` ならノードの `enabled` を置き換える。
3. `layout`: `Some` の項目だけノードの `layout` に書き込む。
4. 固定寸法と `grow` の関係: `width` / `height` に `Points` を指定し、`grow` を明示していない場合は `grow = 0.0` にする。
   既定が「残りを埋める」ビュー(`ScrollView`、`Table`)で、現在の `ScrollView::height` と同じ挙動を保つためです。
5. 既存どおり `title_bar_interactive`、`help`、`accessibility` を重ねる。

対象は `Element::Node { value: RenderNode::View { .. }, .. }` だけです。
それ以外を返す `base_element` はデバッグビルドで `debug_assert!` に掛けます。

### 4. ビュー固有ロジックの扱い

`base_element` の中から `self.modifiers().layout` を参照して判断します。

| 型 | 現在の固有ロジック | 変更後 |
|---|---|---|
| `Button` | `declares_minimum` フラグでアイコンのみボタンの最小サイズを切り替える | `modifiers().layout.minimum_size.is_some()` で判定し、フラグを削除 |
| `TextField`(ラベル) | `fill_width` 時に `resize_label` を呼ぶ | `base_element` で `layout.width` が `Percent` かを見て同じ計算をする |
| `ScrollView` | `height` が `grow = 0` にする | 適用規則 4 に吸収 |

### 5. 合成ビュー

外側のコンテナノードに重ねます。`Toggle`(Switch スタイルの行)、`Picker`(Radio)、`BarChart`、`DisclosureGroup` が該当します。
`enabled` は、合成ビューが内側のコントロールへ自分で渡します(`Toggle` と `Picker` は現在もそうしています)。

### 6. コンテナ専用として残すもの

`spacing`、`padding`、`alignment`、`space_between`、`equal_width_children`、`ignores_safe_area` は子の並べ方の指定なので、スタック類の固有メソッドのままにします(改名はステップ 8)。

## 削除するもの

- 各型の `id` フィールドと `pub fn id`。
- 各型の `minimum_size` / `fill_width` / `grow` / `width` / `height` / `size`。
- native の `control!` マクロのビルダー部分と `enabled!` マクロ。
- desktop の `stack!` マクロのサイズ指定部分、`wrapper!` マクロの `id` / `minimum_size`。
- `match &self.id { Some(id) => view.id(id.clone()), None => view }` の繰り返し(`Stack::native_element`、`Container::element`、`Toggle`、`Picker` など)。

`Popover::size` と `TableColumn::width` はビューではないので対象外です。

## 手順

1. `Modifiers` / `LayoutOverrides` / `Layout` / `Control` を native に追加し、`native_element` に適用規則を実装する。
2. native の各型から個別メソッドを削除し、`base_element` の `id` / `enabled` 出力を既定値に揃える。
3. desktop の各型を同様に直す。
4. `examples/`、`tests/ui`、`examples/notes` のコンパイルエラーを直す。メソッド名は同じなので、多くは `use` の追加だけで済む見込みです。

## テスト

- 各 `NativeView` 型について、`width` / `height` / `minimum_size` / `grow` / `id` がレンダーノードに届くこと(型の一覧を回すテスト 1 本)。
- `ScrollView::new(..).height(120.0)` の `grow` が `0.0` になること。`.height(120.0).grow()` では `1.0` のままであること。
- アイコンのみの `Button` が、`minimum_size` 指定の有無で従来どおりの最小サイズになること。
- `fill_width` したラベルが従来どおり折り返しサイズを再計算すること。
- `Control` 実装型で `enabled(false)` がノードに届くこと。

## 未決事項

1. コンテナへの `enabled(false)` を子孫に伝播させるか。伝播させるなら `Control` をブランケット実装にでき、バックエンド側の対応が必要です。
2. `padding` を辺ごとに指定できるようにするか。`LayoutStyle::padding` が `f32` 1 つなので、`zintl-ui-layout` と両バックエンドの変更が伴います。本ステップからは外す想定です。
3. `fill_height` を新設するか。現在どの型にも無く、`grow` と `CrossAxisAlignment::Stretch` で代用されています。
