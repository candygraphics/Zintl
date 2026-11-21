# UI ライブラリ リファクタリング計画

`ui/` 配下、主に `zintl-ui-desktop` と `zintl-ui-native` の API を揃えるための仕様書です。
クロスプラットフォーム対応の評価は対象外です。

## 進め方

- 互換シムや非推奨エイリアスは残しません(`AGENTS.md` の方針)。各ステップで `examples/`、`tests/ui`、`examples/notes` も同時に直します。
- 1 ステップ = 1 PR を基本にします。ステップ内に「段階」がある場合は段階ごとに分けても構いません。
- 各文書の仕様は確定済みです。採らなかった案は各文書の「見送ったこと」に理由とともに残し、後で拾う価値のあるものは `TODO.md` に移しています。
- バックエンド(`ui/zintl-ui-*-backend`)と `platform/` への変更は、各文書の「プラットフォームコード」に場所と内容をまとめています。

## ステップ一覧

ステップ 1〜5(共通モディファイア、ラッパー層の廃止、値とコールバック、選択と ID、公開面)は完了済みです。

| # | 文書 | 内容 | バックエンド変更 | `platform/` の変更 |
|---|---|---|---|---|
| 6 | [06-custom-views.md](06-custom-views.md) | 利用者定義ビューに共通モディファイアを開放、拡張方針、テーマ、文字幅の見積もり | AppKit の `Palette` 変換のみ | `zpd-appkit` の `CanvasPalette` に 2 色 |
| 7 | [07-view-state.md](07-view-state.md) | 「外部バインドかローカル状態か」の共通型、`Store` の初期化、`SplitView` のドラッグ不具合 | なし | なし |
| 8 | [08-small-fixes.md](08-small-fixes.md) | コンストラクタと命名の細かい不揃い | 8.1 `Window`(両方)、8.6 `TableColumn`(変換のみ) | 8.1 のみ(`zpd-appkit`、`zpd-winui3`、`ZintlWinUI3Support`) |

## 推奨順序

ステップ 6 と 7 は `split_view.rs` を共有するので、次の順に進めると衝突しません。ステップ 8 の各 PR は独立していて、いつ入れても構いません。

1. 7-1 `#[track_caller]` と型名入りメッセージ
2. 6-1 `SplitView` の `NativeView` 化と重ね順
3. 7-2 `Local<T>`
4. 7-3 `SplitView` のドラッグ状態を `Divider` に移す(不具合修正)
5. 6-2 合成ビューの公開手順
6. 6-3 `TextEstimate`
7. 6-4 `Palette` の 2 色(`Divider` の色を置き換えるので 7-3 の後)

`platform/` に触れるのは 6-4 と 8.1 だけです。どちらも macOS での実機確認(`tests/ui`)を完了条件に含めます。

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
