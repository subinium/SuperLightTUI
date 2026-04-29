//! Demo: IME (input method editor) flow with Korean / Japanese / Chinese
//! composition input across two text inputs and a textarea, plus a tiny
//! filter list to exercise live search across CJK content.
//!
//! Verifies:
//! - Multi-byte composition input updates `TextInputState.value` correctly.
//! - CJK + ASCII mixed strings round-trip through `text_input` / `textarea`.
//! - Live filtering on a vec of CJK strings stays consistent each frame.
//!
//! Archetype: Standard. The render function holds no overlays and does not
//! write to scrollback, so it composes cleanly into a tabbed tour.

use slt::widgets::{TextInputState, TextareaState};
use slt::{Context, KeyCode, RunConfig};

const ITEMS: &[&str] = &[
    "한글 입력 테스트",
    "日本語テスト",
    "中文测试",
    "English test",
    "Emoji 🎉🔥",
    "Mixed 한글+English",
    "서울특별시",
    "부산광역시",
    "대구광역시",
    "인천광역시",
];

/// State persisted across frames for the IME demo. Owned by either the
/// standalone `main` or the parent tour.
pub struct DemoState {
    pub name: TextInputState,
    pub search: TextInputState,
    pub message: TextareaState,
    pub results: Vec<String>,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            name: TextInputState::with_placeholder("이름을 입력하세요"),
            search: TextInputState::with_placeholder("검색어 입력..."),
            message: TextareaState::new(),
            results: Vec::new(),
        }
    }
}

/// Render one frame of the IME demo into the supplied context.
///
/// Composing demos (e.g. `examples/text_tour.rs`) call this with their
/// owned state so input persists across frames.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let theme = *ui.theme();
    let term_h = ui.height();

    let _ = ui.col(|ui| {
        let _ = ui.container().grow(1).gap(1).p(1).col(|ui| {
            ui.text("IME Input Demo").bold().fg(theme.primary);
            ui.text("Hangul / Japanese / Chinese composition input")
                .dim();
            ui.separator();

            let _ = ui.container().gap(2).row(|ui| {
                let _ = ui.container().grow(1).gap(1).col(|ui| {
                    ui.text("Name").bold();
                    let _ = ui.text_input(&mut state.name);
                    if !state.name.value.is_empty() {
                        ui.line(|ui| {
                            ui.text("-> ");
                            ui.text(&state.name.value).fg(theme.accent);
                            ui.text(format!(" ({} chars)", state.name.value.chars().count()))
                                .dim();
                        });
                    }
                });

                let _ = ui.container().grow(1).gap(1).col(|ui| {
                    ui.text("Search").bold();
                    let _ = ui.text_input(&mut state.search);

                    let query = state.search.value.to_lowercase();
                    let tokens: Vec<&str> = query.split_whitespace().collect();
                    state.results = ITEMS
                        .iter()
                        .filter(|item| {
                            let lower = item.to_lowercase();
                            tokens.is_empty() || tokens.iter().all(|t| lower.contains(t))
                        })
                        .map(|s| s.to_string())
                        .collect();
                    ui.text(format!("{}/{} items", state.results.len(), ITEMS.len()))
                        .dim();
                });
            });

            ui.separator();

            ui.text("Message").bold();
            let rows = term_h.saturating_sub(16).max(5);
            let _ = ui.textarea(&mut state.message, rows);

            let total: usize = state.message.lines.iter().map(|l| l.chars().count()).sum();
            ui.text(format!(
                "{} lines, {} chars",
                state.message.lines.len(),
                total
            ))
            .dim();
        });

        let _ = ui.help(&[
            ("^Q/Esc", "quit"),
            ("Tab", "next field"),
            ("Type", "CJK input"),
        ]);
    });
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();
    slt::run_with(
        RunConfig::default().mouse(true).kitty_keyboard(true),
        move |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
                ui.quit();
            }
            render(ui, &mut state);
        },
    )
}
