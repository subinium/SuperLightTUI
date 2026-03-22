<div align="center">

# SuperLightTUI

**書くのが速い。動くのが軽い。**

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[ドキュメント] · [クイックスタート] · [ウィジェットガイド] · [サンプル集] · [パターンガイド] · [アーキテクチャ] · [コントリビュート]

[English](../README.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · **日本語** · [한국어](README.ko.md)

</div>

SuperLightTUI は Rust 用のイミディエイトモード TUI ライブラリです。
クロージャを1つ書くだけで、SLT が毎フレームそれを呼び出し、レイアウト、差分、フォーカス、レンダリングを処理します。

## ショーケース

<table>
  <tr>
    <td align="center"><img src="../assets/demo.png" alt="Widget Demo" /><br/><b>Widget Demo</b><br/><sub><code>cargo run --example demo</code></sub></td>
    <td align="center"><img src="../assets/demo_dashboard.png" alt="Dashboard" /><br/><b>Dashboard</b><br/><sub><code>cargo run --example demo_dashboard</code></sub></td>
    <td align="center"><img src="../assets/demo_website.png" alt="Website" /><br/><b>Website Layout</b><br/><sub><code>cargo run --example demo_website</code></sub></td>
  </tr>
  <tr>
    <td align="center"><img src="../assets/demo_spreadsheet.png" alt="Spreadsheet" /><br/><b>Spreadsheet</b><br/><sub><code>cargo run --example demo_spreadsheet</code></sub></td>
    <td align="center"><img src="../assets/demo_game.gif" alt="Games" /><br/><b>Games</b><br/><sub><code>cargo run --example demo_game</code></sub></td>
    <td align="center"><img src="../assets/demo_fire.gif" alt="DOOM Fire" /><br/><b>DOOM Fire Effect</b><br/><sub><code>cargo run --release --example demo_fire</code></sub></td>
  </tr>
</table>

## はじめに

```sh
cargo add superlighttui
```

```rust
fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut slt::Context| {
        ui.text("hello, world");
    })
}
```

5行。`App` 構造体なし。`Model`/`Update`/`View` なし。イベントループなし。Ctrl+C はそのまま動きます。

## 実際のアプリ

```rust
use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let mut count: i32 = 0;

    slt::run(|ui: &mut Context| {
        if ui.key('q') {
            ui.quit();
        }
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            count += 1;
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            count -= 1;
        }

        ui.bordered(Border::Rounded).title("Counter").pad(1).gap(1).col(|ui| {
            ui.text("Counter").bold().fg(Color::Cyan);
            ui.row(|ui| {
                ui.text("Count:");
                let color = if count >= 0 { Color::Green } else { Color::Red };
                ui.text(format!("{count}")).bold().fg(color);
            });
            ui.text("k +1 / j -1 / q quit").dim();
        });
    })
}
```

状態はクロージャの中に。レイアウトは `row()` と `col()`。スタイルはメソッドチェーン。それだけです。

## なぜ SLT か

- **クロージャがそのままアプリになる** — フレームワークの状態管理なし、トレイト実装のボイラープレートなし、メッセージループ API なし。
- **CSS のようなレイアウト、Tailwind のような構文** — `row()`、`col()`、`gap()`、`grow()`、`spacer()`。ショートハンド: `.p()`、`.px()`、`.m()`、`.w()`、`.max_w()`。
- **ウィジェットが面倒な部分を自動処理** — フォーカス順序、ホバー、クリック処理、スクロール、一般的なキーボード操作が組み込み済み。
- **小さなコア、オプションの拡張** — コア依存は `unicode-width` と `compact_str`。ターミナル I/O はオプションの `crossterm`。async、serde、image、qrcode、シンタックスハイライトはフィーチャーフラグで提供。
- **ライブラリの品質にこだわる** — `unsafe` ゼロ、明示的なフィーチャーフラグ、ドキュメント、サンプル、テスト、semver に基づくリリース規律。

## 主要 API

```rust
// テキストとレイアウト
ui.text("Hello").bold().fg(Color::Cyan);
ui.row(|ui| {
    ui.text("left");
    ui.spacer();
    ui.text("right");
});

// 入力とアクション
ui.text_input(&mut name);
if ui.button("Save").clicked {}
ui.checkbox("Dark mode", &mut dark);

// データとナビゲーション
ui.tabs(&mut tabs);
ui.list(&mut items);
ui.table(&mut data);
ui.command_palette(&mut palette);

// オーバーレイとリッチ出力
ui.toast(&mut toasts);
ui.modal(|ui| {
    ui.text("Confirm?").bold();
});
ui.markdown("# Hello **world**");

// ビジュアライゼーション
ui.chart(|c| {
    c.line(&data);
    c.grid(true);
}, 50, 16);
ui.sparkline(&values, 16);
ui.canvas(40, 10, |cv| {
    cv.circle(20, 20, 15);
});
```

ウィジェットの分類一覧は [ウィジェットガイド] を参照してください。

## ライブラリガイド

| ドキュメント | 内容 |
|-------------|------|
| [ドキュメント] | ドキュメント構造とガイドマップ |
| [クイックスタート] | インストール、初めてのアプリ、カウンター、レイアウト |
| [ウィジェットガイド] | ウィジェットカタログと主要 API |
| [パターンガイド] | 状態管理、フォーム、オーバーレイ、非同期、カスタムウィジェット |
| [サンプル集] | サンプルインデックスと実行コマンド |
| [アーキテクチャ] | モジュールマップ、フレームライフサイクル |
| [バックエンドガイド] | `Backend`、`AppState`、`frame()`、インラインモード |
| [テストガイド] | `TestBackend`、`EventBuilder`、インタラクションテスト |
| [デバッグガイド] | F12 オーバーレイ、クリッピング、1フレーム遅延 |
| [AIガイド] | AI コーディングエージェント向けクイックリファレンス |
| [アニメーションガイド] | Tween、Spring、Keyframes、Sequence、Stagger |
| [テーマガイド] | テーマプリセット、ThemeBuilder、カスタムテーマ |
| [機能フラグガイド] | フィーチャーフラグ、オプション依存関係 |
| [`docs/DESIGN_PRINCIPLES.md`](DESIGN_PRINCIPLES.md) | API がこの形になった理由 |

## サンプルハイライト

| サンプル | コマンド | 内容 |
|---------|---------|------|
| hello | `cargo run --example hello` | 最小構成のアプリ |
| counter | `cargo run --example counter` | 状態 + キーボード入力 |
| demo | `cargo run --example demo` | ウィジェット全体ツアー |
| demo_dashboard | `cargo run --example demo_dashboard` | ダッシュボードレイアウト |
| demo_cli | `cargo run --example demo_cli` | CLI ツールレイアウト |
| demo_infoviz | `cargo run --example demo_infoviz` | チャートとデータビジュアライゼーション |
| demo_game | `cargo run --example demo_game` | イミディエイトモードのインタラクション |
| async_demo | `cargo run --example async_demo --features async` | バックグラウンドメッセージ |

すべてのサンプルの分類インデックスは [サンプル集] にあります。

## カスタムウィジェットとバックエンド

- `Widget` トレイトを実装して、フォーカス、レイアウト、イベント、テーマにフルアクセスできる再利用可能なウィジェットを構築できます。
- `Backend` を実装し `frame()` を駆動すれば、ターミナル以外のレンダラー、テストハーネス、組み込みターゲットに対応できます。
- `TestBackend` でヘッドレスレンダリングとスナップショット形式のアサーションが可能です。

詳細は [パターンガイド] と [アーキテクチャ] を参照してください。

## コントリビューション

[コントリビュート] を読んでから、`docs/DESIGN_PRINCIPLES.md` と [アーキテクチャ] を参照してください。
リリースと CI プロセスでは、フォーマット、チェック、clippy、テスト、サンプルのコンパイルがすべて通ることが求められます。

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
[ドキュメント]: README.md
[クイックスタート]: QUICK_START.md
[ウィジェットガイド]: WIDGETS.md
[サンプル集]: EXAMPLES.md
[パターンガイド]: PATTERNS.md
[アーキテクチャ]: ARCHITECTURE.md
[バックエンドガイド]: BACKENDS.md
[テストガイド]: TESTING.md
[デバッグガイド]: DEBUGGING.md
[AIガイド]: AI_GUIDE.md
[アニメーションガイド]: ANIMATION.md
[テーマガイド]: THEMING.md
[機能フラグガイド]: FEATURES.md
[コントリビュート]: ../CONTRIBUTING.md
[License]: ../LICENSE
