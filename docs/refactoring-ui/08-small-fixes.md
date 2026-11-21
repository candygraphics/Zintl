# ステップ 8: 細かい不揃いの修正

## 目的

コンストラクタ、真偽値メソッド、数値型、命名の小さな不揃いをまとめて直します。
項目は互いに独立しているので、1 項目ずつ別の PR にできます。

## 項目一覧

| # | 項目 | バックエンド変更 | `platform/` の変更 |
|---|---|---|---|
| 8.1 | `Window` のコンストラクタ | 両バックエンド | `zpd-appkit`、`zpd-winui3`、`ZintlWinUI3Support` |
| 8.2 | アイコンのみのボタンとヘルプボタン | なし | なし |
| 8.3 | 真偽値メソッドの形 | なし | なし |
| 8.4 | `Slider` の目盛り | なし | なし |
| 8.5 | スタックのメソッド名 | なし | なし |
| 8.6 | `f32` / `f64` の使い分け | 両バックエンドの変換のみ | なし |
| 8.7 | `Grid` の行と縞 | なし | なし |
| 8.8 | `TextField::multiline` と `TextEditor` の関係 | なし | なし |

## 8.1 `Window` のコンストラクタ

**現状**:

- `Window::new(bounds: Rect, title)`。位置が必須で、サンプルはすべて `Rect::new(100.0, 100.0, w, h)` のような決め打ちです。`autosave` で復元する場合も初期位置を書く必要があります。
- `bounds` の意味がバックエンドで食い違っています。AppKit(`zpd-appkit` の `Window::set_bounds`)はタイトルバーを含むフレームとして `setFrame:display:` に渡します。WinUI(`zpd_winui3_window_move_and_resize`)は内容領域として `AppWindow::ResizeClient` に渡します。

**仕様**:

```rust
Window::new("Zintl")                    // 既定サイズ、画面中央
    .size(Size::new(640.0, 400.0))      // 内容領域の大きさ
    .position(Point::new(100.0, 100.0)) // 省略時は中央
```

- `size` は**内容領域**(タイトルバーを除く)の大きさです。レイアウトが受け取る大きさと一致し、WinUI の現在の解釈とも同じです。型はレイアウトの `Size`(`f32`)。
- `position` はウィンドウ枠の左上の画面座標(左上原点)です。型は `graphics::Point`(`f64`)。`zintl_ui_desktop` のルートから `Point` を `Rect` と並べて再公開します。
- 既定サイズ: `Window::new` は 640 × 480、`Window::panel(title)` は 320 × 400。パネルはインスペクタやパレットのような補助ウィンドウなので、縦長で小さくします。
- `Window::panel(title)` も同じ形にします。
- `RenderNode::Window::bounds: Rect` を `size: Size` と `position: Option<Point>` に分けます。

バックエンドの挙動:

| 状況 | 挙動 |
|---|---|
| 新しいウィンドウ | 内容領域を `size` にする → `position` があればそこへ、無ければ画面中央へ → `autosave` の保存済みフレームがあれば、それで上書き(従来どおり保存済みが優先) |
| `size` が変わった | 内容領域だけを変える。左上は動かさない |
| `position` が `Some` で変わった | そこへ動かす |
| `position` が `Some` から `None` に変わった | 動かさない(中央寄せは新しいウィンドウのときだけ) |
| タイトルバーの形が変わった(AppKit) | 内容領域を `size` に合わせ直す。左上は動かさない |

`size` と `position` は別々に「最後に適用した値」を記録し、変わったものだけを適用します(現在の `applied_bounds` を分ける)。

## 8.2 アイコンのみのボタンとヘルプボタン

**現状**:

- `Button::new("").system_image("gearshape")`。空文字列のタイトルが「アイコンのみ」の合図になっています。
- アクセシビリティラベルは別途 `accessibility_label` で付ける必要があり、付け忘れても気付けません。
- ヘルプボタンは `Button::new("").style(ButtonStyle::Help)` です。ヘルプのベゼルはタイトルを持てないので、空文字列を渡すしかありません。
- `DisclosureGroup` の開閉ボタンも `Button::new("")` です(シンボルなし、ベゼルは `Disclosure`)。

**仕様**:

```rust
impl Button {
    /// シンボルだけを表示するボタン。`label` は VoiceOver が読む名前。
    pub fn icon(system_image: impl Into<String>, label: impl Into<String>) -> Self;

    /// 丸い「?」のヘルプボタン。
    pub fn help() -> Self;
}
```

- `Button::icon` はアクセシビリティラベルを必須の引数にします。型で付け忘れを防ぐので、デバッグビルドでの警告は設けません。
  - `label` は `modifiers.accessibility.label` の初期値として入れます。後から `accessibility_label(..)` を呼べば、そちらで上書きされます。
- `Button::help` を追加し、`ButtonStyle::Help` を削除します。ヘルプのベゼルを他のコンストラクタと組み合わせる意味は無いためです。ラベルは付けません(AppKit のヘルプボタンは「ヘルプ」と読まれる)。
- 「アイコンのみ」の判定(`title.is_empty() && system_image.is_some()` で最小サイズをバックエンドの計測に任せる処理)は、`Button::icon` で作られたかどうかのフラグ(`icon_only: bool`)に置き換えます。
  - `Button::new("").system_image(..)` は、通常の押しボタンの大きさになります。
- `Button::system_image` の doc を「タイトルの横にシンボルを表示する」に改めます。
- `DisclosureGroup` 内部は、crate 内の `Button::untitled()`(`pub(crate)`)を使います。
- `DatePicker` 内部のクリアボタン(`controls.rs`)は `Button::icon("xmark.circle.fill", "Clear Date")` にします。

## 8.3 真偽値メソッドの形

**現状**: 引数ありと引数なしが混在しています。

| 引数あり | 引数なし |
|---|---|
| `enabled(bool)` `striped(bool)` `focusable(bool)` `edited(bool)` `selected(bool)` `grouping(bool)` `fit_columns(bool)` `choose_files(bool)` `shows_alpha(bool)` | `multiline()` `multiple()` `accessibility_hidden()` `title_bar_interactive()` `ignores_safe_area()` |

**規則**:

- **状態・機能のオンオフは `bool` を取る。** 条件付きで切り替えるときに `if` でビルダーを分岐させずに済みます。
- **レイアウトの指示は引数なし。** `fill_width()`、`fill_height()`、`grow()`、`ignores_safe_area()`。どれも「どこに、どれだけの大きさで置くか」の指示です。
- **値を組み立てるメソッドは引数なし。** `Shortcut::shift()` などの修飾キーの追加はオンオフではなく組み合わせの指定なので、対象外です。

**変更**: `TextField::multiline(bool)`、`FileDialog::multiple(bool)`、`accessibility_hidden(bool)`、`title_bar_interactive(bool)`。
`ignores_safe_area()`(`HStack` / `VStack` / `Canvas2D`)は引数なしのままにします。

## 8.4 `Slider` の目盛り

**現状**: `ticks(count)` と `snapping_ticks(count)` の 2 メソッドで、`ticks` は吸着をオフに戻します。

**仕様**: `ticks(count: usize)` と `snaps_to_ticks(bool)` に分けます。`snapping_ticks` は削除します。

- 2 つは独立した設定で、呼ぶ順番に依存しません(`ticks` は吸着の設定を変えない)。
- `snaps_to_ticks(true)` で `ticks` が 0 の場合は何もしません。ノードには `snaps_to_ticks: true` のまま出し、AppKit の `allowsTickMarkValuesOnly` に任せます(目盛りが無ければ吸着しない)。

## 8.5 スタックのメソッド名

| 現状 | 変更後 | 理由 |
|---|---|---|
| `equal_width_children()` | `equal_children()` | `VStack` では高さを等分するので名前が合わない |
| `space_between()` | `distribution(MainAxisDistribution)` | 他の配分を指定する手段が無い |
| `alignment(CrossAxisAlignment)` | そのまま | |
| `spacing(f32)` | そのまま | `Grid::spacing` と同名 |

`MainAxisDistribution` は現在 `Start` と `SpaceBetween` だけです。`Center` と `End` を `zintl-ui-layout` に足し、taffy の `JustifyContent::CENTER` / `END` に対応させます。
`Spacer` を両端に置く書き方でも中央寄せはできますが、`distribution` を引数にする以上、主要な配分は選べるようにします。
`SpaceAround` / `SpaceEvenly` は使う箇所が無いので足しません。

## 8.6 `f32` / `f64` の使い分け

**現状**: レイアウトは `f32`(`LayoutStyle`、`Size`)、描画とイベントは `f64`(`graphics`、`PointerEvent`、`ScrollStatus`)。
`TableColumn::width(f64)` と `backend-core` の `TableColumn::width: Option<f64>` だけが、レイアウト値なのに `f64` です。`SplitView` はポインタ位置(`f64`)とペイン幅(`f32`)の間でキャストしています。

**規則**: レイアウトの長さは `f32`、描画座標・入力座標・画面座標は `f64`。境界でのキャストは受け取る側で行います。

**変更**:

- `TableColumn::width(f32)` と `backend-core` の `TableColumn::width: Option<f32>`。AppKit の変換(`convert.rs`)で `f64` にし、WinUI の `column_widths` からはキャストが消えます。
- `SplitView` はステップ 7 で `DividerDrag { delta: f64 }` を受け取る側でキャストする形になります。
- 8.1 の `Window::size`(`Size`、`f32`)と `Window::position`(`Point`、`f64`)もこの規則に従います。

## 8.7 `Grid` の行と縞

**現状**: `Grid::new(columns, children)` は子を平らに受け取り、縞の行数を `children.len().div_ceil(columns)` で求めています。`GridCell::column_span` を使うと行数がずれます(doc コメントに制限として記載)。
`ListFactory<RenderNode>` からは子の `column_span` を取り出せません。

**仕様**: `Grid` を行単位で受け取ります。

```rust
/// Grid の 1 行。
#[derive(Clone)]
pub struct GridRow { .. }

impl GridRow {
    /// 各セルが 1 列を占める行。
    pub fn new(cells: ListFactory<RenderNode>) -> Self;
    /// `index` 番目のセルを `columns` 列にまたがらせる。
    pub fn span(self, index: usize, columns: u16) -> Self;
}

impl Grid {
    pub fn new(rows: impl IntoIterator<Item = GridRow>) -> Self;
}

// 例: 4 列の表の途中に見出し行を挟む
Grid::new([
    GridRow::new(list![Text::new("Food"), bar, Text::new("3480"), Text::new("62%")]),
    GridRow::new(list![Text::new("Transport")]).span(0, 4),
])
```

- `Grid::new(columns, children)` と公開型の `GridCell` は廃止します。列結合は `GridRow::span` で指定します。
  - `GridRow` がセルごとの幅を持つので、`ListFactory` の中身を覗かずに各行の幅が分かります。
  - `span` を指定したセルだけを、現在の `GridCell` と同じく `column_span` 付きのコンテナで包みます。他のセルのノード構造は変わりません。
  - 範囲外の `index` は `debug_assert!` で検出し、リリースビルドでは無視します。
- 列数は、各行の幅(セルの `column_span` の合計)の最大値です。
- 列数より短い行は、空の要素で埋めます。taffy の自動配置が次の行のセルを詰めてくるのを防ぐためです。
- 縞は行の番号(`GridRow` の順番)で奇数行に付けます。セル数から逆算しないので、列結合があってもずれません。
- `Arrangement::Grid { columns }` と `zintl_ui_layout::GridCell` はそのまま使います(レイアウトクレートの変更なし)。

案 B(レイアウト後の実際の行位置に Canvas を敷く)は、レイアウト結果を Canvas に渡す仕組みが要るので採りません。表形式のデータは行単位で組み立てることが多く、行単位の API のほうが自然です。

## 8.8 `TextField::multiline` と `TextEditor`

**現状**: 複数行テキストの入口が 2 つあります。

| | 値の型 | 用途 |
|---|---|---|
| `TextField::multiline(true)` | `String` | 折り返すテキストフィールド |
| `TextEditor` | `AttributedText` | スクロールするリッチテキスト |

**仕様**: 型は分けたまま、doc で使い分けを明記します。

- `TextField` の doc: 「数行のプレーンテキスト。スクロールしない」
- `TextEditor` の doc: 「文書本文。スクロールし、書式を持つ。プレーンテキストとの変換は `AttributedText::from(&str)` と `AttributedText::text()`」

`TextEditor` にプレーンテキスト用のバインドは足しません(「見送ったこと」)。

## プラットフォームコード

8.1 と 8.6 だけがバックエンドに触れ、`platform/` に触れるのは 8.1 だけです。

### 8.1 `Window`

| 場所 | 変更 |
|---|---|
| `ui/zintl-ui-backend-core/src/lib.rs` | `RenderNode::Window` の `bounds: Rect` → `size: Size`、`position: Option<Point>` |
| `platform/zpd-appkit/src/ui/window.rs` | `set_content_size(width, height)` を追加。`frameRectForContentRect:` で枠の大きさを求め、枠の上端を保ったまま `setFrame:display:` する。`center()` を追加(`[NSWindow center]`)。`set_bounds` は呼び出し元が無くなるので削除する |
| `ui/zintl-ui-appkit-backend/src/backend/mod.rs` | `synchronize_window` を上の「バックエンドの挙動」の表どおりに書き換える。`applied_bounds` を `applied_size` と `applied_position` に分け、`window_bounds_need_update` を置き換える |
| `platform/ZintlWinUI3Support/include/zpd_winui3.h`、`src/zpd_winui3.cpp` | `zpd_winui3_window_move_and_resize` を 3 つに分ける: `zpd_winui3_window_resize_client(width, height)`(`AppWindow::ResizeClient`)、`zpd_winui3_window_move(x, y)`(`AppWindow::Move`)、`zpd_winui3_window_center()`(`DisplayArea::GetFromWindowId(app.Id(), DisplayAreaFallback::Nearest)` の `WorkArea` の中央へ `Move`)。どれも DPI 換算は現在と同じ |
| `platform/zpd-winui3/src/ffi.rs`、`src/lib.rs` | 上の 3 関数の宣言とラッパー(`resize_client` / `move_to` / `center`)。`move_and_resize` は削除する |
| `ui/zintl-ui-winui3-backend/src/backend/mod.rs` | AppKit と同じ表どおりに書き換える |
| 両バックエンドの `tests.rs` | `RenderNode::Window` の組み立てを新しいフィールドに直す |

AppKit の `size` の意味がフレームから内容領域に変わるので、同じ数値を渡したウィンドウは、タイトルバーの分(標準で 28pt)だけ縦に大きくなります。`examples/` と `tests/ui` の数値はそのまま移します(内容領域として妥当な大きさのため)。

### 8.6 `TableColumn::width`

| 場所 | 変更 |
|---|---|
| `ui/zintl-ui-backend-core/src/model.rs` | `TableColumn::width: Option<f32>` |
| `ui/zintl-ui-appkit-backend/src/convert.rs` | `width: self.width.map(f64::from)`。`zpd-appkit` の `TableColumn::width` は `CGFloat` なので `f64` のまま |
| `ui/zintl-ui-winui3-backend/src/backend/native.rs` | `column_widths` の `width as f32` を外す |

### 実機確認

実装後、`tests/ui` で次を確認します(macOS のみ。Windows は既存の CI の範囲)。

- `windows` シナリオ: `position` を省略したウィンドウが画面中央に出ること。`size` が内容領域の大きさになること(`TitleBar::Standard` と `Hidden` の両方)。
- `autosave` を付けたウィンドウを動かして再起動すると、保存した位置に出ること。
- `table` シナリオ: 列幅の指定が従来どおり効くこと。

## 段階と PR

1 項目 1 PR にします。8.8 は doc だけなので、同じ `TextField` を触る 8.3 と同じ PR にします。

| PR | 項目 | 備考 |
|---|---|---|
| 1 | 8.1 | `platform/` と両バックエンドを含む。macOS の実機確認が要る |
| 2 | 8.2 | |
| 3 | 8.3 + 8.8 | |
| 4 | 8.4 | |
| 5 | 8.5 | `zintl-ui-layout` に 2 バリアント |
| 6 | 8.6 | 両バックエンドの変換 |
| 7 | 8.7 | |

## テスト

| 項目 | 確認すること |
|---|---|
| 8.1 | `position` 省略時にノードの `position` が `None` になること。`size` がノードに届くこと。`Window::new(title)` と `Window::panel(title)` の既定サイズ。AppKit バックエンドで、`size` だけ変わったときに位置を適用しないこと(`applied_size` / `applied_position` の判定) |
| 8.2 | `Button::icon(..)` のノードが、従来の `Button::new("").system_image(..)` と同じ種類・最小サイズで、ラベルが `accessibility.label` に入ること。`accessibility_label(..)` で上書きできること。`Button::help()` が `BezelStyle::Help` になること |
| 8.3 | `multiline(false)` が単一行になること(他は型で担保) |
| 8.4 | `ticks(5).snaps_to_ticks(true)` と `snaps_to_ticks(true).ticks(5)` が、どちらも `tick_marks: 5, snaps_to_ticks: true` になること |
| 8.5 | `distribution(SpaceBetween)` が従来の `space_between()` と同じレイアウトになること。`Center` / `End` が taffy の対応する配置になること(`zintl-ui-layout` のテスト) |
| 8.6 | `TableColumn::width(120.0)` がノードに `Some(120.0_f32)` として出ること |
| 8.7 | 列結合を含むグリッドで、縞が 2 行目・4 行目に付くこと。短い行の後でも、次の行が 1 列目から始まること |

## 見送ったこと

| 項目 | 理由 |
|---|---|
| アクセシビリティラベルの無いアイコンボタンの警告 | `Button::icon` がラベルを必須にしたので不要。 |
| `TextEditor` のプレーンテキストバインド(`bind_string(Store<String>)`) | 変換は `AttributedText::from` / `text()` で 1 行。本当にプレーンテキスト専用のエディタにするには、貼り付けた書式を落とす設定(AppKit の `NSTextView.isRichText = false`)をバックエンドと `platform/` に足す必要があり、別の機能になる。`TODO.md` に記録します。 |
| `system_image` の引数を `Symbol` 型にする | 引数は SF Symbols の名前で、WinUI バックエンドはすでに名前からグリフへの対応表(`native.rs`)を持っている。文字列のままでも両バックエンドで動く。型を導入するのはクロスプラットフォーム評価の範囲で、そのときは `Button::icon`、`Button::system_image`、`SidebarItem::system_image`、`ToolbarItem::button` を同時に変える。 |
