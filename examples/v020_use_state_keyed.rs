//! v0.20.0 use_state_keyed demo — runtime-keyed per-item state.
//!
//! Demonstrates: #215
//!
//! Each list row owns its own counter, keyed by a runtime
//! `format!("counter-{i}")` string. Rows can be added or removed at any
//! time; counters that survive across frames keep their values. Stale
//! entries from removed rows are tolerated — the state map keeps them
//! but the closure simply stops reading them.
//!
//! Run: `cargo run --example v020_use_state_keyed`
//!
//! Keys:
//!   j / Down       — move selection down
//!   k / Up         — move selection up
//!   l / Right      — bump selected counter (+1)
//!   h / Left       — drop selected counter (-1)
//!   +              — add a row (max 20)
//!   -              — remove a row (min 1)
//!   Ctrl-Q / Esc   — quit
//!
//! Layout:
//!   ┌── use_state_keyed: per-item counters ──┐
//!   │ helper text                              │
//!   │                                          │
//!   │ ▶ item  0  count =    0                  │
//!   │   item  1  count =    7                  │
//!   │   item  2  count =   -3                  │
//!   └──────────────────────────────────────────┘

use slt::{Border, Color, Context, KeyCode, KeyModifiers, RunConfig};

/// Per-frame inputs for [`render`]. Kept on the stack in `main`; passed by
/// `&mut` so snapshot tests can drive the same render path frame-by-frame.
pub struct DemoState {
    pub item_count: usize,
    pub selected: usize,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            item_count: 3,
            selected: 0,
        }
    }
}

const MAX_ROWS: usize = 20;
const MIN_ROWS: usize = 1;

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();
    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        render(ui, &mut state)
    })
}

/// Render one frame of the keyed-state demo.
///
/// Public so snapshot tests can pin frames against this exact UI shape
/// without re-deriving widget composition in test fragments.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
        ui.quit();
        return;
    }
    handle_input(ui, state);

    // Capture intent BEFORE the render closure so the inner loop can use
    // it without reborrowing `ui` for key checks (which would clash with
    // the mutable closure borrow).
    let bump = ui.key('l') || ui.key_code(KeyCode::Right);
    let drop = ui.key('h') || ui.key_code(KeyCode::Left);
    let item_count = state.item_count;
    let selected = state.selected;

    let pad = ui.spacing().xs();
    let gap = ui.spacing().xs();

    let _ = ui
        .bordered(Border::Rounded)
        .title("use_state_keyed: per-item counters")
        .p(pad)
        .gap(gap)
        .col(|ui| {
            ui.text(
                "Each row owns its own state via use_state_keyed(format!(\"counter-{i}\"), …).",
            )
            .dim();
            ui.text(
                "j/k = move   l/h = bump/drop selected   +/- = add/remove rows   Ctrl+Q = quit",
            )
            .dim();

            for i in 0..item_count {
                // Runtime key — the equivalent `use_state_named` would not
                // compile because it requires a `&'static str`.
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
                ui.text(label).fg(row_color(i == selected, value));
            }
        });
}

/// Apply growth / shrink / selection-move keystrokes.
///
/// Split out so `render` reads as a pure render path and so snapshot tests
/// can mutate `DemoState` directly without going through key events.
fn handle_input(ui: &mut Context, state: &mut DemoState) {
    if ui.key_code(KeyCode::Char('+')) {
        state.item_count = (state.item_count + 1).min(MAX_ROWS);
    }
    if ui.key_code(KeyCode::Char('-')) {
        state.item_count = state.item_count.saturating_sub(1).max(MIN_ROWS);
    }
    if ui.key('k') || ui.key_code(KeyCode::Up) {
        state.selected = state.selected.saturating_sub(1);
    }
    if ui.key('j') || ui.key_code(KeyCode::Down) {
        state.selected = (state.selected + 1).min(state.item_count.saturating_sub(1));
    }
    // Trim selection if rows shrank below the cursor.
    if state.selected >= state.item_count {
        state.selected = state.item_count.saturating_sub(1);
    }
}

/// Colour a row by selection + sign. Keeps the rule out of `render` so the
/// visual contract (cyan = selected, green/red = sign, default otherwise) is
/// readable at a glance.
fn row_color(selected: bool, value: i32) -> Color {
    if selected {
        Color::Cyan
    } else if value > 0 {
        Color::Green
    } else if value < 0 {
        Color::Red
    } else {
        Color::Reset
    }
}
