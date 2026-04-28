//! Cookbook: searchable and sortable data table.
//!
//! Archetype: **Standard** (full-canvas, no overlay, no scrollback).
//!
//! Demonstrates:
//! - `TextInputState` wired to `TableState::set_filter` (Tab to focus it)
//! - `s` cycles the sort column, Enter inverts direction
//! - global shortcuts use `consume_key*` so typed characters in the input
//!   are never double-handled
//! - footer status bar shows rows + active sort
//! - Ctrl+Q or Esc to quit
//!
//! §2 (Demo Guide): exposes `pub fn render(ui, &mut DemoState)` so a
//! composing demo can preserve the typed filter, current sort column,
//! and table cursor across tab switches. The standalone `main()` is a
//! thin wrapper.

use slt::{Border, Color, Context, KeyCode, KeyModifiers, TableState, TextInputState};

const HEADERS: &[&str] = &["id", "name", "role", "score"];

const ROWS: &[[&str; 4]] = &[
    ["1", "Alice", "engineer", "92"],
    ["2", "Bob", "designer", "78"],
    ["3", "Carol", "engineer", "84"],
    ["4", "Dan", "manager", "66"],
    ["5", "Eve", "engineer", "99"],
    ["6", "Frank", "designer", "71"],
    ["7", "Grace", "researcher", "88"],
    ["8", "Heidi", "engineer", "74"],
    ["9", "Ivan", "manager", "59"],
    ["10", "Judy", "researcher", "95"],
];

/// Persistent table + filter state.
pub struct DemoState {
    pub table: TableState,
    pub search: TextInputState,
}

impl DemoState {
    pub fn new() -> Self {
        let mut table = TableState::new(
            HEADERS.to_vec(),
            ROWS.iter().map(|r| r.to_vec()).collect::<Vec<_>>(),
        );
        table.zebra = true;
        table.sort_column = Some(0);

        Self {
            table,
            search: TextInputState::with_placeholder("filter... (press / anywhere)"),
        }
    }
}

impl Default for DemoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render one frame of the table demo. Caller owns `DemoState` so the
/// filter text, sort column, and table cursor persist across frames.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let _ = ui
        .bordered(Border::Rounded)
        .title("Cookbook: Table")
        .p(1)
        .gap(1)
        .grow(1)
        .col(|ui| {
            let _ = ui.row_gap(1, |ui| {
                ui.text("Search:").dim();
                let resp = ui.text_input(&mut state.search);
                if resp.changed {
                    state.table.set_filter(state.search.value.clone());
                }
            });

            let _ = ui.table(&mut state.table);

            let sort_label = match state.table.sort_column {
                Some(c) => {
                    let dir = if state.table.sort_ascending {
                        "asc"
                    } else {
                        "desc"
                    };
                    format!("sorted by {} {dir}", HEADERS[c])
                }
                None => "unsorted".to_string(),
            };
            let n = state.table.visible_indices().len();
            let _ = ui.row(|ui| {
                ui.text(format!("{n} rows / {sort_label}")).fg(Color::Cyan);
                ui.spacer();
                ui.text("s cycle sort   Enter invert   Esc quit").dim();
            });
        });

    // Global shortcuts run AFTER widgets so a focused text_input can
    // consume typed characters first.
    if ui.consume_key('s') {
        let next = state
            .table
            .sort_column
            .map(|c| (c + 1) % HEADERS.len())
            .unwrap_or(0);
        state.table.sort_by(next);
    }
    if ui.consume_key_code(KeyCode::Enter) {
        if let Some(c) = state.table.sort_column {
            state.table.toggle_sort(c);
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::new();
    slt::run(move |ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        render(ui, &mut state);
    })
}
