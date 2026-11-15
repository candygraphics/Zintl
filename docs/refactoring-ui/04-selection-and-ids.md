# ステップ 4: 選択モデルと ID の型

## 目的

- `Picker` と `TabView` の選択を、位置(インデックス)ではなく値で表せるようにします。
- ビュー ID を素の `String` から専用の型にし、取り違えを防ぎます。

## 現状

| 型 | 選択の表現 |
|---|---|
| `Picker` | `Store<usize>`(選択肢のインデックス) |
| `TabView` | `Store<usize>` |
| `List` `Table` `Sidebar` | `Store<Option<String>>`(行・項目の ID) |

- `Picker` の選択肢が動的に増減すると、同じインデックスが別の選択肢を指します。
- 利用側は `match index { 0 => Size::Small, .. }` のような変換を毎回書くことになります。
- `ViewKind::PopUpButton::selected` と `SegmentedControl::selected` はすでに `Option<usize>` で、「未選択」を表せます。

素の `String` が ID として使われている箇所:

- ビュー ID: `id(..)`、`accessibility_labeled_by(..)`、`ScrollView::scroll_to` の対象
- 行 ID: `ListRow::new(id, ..)`、`TableRow::new(id, ..)`、`SidebarItem::new(id, ..)`
- `ChartBar::id`、`Window::id`

## 仕様(提案)

### `Picker<V>`

```rust
pub struct Picker<V: Clone + PartialEq + 'static = usize> { .. }

impl<V: Clone + PartialEq + 'static> Picker<V> {
    /// 値とラベルの組から作る。
    pub fn new<L: Into<String>>(options: impl IntoIterator<Item = (V, L)>) -> Self;
    pub fn selection(self, value: V) -> Self;
    pub fn bind(self, store: Store<V>) -> Self;
    pub fn on_change(self, action: impl for<'a> Fn(&mut Context<'a>, &V) + 'static) -> Self;
    pub fn style(self, style: PickerStyle) -> Self;
}

impl Picker<usize> {
    /// ラベルだけから作り、インデックスで選択する。
    pub fn indexed<L: Into<String>>(labels: impl IntoIterator<Item = L>) -> Self;
}
```

挙動:

- 現在値が選択肢に無い場合は未選択として描画する(`selected: None`)。Radio スタイルではどのボタンもオンにしない。
- 同じ値が複数ある場合は最初のものを選択扱いにする。デバッグビルドでは `debug_assert!` で重複を検出する。
- イベントはバックエンドからインデックスで届くので、`base_element` 内で値へ変換する。範囲外のインデックスは無視する。
- `on_change` は `&V` を受け取る(ステップ 3 の規則 3 は `Copy` 値を値渡しとするが、`V` は `Clone` しか要求しないため参照にする)。

### `TabView<V>`

```rust
pub struct Tab<V = usize> { .. }

impl<V> Tab<V> {
    pub fn new<C>(value: V, title: impl Into<String>, content: C) -> Self
    where C: Clone + IntoElement<Output = RenderNode> + 'static;
}

pub struct TabView<V: Clone + PartialEq + 'static = usize> { .. }
// new / selection / bind / on_change は Picker<V> と同じ形
```

`Picker::indexed` に相当する簡易コンストラクタは設けません。タブは数が少なく固定的で、値を付ける負担が小さいためです。

### `ViewId`

```rust
/// ビューを識別する名前。アクセシビリティ、UI テスト、スクロール先の指定に使う。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ViewId(String);

impl From<&str> for ViewId { .. }
impl From<String> for ViewId { .. }
impl ViewId { pub fn as_str(&self) -> &str; }
```

- `id(impl Into<ViewId>)`、`accessibility_labeled_by(impl Into<ViewId>)`、`scroll_to(Trigger<ViewId>, ..)` に使います。
- 文字列リテラルはそのまま渡せるので、呼び出し側の変更はほぼありません。
- 定数として共有できるよう `ViewId::from_static(&'static str)` を `const fn` で用意するかは未決事項 2。

行 ID(`ListRow`、`TableRow`、`SidebarItem`)は本ステップでは `String` のままにします。
これらはビュー ID と名前空間が異なり、選択の `Store<Option<String>>` と対になっているためです(未決事項 1)。

## 影響範囲

- `Picker` / `TabView` を使う全コード。インデックスのまま使いたい箇所は `Picker::indexed` に置き換えるだけです。
- `ViewId` を `RenderNode::View::id` にまで通すかどうかでバックエンドの変更量が変わります。
  - **A(推奨)**: `RenderNode` は `Option<String>` のまま。native 側で `ViewId` → `String` に変換する。バックエンド変更なし。
  - **B**: `backend-core` に `ViewId` を置き、`RenderNode`、`Accessibility::labeled_by`、`ScrollTarget::id` を置き換える。両バックエンドの `convert` 層を修正。

## テスト

- `Picker`: 値で選択した結果が正しいインデックスとしてノードに出ること(3 スタイルとも)。
- `Picker`: 現在値が選択肢に無いとき `selected: None` になること。
- `Picker`: 選択肢を並べ替えても、Store の値に対応する選択肢が選ばれること。
- `Picker`: インデックス 1 の選択イベントで、Store に 2 番目の値が書かれ、`on_change` がその値を受け取ること。
- `Picker::indexed` が従来と同じ挙動であること(既存テストの移行)。
- `TabView`: 上記と同じ 4 点。

## 未決事項

1. 行 ID と選択を `List<K>` / `Table<K>` / `Sidebar<K>` のようにジェネリックにするか。バックエンドは文字列 ID でイベントを返すので、native 側に ID ↔ 値の対応表を持つ必要があります。効果は大きいですが変更量も大きいため、別ステップに切り出す想定です。
2. `ViewId::from_static` を用意するか。
3. `ChartBar::id` と `Window::id` を `ViewId` にするか。`ChartBar::id` はアクセシビリティ要素の ID なので `ViewId` が妥当です。
