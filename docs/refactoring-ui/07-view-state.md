# ステップ 7: ビューの状態と初期化

## 目的

- 「外から Store を渡されたらそれを使い、無ければ自分で持つ」という定型を 1 つの型にします。
- `Store` の未初期化使用を見つけやすくします。
- ドラッグ中の一時状態の置き場所を決め、`SplitView` のドラッグが途中で止まる不具合を直します。

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
- 代入を忘れると、最初の `cx.get` / `cx.update` / `cx.watch` で `"Store was used before View::init initialized it"` というパニックになります。どの Store かも、どこで使ったかも分かりません(パニック位置は `store.rs`)。

### 再マウント時の `init`

親が再描画されると、子のビューは新しいインスタンスに置き換わり、`init` がもう一度呼ばれます(`ViewBuilder::initialized` が `false` の新しいビルダーが `rebuild_bound` で差し替わる)。
このとき `cx.store` は、`init` 内で呼ばれた順番(スロット)ごとに同じ Store を返します(`StoreContext::store` の `init_stores`)。

### 一時状態と、ドラッグが止まる不具合

`SplitView` はドラッグの掴み位置を `grab: Rc<Cell<Option<f64>>>` で持ち、`render` のたびに `self.clone()` をイベントクロージャへ渡しています。
`clone` は `Rc` を共有するので同一インスタンス内では動きますが、親の再描画で `SplitView` が作り直されると、新しい `Rc`(`None`)になります。

**確認結果**: ドラッグ中に親が再描画されると `grab` が失われ、以降のドラッグが無視されます。
`bind(width)` した Store を親自身も `cx.get` で読んでいる場合、最初の移動で親が再描画されるため、ドラッグは 1 回分しか効きません。
ヘッドレスで次の手順を再現しました。

1. 親が `width: Store<f32>`(200)を作り、`render` で `cx.get(width)` を読んで `SplitView::horizontal(..).bind(width)` を返す。
2. 区切り線に `Down`(x = 3)→ `Moved`(x = 23)を送ると、`width` は 220 になる。
3. 続けて `Moved`(x = 43)を送っても、`width` は 220 のまま(期待値は 240)。

## 仕様

### 1. `Local<T>`

```rust
/// ビューが自分で持つか、外から渡される状態。
///
/// `View::init` で `init` を呼ぶと、`bind` された Store か、初期値を入れた
/// ビュー自身の Store を指します。
#[derive(Clone)]
pub struct Local<T: 'static> {
    initial: T,
    bound: Option<Store<T>>,
    store: Store<T>,
}

impl<T: Clone + 'static> Local<T> {
    pub fn new(initial: T) -> Self;
    /// 外の Store を使う。初期値は使われない。
    pub fn bind(&mut self, store: Store<T>);
    /// ローカルの初期値を変える。init より前に呼ぶ。
    pub fn set_initial(&mut self, initial: T);
    /// View::init から呼ぶ。
    pub fn init(&mut self, cx: &mut impl Context<'_>);
    /// init 後に使える Store。
    pub fn store(&self) -> Store<T>;
}
```

- `init` は、バインドの有無にかかわらず `cx.store(initial.clone())` を 1 回呼んでスロットを確保します。そのうえで、バインドがあればそれを、無ければ確保した Store を `store` に入れます。
  - スロットを常に確保するので、再描画で `bind` の有無が変わっても、同じ `init` 内の後続の `cx.store` のスロットがずれません(ずれると `init_stores` の型検査でパニックする)。
  - バインド時はスロットの Store が使われずに残ります。ビュー 1 つにつき値 1 つ分で、無視できる量です。
- 再マウント時は `cx.store` が同じスロットの Store を返すので、ローカルの状態は保たれます。新しい `initial` は使われません(初期値は最初のマウントでだけ効く)。
- 置き場所は `zintl-ui`(`Store` の隣)。`Context` トレイト越しに `cx.store` を呼ぶので描画先に依存しません。
- 公開: `zintl_ui_native` と `zintl_ui_desktop` のルートから、`Store` と並べて再公開します。`init` を書く利用者のビューで使うためです。

適用先:

| 型 | 置き換え |
|---|---|
| `DisclosureGroup` | `binding` + `expanded` → `expanded: Local<bool>` |
| `SplitView` | `initial` + `binding` + `position` → `position: Local<f32>`。範囲によるクランプは `View::init` で `set_initial(clamped)` してから `init` を呼ぶ(`range` を後から呼んでも効くように) |

`Toggle` / `Picker` / `TabView` は `Local` に載せません(「見送ったこと」)。

### 2. `Store` の未初期化を見つけやすくする

- `Store::handle` と `Hook::hook_id`(`Store` の実装)に `#[track_caller]` を付けます。
- 呼び出し経路にも `#[track_caller]` を付け、パニック位置が利用者コードの行になるようにします。
  - `StoreContext` の `get` / `update` / `watch` / `revision` / `fire`
  - `Context` トレイトの既定メソッド `get` / `update` / `watch` / `revision` / `fire`
  - `Local::store`
  - `testing::Harness` の `get` / `set` / `update` / `fire`
- パニックメッセージに型名を入れます: `"Store<alloc::string::String> was used before View::init initialized it"`(`std::any::type_name::<T>()`)。

採用しない対策と理由:

| 案 | 理由 |
|---|---|
| `Store::default()` を廃止する | `#[derive(Default)]` でビューを書けなくなり、全ビューに手書きのコンストラクタが要る |
| `init` 後に全フィールドを検査する | フィールドを列挙する手段が無い。derive マクロが必要で、proc-macro クレートが増える |
| ビューを関数 + フックにする | 中核の設計変更で、本計画の範囲を超える |

### 3. 一時状態の置き場所

規約: **ポインタ操作中の一時状態は Canvas ハンドラの中に置く。**
マウント中のハンドラは再描画をまたいで保たれ、新しいハンドラは `Canvas2DHandler::reconcile` で渡されるだけだからです(native の `canvas_rerender_reconciles_into_the_mounted_handler` テストが保証)。
ハンドラ側で `reconcile` を上書きし、一時状態を残したまま設定だけを受け取ります。

`SplitView` の変更:

```rust
/// 区切り線のドラッグで、掴んだ位置からの移動量(描画座標)。
#[derive(Clone, Copy, Debug, PartialEq)]
struct DividerDrag {
    delta: f64,
}

#[derive(Clone)]
struct Divider {
    axis: Axis,
    /// 掴んだ位置(軸方向)。ドラッグ中だけ Some。
    grab: Option<f64>,
}
```

- `Divider::event` で `Down` のとき `grab` を記録し、`Moved` のとき `delta = along - grab` を `CanvasEventContext::emit(DividerDrag { delta })` で通知し、`Up` で `None` に戻す。
- `Divider::reconcile` は `axis` だけを受け取り、`grab` を残す。`axis` が変わらなければ `Redraw::No` を返す。
- `SplitView::render` は `Canvas2D::new(Divider { axis, grab: None }).on_action::<DividerDrag>(..)` で受け、`delta as f32` をクランプして `position` を更新する。キャストは受け取る側で行います(ステップ 8.6 の規則)。
- `SplitView` から `Rc<Cell<..>>`、`drag` メソッド、`on_event` の使用が消え、`render` での `self.clone()` も不要になる。クロージャが捕まえるのは `position` の Store(`Copy`)と範囲だけになる。

## プラットフォームコード

**変更不要です。** バックエンド(`zintl-ui-appkit-backend`、`zintl-ui-winui3-backend`)と `platform/` のどちらも触りません。確認結果:

| 必要な挙動 | AppKit | WinUI 3 |
|---|---|---|
| Canvas ハンドラがドラッグ中も同じインスタンスのまま | 再描画で `CanvasNode` は差し替わらず、`reconcile` が呼ばれる(`backend-core` の共通処理) | 同左 |
| `emit` した値が `on_action` に届く | `Event::CanvasActions` として送られる(`canvas_action` シナリオで確認済み) | 同左 |
| ボタンを押したまま区切り線の外へ出ても `Moved` が届く | `NSView` は `mouseDown:` の後の `mouseDragged:` を受け続ける(現状と同じ) | ポインタキャプチャ(現状と同じ) |
| `Local` / `#[track_caller]` | `zintl-ui` と native だけの変更 | 同左 |

実装後、`tests/ui` の `split_view` シナリオで次を実機確認します(macOS のみ)。

- 区切り線のドラッグ中に、バインドした幅を表示するラベルが更新され(=親が再描画され)、ドラッグが最後まで追従すること。

## 段階と PR

各段階を 1 PR にします。どの段階もバックエンド変更はありません。

1. **`#[track_caller]` と型名入りメッセージ**: 単独で入れられます。
2. **`Local<T>`**: 型を追加し、`DisclosureGroup` と `SplitView` に適用する。
3. **`SplitView` のドラッグ状態を `Divider` に移す**: 再現テスト(現状で失敗する)を先にコミットし、同じ PR で直す。

ステップ 6 との順序は README の「推奨順序」を参照してください。

## テスト

`Store`:

- 未初期化 Store のパニックメッセージに型名が入ること(`#[should_panic(expected = "Store<f32>")]`)。

`Local`:

- バインドなしで `init` すると初期値の Store ができること。
- バインドありで `init` すると渡した Store と同じハンドルになること。
- 2 回 `init` しても同じ Store を指し、2 回目の初期値は使われないこと(再マウントの模擬)。
- 1 回目はバインドなし、2 回目はバインドありで `init` しても、後続の `cx.store` が同じスロットの Store を返すこと。

`SplitView`:

- 範囲外の初期位置がクランプされること(既存テストの維持)。
- `range` を `position` より後に呼んでもクランプされること。
- `Down` → `Moved` → `Up` で、`position` が移動量だけ変わること(既存テストの移行。マウント中の `CanvasNode` に `event` を送り、`Event::CanvasActions` をディスパッチする)。
- 親が bind した Store を読んで再描画する構成で、`Moved` を 2 回送ると 2 回分動くこと(上記の再現手順。修正前は失敗する)。
- 再描画をまたいでマウント中の `CanvasNode` が同じであること。

## 見送ったこと

| 項目 | 理由 |
|---|---|
| `Toggle` / `Picker` / `TabView` の非制御モード(バインドも値も無ければ内部状態を持つ) | ステップ 3 で「離散的なコントロールは常に `value` を表示し、`on_change` が値にしない変更は取り消す」と決め、`Control` トレイトの doc とバックエンドの `ViewKind::reverts_user_changes` がその前提で動いている。非制御モードはこの規則の例外になる。また、これらが `init` を持つと、`native_element` を直接呼べない型が増える(ステップ 6 の規約)。既存の呼び出し(`examples/`、`tests/ui`)はすべて `bind` か `value` / `selection` を渡している。各型の doc に「状態を変えるには `bind` する」と明記します。 |

名前は `Local` に決めます。フィールド宣言(`expanded: Local<bool>`)で「既定はビュー自身の状態」と読めて短いためです。`OwnedOrBound` は長く、`StateSlot` は `init_stores` のスロットと紛らわしいため採りません。
