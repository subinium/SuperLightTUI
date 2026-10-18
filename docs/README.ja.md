<div align="center">

# SuperLightTUI

**Web ページのようにターミナル UI を作る** — flexbox レイアウト、Tailwind 風のメソッドチェーン、hooks、クリック/ホバー/スクロールを Rust のクロージャ 1 つで。

**書くのは速く。依存ツリーは軽量**（直接の必須依存は 4 つ、デフォルト機能で解決後 26 クレート。ratatui + crossterm は 69）。

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[ドキュメント] · [クイックスタート] · [ウィジェットガイド] · [パターンガイド] · [サンプル集] · [バックエンドガイド] · [アーキテクチャ]

[English](../README.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · **日本語** · [한국어](README.ko.md)

</div>

> このページは v0.25.0 時点の [英語版 README](../README.md) の翻訳要約です。内容に差異がある場合は英語版 README が正となります。

SuperLightTUI は、公開される文法を意図的に小さく保った Rust 向けの immediate-mode TUI ライブラリです。
クロージャを 1 つ書けば、SLT が毎フレームそれを呼び出し、レイアウト、フォーカス、差分計算、レンダリングを処理します。

高速なプロダクト反復、読みやすい Rust 構文、そして真面目なバックエンド規律を両立するために設計されています。
そのため、ツールを素早く試作する人にも、ドキュメントから UI を生成するコーディングエージェントにも自然に合います。

## クイックスタート

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

5 行で始められます。`App` トレイトも、`Model`/`Update`/`View` も、手書きのイベントループも不要です。
Ctrl+C はデフォルトでループをきれいに終了します（`RunConfig::handle_ctrl_c(false)` で無効化できます）。

マウスは **デフォルトで有効** です。素の `slt::run` でクリック、ホイールスクロール、ドラッグ
（分割ハンドル、スクロールバー、テキスト選択）が届きます。SSH 越しではクリックとホイールのみになります。
Tab/Shift+Tab でのフォーカス移動と Enter/Space でのボタン操作は常に使えます。

```rust
use slt::Context;

fn main() -> std::io::Result<()> {
    let mut clicks = 0;

    slt::run(|ui: &mut Context| {
        ui.text(format!("clicked {clicks} times")).bold();
        if ui.button("Click me").clicked {
            clicks += 1;
        }
    })
}
```

ホバーのスタイルとツールチップはすべてのポインタ移動を必要とするため、別途有効にします:
`slt::run_with(RunConfig::default().mouse_mode(MouseMode::Hover), |ui| { ... })`。
マウスをキャプチャしている間も、多くのターミナルでは Shift+ドラッグでネイティブのテキスト選択ができます。
`MouseMode::Off` でマウスを完全に無効化できます。

MSRV: Rust 1.88。デフォルト feature は `crossterm` バックエンドと `bidi` です。

## ショーケース

<table>
  <tr>
    <td align="center"><img src="../assets/demo.png" alt="Widget Demo" /><br/><b>Widget Demo</b><br/><sub><code>cargo run --example demo</code></sub></td>
    <td align="center"><img src="../assets/demo_dashboard.png" alt="Dashboard" /><br/><b>Dashboard</b><br/><sub><code>cargo run --example showcase_tour</code></sub></td>
    <td align="center"><img src="../assets/demo_website.png" alt="Website" /><br/><b>Website Layout</b><br/><sub><code>cargo run --example showcase_tour</code></sub></td>
  </tr>
  <tr>
    <td align="center"><img src="../assets/demo_spreadsheet.png" alt="Spreadsheet" /><br/><b>Spreadsheet</b><br/><sub><code>cargo run --example showcase_tour</code></sub></td>
    <td align="center"><img src="../assets/demo_game.gif" alt="Games" /><br/><b>Games</b><br/><sub><code>cargo run --example canvas_tour</code></sub></td>
    <td align="center"><img src="../assets/demo_fire.gif" alt="DOOM Fire" /><br/><b>DOOM Fire Effect</b><br/><sub><code>cargo run --example canvas_tour</code></sub></td>
  </tr>
  <tr>
    <td align="center" colspan="3"><img src="../assets/demo_pretext.gif" alt="Pretext Reflow" /><br/><b><a href="https://github.com/chenglou/pretext">Pretext</a> Reflow</b> — マウスカーソルの周りでテキストがリアルタイムに再配置されます<br/><sub><code>cargo run --example text_tour</code></sub></td>
  </tr>
</table>

## 60 秒でわかる文法

多くのアプリは次の 4 つから始まります。

1. 状態は普通の Rust の変数や構造体に置く。
2. レイアウトは主に `row()`、`col()`、`container()` で組む。
3. スタイルはメソッドチェーンで付ける。
4. インタラクティブなウィジェットはたいてい `Response` を返す。

```rust
ui.bordered(Border::Rounded).title("Status").p(1).gap(1).col(|ui| {
    ui.text("SLT").bold().fg(Color::Cyan);
    ui.row(|ui| {
        ui.text("mode:");
        ui.text("ready").fg(Color::Green);
        ui.spacer();
        if ui.button("Quit").clicked {
            ui.quit();
        }
    });
});
```

これが中核となる mental model です。残りは別フレームワークではなく、深さと拡張です。
`clicked` はボタンにフォーカスがあるときの Enter/Space で立ち、マウスクリックでも立ちます（マウスはデフォルトで有効）。

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

        ui.bordered(Border::Rounded).title("Counter").p(1).gap(1).col(|ui| {
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

## ランタイムモード

同じクロージャが複数のエントリポイントで動きます。アプリの規模ではなく UI の形で選んでください。

| モード | API | 用途 |
|--------|-----|------|
| フルスクリーン | `slt::run` / `slt::run_with` | alternate screen を使う標準的な TUI アプリ。マウスのクリック、ホイール、ドラッグはデフォルトで有効。ホバーには `RunConfig::mouse_mode(MouseMode::Hover)` が必要です。 |
| Inline | `slt::run_inline` / `slt::run_inline_with` | プロンプトの下に固定高さのウィジェットを描画。alternate screen なし。 |
| Static + inline | `slt::run_static` | ログ行を scrollback に流しつつ、その下で inline UI を動かし続けます。 |
| 非同期メッセージ | `slt::run_async` *(feature: `async`)* | バックグラウンドタスクが wake-aware な `AsyncSender` でメッセージを送ります。返される `AsyncRunHandle` を保持して join/cancel します。 |
| カスタムバックエンド | `slt::frame` + `Backend` + `AppState` | テスト、GUI への埋め込み、WASM、スナップショットハーネスなど、描画を自分で駆動する場合。 |

`RunConfig` で、マウスモード（デフォルト: ローカルは `Drag`、SSH 越しは `Click`。必要なら `Hover` / `Off`）、kitty keyboard、color depth、最大 FPS、スクロール速度、テーマ、タイトル、Ctrl+C の扱いをすべてのモードで調整できます。

## フォーカスとスクロール

スクロールコンテナは、フォーカス、キャレット、レイアウトが変わるとフォーカス中のウィジェットが見えるようにスクロールします（入れ子の横/縦ビューやモーダルフォームも含む）。
その間に手動でスクロールした位置は保持されます。位置をアプリ側で制御したい場合は `scroll.follow_focus = false` を設定してください。コンテナごとに `ScrollState` を 1 つ持たせてください。

フィールドや入力のインタラクション結果が欲しい場合は `ui.form_field_response(...)` を使います。既存の `ui.form_field(...)` 呼び出しはそのまま動きます。
`ui.focused_layout_rect()` と `ui.measured_layout_rect(name)` はスクロール・クリッピング前の前フレームのジオメトリを返し、`Response.rect` は画面上に見えているマウスのヒット領域です。
座標とタイミングのルールは [マイグレーションガイド] を参照してください。

## Feature Flags

```toml
[dependencies]
superlighttui = { version = "0.25.0", features = ["async", "image"] }
```

| Feature | 追加されるもの |
|---------|----------------|
| `async` | `tokio` + バックグラウンドのメッセージループ用 `run_async` |
| `bidi` | UAX #9 の双方向テキスト並べ替え。デフォルトで有効 |
| `serde` | 一部の状態型への `Serialize`/`Deserialize` |
| `theme-watch` | TOML テーマのホットリロード（`serde` + ファイルシステム watcher） |
| `image` | `ui.image` 用の PNG/JPEG デコード |
| `qrcode` | QR コード描画 |
| `kitty-compress` | Kitty 画像プロトコルの zlib 圧縮 |
| `syntax` | 同梱の全言語に対する tree-sitter ハイライト |
| `syntax-<lang>` | 単一言語のハイライト（`rust`、`python`、`typescript` など） |
| `full` | `crossterm + async + serde + theme-watch + image + qrcode + kitty-compress + bidi`。`syntax` は **含みません**（文法が重いため明示的にオプトイン）。 |

## なぜ SLT か

- **公開文法が小さい**。多くの画面は普通の Rust の状態、`row()` / `col()` / `container()`、メソッドチェーン、`Response` で始められます。
- **フレームワークの儀式が少ない**。動き始めるために app trait、retained tree、message enum を先に用意しなくてよいケースが多いです。
- **部品は充実していて、バックエンドは真面目**。共通ウィジェットが focus、hover、click、scroll を自動で結び付け（クリック、ホイール、ドラッグはデフォルト、ホバーは `MouseMode::Hover`）、内部ランタイムは `Backend`、`AppState`、`frame()` を通して保守的な低レベル経路を維持します。
- **内部実装も保守的に固めています**。shared frame kernel、明示的な backend contract test、zero `unsafe`、feature-gated runtime path、`all-features`/`no-default-features`/WASM/clippy/examples/cargo-hack/semver/deny の検証で品質を固定しています。

Rust ユーザーにとっては、retained-mode の TUI フレームワークより初期設定が少なくて済むことが多いです。
AI 支援ワークフローにとっては、ドキュメントとサンプルから公開文法を推測しやすいという意味でもあります。

SLT は、Rust の型安全性やバックエンドの escape hatch を保ったまま、ターミナルアプリを素早く作りたいときに特に向いています。
一方で、retained component tree を主軸にしたい場合や GUI-first のツールキットが欲しい場合は、別のライブラリのほうが適しているかもしれません。

## レンダリングの仕組み

SLT のレンダリングパイプラインが、文法を小さく保てる理由です。
あなたのコードが触れるのは最初のステージだけ — 残りはエンジンが処理します。

```mermaid
graph LR
    subgraph your_code ["Your Code"]
        A["Closure"]
    end
    subgraph engine ["SLT Engine"]
        B[Commands] --> C[Build Tree] --> D[Flexbox] --> E[Collect] --> F[Render] --> G["Diff + Flush"]
    end
    A -->|"records intent"| B
    G -.->|"prev-frame feedback"| A
```

`ui.*()` の呼び出しはコマンドをフラットリストに記録するだけ — ツリー構築もレイアウト計算もしません。
エンジンがそれらのコマンドを**4 ステージの DFS パイプライン**に通します — 各ステージが専門化されています: レイアウトツリー構築、flexbox 計算、インタラクションとフィードバックデータの収集、バックバッファへのセル描画 — その後、前フレームとの diff を取り、変更分だけ flush します。

このアーキテクチャが文法をシンプルにできる理由です:

- **儀式がありません。** Immediate-mode なので `App` トレイト、`Model`/`Message`/`Update`/`View` は不要です。クロージャが UI そのもの。状態は普通の Rust 変数、制御フローは `if`/`for` です。
- **レイアウトが見えません。** `ui.col(|ui| { ... })` は「カラムを開く」コマンドを記録します。エンジンがツリーを構築し flexbox を実行 — `LayoutNode` は露出しません。
- **パフォーマンスが自動です。** ダブルバッファがフレーム間でセルを比較し、変更された ANSI 属性だけを出力します。毎フレーム全体を再描画しても、エンジンが高速に処理します。手動の dirty tracking は不要です。
- **インタラクションが自動接続されます。** `ui.button("Save")` だけでキーボードフォーカスと Enter/Space による起動が得られ、マウスクリックも最初から報告され、`MouseMode::Hover` では hover も報告されます。ヒットテストのコードを書く必要はありません。
- **同期的フィードバック。** インタラクションは前フレームのレイアウト位置を使います（60 FPS では知覚不可能）。コールバックも async レイアウトクエリも不要 — コードはリニアなままです。

完全な 8 ステージのライフサイクルは [アーキテクチャ] を参照してください。

## よく使う API

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

// 可視化
ui.chart(|c| {
    c.line(&data);
    c.grid(true);
}, 50, 16);
ui.sparkline(&values, 16);
ui.canvas(40, 10, |cv| {
    cv.circle(20, 20, 15);
});
```

分類済みの一覧は [ウィジェットガイド]、組み合わせ方の考え方は [パターンガイド] を参照してください。

## ライブラリを学ぶ

| ドキュメント | 内容 |
|-------------|------|
| [ドキュメント] | ドキュメント全体の構造とガイドマップ |
| [クイックスタート] | インストール、最初のアプリ、クロージャの mental model、レイアウト、ウィジェット状態 |
| [ウィジェットガイド] | ウィジェット、ランタイムメソッド、状態型の完全カタログ |
| [パターンガイド] | 状態配置、画面分割、ヘルパー抽出、大きなアプリの構造 |
| [サンプル集] | プロダクト形状や機能別に整理した実行可能サンプル |
| [バックエンドガイド] | `Backend`、`AppState`、`frame()`、inline mode、static output |
| [テストガイド] | `TestBackend`、`EventBuilder`、multi-frame test、backend contract test |
| [デバッグガイド] | F12 オーバーレイ、clipping、focus の意外な挙動、previous-frame behavior |
| [AIガイド] | AI ビルダーやコーディングエージェント向けの最短導線 |
| [アーキテクチャ] | モジュールマップ、フレームライフサイクル、layout/render パイプライン |
| [機能フラグガイド] | feature flag、optional dependency、推奨組み合わせ |
| [アニメーションガイド] | Tween、spring、keyframe、sequence、stagger |
| [テーマガイド] | Theme struct、preset、ThemeBuilder、カスタムテーマ |
| [設計原則] | API の制約と設計哲学 |
| [状態 API ガイド] | ウィジェット状態の公開フィールド、アクセサ、変更メソッド |
| [マイグレーションガイド] | バージョンごとの破壊的変更とアップグレード手順 |
| [クックブック] | よくあるアプリのレシピ集。各レシピは実行可能なサンプルとしても提供 |
| [Web 開発者向けガイド] | Web から来た人向け: CSS/React → SLT の対応表 |
| [インタラクションガイド] | マウス、クリック、ホバー、ドラッグ、ホイール、展開/折りたたみ、ヒットテスト |
| [ポジショニングガイド] | `Anchor` とオーバーレイ API を CSS/Flutter/React Native の概念に対応付け |
| [前フレームガイド] | インタラクションが前フレームのレイアウトを使う理由、`Response.rect` が 0 になる場合、1 フレーム目の正しさ |
| [ターミナル互換性] | ターミナル/マルチプレクサ/SSH の対応表、環境変数、フォールバック |
| [API 設計] | 新しいウィジェットのための 5 つの一貫性ルールと PR レビューチェックリスト |

## 代表的なサンプル

| サンプル | コマンド | 焦点 |
|---------|---------|------|
| `hello` | `cargo run --example hello` | 最小のアプリ |
| `counter` | `cargo run --example counter` | 状態 + キーボード入力 |
| `demo` | `cargo run --example demo` | 幅広いウィジェットツアー |
| `showcase_tour` | `cargo run --example showcase_tour` | ダッシュボード、デザインシステム、infoviz、トレーディング、スプレッドシート、テーブル、Web サイト風レイアウト |
| `cookbook_tour` | `cargo run --example cookbook_tour` | フォーム、テーブル、モーダル、ファイルピッカー、ダッシュボードのレシピ |
| `text_tour` | `cargo run --example text_tour` | CJK 描画、IME 入力、Pretext リフロー、cargo 風 CLI |
| `canvas_tour` | `cargo run --example canvas_tour` | DOOM fire、ゲーム（tetris/snake/minesweeper）、raw draw、Kitty 画像、アニメーション |
| `system_tour` | `cargo run --example system_tour --features async` | error boundary と overlay anchor のライブタブ。async/inline モードはコードで解説 |
| `widgets_tour` | `cargo run --example widgets_tour` | フック、テーマ、モーダルのフォーカストラップ、レイアウトヘルパー、static output（`run_static`） |
| `interaction_tour` | `cargo run --example interaction_tour` | クリック / ダブルクリック / スクロールの `Response` シグナル、`KeyMap`、フォーカス移動 |

リリースごとの feature ツアーやショーケースを含む分類済みの完全な一覧は [サンプル集] にあります。

### デモランチャー

`scripts/ghostty_demos.sh` はデモを新しい Ghostty ウィンドウで開きます。複数のデモを並べて眺めるのに便利です。

```bash
./scripts/ghostty_demos.sh             # 対話的に選択
./scripts/ghostty_demos.sh --features  # feature ツアー一式
./scripts/ghostty_demos.sh --showcase  # 統合ショーケースのみ
```

## カスタムウィジェットとバックエンド

- 再利用できる高レベルの部品が欲しいなら `Widget` を実装します。
- ターミナル以外の描画先、外部イベントループ、組み込みランタイムが必要なら `Backend` を実装して `frame()` を駆動します。
- ヘッドレスな描画確認と安定したインタラクションテストには `TestBackend` を使います。

escape hatch が必要になっても、公開文法は小さいままです。

## コントリビューション

[コントリビュート] を読んだ後、[設計原則] と [アーキテクチャ] を参照してください。
リリース工程では format、check、clippy、test、example、backend gate が通り続けることを前提にしています。

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
[設計原則]: DESIGN_PRINCIPLES.md
[コントリビュート]: ../CONTRIBUTING.md
[License]: ../LICENSE
[状態 API ガイド]: STATE_APIS.md
[マイグレーションガイド]: MIGRATION.md
[API 設計]: API_DESIGN.md
[クックブック]: COOKBOOK.md
[Web 開発者向けガイド]: WEB_DEVELOPERS.md
[インタラクションガイド]: INTERACTION.md
[ポジショニングガイド]: POSITIONING.md
[前フレームガイド]: PREVIOUS_FRAME_GUIDE.md
[ターミナル互換性]: TERMINAL_COMPATIBILITY.md
