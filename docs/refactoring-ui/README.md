# UI ライブラリ リファクタリング計画

`ui/` 配下、主に `zintl-ui-desktop` と `zintl-ui-native` の API を揃えるための仕様書です。
クロスプラットフォーム対応の評価は対象外です。

## 進め方

- 互換シムや非推奨エイリアスは残しません(`AGENTS.md` の方針)。各ステップで `examples/`、`tests/ui`、`examples/notes` も同時に直します。
- 1 ステップ = 1 PR を基本にします。ステップ内に「段階」がある場合は段階ごとに分けても構いません。
- 各文書の「未決事項」は実装前に決める必要がある点です。「提案」と書いた箇所は仕様案で、確定ではありません。

## ステップ一覧

| # | 文書 | 内容 | 依存 | バックエンド変更 |
|---|---|---|---|---|
| 1 | [01-common-modifiers.md](01-common-modifiers.md) | レイアウト・`id`・`enabled` を全ビュー共通にする | なし | なし |
| 2 | [02-wrapper-layer.md](02-wrapper-layer.md) | desktop → native の転送ラッパーをやめ、ビューを 1 クレートに集める | 1 | なし |
| 3 | [03-values-and-callbacks.md](03-values-and-callbacks.md) | 制御値・バインド・`on_change`・トリガーの語彙を揃える | 2 | なし |
| 4 | [04-selection-and-ids.md](04-selection-and-ids.md) | `Picker` / `TabView` / `List` などの選択を値ベースにし、ビュー ID に型を付ける | 3 | なし |
| 5 | [05-public-surface.md](05-public-surface.md) | 公開面のモジュール化、検査 API の分離、ファイル分割 | 2 | なし |
| 6 | [06-custom-views.md](06-custom-views.md) | 利用者定義ビューに共通モディファイアを開放、拡張方針、テーマ | 1 | 任意 |
| 7 | [07-view-state.md](07-view-state.md) | 「外部バインドかローカル状態か」の共通型、`Store` の初期化 | なし | なし |
| 8 | [08-small-fixes.md](08-small-fixes.md) | コンストラクタと命名の細かい不揃い | 1, 2 | `Window` のみ |

推奨順は 1 → 2 → 5 → 3 → 4 → 6 → 7 → 8 です。
1 で共通トレイトができると、2 で消す転送メソッドが大きく減ります。5 は 2 の直後に行うとファイル移動が一度で済みます。

## 現状の層構造

```
zintl-ui                 View / Store / Element / Composer(描画先に依存しない中核)
zintl-ui-layout          LayoutStyle と taffy によるレイアウト
zintl-ui-graphics        Canvas 用の 2D 描画型
zintl-ui-backend-core    RenderNode / ViewKind / Event / Context / HeadlessBackend
zintl-ui-native          RenderNode を組み立てる宣言的ビュー(AppKit 寄りの語彙)
zintl-ui-desktop         native の改名ラッパー + 合成ビュー + App
zintl-ui-appkit-backend  AppKit バックエンド
zintl-ui-winui3-backend  WinUI 3 バックエンド
zintl-ui-test            zintl-ui 中核用のテストバックエンド
```

## 共通の完了条件

- `cd ui && cargo check && cargo test` が通る。
- `cd ui && cargo fmt` 済み。
- 追加・変更したテストには、検証する挙動を説明する短いコメントが付いている。
- `ui/zintl-ui-desktop/README.md` のサンプルが新しい API でコンパイルできる形になっている。
