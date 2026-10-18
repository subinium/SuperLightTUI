<div align="center">

# SuperLightTUI

**웹 페이지처럼 터미널 UI를 만드세요** — flexbox 레이아웃, Tailwind 스타일 체이닝, hooks, 클릭/hover/스크롤을 Rust 클로저 하나로.

**빠르게 작성하고, 의존성 트리는 가볍게**(직접 필수 의존성 4개, 기본 feature 기준 26개 resolved — ratatui 0.30.2 + crossterm은 67개, 2026-10 측정).

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[문서 인덱스] · [빠른 시작] · [위젯 가이드] · [패턴 가이드] · [예제 가이드] · [런타임 모드 가이드] · [아키텍처 가이드]

[English](../README.md) · [中文](README.zh-CN.md) · [Español](README.es.md) · [日本語](README.ja.md) · **한국어**

</div>

> 이 문서는 [영문 README](../README.md)의 요약 번역입니다. 내용이 다르면 영문 README와 영문 가이드가 기준입니다.

SuperLightTUI(SLT)는 public grammar를 의도적으로 작게 유지하는 Rust용 immediate-mode TUI 라이브러리입니다.
클로저 하나를 작성하면 SLT가 매 프레임 그 클로저를 호출하고, 레이아웃, 포커스, diff, 렌더링은 라이브러리가 처리합니다.
상태는 일반 Rust 변수이며, 앱 트레이트나 메시지 enum, retained 위젯 트리를 따로 준비할 필요가 없습니다.

## 빠른 시작

```sh
cargo add superlighttui
```

<!-- slt-check: {"name": "readme_ko_hello", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut slt::Context| {
        ui.text("hello, world");
    })
}
```

5줄이면 됩니다. `App` 트레이트도, `Model`/`Update`/`View`도, 수동 이벤트 루프도 없습니다.
Ctrl+C를 누르면 기본적으로 깔끔하게 종료됩니다(`RunConfig::handle_ctrl_c(false)`로 끌 수 있습니다).

저장소를 클론했다면 `cargo run --example hello`로 바로 실행해 볼 수 있습니다. 전체 예제 목록은 [예제 가이드]에 있습니다.

## 주요 특징

- **마우스 기본 활성화**: 클릭, 휠, 드래그가 기본으로 동작하고(SSH에서는 클릭과 휠만), hover는 `MouseMode::Hover`, 완전히 끄려면 `MouseMode::Off`.
- **Flexbox 레이아웃**: 대부분의 화면은 `row()`, `col()`, `container()`로 구성합니다.
- **Tailwind 스타일 체이닝**: `.bold().fg(Color::Cyan).p(1).gap(1)`처럼 메서드 체이닝으로 스타일링합니다.
- **Hooks**: 컴포넌트 로컬 상태를 위한 hook을 제공하고, 앱 상태는 일반 Rust 변수로 둡니다.
- **50개 이상의 위젯**: 입력, 테이블, 리스트, 탭, 모달, 토스트, 차트, 마크다운, 커맨드 팔레트 등.
- **테스트**: `TestBackend`로 헤드리스 렌더링과 상호작용을 검증합니다.
- **여러 런타임**: 풀스크린, inline, static + inline, 브라우저(`slt-wasm`)에서 같은 클로저가 동작합니다.
- **성능**: 동일 조건의 ratatui 비교 하네스에서 프레임당 렌더링이 더 빠릅니다. 자세한 내용은 [성능 가이드]를 보세요.

## 라이브러리 익히기

아래 가이드는 **영어로 작성되어 있습니다**. 학습 경로는 순서대로 읽으세요. 각 가이드는 앞의 가이드를 전제로 합니다.
전체 페이지 목록은 [문서 인덱스]에 있습니다.

| 단계 | 가이드 | 내용 |
|------|--------|------|
| 1 | [빠른 시작] | 설치, 첫 앱, 클로저 mental model, 레이아웃, 위젯 상태 |
| 2 | [상태 가이드](guide/02-state.md) | 앱 상태 vs hooks vs 위젯 상태, 그리고 이전 프레임 규칙 |
| 3 | [패턴 가이드] | 컴포넌트, 폼, 화면, 모달, async 메시지, 큰 앱 구조 |
| 4 | [상호작용 가이드](guide/04-interaction.md) | 마우스, 키보드, 포커스, 단축키, hover, 드래그, 휠과 스크롤 |
| 5 | [오버레이 가이드](guide/05-overlays.md) | `Anchor`와 오버레이 위치 지정을 CSS/Flutter/React Native에 대응 |
| 6 | [테마 가이드](guide/06-theming.md) | Theme struct, preset, ThemeBuilder, 커스텀 테마 |
| 7 | [애니메이션 가이드](guide/07-animation.md) | Tween, spring, keyframe, sequence, stagger |
| 8 | [테스트 가이드](guide/08-testing.md) | `TestBackend`, `EventBuilder`, multi-frame 테스트, 스냅샷 |
| 9 | [런타임 모드 가이드] | `run` vs `run_inline` vs `run_static`, `RunConfig`, `frame()` 직접 구동 |
| 10 | [브라우저 가이드](guide/10-browser.md) | `slt-wasm` 브라우저 런타임 |
| 11 | [터미널 호환성](guide/11-terminal-compatibility.md) | 터미널/멀티플렉서/SSH 지원 표, 환경 변수, fallback |

다른 스택에서 오셨다면: [웹 개발자 가이드](coming-from/web.md) (HTML/CSS/React → SLT), [Ratatui 가이드](coming-from/ratatui.md) (위젯, 레이아웃, 점진적 `ratatui` 브리지).

| 레퍼런스 | 내용 |
|----------|------|
| [위젯 가이드] | 위젯, 런타임 메서드, 상태 타입의 분류별 카탈로그 |
| [상태 API 가이드](reference/state.md) | 위젯 상태의 public 필드, accessor, 변경 메서드 |
| [쿡북](reference/cookbook.md) | 자주 만드는 앱의 복사용 레시피. 각 레시피는 실행 가능한 예제로 뒷받침 |
| [예제 가이드] | 제품 형태와 기능 영역별로 묶은 실행 예제 |
| [기능 가이드](reference/features.md) | feature flag, optional dependency, 추천 조합 |
| [마이그레이션 가이드](reference/migration.md) | 버전별 breaking change와 업그레이드 경로 |
| [디버깅 가이드](reference/debugging.md) | F12 오버레이, clipping, 포커스 관련 함정 |
| [AI 가이드](reference/ai-guide.md) | AI 보조 빌더와 코딩 에이전트를 위한 가장 빠른 경로 |

기여자용: [설계 원칙](internals/design-principles.md) (API 규칙과 네이밍), [아키텍처 가이드] (모듈 맵, 프레임 라이프사이클), [성능 가이드] (예산과 벤치마크).

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
[빠른 시작]: guide/01-quick-start.md
[패턴 가이드]: guide/03-patterns.md
[런타임 모드 가이드]: guide/09-runtime-modes.md
[위젯 가이드]: reference/widgets.md
[예제 가이드]: reference/examples.md
[아키텍처 가이드]: internals/architecture.md
[성능 가이드]: internals/performance.md
