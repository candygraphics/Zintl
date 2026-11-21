# UI ライブラリ リファクタリング計画

`ui/` 配下、主に `zintl-ui-desktop` と `zintl-ui-native` の API を揃えるための仕様書です。
クロスプラットフォーム対応の評価は対象外です。

## 進め方

- 互換シムや非推奨エイリアスは残しません(`AGENTS.md` の方針)。各ステップで `examples/`、`tests/ui`、`examples/notes` も同時に直します。
- 1 ステップ = 1 PR を基本にします。ステップ内に「段階」がある場合は段階ごとに分けても構いません。
- 各文書の「未決事項」は実装前に決める必要がある点です。「提案」と書いた箇所は仕様案で、確定ではありません。

## ステップ一覧

ステップ 1〜5(共通モディファイア、ラッパー層の廃止、値とコールバック、選択と ID、公開面)は完了済みです。

| # | 文書 | 内容 | バックエンド変更 |
|---|---|---|---|
| 6 | [06-custom-views.md](06-custom-views.md) | 利用者定義ビューに共通モディファイアを開放、拡張方針、テーマ | 任意 |
| 7 | [07-view-state.md](07-view-state.md) | 「外部バインドかローカル状態か」の共通型、`Store` の初期化 | なし |
| 8 | [08-small-fixes.md](08-small-fixes.md) | コンストラクタと命名の細かい不揃い | `Window` のみ |

## 現状の層構造

```
zintl-ui                 View / Store / Element / Composer(描画先に依存しない中核)
zintl-ui-layout          LayoutStyle と taffy によるレイアウト
zintl-ui-graphics        Canvas 用の 2D 描画型
zintl-ui-backend-core    RenderNode / ViewKind / Event / Context / HeadlessBackend
zintl-ui-native          すべての宣言的ビュー(RenderNode を組み立てる、AppKit 寄りの語彙)
zintl-ui-desktop         App ランナーと、native のビューを領域別モジュールで再公開する入口
zintl-ui-appkit-backend  AppKit バックエンド
zintl-ui-winui3-backend  WinUI 3 バックエンド
zintl-ui-test            zintl-ui 中核用のテストバックエンド
```

## 共通の完了条件

- `cd ui && cargo check && cargo test` が通る。
- `cd ui && cargo fmt` 済み。
- 追加・変更したテストには、検証する挙動を説明する短いコメントが付いている。
- `ui/zintl-ui-desktop/README.md` のサンプルが新しい API でコンパイルできる形になっている。
