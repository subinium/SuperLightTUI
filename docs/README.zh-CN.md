<div align="center">

# SuperLightTUI

**像写网页一样构建终端 UI** —— flexbox 布局、Tailwind 风格链式调用、hooks、点击/悬停/滚动，全部在一个 Rust 闭包里完成。

**写得快。依赖树轻量**（4 个直接必需依赖，默认特性下解析为 26 个 crate；ratatui + crossterm 为 69 个）。

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[文档索引] · [快速开始] · [组件指南] · [模式指南] · [示例指南] · [后端指南] · [架构指南]

[English](../README.md) · **中文** · [Español](README.es.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

</div>

> 本文是 v0.25.0 版 [英文 README](../README.md) 的翻译摘要。如有出入，以英文 README 为准。

SuperLightTUI 是一个为 Rust 提供的 immediate-mode TUI 库，并且有意把公开语法保持得很小。
你只需要写一个闭包，SLT 会在每一帧调用它，库本身负责布局、焦点、差分和渲染。

它面向快速产品迭代、易读的 Rust 语法，以及认真的后端纪律。
这让它同样适合快速做工具原型的人，也适合根据文档生成 UI 的 coding agent。

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

5 行代码就能启动。没有 `App` trait，没有 `Model`/`Update`/`View`，也不需要手写事件循环。
默认情况下 Ctrl+C 会干净地退出循环（可用 `RunConfig::handle_ctrl_c(false)` 关闭）。

鼠标捕获需要**手动开启（opt-in）**。直接使用 `slt::run` 时以键盘操作为主：Tab/Shift+Tab 切换焦点，
Enter/Space 触发当前聚焦的按钮。如果需要点击、悬停、滚轮滚动和拖拽，
请用 `RunConfig::default().mouse(true)` 启动循环：

```rust
use slt::{Context, RunConfig};

fn main() -> std::io::Result<()> {
    let mut clicks = 0;

    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        ui.text(format!("clicked {clicks} times")).bold();
        if ui.button("Click me").clicked {
            clicks += 1;
        }
    })
}
```

MSRV：Rust 1.88。默认特性启用 `crossterm` 后端和 `bidi`。

## 效果展示

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
    <td align="center" colspan="3"><img src="../assets/demo_pretext.gif" alt="Pretext Reflow" /><br/><b><a href="https://github.com/chenglou/pretext">Pretext</a> Reflow</b> — 文本围绕鼠标光标实时重排<br/><sub><code>cargo run --example text_tour</code></sub></td>
  </tr>
</table>

## 60 秒理解语法

大多数应用都从四个概念开始：

1. 状态放在普通的 Rust 变量或结构体里。
2. 布局主要靠 `row()`、`col()` 和 `container()`。
3. 样式通过链式方法调用完成。
4. 交互组件通常返回 `Response`。

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

这就是核心 mental model。剩下的是深度，不是第二套框架。
按钮获得焦点时按 Enter/Space 会触发 `clicked`；用 `RunConfig::default().mouse(true)` 启动时，鼠标点击也会触发。

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

## 运行模式

同一个闭包可以在多个入口点运行。按 UI 形态选择，而不是按应用大小。

| 模式 | API | 适用场景 |
|------|-----|----------|
| 全屏 | `slt::run` / `slt::run_with` | 使用 alternate screen 的标准 TUI 应用。只有向 `run_with` 传入 `RunConfig::mouse(true)` 时才开启鼠标捕获。 |
| Inline | `slt::run_inline` / `slt::run_inline_with` | 在提示符下方渲染固定高度的组件，不使用 alternate screen。 |
| Static + inline | `slt::run_static` | 日志行流入 scrollback，下方的 inline UI 保持实时更新。 |
| 异步消息 | `slt::run_async` *（特性：`async`）* | 后台任务通过可唤醒的 `AsyncSender` 发送消息；保留返回的 `AsyncRunHandle` 以便 join 或取消循环。 |
| 自定义后端 | `slt::frame` + `Backend` + `AppState` | 自己驱动渲染：测试、嵌入 GUI、WASM、快照测试工具。 |

`RunConfig` 可在所有模式下调整鼠标捕获（默认：关闭）、kitty keyboard、色深、最大 FPS、滚动速度、主题、标题以及 Ctrl+C 处理。

## 焦点与滚动

当焦点、光标或布局变化时，可滚动容器会自动滚动以显示聚焦的组件，包括嵌套的横向/纵向视图和模态表单。
两次变化之间的手动滚动位置会被保留；如果需要由应用控制位置，请设置 `scroll.follow_focus = false`。每个容器使用一个 `ScrollState`。

需要字段/输入的交互反馈时，使用 `ui.form_field_response(...)`，现有的 `ui.form_field(...)` 调用无需修改。
`ui.focused_layout_rect()` 和 `ui.measured_layout_rect(name)` 返回滚动和裁剪之前的上一帧几何信息；`Response.rect` 仍是屏幕上可见的鼠标命中区域。
坐标和时序规则见[迁移指南]。

## 特性开关

```toml
[dependencies]
superlighttui = { version = "0.25.0", features = ["async", "image"] }
```

| 特性 | 新增内容 |
|------|----------|
| `async` | `tokio` + 用于后台消息循环的 `run_async` |
| `bidi` | UAX #9 双向文本重排；默认启用 |
| `serde` | 部分状态类型的 `Serialize`/`Deserialize` |
| `theme-watch` | TOML 主题热重载（`serde` + 文件系统监听） |
| `image` | `ui.image` 所需的 PNG/JPEG 解码 |
| `qrcode` | 二维码渲染 |
| `kitty-compress` | Kitty 图像协议的 zlib 压缩 |
| `syntax` | 所有内置语言的 tree-sitter 高亮 |
| `syntax-<lang>` | 单一语言高亮（`rust`、`python`、`typescript` 等） |
| `full` | `crossterm + async + serde + theme-watch + image + qrcode + kitty-compress + bidi`；**不**包含 `syntax`（语法文件较大，需显式开启） |

## 为什么是 SLT

- **公开语法足够小**。很多界面只需要普通 Rust 状态、`row()` / `col()` / `container()`、方法链和 `Response`。
- **更少框架仪式感**。很多应用一开始并不需要 app trait、retained tree 或 message enum。
- **组件够用，后端也认真**。常用组件会自动接好 focus、hover、click、scroll（鼠标事件需设置 `RunConfig::mouse(true)`），而 runtime 通过 `Backend`、`AppState` 和 `frame()` 保持一条保守的底层路径。
- **内部实现同样保守**。SLT 保持公开表面紧凑，但内部用 shared frame kernel、明确的 backend contract test、零 `unsafe`、feature-gated runtime path，以及覆盖 `all-features`、`no-default-features`、WASM、clippy、examples、cargo-hack、semver 和 deny 的验证来锁定质量。

对 Rust 用户来说，这通常意味着比 retained-mode TUI 框架更少的启动样板代码。
对 AI 辅助工作流来说，则意味着只看文档和示例就更容易推断出公开语法。

如果你想快速做出终端应用，同时保留 Rust 的类型安全和后端 escape hatch，SLT 很合适。
如果你更需要 retained component tree，或者更偏 GUI-first 的工具包，那么别的库可能更适合。

## 渲染管线

SLT 的渲染管线是语法能保持简洁的原因。
你的代码只接触第一个阶段 — 其余由引擎处理。

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

每次 `ui.*()` 调用只是往一个扁平列表中记录命令 — 不构建树，不计算布局。
引擎随后将这些命令通过一条**四阶段 DFS 管线**处理 — 每个阶段各司其职：构建布局树、计算 flexbox、收集交互与反馈数据、渲染单元格到后缓冲区 — 然后与前一帧做 diff，只 flush 变更的部分。

这个架构正是简洁语法的来源：

- **零仪式。** Immediate-mode 意味着不需要 `App` trait、`Model`/`Message`/`Update`/`View`。你的闭包就是整个 UI。状态是普通 Rust 变量，控制流就是 `if`/`for`。
- **布局不可见。** `ui.col(|ui| { ... })` 只是记录一条"打开列"的命令。引擎负责构建树和运行 flexbox — 你永远不会看到 `LayoutNode`。
- **性能是自动的。** 双缓冲在帧间对比每个单元格，只输出变化的 ANSI 属性。你在概念上每帧全部重绘，引擎让它变快。无需手动 dirty tracking。
- **交互自动接线。** `ui.button("Save")` 免费提供键盘焦点和 Enter/Space 触发；开启 `RunConfig::mouse(true)` 后还会报告 hover 和 click。无需自己编写命中测试代码。
- **同步反馈。** 交互使用前一帧的布局位置（60 FPS 下不可察觉）。无回调、无 async 布局查询 — 代码保持线性。

完整的八阶段生命周期请见[架构指南]。

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

// 浮层和富输出
ui.toast(&mut toasts);
ui.modal(|ui| {
    ui.text("Confirm?").bold();
});
ui.markdown("# Hello **world**");

// 可视化
ui.chart(|c| {
    c.line(&data);
    c.grid(true);
}, 50, 16);
ui.sparkline(&values, 16);
ui.canvas(40, 10, |cv| {
    cv.circle(20, 20, 15);
});
```

完整的分类组件列表请看[组件指南]，组合方式和组织建议请看[模式指南]。

## 学习路线

| 文档 | 覆盖内容 |
|------|----------|
| [文档索引] | 完整文档结构和指南地图 |
| [快速开始] | 安装、第一个应用、闭包 mental model、布局和组件状态 |
| [组件指南] | 组件、runtime 方法和状态类型的完整目录 |
| [模式指南] | 状态放置、界面组合、helper 提取和大型应用结构 |
| [示例指南] | 按产品形态和功能分组的可运行示例 |
| [后端指南] | `Backend`、`AppState`、`frame()`、inline mode、static output |
| [测试指南] | `TestBackend`、`EventBuilder`、multi-frame test 和 backend contract test |
| [调试指南] | F12 覆盖层、clipping、focus 异常和 previous-frame behavior |
| [AI 指南] | 给 AI builder 和 coding agent 的最快入口 |
| [架构指南] | 模块图、帧生命周期、layout/render 管线 |
| [特性指南] | feature flag、optional dependency 和推荐组合 |
| [动画指南] | Tween、spring、keyframe、sequence、stagger |
| [主题指南] | Theme、preset、ThemeBuilder 和自定义主题 |
| [设计原则] | API 约束和设计哲学 |
| [状态 API 指南] | 组件状态的公开字段、访问器和修改方法 |
| [迁移指南] | 各版本的破坏性变更与升级路径 |
| [菜谱] | 常见应用的可复制示例，每个都附带可运行的 example |
| [Web 开发者指南] | 从 Web 过来：CSS/React → SLT 对照 |
| [交互指南] | 鼠标、点击、悬停、拖拽、滚轮、展开/折叠、命中测试 |
| [定位指南] | 用 CSS/Flutter/React Native 概念解释 `Anchor` 和浮层 API |
| [上一帧指南] | 交互为什么使用上一帧布局、`Response.rect` 何时为零、第 1 帧的正确性 |
| [终端兼容性] | 终端/多路复用器/SSH 支持矩阵、环境变量、降级方案 |
| [API 设计] | 新组件的五条一致性规则和 PR 审查清单 |

## 代表示例

| 示例 | 命令 | 重点 |
|------|------|------|
| `hello` | `cargo run --example hello` | 最小应用 |
| `counter` | `cargo run --example counter` | 状态 + 键盘输入 |
| `demo` | `cargo run --example demo` | 大范围组件导览 |
| `showcase_tour` | `cargo run --example showcase_tour` | 仪表盘、设计系统、infoviz、交易、电子表格、表格和网站布局 |
| `cookbook_tour` | `cargo run --example cookbook_tour` | 表单、表格、模态框、文件选择器、仪表盘示例 |
| `text_tour` | `cargo run --example text_tour` | CJK 渲染、IME 输入、Pretext 重排、cargo 风格 CLI |
| `canvas_tour` | `cargo run --example canvas_tour` | DOOM 火焰、游戏（tetris/snake/minesweeper）、raw draw、Kitty 图像、动画 |
| `system_tour` | `cargo run --example system_tour --features async` | error boundary 和 overlay anchor 实时标签页；async/inline 模式以代码说明 |
| `widgets_tour` | `cargo run --example widgets_tour` | Hooks、主题、模态焦点陷阱、布局辅助与 static output（`run_static`） |
| `interaction_tour` | `cargo run --example interaction_tour` | `Response` 点击 / 双击 / 滚动信号、`KeyMap`、焦点遍历 |

完整的分类索引（包括各版本的特性导览和展示 demo）见[示例指南]。

### Demo 启动器

`scripts/ghostty_demos.sh` 会在新的 Ghostty 窗口中打开 demo，方便并排浏览全部示例。

```bash
./scripts/ghostty_demos.sh             # 交互式选择
./scripts/ghostty_demos.sh --features  # 全部特性导览
./scripts/ghostty_demos.sh --showcase  # 仅集成展示
```

## 自定义组件和后端

- 如果你需要可复用的高层构件，可以实现 `Widget`。
- 如果你需要非终端目标、外部事件循环或嵌入式 runtime，可以实现 `Backend` 并驱动 `frame()`。
- 如果你想做无头渲染验证和稳定的交互测试，可以使用 `TestBackend`。

即使需要 escape hatch，公开语法也依然保持紧凑。

## 贡献

先阅读[贡献指南]，然后查看[设计原则]和[架构指南]。
发布流程要求 format、check、clippy、test、example 和 backend gate 保持通过。

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
[设计原则]: DESIGN_PRINCIPLES.md
[贡献指南]: ../CONTRIBUTING.md
[License]: ../LICENSE
[状态 API 指南]: STATE_APIS.md
[迁移指南]: MIGRATION.md
[API 设计]: API_DESIGN.md
[菜谱]: COOKBOOK.md
[Web 开发者指南]: WEB_DEVELOPERS.md
[交互指南]: INTERACTION.md
[定位指南]: POSITIONING.md
[上一帧指南]: PREVIOUS_FRAME_GUIDE.md
[终端兼容性]: TERMINAL_COMPATIBILITY.md
