# ステップ 2: desktop / native の二重ラッパーをやめる

## 目的

「native の型を desktop が 1 メソッドずつ転送する」構造をなくし、ビューの定義を 1 か所に集めます。

## 現状

- `ui/zintl-ui-desktop/src/lib.rs` の `Window` は 15 個のメソッドをすべて native の `Window` へ転送しています。
- `ui/zintl-ui-desktop/src/controls.rs` の `ComboBox`、`Slider`、`NumberField`、`IntegerField`、`OptionalIntegerField`、`SearchField`、`ColorWell`、`ScrollView`、`TabView` は `wrapper!` マクロ + 手書き転送です。
- 一方 `List`、`Table`、`DatePicker`、`Canvas2D`、`ProgressIndicator`、`Separator`、`TextEditor`、`Sidebar`、`Toolbar`、`Alert` などは native からそのまま再エクスポートされています。
- 転送漏れがあります。native の `TextField::fill_width` と `TabView::selected` は desktop 側にありません。メソッドを足すたびに 2 か所書く必要があります。

- desktop の合成ビュー(`Toggle`、`Picker`、`Text`、スタック、`Grid`、`ZStack`、`DisclosureGroup`、`SplitView`、`BarChart`)はバックエンドに依存しておらず、native と layout だけを使っています。desktop に置く必然性がありません。
- native の公開 API には AppKit の語彙が出ています(`TextField::label_with_string`、`Button::bezel_style`、`Button::button_type`、`PopUpButton`、`SegmentedControl`、`Switch`)。desktop はこれを隠すために包んでいます。

## 仕様(提案)

### 役割の再定義

| クレート | 役割 |
|---|---|
| `zintl-ui-native` | **すべてのビュー**。バックエンド非依存。公開 API はプラットフォーム中立の語彙のみ |
| `zintl-ui-desktop` | `App`、`AppError`、バックエンド選択、再エクスポート(ステップ 5 の公開面) |

### 型ごとの扱い

| 型 | 扱い |
|---|---|
| `Window` | native の型をそのまま公開。`Window::panel` を native に移す。`style(WindowStyle)` は `pub(crate)` にするか残すか未決(未決事項 2) |
| `ComboBox` `Slider` `NumberField` `SearchField` `ColorWell` `ScrollView` | native の型をそのまま公開。desktop のラッパーを削除 |
| `IntegerField` / `OptionalIntegerField` | native の `IntegerField<T>` をそのまま公開。`OptionalIntegerField` は削除し `IntegerField<Option<i64>>` を使う |
| `TabView` / `Tab` | native の型を公開。`TabViewItem` を `Tab` に改名し、型エイリアスを削除 |
| `Slider::new` | 引数を `RangeInclusive<f64>` に変更(desktop の形に合わせる)。`NumberField::range` も同様 |
| `Text` | native に移す。`TextField::label_with_string` を公開 API から外し、`Text` の内部実装にする |
| `TextField` | native の型を公開。`with_string` は `value` と重複するので削除 |
| `Button` | native の型を公開。`style(ButtonStyle)` を native に移す。下表の低レベル API は `pub(crate)` |
| `Toggle` `Picker` | native に移す |
| `HStack` `VStack` `Spacer` `ZStack` `Grid` `GridCell` `DisclosureGroup` `SplitView` `BarChart` | native に移す |
| native の `View`(コンテナ構造体) | `Container` に改名して `pub(crate)`。トレイト `View` との名前衝突を解消 |

### `pub(crate)` にする低レベル API

合成ビューが同一クレートに入るので、外に出す必要がなくなります。

- `Switch`、`PopUpButton`、`SegmentedControl`(`Toggle` / `Picker` の部品)
- `Button::checkbox`、`Button::radio`、`Button::button_type`、`Button::bezel_style`、`Button::on`、`Button::bind_state`、`Button::on_state_change`
- `TextField::label_with_string`
- `BezelStyle`、`ButtonType` の再エクスポート

### 削除するもの

- desktop の `native_view!`、`wrapper!`、`stack!` マクロ(スタックは native 側で 1 つの定義にする)。
- desktop の `Stack` と `Container`(同じ処理の重複。native の `Container` に一本化)。
- `NumberField::integer()` / `NumberField::integer_optional()`(`IntegerField::new()` / `IntegerField::optional()` を直接使う)。

## 段階

| 段階 | 内容 | 影響 |
|---|---|---|
| 2a | 合成ビューとそのテスト(`widget_tests.rs`)を native に移す。desktop は再エクスポートのみ | 利用者から見た API は不変 |
| 2b | 転送ラッパーを削除し native の型を再エクスポート。`OptionalIntegerField`、`Tab` エイリアス、`NumberField::integer*` を削除 | 破壊的(小) |
| 2c | 低レベル API を `pub(crate)` にする。`View` → `Container` | native を直接使っているコードがあれば破壊的 |

## テストの移動

`widget_tests.rs`(2294 行)と `lib.rs` 内テストは、`App` の `#[cfg(test)]` ヘルパー(`dispatch_at`、`render_roots` など)に依存しています。
2a では native の既存テストヘルパー(`HeadlessBackend` + `Composer` を組む `composer()`)に載せ替えます。
ステップ 5 で導入する検査用ハーネスを先に作り、それを使うのが手戻りが少ないです。

## テスト

- 移動した既存テストがすべて通ること(挙動は変えない)。
- `IntegerField::optional()` が従来の `OptionalIntegerField` と同じノードとイベント処理になること(既存テストの型名差し替え)。
- desktop クレートの公開名一覧が、削除を明示したもの以外は変わっていないこと(`cargo doc` の出力か、公開名を列挙するテストで確認)。

## 未決事項

1. `zintl-ui-native` という名前を保つか。全ビューを持つなら `zintl-ui-views` などが実態に合います。改名は依存クレート(両バックエンドは native に依存していない)への影響が小さい今が安価です。
2. `WindowStyle` を公開するか、`Window::new` / `Window::panel` のコンストラクタだけにするか。
3. `tests/ui` と `examples/notes` は `zintl-ui-native` の名前を直接参照していないことを grep で確認済みですが、`Switch` などの低レベル型を desktop 経由で使っていないかは 2c の前に再確認します(現状 desktop は再エクスポートしていないので使えないはずです)。

## 実装メモ

2a〜2c をまとめて実装しました。仕様との差分と、未決事項の決定は次のとおりです。

- 未決 1: クレート名は `zintl-ui-native` のままです。改名は別の変更にします。
- 未決 2: `WindowStyle` は公開しません。`Window::new` と `Window::panel` だけで選びます。
- 未決 3: `tests/ui` と `examples/notes` は低レベル型を使っていませんでした。変更は `NumberField::integer*` の置き換えだけです。
- スタックは `Stack<A>` の 1 定義で、`HStack` と `VStack` はその型エイリアスです。軸を表す `StackAxis` と目印の型は公開モジュールに出していません。
- `Slider` は desktop の形に合わせ、`ticks(count)` と `snapping_ticks(count)` を持ちます。`tick_marks(count, snap)` は削除しました(8.4 でさらに整理します)。
- `IntegerField::range` も `RangeInclusive<i64>` を取ります。
- native の `TextField` をそのまま公開したので、`font`、`color`、`alignment`、`line_limit`、`TabView::selected`、`ComboBox::value`、`Slider::value` が desktop からも使えるようになりました。
- テストは `zintl-ui-native/src/testing.rs` の `TestApp`(ヘッドレスバックエンド)に載せ替えました。ステップ 5 の検査用ハーネスができたら置き換えます。
- desktop には、`App` の描画結果を読むテストと、公開名が desktop から届くことを確かめるテストだけを残しました。
