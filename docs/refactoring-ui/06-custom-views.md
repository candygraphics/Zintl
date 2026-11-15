# ステップ 6: 利用者定義ビューと拡張方針

## 目的

- 利用者が合成で作ったビューにも、組み込みと同じモディファイア(`id`、レイアウト、`context_menu`、`accessibility_*`)を使えるようにします。
- ネイティブコントロールの追加に関する方針を明文化します。
- 合成ビューが色と文字幅を直書きしなくて済むようにします。

## 現状

### モディファイア

- `Attach` は `NativeView` にだけブランケット実装されています。
- `View` だけを実装した型は対象外です。組み込みでも `SplitView` がこれに当たり、`id` も `context_menu` も `accessibility_label` も使えません。
- `NativeView` を実装するには `Attachments` フィールドとアクセサ 2 つが必要で、そのための `attachments!` マクロは native の `pub(crate)` です。

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
| `Toggle` の Button スタイルの幅 | `文字数 * 7.0 + 32.0` |
| `BarChart::text_width` | 独自の文字幅見積もり |
| ラベルのサイズ(`TextField::resize_label`) | 独自の見積もり |

`Palette` が持つ色は `accent` と 6 色のシステムカラーだけで、区切り線や交互行の背景がありません。

## 仕様(提案)

### 1. 合成ビューを `NativeView` にする手順を公開する

「ルートが 1 つのネイティブ要素になる合成ビュー」は、次の形で `NativeView` を実装できます。

```rust
#[derive(Clone)]
pub struct Card {
    title: String,
    modifiers: Modifiers,
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

- `Modifiers`、`modifiers!`、`native_view!` を `render` モジュールから公開する(ステップ 1・5)。
- モディファイアの重ね順を仕様として固定する。外側(`Card` に付けたもの)が、内側(ルートの `VStack` に付けたもの)に勝つ。
  - `id`、`enabled`、レイアウト各項目、`help`: 外側が `Some` なら置き換え
  - `accessibility`: 既存の `merge` と同じく、外側が指定した項目だけ置き換え
  - `context_menu`: 外側が指定すれば置き換え
  - `popovers`: 両方を残す(内側が先)
- `base_element` が `RenderNode::View` のノード以外(`cx.watch` の結果など)を返した場合、モディファイアは適用できません。デバッグビルドで `debug_assert!` に掛け、doc に条件を書きます。

### 2. `init` を持つ合成ビュー

状態を持つ合成ビュー(`DisclosureGroup`、`SplitView`)は `View::init` で Store を作ります。
`native_element` を親が直接呼ぶと `init` を通らないため、次を規約にします。

- `init` を持つ型は、必ず子リスト(`list!`)や `content(..)` 経由でマウントする。他のビューの `base_element` の中で `native_element` を直接呼ばない。
- `NativeView` の doc にこの条件を書く。

`SplitView` に `Modifiers` を持たせ、`NativeView` を実装します。ルートは `HStack` / `VStack` なので条件を満たします。

### 3. ネイティブコントロール追加の方針

当面の方針として次を README に書きます。

- 外部クレートは `ViewKind` を追加できない。新しいネイティブコントロールは `backend-core` と各バックエンドへの変更として追加する。
- 外部クレートの拡張手段は、合成、`Canvas2D`、`WgpuCanvas`。Canvas からアプリへの通知は `CanvasEventContext::emit` と `on_action` を使う。

`Event` の整理(任意、バックエンド変更あり):

| 統合先 | 統合するもの |
|---|---|
| `SelectionChanged { id }` と `IndexSelected { index }` の 2 つ | `SelectionChanged` `ListSelectionChanged` `SidebarSelectionChanged` |
| `TextChanged { value }` | `ToolbarSearchChanged` `SidebarSearchChanged` |

サイドバーとツールバーはウィンドウノードに付属していて、イベントの宛先がウィンドウになる点が統合の障害です。
送信元を区別するフィールド(`source`)を足す必要があり、効果に対して変更が大きいので、本ステップでは見送りを推奨します。

### 4. テーマと文字幅

**色**: `Palette` に意味のある色を追加し、Canvas の `Frame::palette` から引きます。

| 追加する色 | 用途 | AppKit での対応 |
|---|---|---|
| `separator` | `SplitView` の区切り線、罫線 | `NSColor.separatorColor` |
| `alternating_row` | `Grid` の縞 | `NSColor.alternatingContentBackgroundColors[1]` |

`PaletteColor` にも `Separator` と `AlternatingRow` を追加します。`Palette` を埋めるのはバックエンドなので、両バックエンドに小さな変更が入ります。

**文字幅**: 見積もり関数を 1 か所にまとめます。

```rust
/// バックエンドに問い合わせずに求める、テキストのおおよその大きさ。
pub struct TextEstimate;

impl TextEstimate {
    pub fn width(text: &str, font: &Font) -> f32;
    pub fn size(text: &str, font: &Font, wrap_width: Option<f32>) -> Size;
}
```

- `BarChart::text_width`、`TextField::resize_label`、`Toggle` の幅計算をこれに置き換えます。
- 実測ではなく見積もりであることを名前と doc で明示します。実測 API(バックエンドへの問い合わせ)は別課題です。

## 手順

1. `SplitView` を `NativeView` 化し、重ね順のテストを書く(ステップ 1 の直後に可能)。
2. `render` モジュールの doc に、合成ビューの書き方と条件を載せる。`examples/` に `custom_view.rs` を追加する。
3. `TextEstimate` を導入し、3 か所を置き換える。
4. `Palette` に 2 色を追加し、`Stripe` と `Divider` を置き換える。

## テスト

- 合成ビューに付けた `id` / `width` / `accessibility_label` が、ルートノードに届くこと。
- 内側と外側の両方に `id` を付けたとき、外側が勝つこと。
- 内側と外側の両方に `popover` を付けたとき、両方が子として残ること。
- `SplitView::horizontal(..).id("split")` がルートノードの ID になること。
- `TextEstimate` 置き換え前後で、`BarChart` の列幅とラベルの最小サイズが変わらないこと(既存テストで担保)。

## 未決事項

1. `Event` の統合を行うか(推奨は見送り)。
2. `Palette` への色追加を本ステップに含めるか。クロスプラットフォーム評価を対象外としているため、WinUI 側の対応色の選定は後回しでも構いません。
3. 実測ベースの文字幅 API を将来用意するか。用意する場合、`Context` 経由でバックエンドに問い合わせる形になり、ヘッドレス時の扱いを決める必要があります。
