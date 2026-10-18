<div align="center">

# SuperLightTUI

**Web ページのようにターミナル UI を作る** — flexbox レイアウト、Tailwind 風のメソッドチェーン、hooks、クリック/ホバー/スクロールを Rust のクロージャ 1 つで。

**書くのは速く。依存ツリーは軽量**（直接の必須依存は 4 つ、デフォルト機能で解決後 26 クレート。ratatui 0.30.2 + crossterm は 67、2026-10 計測）。

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[ドキュメント] · [クイックスタート] · [ウィジェットガイド] · [パターンガイド] · [サンプル集] · [ランタイムモードガイド] · [アーキテクチャ]

[English](../README.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · **日本語** · [한국어](README.ko.md)

</div>

> このページは [英語版 README](../README.md) の要約翻訳です。内容に差異がある場合は英語版 README と英語版ガイドが正となります。

SuperLightTUI (SLT) は、公開 API の文法を意図的に小さく保った Rust 向けの immediate-mode TUI ライブラリです。
クロージャを 1 つ書けば SLT が毎フレームそれを呼び出し、レイアウト、フォーカス、差分計算、描画はライブラリが処理します。
状態は普通の Rust 変数で、アプリトレイトやメッセージ enum、保持型のウィジェットツリーを用意する必要はありません。

## クイックスタート

```sh
cargo add superlighttui
```

<!-- slt-check: {"name": "readme_ja_hello", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut slt::Context| {
        ui.text("hello, world");
    })
}
```

5 行で動きます。`App` トレイトも、`Model`/`Update`/`View` も、手書きのイベントループも不要です。
Ctrl+C でデフォルトできれいに終了します (`RunConfig::handle_ctrl_c(false)` で無効化できます)。

リポジトリをクローンしていれば `cargo run --example hello` ですぐに試せます。サンプルの一覧は [サンプル集] にあります。

## 主な特徴

- **マウスはデフォルトで有効**: クリック、ホイール、ドラッグがそのまま動作し (SSH ではクリックとホイールのみ)、ホバーは `MouseMode::Hover`、完全に無効化するなら `MouseMode::Off`。
- **Flexbox レイアウト**: ほとんどの画面は `row()`、`col()`、`container()` で組み立てます。
- **Tailwind 風チェーン**: `.bold().fg(Color::Cyan).p(1).gap(1)` のようにメソッドチェーンでスタイルを指定します。
- **Hooks**: コンポーネントローカルな状態には hook を使い、アプリ状態は普通の Rust 変数に置きます。
- **50 以上のウィジェット**: 入力、テーブル、リスト、タブ、モーダル、トースト、チャート、Markdown、コマンドパレットなど。
- **テスト**: `TestBackend` でヘッドレスに描画と操作を検証できます。
- **複数のランタイム**: フルスクリーン、inline、static + inline、ブラウザ (`slt-wasm`) で同じクロージャが動きます。
- **パフォーマンス**: 同条件の ratatui 比較ハーネスで、フレームあたりの描画が ratatui より高速です。詳細は [パフォーマンスガイド] を参照してください。

## ライブラリを学ぶ

以下のガイドは **英語** で書かれています。学習パスは順番に読んでください。各ガイドはそれ以前のガイドを前提にしています。
全ページの一覧は [ドキュメント] にあります。

| ステップ | ガイド | 内容 |
|----------|--------|------|
| 1 | [クイックスタート] | インストール、最初のアプリ、クロージャのメンタルモデル、レイアウト、ウィジェット状態 |
| 2 | [状態ガイド](guide/02-state.md) | アプリ状態・hooks・ウィジェット状態の使い分けと、前フレームのルール |
| 3 | [パターンガイド] | コンポーネント、フォーム、画面、モーダル、async メッセージ、大規模アプリの構成 |
| 4 | [インタラクションガイド](guide/04-interaction.md) | マウス、キーボード、フォーカス、ショートカット、ホバー、ドラッグ、ホイールとスクロール |
| 5 | [オーバーレイガイド](guide/05-overlays.md) | `Anchor` とオーバーレイ配置を CSS/Flutter/React Native に対応付け |
| 6 | [テーマガイド](guide/06-theming.md) | Theme 構造体、プリセット、ThemeBuilder、カスタムテーマ |
| 7 | [アニメーションガイド](guide/07-animation.md) | Tween、spring、keyframes、sequence、stagger |
| 8 | [テストガイド](guide/08-testing.md) | `TestBackend`、`EventBuilder`、複数フレームのテスト、スナップショット |
| 9 | [ランタイムモードガイド] | `run` / `run_inline` / `run_static`、`RunConfig`、`frame()` の直接駆動 |
| 10 | [ブラウザガイド](guide/10-browser.md) | `slt-wasm` ブラウザランタイム |
| 11 | [ターミナル互換性](guide/11-terminal-compatibility.md) | ターミナル/マルチプレクサ/SSH の対応表、環境変数、フォールバック |

他のスタックから来た方へ: [Web 開発者ガイド](coming-from/web.md) (HTML/CSS/React → SLT)、[Ratatui ガイド](coming-from/ratatui.md) (ウィジェット、レイアウト、段階的な `ratatui` ブリッジ)。

| リファレンス | 内容 |
|--------------|------|
| [ウィジェットガイド] | ウィジェット、ランタイムメソッド、状態型の分類別カタログ |
| [状態 API ガイド](reference/state.md) | ウィジェット状態の公開フィールド、アクセサ、変更メソッド |
| [クックブック](reference/cookbook.md) | よくあるアプリのコピペ用レシピ。各レシピは実行可能なサンプル付き |
| [サンプル集] | 製品の形と機能領域ごとにまとめた実行可能なサンプル |
| [機能ガイド](reference/features.md) | feature フラグ、オプション依存、おすすめの組み合わせ |
| [移行ガイド](reference/migration.md) | バージョンごとの破壊的変更とアップグレード手順 |
| [デバッグガイド](reference/debugging.md) | F12 オーバーレイ、クリッピング、フォーカスの落とし穴 |
| [AI ガイド](reference/ai-guide.md) | AI 支援で開発する人やコーディングエージェント向けの最短ルート |

コントリビューター向け: [設計原則](internals/design-principles.md) (API ルールと命名)、[アーキテクチャ] (モジュール構成、フレームのライフサイクル)、[パフォーマンスガイド] (予算とベンチマーク)。

## ライセンス

[MIT](../LICENSE)

<!-- Badge definitions -->
[Crate Badge]: https://img.shields.io/crates/v/superlighttui?style=flat-square&logo=rust&color=E05D44
[Docs Badge]: https://img.shields.io/docsrs/superlighttui?style=flat-square&logo=docs.rs
[CI Badge]: https://img.shields.io/github/actions/workflow/status/subinium/SuperLightTUI/ci.yml?branch=main&style=flat-square&label=CI
[MSRV Badge]: https://img.shields.io/crates/msrv/superlighttui?style=flat-square&label=MSRV
[Downloads Badge]: https://img.shields.io/crates/d/superlighttui?style=flat-square
[License Badge]: https://img.shields.io/crates/l/superlighttui?style=flat-square&color=1370D3

<!-- Link definitions -->
[CI]: https://github.com/subinium/SuperLightTUI/actions/workflows/ci.yml
[Crate]: https://crates.io/crates/superlighttui
[Docs]: https://docs.rs/superlighttui
[License]: ../LICENSE
[ドキュメント]: README.md
[クイックスタート]: guide/01-quick-start.md
[パターンガイド]: guide/03-patterns.md
[ランタイムモードガイド]: guide/09-runtime-modes.md
[ウィジェットガイド]: reference/widgets.md
[サンプル集]: reference/examples.md
[アーキテクチャ]: internals/architecture.md
[パフォーマンスガイド]: internals/performance.md
