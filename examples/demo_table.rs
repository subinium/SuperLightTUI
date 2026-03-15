use slt::{Context, TableState, TextInputState, Theme};

fn main() -> std::io::Result<()> {
    let mut table = TableState::new(
        vec!["Rank", "Name", "Language", "Stars", "Category"],
        vec![
            vec!["1", "Bubbletea", "Go", "30200", "TUI"],
            vec!["2", "Textual", "Python", "26800", "TUI"],
            vec!["3", "Charm", "Go", "18500", "CLI"],
            vec!["4", "Ratatui", "Rust", "12500", "TUI"],
            vec!["5", "Rich", "Python", "51000", "CLI"],
            vec!["6", "Ink", "JS/TS", "8200", "TUI"],
            vec!["7", "Blessed", "JS", "11200", "TUI"],
            vec!["8", "Cursive", "Rust", "4200", "TUI"],
            vec!["9", "Prompts", "JS/TS", "9500", "CLI"],
            vec!["10", "Click", "Python", "15800", "CLI"],
            vec!["11", "Cobra", "Go", "39000", "CLI"],
            vec!["12", "Clap", "Rust", "14500", "CLI"],
            vec!["13", "Ncurses", "C", "2100", "Library"],
            vec!["14", "Notcurses", "C", "3700", "Library"],
            vec!["15", "SLT", "Rust", "500", "TUI"],
            vec!["16", "Tview", "Go", "11000", "TUI"],
            vec!["17", "Crossterm", "Rust", "3300", "Library"],
            vec!["18", "Urwid", "Python", "2800", "TUI"],
            vec!["19", "Termion", "Rust", "2200", "Library"],
            vec!["20", "FTXUI", "C++", "7200", "TUI"],
        ],
    );
    table.page_size = 8;

    let mut filter_input = TextInputState::with_placeholder("Type to filter...");
    let mut dark_mode = true;

    slt::run_with(
        slt::RunConfig {
            mouse: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
                ui.quit();
            }
            ui.set_theme(if dark_mode {
                Theme::dark()
            } else {
                Theme::light()
            });
            let theme = *ui.theme();

            ui.container().pad(1).grow(1).col(|ui| {
                ui.row(|ui| {
                    ui.text("Table Demo").bold().fg(theme.primary);
                    ui.spacer();
                    ui.toggle("Dark", &mut dark_mode);
                });

                ui.separator();

                ui.row(|ui| {
                    ui.text("Filter").bold().fg(theme.text_dim);
                    ui.container().grow(1).col(|ui| {
                        ui.text_input(&mut filter_input);
                    });
                });
                table.set_filter(&filter_input.value);

                ui.container().grow(1).gap(0).col(|ui| {
                    ui.table(&mut table);
                });

                ui.separator();

                if let Some(row) = table.selected_row() {
                    ui.row(|ui| {
                        ui.text("Selected").bold().fg(theme.primary);
                        ui.text(row.join(" · "));
                    });
                } else {
                    ui.text("No matching rows").dim();
                }

                ui.row(|ui| {
                    ui.text(format!(
                        "{} / {} rows",
                        table.visible_indices().len(),
                        table.rows.len(),
                    ))
                    .dim();
                    ui.spacer();
                    if let Some(col) = table.sort_column {
                        let dir = if table.sort_ascending { "ASC" } else { "DESC" };
                        ui.text(format!("{} {}", table.headers[col], dir))
                            .fg(theme.text_dim);
                    }
                });

                ui.help(&[
                    ("q", "quit"),
                    ("↑↓/jk", "select"),
                    ("PgUp/Dn", "page"),
                    ("Header click", "sort"),
                ]);
            });
        },
    )
}
