<div align="center">

# SuperLightTUI

**빠르게 작성하고. 가볍게 실행합니다.**

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[문서 인덱스] · [빠른 시작] · [위젯 가이드] · [예제 가이드] · [패턴 가이드] · [아키텍처 가이드] · [기여하기]

[English](../README.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [日本語](README.ja.md) · **한국어**

</div>

SuperLightTUI는 Rust를 위한 즉시 모드(immediate-mode) TUI 라이브러리입니다.
클로저 하나를 작성하면, SLT가 매 프레임 호출하고, 라이브러리가 레이아웃, 디핑, 포커스, 렌더링을 처리합니다.

## 쇼케이스

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

## 시작하기

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

5줄. `App` 구조체 없음. `Model`/`Update`/`View` 없음. 이벤트 루프 없음. Ctrl+C는 그냥 동작합니다.

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

상태는 클로저 안에 있습니다. 레이아웃은 `row()`와 `col()`. 스타일은 메서드 체이닝. 그게 전부입니다.

## SLT를 선택하는 이유

- **클로저가 곧 앱입니다** — 프레임워크 상태 없음, 트레이트 구현 보일러플레이트 없음, 메시지 루프 API 없음.
- **CSS 같은 레이아웃, Tailwind 같은 문법** — `row()`, `col()`, `gap()`, `grow()`, `spacer()`, 그리고 `.p()`, `.px()`, `.m()`, `.w()`, `.max_w()` 같은 단축 표기.
- **위젯이 지루한 작업을 자동 처리** — 포커스 순서, 호버, 클릭 핸들링, 스크롤, 공통 키보드 동작이 내장되어 있습니다.
- **작은 코어, 선택적 확장** — 핵심 의존성은 `unicode-width`와 `compact_str`; 터미널 I/O는 선택적 `crossterm`; async, serde, image, qrcode, 신택스 하이라이팅은 피처 플래그 뒤에 있습니다.
- **라이브러리 위생이 중요합니다** — `unsafe` 없음, 명시적 피처 플래그, 문서, 예제, 테스트, 시맨틱 버저닝 릴리스 규율.

## 주요 API

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

전체 위젯 카탈로그는 [위젯 가이드]를 참고하세요.

## 라이브러리 학습하기

| 문서 | 내용 |
|------|------|
| [문서 인덱스] | 전체 문서 구조와 가이드 맵 |
| [빠른 시작] | 설치, 첫 앱, 카운터, 레이아웃, 입력 |
| [위젯 가이드] | 위젯 카탈로그와 주요 API |
| [패턴 가이드] | 상태 관리, 폼, 오버레이, 비동기, 커스텀 위젯 |
| [예제 가이드] | 예제 인덱스와 실행 명령어 |
| [아키텍처 가이드] | 모듈 맵, 프레임 라이프사이클, 의존성 흐름 |
| [백엔드 가이드] | `Backend`, `AppState`, `frame()`, 인라인 모드 |
| [테스트 가이드] | `TestBackend`, `EventBuilder`, 상호작용 테스트 |
| [디버깅 가이드] | F12 오버레이, 클리핑, 원프레임 딜레이 |
| [AI 가이드] | AI 코딩 에이전트를 위한 빠른 참조 |
| [애니메이션 가이드] | Tween, Spring, Keyframes, Sequence, Stagger |
| [테마 가이드] | 테마 프리셋, ThemeBuilder, 커스텀 테마 |
| [기능 가이드] | 피처 플래그, 선택적 의존성 |
| [`docs/DESIGN_PRINCIPLES.md`](DESIGN_PRINCIPLES.md) | API가 이렇게 설계된 이유 |

## 예제 하이라이트

| 예제 | 커맨드 | 내용 |
|------|--------|------|
| hello | `cargo run --example hello` | 최소 구성 앱 |
| counter | `cargo run --example counter` | 상태 + 키보드 입력 |
| demo | `cargo run --example demo` | 전체 위젯 투어 |
| demo_dashboard | `cargo run --example demo_dashboard` | 대시보드 레이아웃 |
| demo_cli | `cargo run --example demo_cli` | CLI 도구 레이아웃 |
| demo_infoviz | `cargo run --example demo_infoviz` | 차트와 데이터 시각화 |
| demo_game | `cargo run --example demo_game` | 즉시 모드 인터랙션 |
| async_demo | `cargo run --example async_demo --features async` | 백그라운드 메시지 |

전체 분류 인덱스는 [예제 가이드]를 참고하세요.

## 커스텀 위젯과 백엔드

- `Widget` 트레이트를 구현하면 포커스, 레이아웃, 이벤트, 테마에 완전히 접근할 수 있는 재사용 가능한 위젯을 만들 수 있습니다.
- `Backend` 트레이트를 구현하고 `frame()`을 구동하면 비터미널 렌더러, 테스트 하네스, 임베디드 타겟을 만들 수 있습니다.
- `TestBackend`로 헤드리스 렌더링과 스냅샷 스타일 검증을 할 수 있습니다.

자세한 내용은 [패턴 가이드]와 [아키텍처 가이드]를 참고하세요.

## 기여

[기여하기] 가이드를 읽은 다음 `docs/DESIGN_PRINCIPLES.md`와 [아키텍처 가이드]를 참고하세요.
릴리스와 CI 프로세스는 포맷팅, 체크, clippy, 테스트, 예제 컴파일이 모두 통과해야 합니다.

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
[기여하기]: ../CONTRIBUTING.md
