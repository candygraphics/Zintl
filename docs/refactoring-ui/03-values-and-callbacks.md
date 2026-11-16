# ステップ 3: 値・バインド・コールバックの語彙を揃える

## 目的

「値を渡す」「Store に繋ぐ」「変更を受け取る」「命令を送る」の 4 つを、全コントロールで同じ名前・同じ形にします。

## 現状

### 制御値(Store を使わずに値を渡すメソッド)

| 型 | メソッド |
|---|---|
| `Toggle` | `on(bool)` |
| `Picker` | `selected(usize)` |
| `TabView` | `selected(usize)`(native のみ) |
| `TextField` | `value(impl Into<String>)` |
| `NumberField` / `IntegerField` | `value(..)` |
| `ColorWell` | `color(Color)` |
| `DatePicker` | `date(CalendarDate)` |
| `Slider` `ComboBox` `SearchField` | なし |

### `on_change` の引数

| 型 | 引数 |
|---|---|
| `TextField` | `&str` |
| `ComboBox` `SearchField` | `String` |
| `TextEditor` | なし |
| `Toggle` `Picker` `Slider` `NumberField` `ColorWell` `DatePicker` | 値渡し(`Copy` 型) |

### バインド

- 主値は `bind(Store<T>)` で揃っています。
- 副次バインドは `Window::bind_open`、`Table::bind_sort`、`ColorPanel::bind_color`、`Button::bind_state` と個別命名です。
- `ColorPanel` は `bind` が「表示中か」、`bind_color` が色で、他のコントロール(`bind` = 編集対象の値)と逆です。

### 命令的操作

- `TextField::focus_on(Store<u64>)`: カウンタが増えるとフォーカスする。
- `ScrollView::scroll_to(Store<Option<String>>, ..)`: 同じ値でも書き込めば発火する(`cx.revision` を使用)。
- 仕組みが 2 通りあり、どちらも「Store を命令の通り道に使う」ことが型から読めません。

### 制御時の挙動(doc コメントに個別記載)

- `Toggle` / `Picker`: `on_change` が値を更新しなければ、クリックは取り消される。
- `TextField` / `NumberField`: 渡した値が変わるまで、入力中のテキストを保つ。

## 仕様(提案)

### 規則 1: 制御値は `value`、選択は `selection`

| 型 | 変更 |
|---|---|
| `Toggle` | `on` → `value(bool)` |
| `ColorWell` | `color` → `value(Color)` |
| `DatePicker` / `OptionalDatePicker` | `date` → `value(..)` |
| `Slider` `ComboBox` `SearchField` | `value(..)` を追加 |
| `Picker` `TabView` | `selected` → `selection(..)`(引数の型はステップ 4) |

`List`、`Table`、`Sidebar` は現在 `bind` しか持ちません。`selection(Option<&str>)` を追加するかは未決事項 1。

### 規則 2: `bind` は主値、それ以外は `bind_<対象>`

| 型 | 変更 |
|---|---|
| `ColorPanel` | `bind_color` → `bind(Store<Color>)`、`bind` → `bind_presented(Store<bool>)` |
| `Alert` `FileDialog` `Popover` | `bind` → `bind_presented`(主値を持たない表示物は表示状態を明示する) |
| `Window` | `bind_open` のまま(ウィンドウの「開いている」は表示物の `presented` と意味が同じなので `bind_presented` に寄せる案もある。未決事項 2) |
| `Table` | `bind_sort` のまま |
| `ScrollView` | `bind(Store<ScrollStatus>)` → `bind_offset` |

優先順位は全型共通で「`bind` があれば `value` より優先」。この一文を `Control` トレイトの doc に 1 回だけ書き、各メソッドの doc からは重複を削ります。

### 規則 3: `on_change` の引数

- `Copy` な値は値渡し。
- 文字列は `&str`。`ComboBox` と `SearchField` を `&str` に変更。
- `TextEditor::on_change` は `&AttributedText` を受け取る。
- 確定イベントは `on_commit`(`TextField` のみ現存)。`NumberField` / `IntegerField` に同名で追加するかは未決事項 3。

### 規則 4: 制御時の挙動を 2 分類として明文化

| 分類 | 型 | 挙動 |
|---|---|---|
| 離散コントロール | `Toggle` `Picker` `TabView` `Slider` `ColorWell` `DatePicker` | 常に渡された値を表示する。`on_change` が値を更新しなければ操作は取り消される |
| テキスト入力 | `TextField` `NumberField` `IntegerField` `ComboBox` `SearchField` | 渡された値が変わるまで入力中のテキストを保つ |

`Slider`、`ColorWell`、`DatePicker` が現状この表どおりかは、実装時に各バックエンドで確認します。違っていれば表に合わせて直します。

### 規則 5: 命令は `Trigger<T>`

```rust
/// 値を伴う一回きりの命令。書き込むたびに発火する。
pub struct Trigger<T: 'static = ()>(Store<Option<T>>);

impl Context<'_> {
    pub fn trigger<T: 'static>(&mut self) -> Trigger<T>;   // View::init で作る
    pub fn fire<T: 'static>(&mut self, trigger: Trigger<T>, value: T);
}
```

- ビュー側: `TextField::focus_on(Trigger)`、`TextEditor::focus_on(Trigger)`、`ToolbarItem::focus_on(Trigger)`、`ScrollView::scroll_to(Trigger<String>, ScrollAlignment)`。
- 実装: ビューは `cx.revision(store)` を読み、その値を `ViewKind::TextField::focus` や `ScrollTarget::revision` に渡します。バックエンドの比較方法(前回より増えたら実行)は現状のままです。
- 発火前の `revision` は 0 なので、マウント直後に誤発火しません(現在の `ScrollTarget` と同じ前提)。
- `Trigger` は `Copy`。`Store` と同じく `Default` で未初期化値を作れるようにします。

### 内部実装の統一

native の `controls.rs` にある `Binding<T>`(Store への書き込み + `on_change` 呼び出し)を、次の重複箇所でも使います。

- `Button` の `StateChanged` 処理
- `TextField` の `TextChange::route`
- `Toggle` の Switch 分岐と Button 分岐
- `Picker` の Radio 分岐

名前はステップ 7 の公開型と紛らわしいので `ChangeRoute<T>` に改名します。

## 手順

1. 規則 3(引数型)と内部統一。コンパイルエラーで漏れが分かります。
2. 規則 1・2 の改名。
3. `Trigger` の導入と `focus_on` / `scroll_to` の差し替え。
4. 規則 4 の doc 整理と、挙動の確認テスト追加。

## テスト

- 各コントロールで「`value` のみ」「`bind` のみ」「両方(bind 優先)」の表示値を確認する。
- 離散コントロール: `value` 固定 + 何もしない `on_change` でイベントを送った後、ノードの値が元のままであること。
- `Trigger`: マウント直後は発火しないこと。`fire` 1 回で revision が 1 増えること。同じ値で 2 回 `fire` して 2 回とも伝わること。
- `ComboBox` / `SearchField` の `on_change` が `&str` を受け取ること(コンパイルで担保)。

## 実装時の決定

- 未決事項 1: 追加していません。
- 未決事項 2: `Window::bind_open` のままです。
- 未決事項 3: 追加していません。
- 未決事項 4: `Trigger` は `zintl-ui`(`zintl_ui::store`)に置き、`trigger` / `fire` は `StoreContext` と `Context` トレイトの既定メソッドです。
- 規則 4: `Slider`、`ColorWell`、`TabView` は取り消しが未実装だったので、表に合わせました。`ViewKind::reverts_user_changes` に 3 種を加え、両バックエンドで `ValueChanged` と `ColorChanged` を利用者の変更として記録し、`ColorWell` は表示中の色と比較して書き戻します。このためステップ 3 にはバックエンド変更が含まれます。
- 内部の `Switch` / `PopUpButton` / `SegmentedControl` / `Button` は `bind` と `on_change` の代わりに `ChangeRoute` を 1 つ受け取ります。

## 未決事項

1. `List` / `Table` / `Sidebar` に Store を使わない `selection(..)` を追加するか。
2. `Window::bind_open` を `bind_presented` に寄せるか。
3. 数値フィールドに `on_commit` を追加するか。`IntegerField` は「入力中も整数なら通知、編集終了時に整形」という挙動で、確定通知の需要がありそうです。
4. `Trigger` を置くクレート。`cx.revision` は `zintl-ui` にあるので `zintl-ui` に置けますが、`cx.fire` を `Context` トレイトの既定メソッドにする形になります。
