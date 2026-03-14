use slt::{Border, Context, KeyCode, RunConfig};

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

            ui.col(|ui| {
                ui.container().grow(1).gap(1).p(1).col(|ui| {
                    ui.text("IME Input Demo").bold().fg(theme.primary);
                    ui.text("한글, 日本語, 中文 조합 입력 테스트").dim();
                    ui.separator();

                    ui.row_gap(2, |ui| {
                        ui.container().grow(1).gap(1).col(|ui| {
                            ui.text("Text Input").bold();
                            ui.text_input(&mut name);
                            if !name.value.is_empty() {
                                ui.line(|ui| {
                                    ui.text("입력값: ");
                                    ui.text(&name.value).fg(theme.accent);
                                    ui.text(format!(" ({} chars)", name.value.chars().count()))
                                        .dim();
                                });
                            }

                            ui.text("Search (multi-token filter)").bold();
                            ui.text_input(&mut search);

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

                            ui.bordered(Border::Rounded).col(|ui| {
                                if results.is_empty() {
                                    ui.text("검색 결과 없음").dim();
                                } else {
                                    for item in &results {
                                        ui.text(item);
                                    }
                                }
                            });
                            ui.text(format!("{}/{} items", results.len(), items.len()))
                                .dim();
                        });

                        ui.container().grow(1).gap(1).col(|ui| {
                            ui.text("Textarea (auto word-wrap)").bold();
                            let half_w = ui.width() / 2;
                            message.wrap_width = Some(half_w.saturating_sub(4));
                            ui.textarea(&mut message, 8);
                            let total: usize =
                                message.lines.iter().map(|l| l.chars().count()).sum();
                            ui.text(format!(
                                "{} lines, {} chars, wrap@{}",
                                message.lines.len(),
                                total,
                                half_w.saturating_sub(4),
                            ))
                            .dim();
                        });
                    });
                });

                ui.help(&[
                    ("^Q/Esc", "quit"),
                    ("Tab", "next field"),
                    ("Type", "한글/CJK input"),
                ]);
            });
        },
    )
}
