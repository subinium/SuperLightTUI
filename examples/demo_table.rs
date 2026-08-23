//! Demo: searchable, sortable table.
//!
//! Archetype: **Standard** (full-canvas, no overlay, no scrollback).
//!
//! §2 (Demo Guide): exposes `pub fn render(ui, &mut DemoState)` so a
//! composing demo (e.g. `examples/showcase_tour.rs`) can preserve the
//! typed filter, current sort column, and table cursor across tab
//! switches. The standalone `main()` is a thin wrapper that owns the
//! quit triple and a single persistent `DemoState`.

use slt::{Context, TableState, TextInputState, Theme};

/// Persistent state for the table demo: the data table, the filter
/// input, and a `dark_mode` toggle.
pub struct DemoState {
    pub table: TableState,
    pub filter: TextInputState,
    pub dark_mode: bool,
}

impl DemoState {
    pub fn new() -> Self {
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

        Self {
            table,
            filter: TextInputState::with_placeholder("Type to filter..."),
            dark_mode: true,
        }
    }
}

impl Default for DemoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render one frame of the table demo. Caller owns `DemoState` so the
/// typed filter, sort column, and table cursor persist across frames.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    ui.set_theme(if state.dark_mode {
        Theme::dark()
    } else {
        Theme::light()
    });
    let theme = *ui.theme();

    let _ = ui.container().p(1).grow(1).col(|ui| {
        let _ = ui.row(|ui| {
            ui.text("Table Demo").bold().fg(theme.primary);
            ui.spacer();
            let _ = ui.toggle("Dark", &mut state.dark_mode);
        });

        let _ = ui.separator();

        let _ = ui.row(|ui| {
            ui.text("Filter").bold().fg(theme.text_dim);
            let _ = ui.container().grow(1).col(|ui| {
                let _ = ui.text_input(&mut state.filter);
            });
        });
        state.table.set_filter(&state.filter.value);

        let _ = ui.container().grow(1).gap(0).col(|ui| {
            let _ = ui.table(&mut state.table);
        });

        let _ = ui.separator();

        if let Some(row) = state.table.selected_row() {
            let _ = ui.row(|ui| {
                ui.text("Selected").bold().fg(theme.primary);
                ui.text(row.join(" \u{00b7} "));
            });
        } else {
            ui.text("No matching rows").dim();
        }

        let _ = ui.row(|ui| {
            ui.text(format!(
                "{} / {} rows",
                state.table.visible_indices().len(),
                state.table.rows().len(),
            ))
            .dim();
            ui.spacer();
            if let Some(col) = state.table.sort_column {
                let dir = if state.table.sort_ascending {
                    "ASC"
                } else {
                    "DESC"
                };
                ui.text(format!("{} {}", state.table.headers()[col], dir))
                    .fg(theme.text_dim);
            }
        });

        let _ = ui.help(&[
            ("Ctrl-Q / Esc", "quit"),
            ("\u{2191}\u{2193}/jk", "select"),
            ("PgUp/Dn", "page"),
            ("Header click", "sort"),
        ]);
    });
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::new();
    slt::run_with(
        slt::RunConfig::default().mouse(true),
        move |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
                ui.quit();
            }
            render(ui, &mut state);
        },
    )
}
