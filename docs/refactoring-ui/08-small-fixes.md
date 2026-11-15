# ステップ 8: 細かい不揃いの修正

## 目的

コンストラクタ、真偽値メソッド、数値型、命名の小さな不揃いをまとめて直します。
項目は互いに独立しているので、1 項目ずつ別の PR にできます。

## 項目一覧

| # | 項目 | バックエンド変更 |
|---|---|---|
| 8.1 | `Window` のコンストラクタ | あり |
| 8.2 | アイコンのみのボタン | なし |
| 8.3 | 真偽値メソッドの形 | なし |
| 8.4 | `Slider` の目盛り | なし |
| 8.5 | スタックのメソッド名 | なし |
| 8.6 | `f32` / `f64` の使い分け | なし |
| 8.7 | `Grid` の縞と列結合 | なし |
| 8.8 | `TextField::multiline` と `TextEditor` の関係 | なし |

## 8.1 `Window` のコンストラクタ

**現状**: `Window::new(bounds: Rect, title)`。位置が必須で、サンプルはすべて `Rect::new(100.0, 100.0, w, h)` のような決め打ちです。`autosave` で復元する場合も初期位置を書く必要があります。

**仕様**:

```rust
Window::new("Zintl")                    // 既定サイズ、画面中央
    .size(Size::new(640.0, 400.0))      // 初期の内容サイズ
    .position(Point::new(100.0, 100.0)) // 省略時は中央
```

- `Window::panel(title)` も同じ形にします。
- `RenderNode::Window::bounds: Rect` を `size: Size` と `position: Option<Point>` に分けます。
- バックエンドは `position` が `None` のとき、初回表示時に中央へ置きます(AppKit は `NSWindow.center()`)。
- `autosave` で復元されたフレームがあれば、従来どおりそちらが優先です。
- 既定サイズは未決事項 1。

## 8.2 アイコンのみのボタン

**現状**: `Button::new("").system_image("gearshape")`。空文字列のタイトルが「アイコンのみ」の合図になっています。

**仕様**:

- `Button::icon(system_image: impl Into<String>)` を追加します。
- `Button::new("")` の特別扱い(`title.is_empty() && system_image.is_some()` で最小サイズを切り替える処理)は、`Button::icon` で作られたかどうかのフラグに置き換えます。
- doc に「アイコンのみのボタンには `accessibility_label` を付ける」と書きます。デバッグビルドで未設定を警告するかは未決事項 2。
- `DisclosureGroup` 内部の `Button::new("")` と `ButtonStyle::Help` のボタンも、タイトルなしコンストラクタを使う形に直します。

## 8.3 真偽値メソッドの形

**現状**: 引数ありと引数なしが混在しています。

| 引数あり | 引数なし |
|---|---|
| `enabled(bool)` `striped(bool)` `focusable(bool)` `edited(bool)` `selected(bool)` `grouping(bool)` `fit_columns(bool)` `choose_files(bool)` `shows_alpha(bool)` | `multiline()` `multiple()` `accessibility_hidden()` `title_bar_interactive()` `ignores_safe_area()` |

**規則**:

- **状態・機能のオンオフは `bool` を取る。** 条件付きで切り替えるときに `if` でビルダーを分岐させずに済みます。
- **レイアウトの指示は引数なし。** `fill_width()`、`fill_height()`、`grow()`。

**変更**: `TextField::multiline(bool)`、`FileDialog::multiple(bool)`、`accessibility_hidden(bool)`、`title_bar_interactive(bool)`、`ignores_safe_area(bool)`。

## 8.4 `Slider` の目盛り

**現状**: `ticks(count)` と `snapping_ticks(count)` の 2 メソッド。native は `tick_marks(count, snaps: bool)`。

**仕様**: `ticks(count: usize)` と `snaps_to_ticks(bool)` に分けます。`snapping_ticks` と `tick_marks` は削除します。
`snaps_to_ticks(true)` で `ticks` が 0 の場合は何もしません。

## 8.5 スタックのメソッド名

| 現状 | 変更後 | 理由 |
|---|---|---|
| `equal_width_children()` | `equal_children()` | `VStack` では高さを等分するので名前が合わない |
| `space_between()` | `distribution(MainAxisDistribution)` | 他の配分(`Start` など)を指定する手段が無い |
| `alignment(CrossAxisAlignment)` | そのまま | |
| `spacing(f32)` | そのまま | `Grid::spacing` と同名 |

`MainAxisDistribution` のバリアントは実装時に確認し、不足があれば `zintl-ui-layout` に足します。

## 8.6 `f32` / `f64` の使い分け

**現状**: レイアウトは `f32`(`LayoutStyle`、`Size`)、描画とイベントは `f64`(`graphics`、`PointerEvent`、`ScrollStatus`)。
`TableColumn::width(f64)` だけがレイアウト値なのに `f64` です。`SplitView` はポインタ位置(`f64`)とペイン幅(`f32`)の間でキャストしています。

**規則**: レイアウトの長さは `f32`、描画座標と入力座標は `f64`。境界でのキャストは受け取る側で行います。

**変更**: `TableColumn::width(f32)`。
`Rect`(`f64`)と `Size`(`f32`)が同じクレート直下に並ぶ点は、ステップ 5 のモジュール分けで `graphics::Rect` とレイアウトの `Size` を区別しやすくします。

## 8.7 `Grid` の縞と列結合

**現状**: 縞の行数を `children.len().div_ceil(columns)` で求めています。`GridCell::column_span` を使うと行数がずれます(doc コメントに制限として記載)。

**仕様**: 各子の `column_span` を見て行を数えます。

- `Grid::new` は `ListFactory<RenderNode>` を受け取るので、子から `column_span` を取り出せません。
- 対応案 A: `Grid` に `rows(impl IntoIterator<Item = GridRow>)` 形式のコンストラクタを足し、行単位で受け取る。行数が自明になります。
- 対応案 B: 縞の描画をレイアウト後の実際の行位置に基づいて行う(Canvas を 1 枚、グリッド全体の背面に敷く)。レイアウト結果を Canvas に渡す仕組みが要ります。
- 推奨は A です。表形式のデータは行単位で組み立てることが多く、API としても自然です。

## 8.8 `TextField::multiline` と `TextEditor`

**現状**: 複数行テキストの入口が 2 つあります。

| | 値の型 | 用途 |
|---|---|---|
| `TextField::multiline()` | `String` | 折り返すテキストフィールド |
| `TextEditor` | `AttributedText` | スクロールするリッチテキスト |

**仕様**: 型は分けたまま、doc で使い分けを明記します。

- `TextField` の doc: 「数行のプレーンテキスト。スクロールしない」
- `TextEditor` の doc: 「文書本文。スクロールし、書式を持つ」

`TextEditor` にプレーンテキスト用の `bind_string(Store<String>)` を足すかは未決事項 3。

## テスト

| 項目 | 確認すること |
|---|---|
| 8.1 | `position` 省略時にノードの `position` が `None` になること。`size` がノードに届くこと |
| 8.2 | `Button::icon(..)` のノードが、従来の `Button::new("").system_image(..)` と同じであること |
| 8.3 | `multiline(false)` が単一行になること(他は型で担保) |
| 8.4 | `ticks(5).snaps_to_ticks(true)` が `tick_marks: 5, snaps_to_ticks: true` になること |
| 8.5 | `distribution(SpaceBetween)` が従来の `space_between()` と同じレイアウトになること |
| 8.7 | 列結合を含むグリッドで、縞が正しい行に付くこと |

## 未決事項

1. `Window` の既定サイズ。
2. アクセシビリティラベルの無いアイコンボタンを、デバッグビルドで警告するか。警告の出し方(`eprintln!` か、専用のフック)も含めて決める必要があります。
3. `TextEditor` にプレーンテキストのバインドを足すか。
4. `Button::system_image` の引数は SF Symbols の名前です。クロスプラットフォーム評価の対象外としていますが、将来 `Symbol` 型に置き換える場合、`SidebarItem::system_image` と `ToolbarItem::button` も同時に変わります。
