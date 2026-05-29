//! Demo: CJK (Korean / Chinese / Japanese) title and content rendering with mouse interaction.
//!
//! Verifies:
//! - Title with CJK characters does not overflow the right border.
//! - Long titles are truncated cleanly at the box width.
//! - Mixed ASCII / CJK works in text bodies and form fields.
//! - Wide-char width is accounted for so wrap/truncate stays inside the box.
//! - Mouse hover and click work on bordered cards (group_hover_border_style + clicked).
//! - Focus cycling between text inputs works programmatically (set_focus_index).

use slt::widgets::TextInputState;
use slt::{Border, Color, Context, Style};

const CARD_TITLES: [&str; 4] = ["설정창", "管理设置", "設定パネル", "Mixed 한中日 abc"];

fn main() -> std::io::Result<()> {
    let mut name_input = TextInputState::with_placeholder("이름을 입력하세요");
    let mut tag_input = TextInputState::with_placeholder("태그");

    slt::run(|ui: &mut Context| {
        render_frame(ui, &mut name_input, &mut tag_input);
    })
}

/// Render one frame with fresh, default state — used by visual snapshot tests
/// in `tests/visual_snapshots.rs`. The runtime example uses [`render_frame`]
/// directly so the text-input contents persist across frames.
pub fn render(ui: &mut Context) {
    let mut name_input = TextInputState::with_placeholder("이름을 입력하세요");
    let mut tag_input = TextInputState::with_placeholder("태그");
    render_frame(ui, &mut name_input, &mut tag_input);
}

/// Render one frame of the CJK demo into the supplied context.
///
/// Most state (per-card counts, last-clicked label) is stored via `use_state`
/// hooks on the context, so this is safe to call repeatedly in the runtime
/// loop.  The two `TextInputState` values for the form fields are passed by
/// `&mut` because they are not yet hooked.
pub fn render_frame(
    ui: &mut Context,
    name_input: &mut TextInputState,
    tag_input: &mut TextInputState,
) {
    // Per-frame snapshots of mouse position and focus state, captured up
    // front so the closing status bar can read them without disturbing
    // the layout pass.
    let mouse_pos = ui.mouse_pos();
    let focus_index = ui.focus_index();
    let focus_count = ui.focus_count();

    // Per-card click counters and "last clicked" label persist across
    // frames via `use_state`. Order is fixed at the top of the closure
    // so the hook cursor stays stable.
    let counts_state = ui.use_state(|| [0u32; 4]);
    let last_clicked_state = ui.use_state(|| Option::<String>::None);

    let _ = ui
        .bordered(Border::Rounded)
        .title("한글 / 中文 / 日本語 demo")
        .p(1)
        .grow(1)
        .col(|ui| {
            ui.text("CJK 위젯 데모 — Ctrl+Q to quit · 카드를 클릭해 보세요")
                .bold()
                .fg(Color::Cyan);
            let _ = ui.separator();

            let _ = ui.row(|ui| {
                let _ = ui
                    .bordered(Border::Rounded)
                    .title("짧은 제목")
                    .p(1)
                    .grow(1)
                    .col(|ui| {
                        ui.text("한국어 본문이 박스 안에 잘 들어가는지 확인합니다.")
                            .wrap();
                        ui.text("中文 段落 — 测试中文换行与右边界裁剪。").wrap();
                        ui.text("日本語 — 折り返しと右端の境界を確認します。")
                            .wrap();
                    });

                let _ = ui
                    .bordered(Border::Rounded)
                    .title("Mixed 한·中·日 title overflow test")
                    .p(1)
                    .grow(1)
                    .col(|ui| {
                        ui.text("Long titles must clip without breaking the right border (┐).");
                        ui.text("긴 제목은 오른쪽 테두리를 침범하지 않아야 합니다.")
                            .wrap();
                    });
            });

            let _ = ui.separator();
            let _ = ui.row(|ui| {
                // Group name encodes the focus role so `is_group_focused`
                // (used by `text_input` internally) stays unique even if
                // the title text were to change later.
                let _ = ui
                    .group("input-name")
                    .border(Border::Rounded)
                    .title("입력")
                    .p(1)
                    .grow(1)
                    .col(|ui| {
                        ui.text("이름:").dim();
                        let _ = ui.text_input(name_input);
                        ui.text("태그:").dim();
                        let _ = ui.text_input(tag_input);
                        ui.text("Tab으로 포커스 이동, 또는 아래 [포커스] 버튼")
                            .dim();
                    });

                let _ = ui
                    .bordered(Border::Rounded)
                    .title("결과")
                    .p(1)
                    .grow(1)
                    .col(|ui| {
                        ui.text(format!("이름 = {}", name_input.value)).bold();
                        ui.text(format!("태그 = {}", tag_input.value)).bold();
                        let total: u32 = counts_state.get(ui).iter().sum();
                        ui.text(format!("카드 클릭 합계 = {total}"))
                            .fg(Color::Green);
                    });
            });

            let _ = ui.separator();
            ui.text("Truncation table — 각 박스는 너비 12, 제목이 잘려야 정상 (hover/click)")
                .dim();
            let _ = ui.row(|ui| {
                // Read counts up front so the click handler below can set
                // the next value without holding an immutable borrow on
                // `ui` while we also read `is_group_hovered`.
                let counts_now = *counts_state.get(ui);
                let mut clicked: Option<usize> = None;

                for (idx, title) in CARD_TITLES.iter().enumerate() {
                    // Stable per-card group name. The `card-` prefix keeps
                    // it from colliding with future groups; the index is
                    // the load-bearing part for `is_group_hovered`.
                    let group_name = format!("card-{idx}");
                    let count = counts_now[idx];
                    let resp = ui
                        .group(&group_name)
                        .border(Border::Single)
                        .group_hover_border_style(Style::new().fg(Color::Yellow))
                        .min_w(12)
                        .max_w(12)
                        .p(0)
                        .title(*title)
                        .col(|ui| {
                            if ui.is_group_hovered(&group_name) {
                                ui.text(format!("hits {count}")).fg(Color::Yellow);
                            } else {
                                ui.text(format!("hits {count}")).dim();
                            }
                        });
                    if resp.clicked {
                        clicked = Some(idx);
                    }
                }

                if let Some(idx) = clicked {
                    let counts = counts_state.get_mut(ui);
                    counts[idx] = counts[idx].saturating_add(1);
                    *last_clicked_state.get_mut(ui) = Some(CARD_TITLES[idx].to_string());
                }
            });

            let _ = ui.separator();
            let _ = ui.row(|ui| {
                let last_label = last_clicked_state
                    .get(ui)
                    .clone()
                    .unwrap_or_else(|| "(none)".to_string());
                let mouse_label = match mouse_pos {
                    Some((x, y)) => format!("mouse=({x},{y})"),
                    None => "mouse=(—)".to_string(),
                };
                ui.text(format!("{mouse_label}  ·  last clicked = {last_label}"))
                    .dim();
                ui.text(format!("focus={focus_index}/{focus_count}")).dim();

                if ui.button("초기화 / Reset").clicked {
                    *counts_state.get_mut(ui) = [0; 4];
                    *last_clicked_state.get_mut(ui) = None;
                    name_input.value.clear();
                    tag_input.value.clear();
                    name_input.cursor = 0;
                    tag_input.cursor = 0;
                }
                if ui.button("포커스 / Focus next").clicked && focus_count > 0 {
                    ui.set_focus_index((focus_index + 1) % focus_count);
                }
            });
        });
}
