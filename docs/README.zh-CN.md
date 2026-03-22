<div align="center">

# SuperLightTUI

**写得飞快。跑得极轻。**

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[文档索引] · [快速开始] · [组件指南] · [示例指南] · [模式指南] · [架构指南] · [贡献指南]

[English](../README.md) · **中文** · [Español](README.es.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

</div>

SuperLightTUI 是一个 Rust 的即时模式 TUI 库。
你只需要写一个闭包，SLT 每帧调用它，库负责布局、差分、焦点和渲染。

## 效果展示

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

## 快速开始

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

5 行代码。没有 `App` 结构体，没有 `Model`/`Update`/`View`，没有事件循环。Ctrl+C 直接退出。

## 一个真实的应用

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

状态存在于你的闭包中。布局用 `row()` 和 `col()`。样式链式调用。就这么简单。

## 为什么选择 SLT

- **闭包即应用** — 没有框架状态，没有 trait 实现样板代码，没有消息循环 API。
- **CSS 布局，Tailwind 语法** — `row()`、`col()`、`gap()`、`grow()`、`spacer()`，以及 `.p()`、`.px()`、`.m()`、`.w()`、`.max_w()` 等简写。
- **组件自动处理繁琐部分** — 焦点顺序、悬停、点击处理、滚动和常见键盘行为全部内置。
- **小核心，可选扩展** — 核心依赖是 `unicode-width` 和 `compact_str`；终端 I/O 由可选的 `crossterm` 提供；async、serde、image、qrcode 和语法高亮均在 feature flag 后面。
- **库的工程素养** — 零 `unsafe`，显式 feature flag，文档、示例、测试齐全，遵循语义化版本发布纪律。

## 常用 API 概览

```rust
// 文本和布局
ui.text("Hello").bold().fg(Color::Cyan);
ui.row(|ui| {
    ui.text("left");
    ui.spacer();
    ui.text("right");
});

// 输入和操作
ui.text_input(&mut name);
if ui.button("Save").clicked {}
ui.checkbox("Dark mode", &mut dark);

// 数据和导航
ui.tabs(&mut tabs);
ui.list(&mut items);
ui.table(&mut data);
ui.command_palette(&mut palette);

// 浮层和富文本输出
ui.toast(&mut toasts);
ui.modal(|ui| {
    ui.text("Confirm?").bold();
});
ui.markdown("# Hello **world**");

// 数据可视化
ui.chart(|c| {
    c.line(&data);
    c.grid(true);
}, 50, 16);
ui.sparkline(&values, 16);
ui.canvas(40, 10, |cv| {
    cv.circle(20, 20, 15);
});
```

完整分类组件列表请参阅 [组件指南]。

## 学习指南

| 文档 | 涵盖内容 |
|------|----------|
| [文档索引] | 完整文档结构和指南索引 |
| [快速开始] | 安装、第一个应用、计数器、布局、输入 |
| [组件指南] | 组件目录和主要 API |
| [模式指南] | 状态管理、表单、覆盖层、异步、自定义组件 |
| [示例指南] | 示例索引和运行命令 |
| [架构指南] | 模块映射、帧生命周期、依赖流 |
| [后端指南] | `Backend`、`AppState`、`frame()`、内联模式 |
| [测试指南] | `TestBackend`、`EventBuilder`、交互测试 |
| [调试指南] | F12 覆盖层、裁剪、单帧延迟 |
| [AI 指南] | AI 编程助手快速参考 |
| [动画指南] | Tween、Spring、Keyframes、Sequence、Stagger |
| [主题指南] | 主题预设、ThemeBuilder、自定义主题 |
| [特性指南] | 功能标志、可选依赖、推荐组合 |
| [`docs/DESIGN_PRINCIPLES.md`](DESIGN_PRINCIPLES.md) | API 设计理念 |

## 示例精选

| 示例 | 命令 | 重点 |
|------|------|------|
| hello | `cargo run --example hello` | 最简应用 |
| counter | `cargo run --example counter` | 状态 + 键盘输入 |
| demo | `cargo run --example demo` | 全组件概览 |
| demo_dashboard | `cargo run --example demo_dashboard` | 仪表盘布局 |
| demo_cli | `cargo run --example demo_cli` | CLI 工具布局 |
| demo_infoviz | `cargo run --example demo_infoviz` | 图表和数据可视化 |
| demo_game | `cargo run --example demo_game` | 即时模式交互 |
| async_demo | `cargo run --example async_demo --features async` | 后台消息 |

完整分类索引请参阅 [示例指南]。

## 自定义组件和后端

- 实现 `Widget` trait 来构建可复用组件，完整支持焦点、布局、事件和主题。
- 实现 `Backend` + 驱动 `frame()`，可用于非终端渲染器、测试工具或嵌入式目标。
- 使用 `TestBackend` 进行无头渲染和快照式断言。

更多内容请参阅 [模式指南] 和 [架构指南]。

## 贡献

请先阅读 [贡献指南]，然后查看 `docs/DESIGN_PRINCIPLES.md` 和 [架构指南]。
发布和 CI 流程要求格式化、检查、clippy、测试和示例编译保持通过。

## 许可证

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
[文档索引]: README.md
[快速开始]: QUICK_START.md
[组件指南]: WIDGETS.md
[示例指南]: EXAMPLES.md
[模式指南]: PATTERNS.md
[架构指南]: ARCHITECTURE.md
[后端指南]: BACKENDS.md
[测试指南]: TESTING.md
[调试指南]: DEBUGGING.md
[AI 指南]: AI_GUIDE.md
[动画指南]: ANIMATION.md
[主题指南]: THEMING.md
[特性指南]: FEATURES.md
[贡献指南]: ../CONTRIBUTING.md
[License]: ../LICENSE
