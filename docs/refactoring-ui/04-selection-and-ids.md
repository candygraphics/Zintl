# ステップ 4: 選択モデルと ID の型

## 目的

- `Picker` と `TabView` の選択を、位置(インデックス)ではなく値で表せるようにします。
- `List` / `Table` / `Sidebar` の選択を、文字列ではなく利用側の型で表せるようにします。
- ビュー ID を素の `String` から専用の型にし、取り違えを防ぎます。

## 現状

| 型 | 選択の表現 |
|---|---|
| `Picker` | `Store<usize>`(選択肢のインデックス) |
| `TabView` | `Store<usize>` |
| `List` `Table` `Sidebar` | `Store<Option<String>>`(行・項目の ID) |

- `Picker` の選択肢が動的に増減すると、同じインデックスが別の選択肢を指します。
- 利用側は `match index { 0 => Size::Small, .. }` のような変換を毎回書くことになります。
- `List` などは、利用側の ID 型(`FolderId` など)を毎回文字列へ変換・逆変換しています(`examples/notes`)。
- `ViewKind::PopUpButton::selected` と `SegmentedControl::selected` はすでに `Option<usize>` で、「未選択」を表せます。`ViewKind::TabView::selected` は `usize` です。

素の `String` が ID として使われている箇所:

| 箇所 | 実体 | 本ステップ後 |
|---|---|---|
| `id(..)`、`accessibility_labeled_by(..)`、`ScrollView::scroll_to` の対象 | アクセシビリティ識別子 | `ViewId` |
| `Window::id` | ウィンドウのアクセシビリティ識別子(`set_identifier`) | `ViewId` |
| `ChartBar::id` | バーのアクセシビリティ要素の識別子 | `ViewId` |
| `ToolbarItem::id`(検索欄のみ) | 検索欄のアクセシビリティ識別子 | `ViewId` |
| `ListRow` / `TableRow` / `SidebarItem` の ID | 選択・イベントで返る行キー | `ItemKey` を実装した型 `K` |
| `ToolbarItem::button(id, ..)`、`MenuItem::id` | クリック時にイベントで返るコマンド ID | `String` のまま |

コマンド ID はビューを指す名前ではなく、イベントで返る値なので `ViewId` にしません。

## 仕様

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

内部表現:

```rust
pub struct Picker<V> {
    options: Vec<(V, String)>,
    style: PickerStyle,
    store: Option<Store<V>>,
    selection: Option<V>,          // selection(..) で渡した値
    action: Option<Rc<Change<V>>>,
    modifiers: ViewModifiers,
}
```

挙動:

- 表示する値は `bind` > `selection` の順に決まる(ステップ 3 規則 2)。どちらも無い場合は先頭の選択肢を選択扱いにする(現状の `selected: 0` 既定と同じ)。
- 現在値が選択肢に無い場合は未選択として描画する(`selected: None`)。Radio スタイルではどのボタンもオンにしない。
- 同じ値が複数ある場合は最初のものを選択扱いにする。デバッグビルドでは `debug_assert!` で重複を検出する(`V` は `Hash` を要求しないので O(n²) の比較。選択肢は少ない前提)。
- イベントはバックエンドからインデックスで届くので、`base_element` 内で値へ変換する。範囲外のインデックスは無視する。
- `on_change` は `&V` を受け取る(ステップ 3 の規則 3 は `Copy` 値を値渡しとするが、`V` は `Clone` しか要求しないため参照にする)。`Picker<usize>` も同じ形で、`|cx, index: &usize|` と書く。
- 内部の `PopUpButton` / `SegmentedControl` はインデックスのまま(`pub(crate)`)。`Picker` が `ChangeRoute<V>` を受け取り、インデックス → 値に変換して `send` するクロージャを `ChangeRoute<usize>` として渡す。Radio 分岐も同じ変換を使う。

### `TabView<V>`

```rust
pub struct Tab<V = usize> { .. }

impl<V> Tab<V> {
    pub fn new<C>(value: V, title: impl Into<String>, content: C) -> Self
    where C: Clone + IntoElement<Output = RenderNode> + 'static;
}

pub struct TabView<V: Clone + PartialEq + 'static = usize> { .. }

impl<V: Clone + PartialEq + 'static> TabView<V> {
    pub fn new(tabs: impl IntoIterator<Item = Tab<V>>) -> Self;
    pub fn selection(self, value: V) -> Self;
    pub fn bind(self, store: Store<V>) -> Self;
    pub fn on_change(self, action: impl for<'a> Fn(&mut Context<'a>, &V) + 'static) -> Self;
}
```

- `Picker::indexed` に相当する簡易コンストラクタは設けません。タブは数が少なく固定的で、値を付ける負担が小さいためです。
- 値が無い、または現在値がどのタブにも無い場合は先頭のタブを表示します。`NSTabView` は「どのタブも選ばれていない」状態を持てないため、`ViewKind::TabView::selected` は `usize` のままにします(下記「プラットフォームコード」)。
  - このとき Store には書き込みません。描画中に状態を変えないという原則を優先します。
- 重複値とイベント変換の扱いは `Picker<V>` と同じです。

### `ItemKey` と `List<K>` / `Table<K>` / `Sidebar<K>`

```rust
/// 行や項目を描画をまたいで識別する値。
pub trait ItemKey: Clone + PartialEq + 'static {
    /// バックエンドに渡す文字列。異なるキーは異なる文字列を返すこと。
    fn item_id(&self) -> String;
}

// String、&'static str、全整数型に実装する。
impl ItemKey for String { .. }
impl ItemKey for &'static str { .. }
impl ItemKey for u64 { .. } // ほかの整数型も同様

pub struct ListRow<K = String> { .. }
impl<K: ItemKey> ListRow<K> {
    pub fn new<V>(key: K, content: V) -> Self
    where V: Clone + IntoElement<Output = RenderNode> + 'static;
}

pub struct List<K: ItemKey = String> { .. }
impl<K: ItemKey> List<K> {
    pub fn bind(self, selection: Store<Option<K>>) -> Self;
    pub fn on_select(self, action: impl for<'a> Fn(&mut Context<'a>, Option<&K>) + 'static) -> Self;
}
```

`Table<K>` / `TableRow<K>`、`Sidebar<K>` / `SidebarSection<K>` / `SidebarItem<K>` も同じ形にします。

| 型 | メソッド | 引数 |
|---|---|---|
| `List<K>` | `bind` / `on_select` | `Store<Option<K>>` / `Option<&K>` |
| `Table<K>` | `bind` / `on_select` / `should_select` | `Store<Option<K>>` / `Option<&K>` / `Option<&K>` |
| `Table<K>` | `on_activate` / `on_delete` | `&K` |
| `Sidebar<K>` | `bind` / `on_select` | `Store<Option<K>>` / `&K` |

`Display` ではなく専用トレイトにする理由: 表示用の文字列は重複しうる(同名のフォルダなど)ため、「識別子として一意である」ことを実装者に明示させます。利用側の ID 型には 1 行の実装で足ります。

挙動:

- バックエンドには従来どおり `item_id()` の文字列を渡します。行の要素キー(`KeyedElement::key`)にも同じ文字列を使い、行が動いてもビューの状態を保ちます。
- `base_element` で `(String, K)` の対応表を作り、イベントのクロージャに `Rc` で持たせます。届いた文字列が表に無い場合(描画後に行が消えた場合)はイベントを無視します。
- 現在値が行に無い場合は `selected: None` として描画します(`Picker` と揃える)。
- `item_id()` の重複は `debug_assert!` で検出します(`HashSet<String>`)。
- 文字列リテラルの ID は `K = &'static str` と推論されます。`Store<Option<String>>` と組み合わせる箇所は、Store を `Option<&'static str>` にするか、ID を `String` にします。

### `ViewId`

```rust
/// ビューを識別する名前。アクセシビリティ、UI テスト、スクロール先の指定に使う。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ViewId(String);

impl From<&str> for ViewId { .. }
impl From<String> for ViewId { .. }
impl From<ViewId> for String { .. }
impl From<&ViewId> for ViewId { .. }   // 親ビューの ID を子へ渡す箇所用
impl Display for ViewId { .. }          // format!("{id}-clear") のような派生 ID 用
impl ViewId { pub fn as_str(&self) -> &str; }
```

- 置き場所は `zintl-ui-native`。`RenderNode` は `Option<String>` のままにし、native の `base_element` で `String` に変換します(方式 A)。
- 使う箇所: `Attach::id(impl Into<ViewId>)`、`Attach::accessibility_labeled_by(impl Into<ViewId>)`、`ScrollView::scroll_to(Trigger<ViewId>, ..)`、`Window::id`、`ChartBar::id`、`ToolbarItem::id`。
- `ViewModifiers::id()` は `Option<&ViewId>` を返します。
- 検査 API(`testing.rs` の `find` / `dispatch_to`)は `&str` のままです。ID を共有したいときは `const SIZE_LABEL: &str = "size-label";` のように `&'static str` 定数にすれば、ビュー側(`Into<ViewId>`)と検査側(`&str`)の両方に渡せます。そのため `ViewId::from_static` は設けません。

## プラットフォームコード

**変更不要です。** バックエンド(`zintl-ui-appkit-backend`、`zintl-ui-winui3-backend`)と `platform/` のどちらも触りません。確認結果:

| 必要な挙動 | AppKit | WinUI 3 |
|---|---|---|
| `PopUpButton` の未選択 | `zpd-appkit` `PopUpButton::select(None)` が `selectItemAtIndex:-1` を送る | `set_number(SelectedIndex, -1)` → `ComboBox::SelectedIndex(-1)` |
| `SegmentedControl` の未選択 | `SegmentedControl::select(None)` が `setSelectedSegment:-1` を送る | `SelectorBar::SelectedItem(nullptr)` |
| 未選択にしたときにイベントが出ない | アクションはユーザー操作でのみ発火 | `emit_index` が負の値と同じ値を捨てる |
| 未選択からユーザーが選ぶと通知される | 通常どおり | `state->index` が -1 なので `emit_index` が通る |
| Radio の未選択 | native 側の `Button` 群なのでバックエンド無関係 | 同左 |
| `TabView` | `NSTabView` は未選択を持てない → `usize` のまま、native で先頭にフォールバック | 同左(範囲外は無視される) |
| 行キー | 文字列 ID のまま。対応表は native に持つ | 同左 |
| `ViewId` | 方式 A により `RenderNode` は `String` のまま | 同左 |

実装後、`tests/ui` のシナリオで次を実機確認します(macOS のみ。Windows は既存の CI の範囲)。

- `Picker` Menu / Segmented で、Store に選択肢に無い値を書くと選択が消え、その後ユーザーが選ぶと Store に値が入ること。

## 段階と PR

各段階を 1 PR にします。どの段階もバックエンド変更はありません。

1. **`ViewId`**: 型の追加と、上表の 6 か所の差し替え。呼び出し側はほぼ文字列リテラルのままコンパイルが通ります。
2. **`Picker<V>` / `TabView<V>`**: `ChangeRoute` のインデックス → 値変換を追加。インデックスのまま使う箇所は `Picker::indexed` に置き換え、`TabView` は各 `Tab::new` に値を付けます。
3. **`ItemKey` と `List<K>` / `Table<K>` / `Sidebar<K>`**: 対応表とイベント変換。`examples/notes` のサイドバーとノート一覧を利用側の ID 型に置き換えます(`SmartFolder::ALL` のような文字列定数を enum にする)。

## 影響範囲

- `Picker` / `TabView` を使う全コード(約 40 か所)。
- `List` / `Table` / `Sidebar` を使う全コード。`examples/notes`、`ui/zintl-ui-desktop/examples`、`tests/ui/tests/app/scenarios`、`widget_tests.rs`。
- `ui/zintl-ui-desktop/README.md` のサンプル。
- `ViewModifiers::id()` の戻り値型を使う内部コード(`Toggle` などが子ビューに ID を渡す箇所)。

## テスト

`ViewId`:

- `id("x")` と `id(String::from("x"))` が同じ `RenderNode::View::id` になること。
- `accessibility_labeled_by`、`scroll_to`、`Window::id`、`ChartBar::id`、`ToolbarItem::id` が文字列として出ること。

`Picker<V>`:

- 値で選択した結果が正しいインデックスとしてノードに出ること(3 スタイルとも)。
- 現在値が選択肢に無いとき `selected: None` になり、Radio ではどれもオンにならないこと。
- 値を渡さないとき先頭が選択されること。
- 選択肢を並べ替えても、Store の値に対応する選択肢が選ばれること。
- インデックス 1 の選択イベントで、Store に 2 番目の値が書かれ、`on_change` がその値を受け取ること。
- 範囲外のインデックスのイベントで Store も `on_change` も動かないこと。
- `Picker::indexed` が従来と同じ挙動であること(既存テストの移行)。

`TabView<V>`:

- 上記のうち Radio 以外の 5 点。ただし「選択肢に無い値」では先頭のタブ(`selected: 0`)が出て、Store は書き換わらないこと。

`List<K>` / `Table<K>` / `Sidebar<K>`:

- 独自の ID 型(テスト内で `ItemKey` を実装)で、選択がその `item_id()` としてノードに出ること。
- 行に無い値を Store に入れると `selected: None` になること。
- 選択イベントの文字列が対応する `K` に変換されて Store と `on_select` に届くこと。表に無い文字列のイベントは無視されること。
- `Table` の `on_activate` / `on_delete` / `should_select` が `K` を受け取ること。
- 行の並べ替え後も、行ビューの状態がキーで保たれること(既存テストの移行)。

## 決定事項(旧未決事項)

1. **行 ID と選択のジェネリック化**: 本ステップの段階 3 で行う。`ItemKey` トレイトで文字列化し、バックエンドは文字列 ID のまま。「選択を値で表す」という本ステップの目的と同じ変更なので、別ステップに分けずに段階として扱う。
2. **`ViewId::from_static`**: 設けない。`&'static str` の定数でビュー側と検査側の両方に渡せるので、`Cow` などで内部表現を複雑にする理由がない。
3. **`ChartBar::id` と `Window::id`**: どちらも `ViewId` にする。両方ともアクセシビリティ識別子として使われている(`Window::id` は `set_identifier`)。同じ理由で `ToolbarItem::id`(検索欄)も `ViewId` にする。イベントで返るコマンド ID(`ToolbarItem::button`、`MenuItem::id`)は `String` のまま。
4. **`ViewId` をバックエンドまで通すか**: 通さない(方式 A)。`RenderNode` は描画先に渡す素のデータで、型による取り違え防止は宣言側の API で足りる。
