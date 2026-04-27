//! Cookbook: searchable and sortable data table.
//!
//! Demonstrates:
//! - `TextInputState` wired to `TableState::set_filter` (Tab to focus it)
//! - `s` cycles the sort column, Enter inverts direction
//! - global shortcuts use `consume_key*` so typed characters in the input
//!   are never double-handled
//! - footer status bar shows rows + active sort
//! - Ctrl+Q or Esc to quit

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

fn main() -> std::io::Result<()> {
    let mut table = TableState::new(
        HEADERS.to_vec(),
        ROWS.iter().map(|r| r.to_vec()).collect::<Vec<_>>(),
    );
    table.zebra = true;
    table.sort_column = Some(0);

    let mut search = TextInputState::with_placeholder("filter... (press / anywhere)");

    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("Cookbook — Table")
            .p(1)
            .gap(1)
            .grow(1)
            .col(|ui| {
                let _ = ui.row_gap(1, |ui| {
                    ui.text("Search:").dim();
                    let resp = ui.text_input(&mut search);
                    if resp.changed {
                        table.set_filter(search.value.clone());
                    }
                });

                let _ = ui.table(&mut table);

                let sort_label = match table.sort_column {
                    Some(c) => {
                        let dir = if table.sort_ascending { "asc" } else { "desc" };
                        format!("sorted by {} {dir}", HEADERS[c])
                    }
                    None => "unsorted".to_string(),
                };
                let n = table.visible_indices().len();
                let _ = ui.row(|ui| {
                    ui.text(format!("{n} rows / {sort_label}")).fg(Color::Cyan);
                    ui.spacer();
                    ui.text("s cycle sort   Enter invert   Esc quit").dim();
                });
            });

        // Global shortcuts run AFTER widgets so a focused text_input can
        // consume typed characters first.
        if ui.consume_key('s') {
            let next = table
                .sort_column
                .map(|c| (c + 1) % HEADERS.len())
                .unwrap_or(0);
            table.sort_by(next);
        }
        if ui.consume_key_code(KeyCode::Enter) {
            if let Some(c) = table.sort_column {
                table.toggle_sort(c);
            }
        }
    })
}
