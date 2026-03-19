<div align="center">

# SuperLightTUI

**書くのが速い。動くのが軽い。**

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[Crate] · [Docs] · [Examples] · [Contributing]

[English](../README.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · **日本語** · [한국어](README.ko.md)

</div>

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
        if ui.key('q') { ui.quit(); }
        if ui.key('k') || ui.key_code(KeyCode::Up) { count += 1; }
        if ui.key('j') || ui.key_code(KeyCode::Down) { count -= 1; }

        ui.bordered(Border::Rounded).title("Counter").pad(1).gap(1).col(|ui| {
            ui.text("Counter").bold().fg(Color::Cyan);
            ui.row(|ui| {
                ui.text("Count:");
                let c = if count >= 0 { Color::Green } else { Color::Red };
                ui.text(format!("{count}")).bold().fg(c);
            });
            ui.text("k +1 / j -1 / q quit").dim();
        });
    })
}
```

状態はクロージャの中に。レイアウトは `row()` と `col()`。スタイルはメソッドチェーン。それだけです。

## なぜ SLT か

**クロージャがそのままアプリになる** — フレームワークの状態管理なし。メッセージパッシングなし。トレイト実装なし。関数を書けば、SLT が毎フレーム呼び出します。

**すべて自動でつながる** — Tab でフォーカスが循環。マウスホイールでスクロール。コンテナはクリックとホバーを報告。ウィジェットは自分でイベントを消費します。

**CSS のようなレイアウト、Tailwind のような構文** — `row()`、`col()`、`grow()`、`gap()`、`spacer()` で Flexbox。Tailwind 風ショートハンド: `.p()`、`.px()`、`.py()`、`.m()`、`.mx()`、`.my()`、`.w()`、`.h()`、`.min_w()`、`.max_w()`。

```rust
ui.container()
    .border(Border::Rounded)
    .p(2).mx(1).grow(1).max_w(60)
    .col(|ui| {
        ui.row(|ui| {
            ui.text("left");
            ui.spacer();
            ui.text("right");
        });
    });
```

**コア依存は2つだけ** — ターミナル I/O に `crossterm`、文字幅計測に `unicode-width`。オプション: 非同期に `tokio`、シリアライズに `serde`、画像読み込みに `image`。`unsafe` コードはゼロです。

> **AI 支援開発** — [Claude Code](https://docs.anthropic.com/en/docs/claude-code) の `rust-tui-development-with-slt` スキルで完全な API リファレンス、ベストプラクティス、コード生成テンプレートを利用できます。または [tui.builders](https://tui.builders) でビジュアルデザインも可能です:

[![tui.builders demo](../assets/tui-builders-demo.gif)](https://tui.builders)

> ウィジェットをドラッグし、インスペクターでプロパティを設定して、慣用的な Rust コードをエクスポート。無料、サインアップ不要、オープンソース。

## ウィジェット

55以上の組み込みウィジェット、ボイラープレートなし:

```rust
ui.text_input(&mut name);                    // 単行入力
ui.textarea(&mut notes, 5);                  // 複数行エディタ
if ui.button("Submit").clicked { /* … */ }    // Response を返す
ui.checkbox("Dark mode", &mut dark);         // トグルチェックボックス
ui.toggle("Notifications", &mut on);         // オン/オフスイッチ
ui.tabs(&mut tabs);                          // タブナビゲーション
ui.list(&mut items);                         // 選択可能なリスト
ui.select(&mut sel);                         // ドロップダウン選択
ui.radio(&mut radio);                        // ラジオボタングループ
ui.multi_select(&mut multi);                 // 複数選択チェックボックス
ui.tree(&mut tree);                          // 展開可能なツリービュー
ui.virtual_list(&mut list, 20, |ui, i| {}); // 仮想化リスト
ui.table(&mut data);                         // データテーブル
ui.spinner(&spin);                           // ローディングアニメーション
ui.progress(0.75);                           // プログレスバー
ui.scrollable(&mut scroll).col(|ui| { });    // スクロールコンテナ
ui.toast(&mut toasts);                       // 通知
ui.separator();                              // 水平線
ui.help(&[("q", "quit"), ("Tab", "focus")]); // キーヒント
ui.link("Docs", "https://docs.rs/superlighttui");      // クリック可能なハイパーリンク (OSC 8)
ui.modal(|ui| { ui.text("overlay"); });      // 背景を暗くするモーダル
ui.overlay(|ui| { ui.text("floating"); });   // 背景を暗くしないオーバーレイ
ui.command_palette(&mut palette);            // 検索可能なコマンドパレット
ui.markdown("# Hello **world**");            // Markdown レンダリング
ui.form_field(&mut field);                   // バリデーション付きラベル入力
ui.chart(|c| { c.line(&data); c.grid(true); }, 50, 16); // 折れ線/散布図/棒グラフ
ui.scatter(&points, 50, 16);                 // 散布図
ui.histogram(&values, 40, 12);               // 自動ビニングヒストグラム
ui.bar_chart(&data, 24);                     // 水平棒グラフ
ui.sparkline(&values, 16);                   // トレンドライン ▁▂▃▅▇
ui.canvas(40, 10, |cv| { cv.circle(20, 20, 15); }); // ブレイルキャンバス
ui.grid(3, |ui| { /* 3列グリッド */ });      // グリッドレイアウト
ui.tooltip("Save the current file");         // ホバー時ツールチップ
ui.calendar(&mut cal);                       // 月ナビ付き日付ピッカー
ui.screen("home", &screens, |ui| {});        // 画面ルーティングスタック
ui.confirm("Delete?", &mut yes);             // マウス対応の確認ダイアログ
```

すべてのウィジェットが自分でキーボードイベント、フォーカス状態、マウスインタラクションを処理します。

### カスタムウィジェット

`Widget` トレイトを実装して独自ウィジェットを作成できます:

```rust
use slt::{Context, Widget, Color, Style};

struct Rating { value: u8, max: u8 }

impl Widget for Rating {
    type Response = bool;

    fn ui(&mut self, ui: &mut Context) -> bool {
        let focused = ui.register_focusable();
        let mut changed = false;

        if focused {
            if ui.key('+') && self.value < self.max { self.value += 1; changed = true; }
            if ui.key('-') && self.value > 0 { self.value -= 1; changed = true; }
        }

        let stars: String = (0..self.max)
            .map(|i| if i < self.value { '★' } else { '☆' })
            .collect();
        let color = if focused { Color::Yellow } else { Color::White };
        ui.styled(stars, Style::new().fg(color));
        changed
    }
}

// 使い方: ui.widget(&mut rating);
```

フォーカス、イベント、テーマ、レイアウト — すべて `Context` 経由でアクセス可能。トレイト1つ、メソッド1つ。

## 機能

<details>
<summary><b>レイアウト</b></summary>

| 機能 | API |
|------|-----|
| 縦方向スタック | `ui.col(\|ui\| { })` |
| 横方向スタック | `ui.row(\|ui\| { })` |
| グリッドレイアウト | `ui.grid(3, \|ui\| { })` |
| 子要素間のギャップ | `.gap(1)` |
| Flex grow | `.grow(1)` |
| 末尾へ押し出し | `ui.spacer()` |
| 整列 | `.align(Align::Center)` |
| パディング | `.p(1)`, `.px(2)`, `.py(1)` |
| マージン | `.m(1)`, `.mx(2)`, `.my(1)` |
| 固定サイズ | `.w(20)`, `.h(10)` |
| 制約 | `.min_w(10)`, `.max_w(60)` |
| パーセント指定 | `.w_pct(50)`, `.h_pct(80)` |
| 均等配置 | `.space_between()`, `.space_around()`, `.space_evenly()` |
| テキスト折り返し | `ui.text_wrap("long text...")` |
| タイトル付きボーダー | `.border(Border::Rounded).title("Panel")` |
| 辺ごとのボーダー | `.border_top(false)`, `.border_sides(BorderSides::horizontal())` |
| レスポンシブギャップ | `.gap_at(Breakpoint::Md, 2)` |

</details>

<details>
<summary><b>スタイリング</b></summary>

```rust
ui.text("styled").bold().italic().underline().fg(Color::Cyan).bg(Color::Black);
```

16の名前付きカラー · 256色パレット · 24ビット RGB · 6つの修飾子 · 6つのボーダースタイル

</details>

<details>
<summary><b>テーマ</b></summary>

```rust
// 7つの組み込みプリセット
slt::run_with(RunConfig { theme: Theme::catppuccin(), ..Default::default() }, |ui| {
    ui.set_theme(Theme::dark()); // 実行時に切り替え
});

// カスタムテーマの構築
let theme = Theme::builder()
    .primary(Color::Rgb(255, 107, 107))
    .accent(Color::Cyan)
    .build();
```

7プリセット (dark, light, dracula, catppuccin, nord, solarized_dark, tokyo_night)。15色スロット + `is_dark` フラグでカスタムテーマを作成。すべてのウィジェットが自動的に継承します。

</details>

<details>
<summary><b>スタイルレシピ</b></summary>

```rust
use slt::{ContainerStyle, Border, Color};

const CARD: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded).p(1).bg(Color::Indexed(236));

// 一度定義してどこでも適用
ui.container().apply(&CARD).col(|ui| { ... });

// 複数を合成 — 後から書いたものが優先
ui.container().apply(&CARD).grow(1).gap(2).col(|ui| { ... });
```

`const` スタイルはランタイムコストゼロ。`.apply()` チェーンで合成可能。

</details>

<details>
<summary><b>レスポンシブレイアウト</b></summary>

```rust
ui.container()
    .w(20).md_w(40).lg_w(60)  // ブレークポイントで幅が変わる
    .p(1).lg_p(2)
    .col(|ui| { ... });
```

35のブレークポイント条件付きメソッド (`xs_`, `sm_`, `md_`, `lg_`, `xl_`)。ブレークポイント: Xs (<40), Sm (40–79), Md (80–119), Lg (120–159), Xl (≥160)。

</details>

<details>
<summary><b>アニメーション</b></summary>

```rust
let mut tween = Tween::new(0.0, 100.0, 60).easing(ease_out_bounce);
let value = tween.value(ui.tick());

let mut spring = Spring::new(0.0, 180.0, 12.0);
spring.set_target(100.0);

let mut kf = Keyframes::new(120)
    .stop(0.0, 0.0).stop(0.5, 100.0).stop(1.0, 50.0)
    .loop_mode(LoopMode::PingPong);
```

9つのイージング関数を持つ Tween。スプリング物理演算。ループモード付きキーフレームタイムライン。Sequence チェーン。リストアニメーション用 Stagger。

</details>

<details>
<summary><b>非同期 (Async)</b></summary>

```rust
let tx = slt::run_async(|ui, messages: &mut Vec<String>| {
    for msg in messages.drain(..) { ui.text(msg); }
})?;
tx.send("Hello from background!".into()).await?;
```

オプションの tokio 統合。`cargo add superlighttui --features async` で有効化。

</details>

<details>
<summary><b>インラインモード</b></summary>

```rust
slt::run_inline(3, |ui| {
    ui.text("Renders below your prompt.");
    ui.text("No alternate screen.").dim();
});
```

ターミナルを占有せず、カーソルの下に固定高さの UI をレンダリングします。

</details>

<details>
<summary><b>エラーバウンダリ</b></summary>

```rust
ui.error_boundary(|ui| {
    ui.text("If this panics, the app keeps running.");
});
```

ウィジェットのパニックをキャッチしてアプリをクラッシュさせません。部分的なコマンドはロールバックされ、フォールバックがレンダリングされます。

</details>

<details>
<summary><b>画像レンダリング</b></summary>

```sh
cargo add superlighttui --features image
```

```rust
use slt::HalfBlockImage;

let photo = image::open("photo.png").unwrap();
let img = HalfBlockImage::from_dynamic(&photo, 60, 30);
ui.image(&img);
```

ハーフブロック (▀▄) 画像レンダリング。Sixel プロトコル (v0.13.2): `ui.sixel_image()` で xterm、foot、mlterm 上のピクセルパーフェクト画像表示。

</details>

<details>
<summary><b>フィーチャーフラグ</b></summary>

| フラグ | 説明 |
|--------|------|
| `async` | tokio チャンネルベースのメッセージパッシングで `run_async()` |
| `serde` | Style、Color、Theme、レイアウト型の Serialize/Deserialize |
| `image` | `image` クレートで `HalfBlockImage::from_dynamic()` |
| `full` | 上記すべて |

```toml
[dependencies]
superlighttui = { version = "0.13", features = ["full"] }
```

</details>

<details>
<summary><b>スナップショットテスト</b></summary>

```rust
use slt::TestBackend;

let mut backend = TestBackend::new(40, 10);
backend.render(|ui| {
    ui.bordered(Border::Rounded).pad(1).col(|ui| {
        ui.text("Hello");
    });
});
insta::assert_snapshot!(backend.to_string_trimmed());
```

[insta](https://crates.io/crates/insta) と組み合わせてスナップショットベースの UI リグレッションテストに使用できます。

</details>

<details>
<summary><b>デバッグ</b></summary>

SLT アプリで **F12** を押すとレイアウトデバッガーオーバーレイを切り替えられます。コンテナの境界、ネスト深度、レイアウト構造を表示します。

</details>

## サンプル

| サンプル | コマンド | 内容 |
|---------|---------|------|
| hello | `cargo run --example hello` | 最小構成 |
| counter | `cargo run --example counter` | 状態 + キーボード |
| demo | `cargo run --example demo` | 全ウィジェット |
| demo_dashboard | `cargo run --example demo_dashboard` | ライブダッシュボード |
| demo_cli | `cargo run --example demo_cli` | CLI ツールレイアウト |
| demo_spreadsheet | `cargo run --example demo_spreadsheet` | データグリッド |
| demo_website | `cargo run --example demo_website` | ターミナル内 Web サイト |
| demo_game | `cargo run --example demo_game` | Tetris + Snake + Minesweeper |
| demo_fire | `cargo run --release --example demo_fire` | DOOM 炎エフェクト (ハーフブロック) |
| demo_ime | `cargo run --example demo_ime` | 韓国語/CJK IME 入力 |
| inline | `cargo run --example inline` | インラインモード |
| anim | `cargo run --example anim` | Tween + Spring + Keyframes |
| demo_infoviz | `cargo run --example demo_infoviz` | データビジュアライゼーション |
| demo_trading | `cargo run --example demo_trading` | 取引所スタイルのトレーディング端末 |
| async_demo | `cargo run --example async_demo --features async` | バックグラウンドタスク |

## アーキテクチャ

```
Closure → Context collects Commands → build_tree() → flexbox layout → diff buffer → flush
```

各フレーム: クロージャが実行され、SLT が記述内容を収集し、Flexbox レイアウトを計算し、前フレームとの差分を取り、変更されたセルだけをフラッシュします。

Pure Rust。マクロなし、コード生成なし、ビルドスクリプトなし。

### カスタムバックエンド

SLT のレンダリングは `Backend` トレイトで抽象化されており、ターミナル以外のカスタムレンダリングターゲットを実装できます:

```rust
use slt::{Backend, AppState, Buffer, Rect, RunConfig, Context, Event};

struct MyBackend { buffer: Buffer }

impl Backend for MyBackend {
    fn size(&self) -> (u32, u32) {
        (self.buffer.area.width, self.buffer.area.height)
    }
    fn buffer_mut(&mut self) -> &mut Buffer { &mut self.buffer }
    fn flush(&mut self) -> std::io::Result<()> {
        // self.buffer をターゲット (canvas, GPU, network など) にレンダリング
        Ok(())
    }
}
```

`Backend` トレイトのメソッドは3つ: `size()`、`buffer_mut()`、`flush()`。カスタムバックエンドは完全にレンダリングされた `Buffer` を受け取り、WebGL、egui 埋め込み、SSH トンネル、テストハーネスなど好きな方法で表示できます。

### AI ネイティブウィジェット

SLT には AI/LLM ワークフロー向けの専用ウィジェットが含まれています:

| ウィジェット | 説明 |
|------------|------|
| `streaming_text()` | 点滅カーソル付きトークンバイトークンテキスト表示 |
| `streaming_markdown()` | 見出し、コードブロック、インライン書式付きストリーミング Markdown |
| `tool_approval()` | ツール呼び出しの人間によるアプローブ/リジェクト |
| `context_bar()` | アクティブなコンテキストソースを示すトークンカウンターバー |
| `markdown()` | 静的 Markdown レンダリング |
| `code_block()` | シンタックスハイライト付きコード表示 |

## コントリビューション

ガイドラインは [CONTRIBUTING.md](../CONTRIBUTING.md) を参照してください。

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
[Examples]: https://github.com/subinium/SuperLightTUI/tree/main/examples
[Contributing]: https://github.com/subinium/SuperLightTUI/blob/main/CONTRIBUTING.md
[License]: ../LICENSE
