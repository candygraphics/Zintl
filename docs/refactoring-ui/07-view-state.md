# ステップ 7: ビューの状態と初期化

## 目的

- 「外から Store を渡されたらそれを使い、無ければ自分で持つ」という定型を 1 つの型にします。
- `Store` の未初期化使用を見つけやすくします。
- ドラッグ中の一時状態の置き場所を決めます。

## 現状

### 二重フィールド

`DisclosureGroup`(`arrangements.rs`)と `SplitView`(`split_view.rs`)は同じ形をしています。

```rust
binding: Option<Store<bool>>,   // bind() で渡されたもの
expanded: Store<bool>,          // 実際に使うもの。init までは未初期化

fn init(&mut self, cx: &mut Context<'_>) {
    self.expanded = self.binding.unwrap_or_else(|| cx.store(false));
}
```

`SplitView` はさらに初期値(`initial`)を別フィールドで持っています。

### `Store` の二段階初期化

- `Store::default()` は未初期化のハンドルを返します。ビューの構造体を `#[derive(Default)]` で作り、`View::init` で `cx.store(..)` を代入する書き方が標準です(`examples/app.rs`、`examples/gallery.rs`)。
- 代入を忘れると、最初の `cx.get` / `cx.update` / `cx.watch` で `"Store was used before View::init initialized it"` というパニックになります。どの Store かは分かりません。

### 一時状態

`SplitView` はドラッグの掴み位置を `grab: Rc<Cell<Option<f64>>>` で持ち、`render` のたびに `self.clone()` をイベントクロージャへ渡しています。
`clone` は `Rc` を共有するので同一インスタンス内では動きますが、親の再描画で `SplitView` が作り直されると新しい `Rc` になります。
ドラッグ中に親が再描画された場合の挙動は未確認です(要確認事項 1)。

## 仕様(提案)

### 1. `Local<T>`

```rust
/// ビューが自分で持つか、外から渡される状態。
#[derive(Clone)]
pub struct Local<T: 'static> {
    initial: Option<T>,
    bound: Option<Store<T>>,
    store: Store<T>,
}

impl<T: Clone + 'static> Local<T> {
    pub fn new(initial: T) -> Self;
    /// 外の Store を使う。初期値は無視される。
    pub fn bind(&mut self, store: Store<T>);
    /// ローカルの初期値を変える。
    pub fn set_initial(&mut self, initial: T);
    /// View::init から呼ぶ。
    pub fn init(&mut self, cx: &mut Context<'_>);
    /// init 後に使える Store。
    pub fn store(&self) -> Store<T>;
}
```

- `init` は、バインドがあればそれを、無ければ `cx.store(initial)` を `store` に入れます。
- `View::init` は再マウント時にも呼ばれ、`cx.store` は同じスロットの Store を返します(`StoreContext::store` の `init_stores` の仕組み)。`Local::init` はこの挙動にそのまま乗ります。
- 置き場所は `zintl-ui`(`Store` の隣)。`Context` トレイト越しに `cx.store` を呼べば描画先に依存しません。

適用先:

| 型 | 置き換え |
|---|---|
| `DisclosureGroup` | `binding` + `expanded` → `expanded: Local<bool>` |
| `SplitView` | `initial` + `binding` + `position` → `position: Local<f32>`。範囲によるクランプは `init` の前に `set_initial` で行う |

`TabView`、`Picker`、`Toggle` は現在、バインドが無いときに `value` をそのまま表示する制御コンポーネントです(ステップ 3 の規則 4)。これらを `Local` に載せて非制御にもできるようにするかは未決事項 1。

### 2. `Store` の未初期化を見つけやすくする

採用する対策:

- `Store::handle` と `Hook::hook_id` に `#[track_caller]` を付け、`Context::get` / `update` / `watch` / `revision` にも伝播させる。パニックの位置が利用者コードの行になります。
- パニックメッセージに型名を入れる: `"Store<alloc::string::String> was used before View::init initialized it"`。

採用しない対策と理由:

| 案 | 理由 |
|---|---|
| `Store::default()` を廃止する | `#[derive(Default)]` でビューを書けなくなり、全ビューに手書きのコンストラクタが要る |
| `init` 後に全フィールドを検査する | フィールドを列挙する手段が無い。derive マクロが必要で、proc-macro クレートが増える |
| ビューを関数 + フックにする | 中核の設計変更で、本計画の範囲を超える |

### 3. 一時状態の置き場所

規約: **ポインタ操作中の一時状態は Canvas ハンドラの中に置く。**
`Canvas2DHandler` は `reconcile` を通じてマウント中のインスタンスが保たれるためです(native の `canvas_rerender_reconciles_into_the_mounted_handler` テストが保証)。

`SplitView` の変更:

- `grab` を `Divider` ハンドラのフィールドに移す。
- `Divider::event` で移動量を計算し、`CanvasEventContext::emit(DividerDrag { delta })` で通知する。
- `SplitView::render` は `Canvas2D::new(Divider { .. }).on_action::<DividerDrag>(..)` で受け、`position` を更新する。
- `SplitView` から `Rc<Cell<..>>` と `drag` メソッドが消え、`render` での `self.clone()` も不要になる。

## 手順

1. `#[track_caller]` と型名入りメッセージ(単独で入れられる)。
2. `Local<T>` を追加し、`DisclosureGroup` と `SplitView` に適用する。
3. `SplitView` のドラッグ状態を `Divider` に移す。

## テスト

- `Local`: バインドなしで `init` すると初期値の Store ができること。
- `Local`: バインドありで `init` すると渡した Store と同じハンドルになること。
- `Local`: 2 回 `init` しても同じ Store を指すこと(再マウントの模擬)。
- `SplitView`: 範囲外の初期位置がクランプされること(既存テストの維持)。
- `SplitView`: `Down` → `Moved` → `Up` の順にイベントを送り、`position` が移動量だけ変わること。途中で Store を更新して再描画を挟んでも、ドラッグが続くこと。
- 未初期化 Store のパニックメッセージに型名が入ること(`#[should_panic(expected = ..)]`)。

## 要確認事項

1. 現在の `SplitView` で、ドラッグ中に親が再描画されたとき `grab` が失われるか。失われるなら手順 3 はバグ修正でもあります。再現テストを先に書きます。

## 未決事項

1. `Toggle` / `Picker` / `TabView` に非制御モード(バインドも `value` も無ければ内部状態を持つ)を与えるか。与える場合、それらが `View::init` を持つことになり、ステップ 6 の「`init` を持つ型は `native_element` を直接呼ばれてはならない」制約の対象が増えます。
2. `Local` という名前。代案は `OwnedOrBound`、`StateSlot`。
