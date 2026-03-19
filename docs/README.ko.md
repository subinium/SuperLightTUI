<div align="center">

# SuperLightTUI

**빠르게 작성하고. 가볍게 실행합니다.**

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[Crate] · [Docs] · [Examples] · [Contributing]

[English](../README.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [日本語](README.ja.md) · **한국어**

</div>

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

상태는 클로저 안에 있습니다. 레이아웃은 `row()`와 `col()`. 스타일은 메서드 체이닝. 그게 전부입니다.

## SLT를 선택하는 이유

**클로저가 곧 앱입니다** — 프레임워크 상태 없음. 메시지 패싱 없음. 트레이트 구현 없음. 함수를 작성하면 SLT가 매 프레임 호출합니다.

**모든 것이 자동으로 연결됩니다** — Tab으로 포커스 순환. 마우스 휠로 스크롤. 컨테이너는 클릭과 호버를 보고합니다. 위젯은 자체적으로 이벤트를 소비합니다.

**CSS 같은 레이아웃, Tailwind 같은 문법** — `row()`, `col()`, `grow()`, `gap()`, `spacer()`로 Flexbox. Tailwind 스타일 단축 표기: `.p()`, `.px()`, `.py()`, `.m()`, `.mx()`, `.my()`, `.w()`, `.h()`, `.min_w()`, `.max_w()`.

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

**핵심 의존성은 2개** — 터미널 I/O에 `crossterm`, 문자 너비 측정에 `unicode-width`. 선택적: 비동기에 `tokio`, 직렬화에 `serde`, 이미지 로딩에 `image`. `unsafe` 코드 없음.

> **AI 지원 개발** — [Claude Code](https://docs.anthropic.com/en/docs/claude-code)의 `rust-tui-development-with-slt` 스킬로 전체 API 레퍼런스, 베스트 패턴, 코드 생성 템플릿을 활용하세요. 또는 [tui.builders](https://tui.builders)에서 시각적으로 디자인할 수도 있습니다:

[![tui.builders demo](../assets/tui-builders-demo.gif)](https://tui.builders)

> 위젯을 드래그하고, 인스펙터에서 속성을 설정하고, 관용적인 Rust 코드를 내보냅니다. 무료, 회원가입 불필요, 오픈소스.

## 위젯

55개 이상의 내장 위젯, 보일러플레이트 없음:

```rust
ui.text_input(&mut name);                    // 단일 행 입력
ui.textarea(&mut notes, 5);                  // 다중 행 에디터
if ui.button("Submit").clicked { /* … */ }    // Response 반환
ui.checkbox("Dark mode", &mut dark);         // 토글 체크박스
ui.toggle("Notifications", &mut on);         // 온/오프 스위치
ui.tabs(&mut tabs);                          // 탭 네비게이션
ui.list(&mut items);                         // 선택 가능한 리스트
ui.select(&mut sel);                         // 드롭다운 선택
ui.radio(&mut radio);                        // 라디오 버튼 그룹
ui.multi_select(&mut multi);                 // 다중 선택 체크박스
ui.tree(&mut tree);                          // 확장 가능한 트리 뷰
ui.virtual_list(&mut list, 20, |ui, i| {}); // 가상화 리스트
ui.table(&mut data);                         // 데이터 테이블
ui.spinner(&spin);                           // 로딩 애니메이션
ui.progress(0.75);                           // 프로그레스 바
ui.scrollable(&mut scroll).col(|ui| { });    // 스크롤 컨테이너
ui.toast(&mut toasts);                       // 알림
ui.separator();                              // 수평선
ui.help(&[("q", "quit"), ("Tab", "focus")]); // 키 힌트
ui.link("Docs", "https://docs.rs/superlighttui");      // 클릭 가능한 하이퍼링크 (OSC 8)
ui.modal(|ui| { ui.text("overlay"); });      // 배경 어둡게 하는 모달
ui.overlay(|ui| { ui.text("floating"); });   // 배경 유지 오버레이
ui.command_palette(&mut palette);            // 검색 가능한 커맨드 팔레트
ui.markdown("# Hello **world**");            // Markdown 렌더링
ui.form_field(&mut field);                   // 유효성 검사 포함 레이블 입력
ui.chart(|c| { c.line(&data); c.grid(true); }, 50, 16); // 선/산점도/막대 차트
ui.scatter(&points, 50, 16);                 // 산점도
ui.histogram(&values, 40, 12);               // 자동 빈 히스토그램
ui.bar_chart(&data, 24);                     // 수평 막대 차트
ui.sparkline(&values, 16);                   // 트렌드 라인 ▁▂▃▅▇
ui.canvas(40, 10, |cv| { cv.circle(20, 20, 15); }); // 브레일 캔버스
ui.grid(3, |ui| { /* 3열 그리드 */ });       // 그리드 레이아웃
ui.tooltip("Save the current file");         // 호버 시 툴팁 팝업
ui.calendar(&mut cal);                       // 월 네비게이션 날짜 선택기
ui.screen("home", &screens, |ui| {});        // 화면 라우팅 스택
ui.confirm("Delete?", &mut yes);             // 마우스 지원 확인 다이얼로그
```

모든 위젯이 자체적으로 키보드 이벤트, 포커스 상태, 마우스 인터랙션을 처리합니다.

### 커스텀 위젯

`Widget` 트레이트를 구현해 직접 위젯을 만들 수 있습니다:

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

// 사용법: ui.widget(&mut rating);
```

포커스, 이벤트, 테마, 레이아웃 — 모두 `Context`를 통해 접근 가능합니다. 트레이트 하나, 메서드 하나.

## 기능

<details>
<summary><b>레이아웃</b></summary>

| 기능 | API |
|------|-----|
| 세로 스택 | `ui.col(\|ui\| { })` |
| 가로 스택 | `ui.row(\|ui\| { })` |
| 그리드 레이아웃 | `ui.grid(3, \|ui\| { })` |
| 자식 요소 간 간격 | `.gap(1)` |
| Flex grow | `.grow(1)` |
| 끝으로 밀기 | `ui.spacer()` |
| 정렬 | `.align(Align::Center)` |
| 패딩 | `.p(1)`, `.px(2)`, `.py(1)` |
| 마진 | `.m(1)`, `.mx(2)`, `.my(1)` |
| 고정 크기 | `.w(20)`, `.h(10)` |
| 제약 조건 | `.min_w(10)`, `.max_w(60)` |
| 퍼센트 크기 | `.w_pct(50)`, `.h_pct(80)` |
| 균등 배치 | `.space_between()`, `.space_around()`, `.space_evenly()` |
| 텍스트 줄 바꿈 | `ui.text_wrap("long text...")` |
| 타이틀 포함 보더 | `.border(Border::Rounded).title("Panel")` |
| 면별 보더 | `.border_top(false)`, `.border_sides(BorderSides::horizontal())` |
| 반응형 간격 | `.gap_at(Breakpoint::Md, 2)` |

</details>

<details>
<summary><b>스타일링</b></summary>

```rust
ui.text("styled").bold().italic().underline().fg(Color::Cyan).bg(Color::Black);
```

16개 명명 색상 · 256색 팔레트 · 24비트 RGB · 6개 수정자 · 6개 보더 스타일

</details>

<details>
<summary><b>테마</b></summary>

```rust
// 7개 내장 프리셋
slt::run_with(RunConfig::default().theme(Theme::catppuccin()), |ui| {
    ui.set_theme(Theme::dark()); // 런타임에 전환
});

// 커스텀 테마 빌드
let theme = Theme::builder()
    .primary(Color::Rgb(255, 107, 107))
    .accent(Color::Cyan)
    .build();
```

7개 프리셋 (dark, light, dracula, catppuccin, nord, solarized_dark, tokyo_night). 15개 색상 슬롯 + `is_dark` 플래그로 커스텀 테마 생성. 모든 위젯이 자동으로 상속합니다.

</details>

<details>
<summary><b>스타일 레시피</b></summary>

```rust
use slt::{ContainerStyle, Border, Color};

const CARD: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded).p(1).bg(Color::Indexed(236));

// 한 번 정의하고 어디서든 적용
ui.container().apply(&CARD).col(|ui| { ... });

// 여러 개 합성 — 나중에 쓴 것이 우선
ui.container().apply(&CARD).grow(1).gap(2).col(|ui| { ... });
```

`const` 스타일은 런타임 비용 없음. `.apply()` 체이닝으로 합성 가능.

</details>

<details>
<summary><b>반응형 레이아웃</b></summary>

```rust
ui.container()
    .w(20).md_w(40).lg_w(60)  // 브레이크포인트에서 너비 변경
    .p(1).lg_p(2)
    .col(|ui| { ... });
```

35개 브레이크포인트 조건부 메서드 (`xs_`, `sm_`, `md_`, `lg_`, `xl_`). 브레이크포인트: Xs (<40), Sm (40–79), Md (80–119), Lg (120–159), Xl (≥160).

</details>

<details>
<summary><b>애니메이션</b></summary>

```rust
let mut tween = Tween::new(0.0, 100.0, 60).easing(ease_out_bounce);
let value = tween.value(ui.tick());

let mut spring = Spring::new(0.0, 180.0, 12.0);
spring.set_target(100.0);

let mut kf = Keyframes::new(120)
    .stop(0.0, 0.0).stop(0.5, 100.0).stop(1.0, 50.0)
    .loop_mode(LoopMode::PingPong);
```

9개 이징 함수를 가진 Tween. 스프링 물리 연산. 루프 모드 포함 키프레임 타임라인. Sequence 체이닝. 리스트 애니메이션용 Stagger.

</details>

<details>
<summary><b>비동기 (Async)</b></summary>

```rust
let tx = slt::run_async(|ui, messages: &mut Vec<String>| {
    for msg in messages.drain(..) { ui.text(msg); }
})?;
tx.send("Hello from background!".into()).await?;
```

선택적 tokio 통합. `cargo add superlighttui --features async`로 활성화.

</details>

<details>
<summary><b>인라인 모드</b></summary>

```rust
slt::run_inline(3, |ui| {
    ui.text("Renders below your prompt.");
    ui.text("No alternate screen.").dim();
});
```

터미널을 점유하지 않고 커서 아래에 고정 높이 UI를 렌더링합니다.

</details>

<details>
<summary><b>에러 바운더리</b></summary>

```rust
ui.error_boundary(|ui| {
    ui.text("If this panics, the app keeps running.");
});
```

위젯 패닉을 잡아 앱이 크래시되지 않도록 합니다. 부분적인 커맨드는 롤백되고 폴백이 렌더링됩니다.

</details>

<details>
<summary><b>이미지 렌더링</b></summary>

```sh
cargo add superlighttui --features image
```

```rust
use slt::HalfBlockImage;

let photo = image::open("photo.png").unwrap();
let img = HalfBlockImage::from_dynamic(&photo, 60, 30);
ui.image(&img);
```

하프블록 (▀▄) 이미지 렌더링. Sixel 프로토콜 (v0.13.2): `ui.sixel_image()`로 xterm, foot, mlterm에서 픽셀 퍼펙트 이미지 표시.

</details>

<details>
<summary><b>피처 플래그</b></summary>

| 플래그 | 설명 |
|--------|------|
| `async` | tokio 채널 기반 메시지 패싱으로 `run_async()` |
| `serde` | Style, Color, Theme, 레이아웃 타입의 Serialize/Deserialize |
| `image` | `image` 크레이트로 `HalfBlockImage::from_dynamic()` |
| `full` | 위 모두 포함 |

```toml
[dependencies]
superlighttui = { version = "0.13", features = ["full"] }
```

</details>

<details>
<summary><b>스냅샷 테스트</b></summary>

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

[insta](https://crates.io/crates/insta)와 함께 스냅샷 기반 UI 회귀 테스트에 사용할 수 있습니다.

</details>

<details>
<summary><b>디버그</b></summary>

SLT 앱에서 **F12**를 누르면 레이아웃 디버거 오버레이를 토글할 수 있습니다. 컨테이너 경계, 중첩 깊이, 레이아웃 구조를 표시합니다.

</details>

## 예제

| 예제 | 커맨드 | 내용 |
|------|--------|------|
| hello | `cargo run --example hello` | 최소 구성 |
| counter | `cargo run --example counter` | 상태 + 키보드 |
| demo | `cargo run --example demo` | 전체 위젯 |
| demo_dashboard | `cargo run --example demo_dashboard` | 라이브 대시보드 |
| demo_cli | `cargo run --example demo_cli` | CLI 툴 레이아웃 |
| demo_spreadsheet | `cargo run --example demo_spreadsheet` | 데이터 그리드 |
| demo_website | `cargo run --example demo_website` | 터미널 내 웹사이트 |
| demo_game | `cargo run --example demo_game` | Tetris + Snake + Minesweeper |
| demo_fire | `cargo run --release --example demo_fire` | DOOM 불꽃 효과 (하프블록) |
| demo_ime | `cargo run --example demo_ime` | 한국어/CJK IME 입력 |
| inline | `cargo run --example inline` | 인라인 모드 |
| anim | `cargo run --example anim` | Tween + Spring + Keyframes |
| demo_infoviz | `cargo run --example demo_infoviz` | 데이터 시각화 |
| demo_trading | `cargo run --example demo_trading` | 거래소 스타일 트레이딩 터미널 |
| async_demo | `cargo run --example async_demo --features async` | 백그라운드 태스크 |

## 아키텍처

```
Closure → Context collects Commands → build_tree() → flexbox layout → diff buffer → flush
```

매 프레임: 클로저가 실행되고, SLT가 기술한 내용을 수집하고, Flexbox 레이아웃을 계산하고, 이전 프레임과 비교해 변경된 셀만 플러시합니다.

Pure Rust. 매크로 없음, 코드 생성 없음, 빌드 스크립트 없음.

### 커스텀 백엔드

SLT의 렌더링은 `Backend` 트레이트로 추상화되어 있어 터미널 외의 커스텀 렌더링 타겟을 구현할 수 있습니다:

```rust
use slt::{Backend, AppState, Buffer, Rect, RunConfig, Context, Event};

struct MyBackend { buffer: Buffer }

impl Backend for MyBackend {
    fn size(&self) -> (u32, u32) {
        (self.buffer.area.width, self.buffer.area.height)
    }
    fn buffer_mut(&mut self) -> &mut Buffer { &mut self.buffer }
    fn flush(&mut self) -> std::io::Result<()> {
        // self.buffer를 타겟 (canvas, GPU, network 등)에 렌더링
        Ok(())
    }
}
```

`Backend` 트레이트의 메서드는 3개: `size()`, `buffer_mut()`, `flush()`. 커스텀 백엔드는 완전히 렌더링된 `Buffer`를 받아 WebGL, egui 임베드, SSH 터널, 테스트 하네스 등 원하는 방식으로 표시할 수 있습니다.

### AI 네이티브 위젯

SLT에는 AI/LLM 워크플로우를 위한 전용 위젯이 포함되어 있습니다:

| 위젯 | 설명 |
|------|------|
| `streaming_text()` | 깜빡이는 커서와 함께 토큰 단위 텍스트 표시 |
| `streaming_markdown()` | 제목, 코드 블록, 인라인 서식 포함 스트리밍 Markdown |
| `tool_approval()` | 툴 호출에 대한 사람의 승인/거부 |
| `context_bar()` | 활성 컨텍스트 소스를 보여주는 토큰 카운터 바 |
| `markdown()` | 정적 Markdown 렌더링 |
| `code_block()` | 신택스 하이라이팅 코드 표시 |

## 기여

가이드라인은 [CONTRIBUTING.md](../CONTRIBUTING.md)를 참고하세요.

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
[Examples]: https://github.com/subinium/SuperLightTUI/tree/main/examples
[Contributing]: https://github.com/subinium/SuperLightTUI/blob/main/CONTRIBUTING.md
[License]: ../LICENSE
