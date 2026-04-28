//! v0.20.0 demo: `use_state_keyed` for runtime-keyed per-item state.
//!
//! Each list item carries its own counter, keyed by a runtime
//! `format!("counter-{i}")` string. The list grows when you press `+`, shrinks
//! on `-`, and per-item counters change with `j`/`k` while the row is
//! highlighted. Stale entries from removed rows are tolerated (state stays
//! in the map but is unused).
//!
//! Run: `cargo run --example v020_use_state_keyed`

use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let mut item_count: usize = 3;
    let mut selected: usize = 0;
    slt::run(move |ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
            return;
        }
        if ui.key_code(KeyCode::Char('+')) {
            item_count = (item_count + 1).min(20);
        }
        if ui.key_code(KeyCode::Char('-')) {
            item_count = item_count.saturating_sub(1).max(1);
        }
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            selected = selected.saturating_sub(1);
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            selected = (selected + 1).min(item_count.saturating_sub(1));
        }
        let bump = ui.key('l') || ui.key_code(KeyCode::Right);
        let drop = ui.key('h') || ui.key_code(KeyCode::Left);

        let _ = ui
            .bordered(Border::Rounded)
            .title("use_state_keyed — per-item counters")
            .p(1)
            .gap(1)
            .col(|ui| {
                ui.text("Each row owns its own state via use_state_keyed(format!(\"counter-{i}\"), ...)")
                    .dim();
                ui.text("j/k = move  l/h = bump/drop selected counter  +/- = add/remove rows  Ctrl+Q = quit")
                    .dim();
                for i in 0..item_count {
                    // Runtime key — would not compile with use_state_named.
                    let counter = ui.use_state_keyed(format!("counter-{i}"), || 0i32);
                    if i == selected {
                        if bump {
                            *counter.get_mut(ui) += 1;
                        } else if drop {
                            *counter.get_mut(ui) -= 1;
                        }
                    }
                    let value = *counter.get(ui);
                    let prefix = if i == selected { "▶" } else { " " };
                    let label = format!("{prefix} item {i:>2}  count = {value:>4}");
                    let color = if i == selected {
                        Color::Cyan
                    } else if value > 0 {
                        Color::Green
                    } else if value < 0 {
                        Color::Red
                    } else {
                        Color::Reset
                    };
                    ui.text(label).fg(color);
                }
            });
    })
}
