<div align="center">

# SuperLightTUI

**웹 페이지처럼 터미널 UI를 만드세요** — flexbox 레이아웃, Tailwind 스타일 체이닝, hooks, 클릭/hover/스크롤을 Rust 클로저 하나로.

**빠르게 작성하고. 가벼운 의존성 트리**(직접 필수 의존성 4개, 기본 기능 기준 26개 resolved — ratatui + crossterm은 69개).

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[문서 인덱스] · [빠른 시작] · [위젯 가이드] · [패턴 가이드] · [예제 가이드] · [백엔드 가이드] · [아키텍처 가이드]

[English](../README.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [日本語](README.ja.md) · **한국어**

</div>

> 이 문서는 v0.25.0 기준 [영문 README](../README.md)를 요약 번역한 것입니다. 내용이 다르면 영문 README가 기준입니다.

SuperLightTUI는 Rust를 위한 immediate-mode TUI 라이브러리로, public grammar를 의도적으로 작게 유지합니다.
클로저 하나를 쓰면 SLT가 매 프레임 그 클로저를 호출하고, 라이브러리가 레이아웃, 포커스, diff, 렌더링을 처리합니다.

빠른 제품 반복, 읽기 쉬운 Rust 문법, 그리고 진지한 백엔드 규율을 함께 가져가도록 설계했습니다.
그래서 도구를 빠르게 시제품화하는 사람에게도, 문서로 UI를 생성하는 코딩 에이전트에게도 자연스럽게 맞습니다.

## 빠른 시작

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

5줄이면 시작됩니다. `App` 트레이트도, `Model`/`Update`/`View`도, 수동 이벤트 루프도 필요 없습니다.
Ctrl+C를 누르면 기본적으로 루프가 깔끔하게 종료됩니다(`RunConfig::handle_ctrl_c(false)`로 끌 수 있습니다).

마우스는 **기본으로 켜져 있습니다**. 기본 `slt::run`만으로 클릭, 휠 스크롤, 드래그(분할 핸들,
스크롤바, 텍스트 선택)가 전달되고, SSH에서는 클릭과 휠만 받습니다.
Tab/Shift+Tab 포커스 이동과 Enter/Space 버튼 활성화는 항상 동작합니다.

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

hover 스타일과 툴팁은 모든 포인터 이동이 필요하므로 따로 켭니다:
`slt::run_with(RunConfig::default().mouse_mode(MouseMode::Hover), |ui| { ... })`.
마우스를 캡처하는 동안에도 대부분의 터미널은 Shift+드래그로 네이티브 텍스트 선택을 지원하며,
`MouseMode::Off`로 마우스를 완전히 끌 수 있습니다.

MSRV: Rust 1.88. 기본 feature는 `crossterm` 백엔드와 `bidi`입니다.

## 쇼케이스

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
    <td align="center" colspan="3"><img src="../assets/demo_pretext.gif" alt="Pretext Reflow" /><br/><b><a href="https://github.com/chenglou/pretext">Pretext</a> Reflow</b> — 마우스 커서 주변으로 텍스트가 실시간 재배치됩니다<br/><sub><code>cargo run --example text_tour</code></sub></td>
  </tr>
</table>

## 60초 문법

대부분의 앱은 네 가지 개념으로 시작합니다.

1. 상태는 일반적인 Rust 변수나 구조체에 둡니다.
2. 레이아웃은 대부분 `row()`, `col()`, `container()`로 충분합니다.
3. 스타일링은 메서드 체이닝으로 끝납니다.
4. 상호작용 위젯은 보통 `Response`를 돌려줍니다.

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

이게 핵심 mental model입니다. 나머지는 별도의 프레임워크가 아니라 깊이와 확장입니다.
`clicked`는 버튼에 포커스가 있을 때 Enter/Space로 켜지고, 마우스 클릭으로도 켜집니다(마우스는 기본으로 켜져 있습니다).

## 실제 앱

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

## 런타임 모드

같은 클로저가 여러 진입점에서 동작합니다. 앱 크기가 아니라 UI 형태에 맞춰 고르세요.

| 모드 | API | 용도 |
|------|-----|------|
| 풀스크린 | `slt::run` / `slt::run_with` | alternate screen을 쓰는 일반 TUI 앱. 마우스 클릭, 휠, 드래그는 기본으로 켜져 있고, hover는 `RunConfig::mouse_mode(MouseMode::Hover)`가 필요합니다. |
| Inline | `slt::run_inline` / `slt::run_inline_with` | 프롬프트 아래에 고정 높이 위젯을 그립니다. alternate screen 없음. |
| Static + inline | `slt::run_static` | 로그 줄은 scrollback으로 흘려보내고, 그 아래 inline UI를 계속 유지합니다. |
| Async 메시지 | `slt::run_async` *(feature: `async`)* | 백그라운드 작업이 wake-aware `AsyncSender`로 메시지를 보냅니다. 반환된 `AsyncRunHandle`을 보관해 join/cancel 하세요. |
| 커스텀 백엔드 | `slt::frame` + `Backend` + `AppState` | 테스트, GUI 임베드, WASM, 스냅샷 하네스처럼 렌더링을 직접 구동할 때. |

`RunConfig`로 마우스 모드(기본값: 로컬은 `Drag`, SSH는 `Click`, 필요하면 `Hover`/`Off`), kitty keyboard, color depth, 최대 FPS, 스크롤 속도, 테마, 타이틀, Ctrl+C 처리를 모든 모드에서 조정합니다.

## 포커스와 스크롤

스크롤 컨테이너는 포커스, 캐럿, 레이아웃이 바뀌면 포커스된 위젯이 보이도록 스크롤합니다(중첩된 가로/세로 뷰와 모달 폼 포함).
그 사이에 사용자가 직접 스크롤한 위치는 유지됩니다. 위치를 애플리케이션이 직접 제어하려면 `scroll.follow_focus = false`로 설정하세요. 컨테이너마다 `ScrollState`를 하나씩 두세요.

필드/입력 상호작용 피드백이 필요하면 `ui.form_field_response(...)`를 쓰세요. 기존 `ui.form_field(...)` 호출은 그대로 동작합니다.
`ui.focused_layout_rect()`와 `ui.measured_layout_rect(name)`은 스크롤·클리핑 전의 이전 프레임 geometry를 돌려주고, `Response.rect`는 화면에 보이는 마우스 hit 영역입니다.
좌표와 타이밍 규칙은 [마이그레이션 가이드]를 참고하세요.

## Feature Flags

```toml
[dependencies]
superlighttui = { version = "0.25.0", features = ["async", "image"] }
```

| Feature | 추가되는 것 |
|---------|-------------|
| `async` | `tokio` + 백그라운드 메시지 루프용 `run_async` |
| `bidi` | UAX #9 양방향 텍스트 재배열. 기본 활성화 |
| `serde` | 일부 상태 타입의 `Serialize`/`Deserialize` |
| `theme-watch` | TOML 테마 핫 리로드(`serde` + 파일 시스템 watcher) |
| `image` | `ui.image`용 PNG/JPEG 디코딩 |
| `qrcode` | QR 코드 렌더링 |
| `kitty-compress` | Kitty 이미지 프로토콜용 zlib 압축 |
| `syntax` | 번들된 모든 언어의 tree-sitter 하이라이팅 |
| `syntax-<lang>` | 단일 언어 하이라이팅(`rust`, `python`, `typescript`, ...) |
| `full` | `crossterm + async + serde + theme-watch + image + qrcode + kitty-compress + bidi`. `syntax`는 **포함하지 않습니다**(문법 파일이 무거우므로 명시적으로 opt-in). |

## 왜 SLT인가

- **작은 public grammar**. 대부분의 화면은 일반 Rust 상태, `row()` / `col()` / `container()`, 메서드 체이닝, `Response`로 시작합니다.
- **프레임워크 의식이 적습니다**. 움직이기 위해 앱 트레이트, retained tree, 메시지 enum을 먼저 만들 필요가 없는 경우가 많습니다.
- **기본기는 풍부하지만 백엔드는 진지합니다**. 공용 위젯이 포커스, hover, 클릭, scroll을 기본으로 연결해 주고(클릭, 휠, 드래그는 기본, hover는 `MouseMode::Hover`), 내부 런타임은 `Backend`, `AppState`, `frame()`으로 보수적인 low-level 경로를 유지합니다.
- **내부 구현도 보수적으로 다집니다**. shared frame kernel, explicit backend contract test, zero `unsafe`, feature-gated runtime path, `all-features`/`no-default-features`/WASM/clippy/examples/cargo-hack/semver/deny 검증으로 품질을 잠급니다.

Rust 사용자에게는 retained-mode TUI보다 초기 설정이 덜한 경우가 많습니다.
AI 보조 워크플로에서는 문서와 예제만으로도 public grammar를 추론하기 쉽다는 뜻이기도 합니다.

SLT는 Rust의 타입 안정성을 유지하면서 빠르게 터미널 앱을 만들고 싶을 때 특히 잘 맞습니다.
반대로 retained component tree가 중심이거나 GUI-first 툴킷이 필요하다면 다른 선택지가 더 잘 맞을 수 있습니다.

## 렌더링 파이프라인

SLT의 렌더링 파이프라인 덕분에 문법이 작게 유지됩니다.
여러분의 코드는 첫 단계만 담당하고, 나머지는 엔진이 처리합니다.

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

`ui.*()` 호출은 커맨드를 flat list에 기록합니다 — 트리 구성도, 레이아웃 계산도 없습니다.
엔진이 이 커맨드를 **4단계 DFS 파이프라인**에 통과시킵니다 — 각 단계는 역할이 분리되어 있습니다: 레이아웃 트리 생성, flexbox 계산, 상호작용과 피드백 데이터 수집, back buffer에 셀 렌더링 — 이후 이전 프레임과 diff 후 변경분만 flush합니다.

이 아키텍처가 문법을 단순하게 만드는 이유입니다:

- **의식 절차가 없습니다.** Immediate-mode이므로 `App` 트레이트, `Model`/`Message`/`Update`/`View`가 필요 없습니다. 클로저가 전체 UI이고, 상태는 일반 Rust 변수, 제어 흐름은 `if`/`for`입니다.
- **레이아웃이 보이지 않습니다.** `ui.col(|ui| { ... })`는 "open column" 커맨드를 기록합니다. 엔진이 트리를 빌드하고 flexbox를 실행합니다 — `LayoutNode`는 노출되지 않습니다.
- **성능이 자동입니다.** 더블 버퍼가 프레임 간 셀을 비교하고 변경된 ANSI 속성만 출력합니다. 매 프레임 전체를 다시 그리지만 엔진이 빠르게 처리합니다. 수동 dirty tracking이 필요 없습니다.
- **상호작용이 자동 연결됩니다.** `ui.button("Save")`만으로 키보드 포커스와 Enter/Space 활성화가 제공되고, 마우스 click도 기본으로 보고되며, `MouseMode::Hover`에서는 hover도 보고됩니다. hit-testing 코드를 직접 작성할 필요가 없습니다.
- **동기적 피드백.** 상호작용은 이전 프레임의 레이아웃 위치를 사용합니다(60 FPS에서 감지 불가). 콜백도 async 레이아웃 쿼리도 없이 코드가 선형으로 유지됩니다.

전체 8단계 라이프사이클은 [아키텍처 가이드]를 참고하세요.

## 자주 쓰는 API

```rust
// 텍스트와 레이아웃
ui.text("Hello").bold().fg(Color::Cyan);
ui.row(|ui| {
    ui.text("left");
    ui.spacer();
    ui.text("right");
});

// 입력과 액션
ui.text_input(&mut name);
if ui.button("Save").clicked {}
ui.checkbox("Dark mode", &mut dark);

// 데이터와 네비게이션
ui.tabs(&mut tabs);
ui.list(&mut items);
ui.table(&mut data);
ui.command_palette(&mut palette);

// 오버레이와 리치 출력
ui.toast(&mut toasts);
ui.modal(|ui| {
    ui.text("Confirm?").bold();
});
ui.markdown("# Hello **world**");

// 시각화
ui.chart(|c| {
    c.line(&data);
    c.grid(true);
}, 50, 16);
ui.sparkline(&values, 16);
ui.canvas(40, 10, |cv| {
    cv.circle(20, 20, 15);
});
```

분류된 전체 위젯 목록은 [위젯 가이드]를, 조합 패턴은 [패턴 가이드]를 참고하세요.

## 라이브러리 익히기

| 문서 | 내용 |
|------|------|
| [문서 인덱스] | 전체 문서 구조와 가이드 맵 |
| [빠른 시작] | 설치, 첫 앱, 클로저 mental model, 레이아웃, 위젯 상태 |
| [위젯 가이드] | 위젯, 런타임 메서드, 상태 타입 전체 카탈로그 |
| [패턴 가이드] | 상태 배치, 화면 조합, helper 추출, 큰 앱 구조 |
| [예제 가이드] | 제품 형태와 기능 영역별로 정리된 실행 예제 |
| [백엔드 가이드] | `Backend`, `AppState`, `frame()`, inline mode, static output |
| [테스트 가이드] | `TestBackend`, `EventBuilder`, multi-frame test, backend contract test |
| [디버깅 가이드] | F12 오버레이, clipping, focus surprise, previous-frame behavior |
| [AI 가이드] | AI 빌더와 코딩 에이전트를 위한 가장 빠른 진입 경로 |
| [아키텍처 가이드] | 모듈 맵, 프레임 라이프사이클, layout/render 파이프라인 |
| [기능 가이드] | feature flag, optional dependency, 추천 조합 |
| [애니메이션 가이드] | Tween, spring, keyframe, sequence, stagger |
| [테마 가이드] | Theme struct, preset, ThemeBuilder, 커스텀 테마 |
| [설계 원칙] | API 제약과 설계 철학 |
| [상태 API 가이드] | 위젯 상태의 public 필드, accessor, 변경 메서드 |
| [마이그레이션 가이드] | 버전별 breaking change와 업그레이드 경로 |
| [쿡북] | 자주 만드는 앱 레시피. 각 레시피는 실행 가능한 예제로도 제공 |
| [웹 개발자 가이드] | 웹에서 오셨다면: CSS/React → SLT 대응표 |
| [상호작용 가이드] | 마우스, 클릭, hover, 드래그, 휠, 펼치기/접기, hit-testing |
| [포지셔닝 가이드] | `Anchor`와 오버레이 API를 CSS/Flutter/React Native 개념에 대응 |
| [이전 프레임 가이드] | 상호작용이 이전 프레임 레이아웃을 쓰는 이유, `Response.rect`가 0인 경우, 첫 프레임 정확성 |
| [터미널 호환성] | 터미널/멀티플렉서/SSH 지원 표, 환경 변수, fallback |
| [API 설계] | 새 위젯을 위한 다섯 가지 일관성 규칙과 PR 리뷰 체크리스트 |

## 대표 예제

| 예제 | 커맨드 | 초점 |
|------|--------|------|
| `hello` | `cargo run --example hello` | 가장 작은 앱 |
| `counter` | `cargo run --example counter` | 상태 + 키보드 입력 |
| `demo` | `cargo run --example demo` | 폭넓은 위젯 투어 |
| `showcase_tour` | `cargo run --example showcase_tour` | 대시보드, 디자인 시스템, infoviz, 트레이딩, 스프레드시트, 테이블, 웹사이트 레이아웃 |
| `cookbook_tour` | `cargo run --example cookbook_tour` | 폼, 테이블, 모달, 파일 선택기, 대시보드 레시피 |
| `text_tour` | `cargo run --example text_tour` | CJK 렌더링, IME 입력, Pretext reflow, cargo 스타일 CLI |
| `canvas_tour` | `cargo run --example canvas_tour` | DOOM fire, 게임(tetris/snake/minesweeper), raw draw, Kitty 이미지, 애니메이션 |
| `system_tour` | `cargo run --example system_tour --features async` | error boundary와 overlay anchor 라이브 탭. async/inline 모드는 코드로 설명 |
| `widgets_tour` | `cargo run --example widgets_tour` | 훅, 테마, 모달 포커스 트랩, 레이아웃 헬퍼, static output(`run_static`) |
| `interaction_tour` | `cargo run --example interaction_tour` | 클릭 / 더블클릭 / 스크롤 `Response` 신호, `KeyMap`, 포커스 이동 |

릴리즈별 feature 투어와 쇼케이스 데모를 포함한 전체 분류 인덱스는 [예제 가이드]에 있습니다.

### 데모 런처

`scripts/ghostty_demos.sh`는 데모를 새 Ghostty 창에 띄워 줍니다. 여러 데모를 나란히 훑어볼 때 편합니다.

```bash
./scripts/ghostty_demos.sh             # 대화형 선택
./scripts/ghostty_demos.sh --features  # 전체 feature 투어
./scripts/ghostty_demos.sh --showcase  # 통합 쇼케이스만
```

## 커스텀 위젯과 백엔드

- 재사용 가능한 고수준 빌딩 블록이 필요하면 `Widget`을 구현하세요.
- 비터미널 타깃, 외부 이벤트 루프, 임베디드 런타임이 필요하면 `Backend`를 구현하고 `frame()`을 구동하세요.
- 헤드리스 렌더링 검증과 안정적인 상호작용 테스트에는 `TestBackend`를 쓰면 됩니다.

escape hatch가 필요해져도 public grammar는 작게 유지됩니다.

## 기여

[기여하기]를 읽고, 이어서 [설계 원칙]과 [아키텍처 가이드]를 참고하세요.
릴리즈 과정에서는 format, check, clippy, test, example, backend gate가 계속 녹색이어야 합니다.

## 라이선스

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
[문서 인덱스]: README.md
[빠른 시작]: QUICK_START.md
[위젯 가이드]: WIDGETS.md
[예제 가이드]: EXAMPLES.md
[패턴 가이드]: PATTERNS.md
[아키텍처 가이드]: ARCHITECTURE.md
[백엔드 가이드]: BACKENDS.md
[테스트 가이드]: TESTING.md
[디버깅 가이드]: DEBUGGING.md
[AI 가이드]: AI_GUIDE.md
[애니메이션 가이드]: ANIMATION.md
[테마 가이드]: THEMING.md
[기능 가이드]: FEATURES.md
[설계 원칙]: DESIGN_PRINCIPLES.md
[기여하기]: ../CONTRIBUTING.md
[상태 API 가이드]: STATE_APIS.md
[마이그레이션 가이드]: MIGRATION.md
[API 설계]: API_DESIGN.md
[쿡북]: COOKBOOK.md
[웹 개발자 가이드]: WEB_DEVELOPERS.md
[상호작용 가이드]: INTERACTION.md
[포지셔닝 가이드]: POSITIONING.md
[이전 프레임 가이드]: PREVIOUS_FRAME_GUIDE.md
[터미널 호환성]: TERMINAL_COMPATIBILITY.md
