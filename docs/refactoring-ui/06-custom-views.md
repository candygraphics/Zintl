# ステップ 6: 利用者定義ビューと拡張方針

## 目的

- 利用者が合成で作ったビューにも、組み込みと同じモディファイア(`id`、レイアウト、`context_menu`、`accessibility_*`)を使えるようにします。
- ネイティブコントロールの追加に関する方針を明文化します。
- 合成ビューが色と文字幅を直書きしなくて済むようにします。

## 現状

### モディファイア

- `Attach` / `Layout` / `Control` は `NativeView` にだけブランケット実装されています。
- `View` だけを実装した型は対象外です。組み込みでも `SplitView` がこれに当たり、`id` も `context_menu` も `accessibility_label` も使えません。
- `NativeView`、`ViewModifiers`、`modifiers!` はすでに `render` モジュールから公開されています。
- `View` を実装する `native_view!` マクロは native の非公開マクロです。外部クレートは `View::render` を手書きする必要があります。
- `native_element` は、ルートが `RenderNode::View` でないときに `id` / `enabled` / レイアウトが指定されていれば `debug_assert!` に掛けます。`help` と `accessibility` は黙って捨てられます。
- `context_menu` を内側と外側の両方に付けると、`RenderNode::ContextMenu` の子が 2 つ付きます。どちらが効くかはバックエンドの処理順次第です。

### 拡張点

- `ViewKind`(ネイティブコントロールの種類)と `Event`(32 バリアント)は `backend-core` の閉じた enum です。
- 外部クレートが使える拡張手段は、既存ビューの合成と `Canvas2D` / `WgpuCanvas` です。
- `Event` にはウィジェット専用のバリアントが多く、同じ意味のものが分かれています。
  - 選択: `SelectionChanged` `ListSelectionChanged` `SidebarSelectionChanged`
  - テキスト: `TextChanged` `ToolbarSearchChanged` `SidebarSearchChanged`

### 直書きの値

| 場所 | 値 |
|---|---|
| `Grid` の縞(`arrangements.rs` の `Stripe`) | `Color::rgba(0.5, 0.5, 0.5, 0.1)` |
| `SplitView` の区切り線(`Divider`) | `Color::rgba(0.5, 0.5, 0.5, 0.35)` |
| `BarChart::text_width` | 1 文字 7.5pt(全角は 13pt)+ 8 |
| `TextField::resize_label` | 1 文字 `font_size * 0.54` |
| `Toggle` の Button スタイル | 1 文字 7.0pt + 32 |
| `Button::checkbox` / `Button::radio` | 1 文字 7.0pt + 24 |
| `PopUpButton::new` | 最長項目 1 文字 7.0pt + 44 |
| `SegmentedControl::new` | 各項目 1 文字 7.0pt + 24 |

`Palette` が持つ色は `accent` と 6 色のシステムカラーだけで、区切り線や交互行の背景がありません。
AppKit は `zpd-appkit` の `CanvasPalette::resolve` でビューの外観に合わせて色を解決し、WinUI は `Palette::default()`(= `Palette::LIGHT`)をそのまま渡しています。

## 仕様

### 1. 合成ビューを `NativeView` にする手順を公開する

「ルートが 1 つのネイティブ要素になる合成ビュー」は、次の形で `NativeView` を実装できます。

```rust
use zintl_ui_desktop::render::{NativeView, ViewModifiers, modifiers, native_view};

#[derive(Clone)]
pub struct Card {
    title: String,
    modifiers: ViewModifiers,
}

impl NativeView for Card {
    modifiers!();

    fn base_element(&self, cx: &mut Context<'_>) -> Element<RenderNode> {
        VStack::new(list![Text::new(self.title.clone())])
            .padding(12.0)
            .native_element(cx)
    }
}

native_view!(Card);   // View を実装する
```

必要な変更:

- `native_view!` を `#[macro_export]` にし、`render` モジュールから再公開する。展開先は `$crate::View` と `$crate::IntoElement` を使うので、native に `pub use zintl_ui::view::View;` を足す。
  - 対象は型パラメータの無い型だけです。ジェネリックな型は `View` を手書きします(組み込みの `List<K>` などと同じ)。
- モディファイアの重ね順を仕様として固定する。外側(`Card` に付けたもの)が、内側(ルートの `VStack` に付けたもの)に勝つ。
  - `id`、`enabled`、レイアウト各項目、`help`: 外側が指定していれば置き換え(現状どおり)
  - `title_bar_interactive`: どちらかが指定していれば有効(現状どおり)
  - `accessibility`: 既存の `merge` と同じく、外側が指定した項目だけ置き換え(現状どおり)
  - `context_menu`: 外側が指定すれば置き換え。`ViewModifiers::attach` で外側のメニューを付ける前に、要素の子から `RenderNode::ContextMenu` のノードを取り除く(**変更**)
  - `popovers`: 両方を残す。内側が先(現状どおり)
- `base_element` が `RenderNode::View` のノード以外(`cx.watch` の結果など)を返した場合、ルートに書き込むモディファイアは適用できません。
  - `targets_root` に `help`、`accessibility`、`title_bar_interactive` を加え、これらも `debug_assert!` の対象にする(**変更**)。
  - `NativeView` の doc にこの条件を書く。

### 2. `init` を持つ合成ビュー

状態を持つ合成ビュー(`DisclosureGroup`、`SplitView`)は `View::init` で Store を作ります。
`native_element` を親が直接呼ぶと `init` を通らないため、次を規約にします。

- `init` を持つ型は、必ず子リスト(`list!`)や `content(..)` 経由でマウントする。他のビューの `base_element` の中で `native_element` を直接呼ばない。
- `NativeView` の doc にこの条件を書く。違反するとステップ 7 の型名入りパニック(`Store<f32> was used before View::init initialized it`)になり、`#[track_caller]` で呼び出し位置が分かります。
- `init` を持つ型は `native_view!` を使わず、`View` を手書きして `init` と `render`(`self.native_element(cx)` を返す)を実装する。

`SplitView` に `modifiers: ViewModifiers` を持たせ、`NativeView` を実装します。ルートは `HStack` / `VStack` なので条件を満たします。
ステップ 7 で `init` を持つ型は増えません(ステップ 7 の「見送ったこと」)。

### 3. ネイティブコントロール追加の方針

`ui/zintl-ui-desktop/README.md` に次を書きます。

- 外部クレートは `ViewKind` を追加できない。新しいネイティブコントロールは `backend-core` と各バックエンドへの変更として追加する。
- 外部クレートの拡張手段は、合成(本ステップの手順 1)、`Canvas2D`、`WgpuCanvas`。Canvas からアプリへの通知は `CanvasEventContext::emit` と `on_action` を使う。

`Event` の統合は行いません(「見送ったこと」)。

### 4. テーマ

`Palette` と `PaletteColor` に意味のある色を 2 つ追加し、Canvas の `Frame::palette` から引きます。

```rust
pub struct Palette {
    pub accent: Color,
    // ...システムカラー 6 色...
    /// 区切り線と罫線。
    pub separator: Color,
    /// 一覧の交互行の背景。どの背景にも重ねられる半透明色。
    pub alternating_row: Color,
}

pub enum PaletteColor {
    Accent,
    System(SystemColor),
    Separator,
    AlternatingRow,
    Custom(Color),
}
```

| 色 | 用途 | AppKit | `Palette::LIGHT`(WinUI とヘッドレス) |
|---|---|---|---|
| `separator` | `SplitView` の区切り線 | `NSColor.separatorColor` | `Color::rgba(0.5, 0.5, 0.5, 0.35)`(現在の直書き値) |
| `alternating_row` | `Grid` の縞 | `NSColor.quaternaryLabelColor` | `Color::rgba(0.5, 0.5, 0.5, 0.1)`(現在の直書き値) |

- `alternating_row` に `NSColor.alternatingContentBackgroundColors[1]` は使いません。これは不透明色で、表の白い背景を前提にしています。ウィンドウ背景の上に置く `Grid` では、縞が周囲より明るくなってしまいます。
- `LIGHT` の値は現在の直書き値と同じにします。これで WinUI とヘッドレスの見た目は変わりません。
- WinUI の外観に合わせた色の解決は、他のシステムカラーと同じく後回しにします(クロスプラットフォーム評価は対象外)。

### 5. 文字幅の見積もり

見積もりを `zintl-ui-native` の 1 か所にまとめます。

```rust
/// バックエンドに問い合わせずに求める、テキストのおおよその大きさ。
///
/// 実測ではありません。レイアウトが最初に確保する最小サイズに使います。
pub struct TextEstimate;

impl TextEstimate {
    /// システムフォントの標準サイズ(pt)。
    pub const SYSTEM_FONT_SIZE: f32 = 13.0;

    /// 1 行で表示したときの幅。全角・East Asian Wide の文字は 1 em、それ以外は 0.54 em。
    pub fn width(text: &str, font_size: f32) -> f32;

    /// 1 行の高さ(1.25 em)。
    pub fn line_height(font_size: f32) -> f32;
}
```

- 返り値はレイアウトの長さなので `f32` です(ステップ 8.6 の規則)。
- フォントの太さや書体は見積もりに影響しないので、引数はサイズだけにします。
- 全角判定は `BarChart::is_wide` を移します。
- 余白(+24、+32 など)はコントロールごとの値なので、呼び出し側に残します。
- 置き換え先は「直書きの値」の表の 6 か所すべてです。ASCII の幅は 7.0pt / 7.5pt から 7.02pt にそろい、ボタンやポップアップの全角文字は、7pt ではなく 1 em で見積もられるようになります。
- `render` モジュールから公開し、合成ビューも使えるようにします。
- `RecordingGraphics::measure_text`(graphics クレート、テスト用の記録)は描画座標の `f64` で別の役割なので、置き換えません。

## プラットフォームコード

| 場所 | 変更 | 段階 |
|---|---|---|
| `platform/zpd-appkit/src/ui/canvas.rs` | `CanvasPalette` に `separator` と `alternating_row` を足し、`resolve` で `separatorColor` と `quaternaryLabelColor` を引く | 4 |
| `ui/zintl-ui-appkit-backend/src/convert.rs` | `CanvasPalette` → `Palette` の変換に 2 色を足す | 4 |
| `ui/zintl-ui-backend-core/src/canvas.rs` | `Palette` と `PaletteColor` に 2 色、`Palette::LIGHT` に既定値、`Palette::resolve` に分岐 | 4 |
| `ui/zintl-ui-winui3-backend` | 変更なし。`Palette::default()` を渡しているので新しい色も `LIGHT` の値になる | — |
| `platform/` の WinUI 側(`zpd-winui3`、`ZintlWinUI3Support`) | 変更なし | — |

段階 1〜3 はバックエンドにも `platform/` にも触れません。

`separatorColor` と `quaternaryLabelColor` は `resolve` 内で `NSAppearance.currentAppearance` を切り替えた状態で引くので、既存の色と同じくダークモードと「コントラストを上げる」に追従します。

実装後、`tests/ui` で次を実機確認します(macOS のみ)。

- `split_view` シナリオ: 区切り線が、ライト・ダークの両方で周囲になじむこと。
- `grid_variants` シナリオ: 縞が、ライト・ダークの両方で行の区別に十分で、強すぎないこと。強すぎる場合は AppKit 側の解決だけを変えます(API は変えない)。

## 段階と PR

各段階を 1 PR にします。

1. **`SplitView` の `NativeView` 化と重ね順**: `SplitView` に `ViewModifiers` を持たせる。`context_menu` の置き換えと `targets_root` の拡張を入れる。重ね順のテストを書く。
2. **合成ビューの公開手順**: `native_view!` を公開する。`render` モジュールと `NativeView` の doc に、合成ビューの書き方と条件(ルートが 1 つの `RenderNode::View`、`init` を持つ型の扱い)を書く。`ui/zintl-ui-desktop/examples/custom_view.rs` と README の拡張方針を追加する。
3. **`TextEstimate`**: 型を追加し、6 か所を置き換える。
4. **`Palette` の 2 色**: 上表のプラットフォームコードと、`Stripe` / `Divider` の置き換え。

ステップ 7 との順序は README の「推奨順序」を参照してください。

## テスト

段階 1・2:

- 合成ビューに付けた `id` / `width` / `accessibility_label` / `help` が、ルートノードに届くこと。
- 内側と外側の両方に `id` を付けたとき、外側が勝つこと。
- 内側と外側の両方に `context_menu` を付けたとき、`RenderNode::ContextMenu` の子が 1 つだけで、外側の項目であること。
- 内側と外側の両方に `popover` を付けたとき、両方が子として残り、内側が先であること。
- `SplitView::horizontal(..).id("split").width(400.0)` がルートノードの ID と幅になること。
- `native_view!` で実装した型が `Attach` / `Layout` / `Control` を使えること(`custom_view.rs` のコンパイルと、テスト内の小さな型)。

段階 3:

- `TextEstimate::width` で、全角文字が 1 em、ASCII が 0.54 em になること。
- `BarChart` の全角ラベルが ASCII ラベルより広いこと(既存テストの移行)。
- `Picker` の Menu スタイルで、全角の項目を持つときの最小幅が、同じ文字数の ASCII 項目より広いこと。

段階 4:

- `Palette::resolve(PaletteColor::Separator)` と `AlternatingRow` が、それぞれのフィールドを返すこと。
- `Palette::LIGHT` の 2 色が、置き換え前の直書き値と等しいこと(WinUI とヘッドレスの見た目が変わらない保証)。

## 見送ったこと

| 項目 | 理由 |
|---|---|
| `Event` の統合(選択系 3 つ、テキスト系 3 つ) | サイドバーとツールバーはウィンドウノードに付属し、イベントの宛先がウィンドウになる。統合には送信元を区別する `source` フィールドが要り、両バックエンドのイベント送出を書き換える必要がある。利用者が `Event` を直接扱うのはカスタムビューの一部だけで、効果に対して変更が大きい。 |
| 実測ベースの文字幅 API(`Context` 経由でバックエンドに問い合わせる形) | 実測はすでにレイアウト時に行われている(`LayoutNative::measure_wrapping`。折り返すラベルの高さを AppKit / WinUI が測る)。ビュー構築時に問い合わせる API を足すと、ヘッドレスとの差や再描画のたびの問い合わせが増える。実測を広げるなら、ラベルとボタンの幅もレイアウト時に測る方向が筋が良い。`TODO.md` に記録します。 |
