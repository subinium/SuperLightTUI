use slt::{Context, KeyCode, RunConfig};

fn main() -> std::io::Result<()> {
    let mut name = slt::TextInputState::with_placeholder("이름을 입력하세요");
    let mut message = slt::TextareaState::new();
    let mut search = slt::TextInputState::with_placeholder("검색어 입력...");
    let mut results: Vec<String> = Vec::new();

    let items = vec![
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

    slt::run_with(
        RunConfig {
            mouse: true,
            kitty_keyboard: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
                ui.quit();
            }

            let theme = *ui.theme();
            let term_h = ui.height();

            let _ = ui.col(|ui| {
                let _ = ui.container().grow(1).gap(1).p(1).col(|ui| {
                    ui.text("IME Input Demo").bold().fg(theme.primary);
                    ui.text("한글, 日本語, 中文 조합 입력 테스트").dim();
                    ui.separator();

                    let _ = ui.row_gap(2, |ui| {
                        let _ = ui.container().grow(1).gap(1).col(|ui| {
                            ui.text("Name").bold();
                            let _ = ui.text_input(&mut name);
                            if !name.value.is_empty() {
                                ui.line(|ui| {
                                    ui.text("→ ");
                                    ui.text(&name.value).fg(theme.accent);
                                    ui.text(format!(" ({} chars)", name.value.chars().count()))
                                        .dim();
                                });
                            }
                        });

                        let _ = ui.container().grow(1).gap(1).col(|ui| {
                            ui.text("Search").bold();
                            let _ = ui.text_input(&mut search);

                            let query = search.value.to_lowercase();
                            let tokens: Vec<&str> = query.split_whitespace().collect();
                            results = items
                                .iter()
                                .filter(|item| {
                                    let lower = item.to_lowercase();
                                    tokens.is_empty() || tokens.iter().all(|t| lower.contains(t))
                                })
                                .map(|s| s.to_string())
                                .collect();
                            ui.text(format!("{}/{} items", results.len(), items.len()))
                                .dim();
                        });
                    });

                    ui.separator();

                    ui.text("Message").bold();
                    let rows = term_h.saturating_sub(16).max(5);
                    let _ = ui.textarea(&mut message, rows);

                    let total: usize = message.lines.iter().map(|l| l.chars().count()).sum();
                    ui.text(format!("{} lines, {} chars", message.lines.len(), total,))
                        .dim();
                });

                let _ = ui.help(&[
                    ("^Q/Esc", "quit"),
                    ("Tab", "next field"),
                    ("Type", "한글/CJK input"),
                ]);
            });
        },
    )
}
