<div align="center">

# SuperLightTUI

**写得飞快。跑得极轻。**

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[Crate] · [Docs] · [Examples] · [Contributing]

[English](../README.md) · **中文** · [Español](README.es.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

</div>

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

状态存在于你的闭包中。布局用 `row()` 和 `col()`。样式链式调用。就这么简单。

## 为什么选择 SLT

**闭包即应用** — 没有框架状态，没有消息传递，没有 trait 实现。你写一个函数，SLT 每帧调用它。

**一切自动连接** — Tab 键循环焦点，鼠标滚轮滚动，容器自动上报点击和悬停事件，每个 widget 自己消费事件。

**CSS 布局，Tailwind 语法** — 用 `row()`、`col()`、`grow()`、`gap()`、`spacer()` 实现 Flexbox。Tailwind 风格简写：`.p()`、`.px()`、`.py()`、`.m()`、`.mx()`、`.my()`、`.w()`、`.h()`、`.min_w()`、`.max_w()`。

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

**两个核心依赖** — `crossterm` 处理终端 I/O，`unicode-width` 测量字符宽度。可选：`tokio`（异步）、`serde`（序列化）、`image`（图片加载）。零 `unsafe` 代码。

> **AI 辅助开发** — 在 [Claude Code](https://docs.anthropic.com/en/docs/claude-code) 中使用 `rust-tui-development-with-slt` skill，获取完整 API 参考、最佳实践和代码生成模板。或者用 [tui.builders](https://tui.builders) 可视化设计：

[![tui.builders demo](../assets/tui-builders-demo.gif)](https://tui.builders)

> 拖拽 widget，在检查器中设置属性，导出地道的 Rust 代码。免费，无需注册，开源。

## Widgets

55+ 个内置 widget，零样板代码：

```rust
ui.text_input(&mut name);                    // 单行输入
ui.textarea(&mut notes, 5);                  // 多行编辑器
if ui.button("Submit").clicked { /* … */ }    // 返回 Response
ui.checkbox("Dark mode", &mut dark);         // 复选框
ui.toggle("Notifications", &mut on);         // 开关
ui.tabs(&mut tabs);                          // 标签页导航
ui.list(&mut items);                         // 可选列表
ui.select(&mut sel);                         // 下拉选择
ui.radio(&mut radio);                        // 单选按钮组
ui.multi_select(&mut multi);                 // 多选复选框
ui.tree(&mut tree);                          // 可展开树形视图
ui.virtual_list(&mut list, 20, |ui, i| {}); // 虚拟化列表
ui.table(&mut data);                         // 数据表格
ui.spinner(&spin);                           // 加载动画
ui.progress(0.75);                           // 进度条
ui.scrollable(&mut scroll).col(|ui| { });    // 滚动容器
ui.toast(&mut toasts);                       // 通知提示
ui.separator();                              // 水平分隔线
ui.help(&[("q", "quit"), ("Tab", "focus")]); // 快捷键提示
ui.link("Docs", "https://docs.rs/superlighttui");      // 可点击超链接 (OSC 8)
ui.modal(|ui| { ui.text("overlay"); });      // 带遮罩的模态框
ui.overlay(|ui| { ui.text("floating"); });   // 无遮罩浮层
ui.command_palette(&mut palette);            // 可搜索命令面板
ui.markdown("# Hello **world**");            // Markdown 渲染
ui.form_field(&mut field);                   // 带验证的标签输入
ui.chart(|c| { c.line(&data); c.grid(true); }, 50, 16); // 折线/散点/柱状图
ui.scatter(&points, 50, 16);                 // 独立散点图
ui.histogram(&values, 40, 12);               // 自动分箱直方图
ui.bar_chart(&data, 24);                     // 水平条形图
ui.sparkline(&values, 16);                   // 趋势迷你图 ▁▂▃▅▇
ui.canvas(40, 10, |cv| { cv.circle(20, 20, 15); }); // 盲文点阵画布
ui.grid(3, |ui| { /* 3列网格 */ });          // 网格布局
ui.tooltip("Save the current file");         // 悬停提示弹窗
ui.calendar(&mut cal);                       // 带月份导航的日期选择器
ui.screen("home", &screens, |ui| {});        // 屏幕路由栈
ui.confirm("Delete?", &mut yes);             // 支持鼠标的确认对话框
```

每个 widget 自行处理键盘事件、焦点状态和鼠标交互。

### 自定义 Widget

实现 `Widget` trait 来构建你自己的 widget：

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

// 用法：ui.widget(&mut rating);
```

焦点、事件、主题、布局，全部通过 `Context` 访问。一个 trait，一个方法。

## 功能特性

<details>
<summary><b>布局</b></summary>

| 功能 | API |
|------|-----|
| 垂直堆叠 | `ui.col(\|ui\| { })` |
| 水平堆叠 | `ui.row(\|ui\| { })` |
| 网格布局 | `ui.grid(3, \|ui\| { })` |
| 子元素间距 | `.gap(1)` |
| 弹性增长 | `.grow(1)` |
| 推到末尾 | `ui.spacer()` |
| 对齐方式 | `.align(Align::Center)` |
| 内边距 | `.p(1)`, `.px(2)`, `.py(1)` |
| 外边距 | `.m(1)`, `.mx(2)`, `.my(1)` |
| 固定尺寸 | `.w(20)`, `.h(10)` |
| 约束 | `.min_w(10)`, `.max_w(60)` |
| 百分比尺寸 | `.w_pct(50)`, `.h_pct(80)` |
| 对齐分布 | `.space_between()`, `.space_around()`, `.space_evenly()` |
| 文本换行 | `ui.text_wrap("long text...")` |
| 带标题边框 | `.border(Border::Rounded).title("Panel")` |
| 单侧边框 | `.border_top(false)`, `.border_sides(BorderSides::horizontal())` |
| 响应式间距 | `.gap_at(Breakpoint::Md, 2)` |

</details>

<details>
<summary><b>样式</b></summary>

```rust
ui.text("styled").bold().italic().underline().fg(Color::Cyan).bg(Color::Black);
```

16 种命名颜色 · 256 色调色板 · 24 位 RGB · 6 种修饰符 · 6 种边框样式

</details>

<details>
<summary><b>主题</b></summary>

```rust
// 7 个内置预设
slt::run_with(RunConfig::default().theme(Theme::catppuccin()), |ui| {
    ui.set_theme(Theme::dark()); // 运行时切换
});

// 构建自定义主题
let theme = Theme::builder()
    .primary(Color::Rgb(255, 107, 107))
    .accent(Color::Cyan)
    .build();
```

7 个预设（dark、light、dracula、catppuccin、nord、solarized_dark、tokyo_night）。自定义主题支持 15 个颜色槽 + `is_dark` 标志。所有 widget 自动继承。

</details>

<details>
<summary><b>样式复用</b></summary>

```rust
use slt::{ContainerStyle, Border, Color};

const CARD: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded).p(1).bg(Color::Indexed(236));

// 应用并组合
ui.container().apply(&CARD).grow(1).gap(2).col(|ui| { ... });
```

定义一次，随处使用。`const` 样式零运行时开销。链式 `.apply()` 组合，内联方法始终覆盖。

</details>

<details>
<summary><b>响应式布局</b></summary>

```rust
ui.container()
    .w(20).md_w(40).lg_w(60)  // 在断点处改变宽度
    .p(1).lg_p(2)
    .col(|ui| { ... });
```

35 个断点条件方法（`xs_`、`sm_`、`md_`、`lg_`、`xl_` × `w`、`h`、`min_w`、`max_w`、`gap`、`p`、`grow`）。断点：Xs (<40)、Sm (40-79)、Md (80-119)、Lg (120-159)、Xl (≥160)。

</details>

<details>
<summary><b>Hooks</b></summary>

```rust
let count = ui.use_state(|| 0i32);
ui.text(format!("{}", count.get(ui)));
if ui.button("+1") { *count.get_mut(ui) += 1; }
```

即时模式下的 React 风格持久状态。`State<T>` 句柄模式，每帧按相同顺序调用。

</details>

<details>
<summary><b>渲染</b></summary>

- **双缓冲差分** — 只有变化的单元格才会输出到终端
- **同步输出** — DECSET 2026 防止支持的终端出现撕裂
- **视口裁剪** — 屏幕外的 widget 完全跳过
- **FPS 上限** — `RunConfig::default().max_fps(60)` 控制 CPU 占用
- **自动重排** — 终端大小变化时自动重新布局
- **`collect_all()`** — 单次 DFS 遍历替代 7 次独立树遍历 (v0.9)

</details>

<details>
<summary><b>动画</b></summary>

```rust
let mut tween = Tween::new(0.0, 100.0, 60).easing(ease_out_bounce);
let value = tween.value(ui.tick());

let mut spring = Spring::new(0.0, 180.0, 12.0);
spring.set_target(100.0);

let mut kf = Keyframes::new(120)
    .stop(0.0, 0.0).stop(0.5, 100.0).stop(1.0, 50.0)
    .loop_mode(LoopMode::PingPong);
```

Tween（9 种缓动函数）、弹簧物理、关键帧时间轴（含循环模式）、Sequence 链、Stagger 列表动画。全部支持 `.on_complete()` 回调。

</details>

<details>
<summary><b>异步支持</b></summary>

```rust
let tx = slt::run_async(|ui, messages: &mut Vec<String>| {
    for msg in messages.drain(..) { ui.text(msg); }
})?;
tx.send("Hello from background!".into()).await?;
```

可选 tokio 集成。通过 `cargo add superlighttui --features async` 启用。

</details>

<details>
<summary><b>错误边界</b></summary>

```rust
ui.error_boundary(|ui| {
    ui.text("If this panics, the app keeps running.");
});
```

捕获 widget panic 而不崩溃整个应用。部分命令回滚，渲染降级内容。

</details>

<details>
<summary><b>模态框与浮层</b></summary>

```rust
ui.modal(|ui| {
    ui.bordered(Border::Rounded).pad(2).col(|ui| {
        ui.text("Confirm?").bold();
        if ui.button("OK") { show = false; }
    });
});
```

`modal()` 遮暗背景并在顶层渲染内容。`overlay()` 渲染浮动内容但不遮暗背景。两者均支持完整布局和交互。

</details>

<details>
<summary><b>快照测试</b></summary>

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

配合 [insta](https://crates.io/crates/insta) 进行基于快照的 UI 回归测试。

</details>

<details>
<summary><b>图片渲染</b></summary>

```sh
cargo add superlighttui --features image
```

```rust
use slt::HalfBlockImage;

let photo = image::open("photo.png").unwrap();
let img = HalfBlockImage::from_dynamic(&photo, 60, 30);
ui.image(&img);
```

半块字符（▀▄）图片渲染。Sixel 协议（v0.13.2）：`ui.sixel_image(&rgba, w, h, cols, rows)` 在 xterm、foot、mlterm 上实现像素级图片。

</details>

<details>
<summary><b>Feature Flags</b></summary>

| Flag | 说明 |
|------|------|
| `async` | `run_async()`，基于 tokio channel 的消息传递 |
| `serde` | 为 Style、Color、Theme、布局类型实现序列化/反序列化 |
| `image` | `HalfBlockImage::from_dynamic()`，依赖 `image` crate |
| `full` | 以上全部 |

```toml
[dependencies]
superlighttui = { version = "0.13", features = ["full"] }
```

</details>

<details>
<summary><b>调试</b></summary>

在任意 SLT 应用中按 **F12** 切换布局调试器浮层，显示容器边界、嵌套深度和布局结构。

</details>

## 示例

| 示例 | 命令 | 展示内容 |
|------|------|----------|
| hello | `cargo run --example hello` | 最简配置 |
| counter | `cargo run --example counter` | 状态 + 键盘 |
| demo | `cargo run --example demo` | 所有 widget |
| demo_dashboard | `cargo run --example demo_dashboard` | 实时仪表盘 |
| demo_cli | `cargo run --example demo_cli` | CLI 工具布局 |
| demo_spreadsheet | `cargo run --example demo_spreadsheet` | 数据表格 |
| demo_website | `cargo run --example demo_website` | 终端中的网站 |
| demo_game | `cargo run --example demo_game` | 俄罗斯方块 + 贪吃蛇 + 扫雷 |
| demo_fire | `cargo run --release --example demo_fire` | DOOM 火焰效果（半块字符） |
| demo_ime | `cargo run --example demo_ime` | 韩文/CJK 输入法 |
| inline | `cargo run --example inline` | 内联模式 |
| anim | `cargo run --example anim` | Tween + Spring + Keyframes |
| demo_infoviz | `cargo run --example demo_infoviz` | 数据可视化 |
| demo_trading | `cargo run --example demo_trading` | 交易所风格终端 |
| async_demo | `cargo run --example async_demo --features async` | 后台任务 |

## 架构

```
Closure → Context collects Commands → build_tree() → flexbox layout → diff buffer → flush
```

每一帧：你的闭包运行，SLT 收集你描述的内容，计算 flexbox 布局，与上一帧做差分，只刷新变化的单元格。

纯 Rust。无宏，无代码生成，无构建脚本。

### 自定义 Backend

SLT 的渲染通过 `Backend` trait 抽象，支持终端以外的自定义渲染目标：

```rust
use slt::{Backend, AppState, Buffer, Rect, RunConfig, Context, Event};

struct MyBackend { buffer: Buffer }

impl Backend for MyBackend {
    fn size(&self) -> (u32, u32) {
        (self.buffer.area.width, self.buffer.area.height)
    }
    fn buffer_mut(&mut self) -> &mut Buffer { &mut self.buffer }
    fn flush(&mut self) -> std::io::Result<()> {
        // 将 self.buffer 渲染到你的目标（canvas、GPU、网络等）
        Ok(())
    }
}
```

`Backend` trait 只有 3 个方法：`size()`、`buffer_mut()`、`flush()`。自定义 backend 接收完整渲染好的 `Buffer`，可以用任何方式呈现，WebGL、egui 嵌入、SSH 隧道、测试框架等皆可。

### AI 原生 Widgets

SLT 内置专为 AI/LLM 工作流设计的 widget：

| Widget | 说明 |
|--------|------|
| `streaming_text()` | 逐 token 文本显示，带闪烁光标 |
| `streaming_markdown()` | 流式 Markdown，支持标题、代码块、内联格式 |
| `tool_approval()` | 人工审批/拒绝工具调用 |
| `context_bar()` | 显示活跃上下文来源的 token 计数条 |
| `markdown()` | 静态 Markdown 渲染 |
| `code_block()` | 语法高亮代码展示 |

## 贡献

参见 [CONTRIBUTING.md](../CONTRIBUTING.md)。

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
[Examples]: https://github.com/subinium/SuperLightTUI/tree/main/examples
[Contributing]: https://github.com/subinium/SuperLightTUI/blob/main/CONTRIBUTING.md
[License]: ../LICENSE
