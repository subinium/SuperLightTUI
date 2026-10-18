<div align="center">

# SuperLightTUI

**像写网页一样构建终端 UI** —— flexbox 布局、Tailwind 风格链式调用、hooks、点击/悬停/滚动，全部在一个 Rust 闭包里完成。

**写得快。依赖树轻量**（4 个直接必需依赖，默认特性下解析为 26 个 crate；ratatui 0.30.2 + crossterm 为 67 个，2026-10 测得）。

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[文档索引] · [快速开始] · [组件指南] · [模式指南] · [示例指南] · [运行模式指南] · [架构指南]

[English](../README.md) · **中文** · [Español](README.es.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

</div>

> 本文是 [英文 README](../README.md) 的摘要翻译。如有出入，以英文 README 和英文指南为准。

SuperLightTUI（SLT）是一个 Rust 即时模式（immediate-mode）TUI 库，公开语法刻意保持精简。
你只需编写一个闭包，SLT 每帧调用它，布局、焦点、差分和渲染都由库来处理。
状态就是普通的 Rust 变量，不需要 App trait、消息枚举，也不需要搭建保留式组件树。

## 快速开始

```sh
cargo add superlighttui
```

<!-- slt-check: {"name": "readme_zh_cn_hello", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut slt::Context| {
        ui.text("hello, world");
    })
}
```

5 行代码。没有 `App` trait，没有 `Model`/`Update`/`View`，没有手写事件循环。
默认按 Ctrl+C 即可干净退出（可用 `RunConfig::handle_ctrl_c(false)` 关闭）。

克隆仓库后可以直接运行 `cargo run --example hello`。完整示例列表见 [示例指南]。

## 主要特性

- **鼠标默认开启**：点击、滚轮、拖拽开箱即用（SSH 下仅点击和滚轮）；悬停需要 `MouseMode::Hover`，`MouseMode::Off` 可完全关闭鼠标。
- **Flexbox 布局**：大多数界面用 `row()`、`col()`、`container()` 组合即可。
- **Tailwind 风格链式调用**：用 `.bold().fg(Color::Cyan).p(1).gap(1)` 这样的方法链设置样式。
- **Hooks**：组件局部状态用 hook 管理，应用状态放在普通 Rust 变量里。
- **50+ 组件**：输入框、表格、列表、标签页、模态框、toast、图表、Markdown、命令面板等。
- **测试**：用 `TestBackend` 无头验证渲染和交互。
- **多种运行时**：同一个闭包可运行于全屏、inline、static + inline 以及浏览器（`slt-wasm`）。
- **性能**：在对等的 ratatui 对比基准中，SLT 每帧渲染更快；详见 [性能指南]。

## 学习本库

以下指南均为**英文**。学习路径请按顺序阅读，每篇指南都以前面的内容为基础。
[文档索引] 列出了所有页面。

| 步骤 | 指南 | 内容 |
|------|------|------|
| 1 | [快速开始] | 安装、第一个应用、闭包心智模型、布局、组件状态 |
| 2 | [状态指南](guide/02-state.md) | 应用状态、hooks 与组件状态的区别，以及上一帧规则 |
| 3 | [模式指南] | 组件、表单、屏幕、模态框、异步消息、大型应用结构 |
| 4 | [交互指南](guide/04-interaction.md) | 鼠标、键盘、焦点、快捷键、悬停、拖拽、滚轮与滚动 |
| 5 | [浮层指南](guide/05-overlays.md) | `Anchor` 与浮层定位，对照 CSS/Flutter/React Native |
| 6 | [主题指南](guide/06-theming.md) | Theme 结构体、预设、ThemeBuilder、自定义主题 |
| 7 | [动画指南](guide/07-animation.md) | Tween、spring、keyframes、sequence、stagger |
| 8 | [测试指南](guide/08-testing.md) | `TestBackend`、`EventBuilder`、多帧测试、快照 |
| 9 | [运行模式指南] | `run` / `run_inline` / `run_static`、`RunConfig`、直接驱动 `frame()` |
| 10 | [浏览器指南](guide/10-browser.md) | `slt-wasm` 浏览器运行时 |
| 11 | [终端兼容性](guide/11-terminal-compatibility.md) | 终端/复用器/SSH 支持矩阵、环境变量、回退方案 |

来自其他技术栈：[Web 开发者指南](coming-from/web.md)（HTML/CSS/React → SLT）和 [Ratatui 指南](coming-from/ratatui.md)（组件、布局以及渐进式 `ratatui` 桥接）。

| 参考 | 内容 |
|------|------|
| [组件指南] | 组件、运行时方法和状态类型的分类目录 |
| [状态 API 指南](reference/state.md) | 组件状态的公开字段、访问器和修改方法 |
| [Cookbook](reference/cookbook.md) | 常见应用的可复制配方，每个都有可运行示例 |
| [示例指南] | 按产品形态和功能领域分组的可运行示例 |
| [特性指南](reference/features.md) | feature 标志、可选依赖、推荐组合 |
| [迁移指南](reference/migration.md) | 各版本的破坏性变更和升级路径 |
| [调试指南](reference/debugging.md) | F12 叠加层、裁剪、焦点相关的意外行为 |
| [AI 指南](reference/ai-guide.md) | 面向 AI 辅助开发者和编码代理的最快路径 |

贡献者：[设计原则](internals/design-principles.md)（API 规则与命名）、[架构指南]（模块地图、帧生命周期）和 [性能指南]（性能预算与基准测试）。

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
[License]: ../LICENSE
[文档索引]: README.md
[快速开始]: guide/01-quick-start.md
[模式指南]: guide/03-patterns.md
[运行模式指南]: guide/09-runtime-modes.md
[组件指南]: reference/widgets.md
[示例指南]: reference/examples.md
[架构指南]: internals/architecture.md
[性能指南]: internals/performance.md
