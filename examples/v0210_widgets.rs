//! v0.21.0 Widgets — coverage for the v0.21.0 additions that previously
//! shipped without a runnable example: the standalone `paginator`, the
//! numeric stepper (`number_input`), the variable-height `virtual_list`,
//! the async-free frame-clock scheduler (`schedule` / `every` / `debounce`),
//! and the devtools inspector panel (Ctrl+F12 / `set_inspector`).
//!
//! Run: `cargo run --example v0210_widgets`
//!
//! Keys:
//!   Tab / Shift-Tab  — cycle focus across the widgets
//!   Left / Right     — paginator: previous / next page (when focused)
//!   Up / Down        — number_input: step value · virtual_list: move selection
//!   Enter            — number_input: commit a typed value
//!   i                — toggle the devtools inspector panel (also Ctrl+F12)
//!   q / Esc / Ctrl-Q — quit
//!
//! Every widget here is **async-free** — the scheduler (#248) is wall-clock
//! based and works on the default feature set, so this whole example builds
//! and runs without the `async` feature. See the note at the bottom for how
//! in-frame async (`Context::spawn`/`poll`) would be demoed instead.

use std::time::Duration;

use slt::widgets::{ListState, NumberInputState, PaginatorState, PaginatorStyle, TextInputState};
use slt::{Border, Color, Context, KeyCode, KeyModifiers, RunConfig};

/// One catalog row paged through by the paginator. Kept tiny so the whole
/// example stays a single self-contained screen.
const CATALOG: &[(&str, &str)] = &[
    ("SLT-001", "Rounded border kit"),
    ("SLT-002", "Flexbox row/col layout"),
    ("SLT-003", "Tabs + scrollable shell"),
    ("SLT-004", "Theme presets (10)"),
    ("SLT-005", "Sparkline + heatmap"),
    ("SLT-006", "Candlestick chart"),
    ("SLT-007", "Command palette"),
    ("SLT-008", "Virtual list (fixed)"),
    ("SLT-009", "Virtual list (variable)"),
    ("SLT-010", "Standalone paginator"),
    ("SLT-011", "Numeric stepper"),
    ("SLT-012", "Frame-clock scheduler"),
    ("SLT-013", "Devtools inspector"),
    ("SLT-014", "Tree + directory tree"),
    ("SLT-015", "Sixel / halfblock image"),
];

/// Variable-height feed rows: a short reply next to a tall code block, the
/// canonical `virtual_list_variable` use case.
const FEED: &[(&str, u32)] = &[
    ("ok 👍", 1),
    (
        "here is the patch:\n  fn render(ui) {\n    ui.text(\"hi\");\n  }",
        4,
    ),
    ("thanks!", 1),
    ("one-line note", 1),
    ("stack trace:\n  at frame()\n  at run()\n  at main()", 4),
    ("done", 1),
    ("a slightly longer\nwrapped reply", 2),
    ("👌", 1),
    ("final summary line\nspanning two rows", 2),
    ("end", 1),
];

/// Persistent state for the whole screen. Held across frames by `run_with`'s
/// `move` closure so cursors, pages, and the debounce signal settle correctly.
/// `pub` so a tour binary can embed this demo via `#[path = ...] mod` and call
/// [`render`] directly, matching the other `examples/*` demos.
pub struct DemoState {
    /// Standalone paginator over `CATALOG` rows.
    paginator: PaginatorState,
    /// Integer quantity stepper, clamped to `[0, 99]`.
    qty: NumberInputState,
    /// Float price stepper with a 0.25 step.
    price: NumberInputState,
    /// Variable-height feed list (chat bubbles of differing heights).
    feed: ListState,
    /// Search box whose keystrokes drive the `debounce` timer.
    search: TextInputState,
    /// Last query the debounce timer let through, for display.
    settled_query: String,
    /// `every`-driven second counter, proving recurring ticks accumulate.
    seconds: u64,
    /// `schedule`-driven one-shot banner flag (fires ~1.5s after launch).
    splash_dismissed: bool,
    /// Mirror of `Context::inspector()`, toggled by the local `i` key.
    inspector_on: bool,
}

impl Default for DemoState {
    fn default() -> Self {
        let mut paginator = PaginatorState::new(CATALOG.len(), 4);
        paginator.style = PaginatorStyle::Arabic;
        let heights: Vec<u32> = FEED.iter().map(|&(_, h)| h).collect();
        Self {
            paginator,
            qty: NumberInputState::integer(3, 0, 99).step(1.0),
            price: NumberInputState::new(9.5, 0.0, 100.0).step(0.25),
            feed: ListState::new(FEED.iter().map(|&(t, _)| t).collect::<Vec<_>>())
                .with_item_heights(heights),
            search: TextInputState::with_placeholder("type to search (debounced)..."),
            settled_query: String::new(),
            seconds: 0,
            splash_dismissed: false,
            inspector_on: false,
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();
    slt::run_with(
        RunConfig::default()
            .mouse(true)
            .tick_rate(Duration::from_millis(100)),
        move |ui: &mut Context| {
            // Ctrl-Q always quits, even with a focused text_input. Plain 'q'
            // and Esc are checked at the end so the search box can consume them.
            if ui.key_mod('q', KeyModifiers::CONTROL) {
                ui.quit();
                return;
            }

            // ── Frame-clock scheduler (#248), all async-free ───────────────
            // One-shot: dismiss the launch banner ~1.5s after the first frame.
            if ui.schedule("v0210::splash", Duration::from_millis(1500)) {
                state.splash_dismissed = true;
            }
            // Recurring: advance a once-per-second counter; `ticks` is the
            // number of whole intervals elapsed since last frame (usually 1,
            // > 1 only if the loop stalled), so the count never drifts.
            let ticks = ui.every("v0210::second", Duration::from_secs(1));
            state.seconds = state.seconds.saturating_add(ticks as u64);

            render(ui, &mut state);

            if ui.key('q') || ui.key_code(KeyCode::Esc) {
                ui.quit();
            }
        },
    )
}

/// Draw the whole screen. Pure render path so the example could also be
/// embedded into a tour via `#[path = ...] mod` like the other demos.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let pad = ui.spacing().xs();

    // Local devtools toggle. `i` mirrors the runtime Ctrl+F12 shortcut; both
    // flip `inspector_mode`, so we read it back to keep our flag in sync.
    if ui.key('i') {
        state.inspector_on = !state.inspector_on;
        ui.set_inspector(state.inspector_on);
    }
    state.inspector_on = ui.inspector();

    let _ = ui
        .bordered(Border::Rounded)
        .title(
            "SLT v0.21.0 Widgets — paginator · number_input · virtual_list · scheduler · inspector",
        )
        .p(pad)
        .grow(1)
        .col(|ui| {
            // Launch banner: driven by the one-shot `schedule` timer above.
            if !state.splash_dismissed {
                ui.text("● booting widgets… (one-shot schedule timer dismisses this ~1.5s in)")
                    .fg(Color::Yellow);
            } else {
                ui.text(format!(
                    "● up {}s — recurring `every(\"…\", 1s)` tick counter (drift-free)",
                    state.seconds
                ))
                .fg(Color::Green);
            }
            let _ = ui.separator();

            // Top row: paginator + numeric steppers.
            let _ = ui.row(|ui| {
                render_paginator(ui, state);
                render_steppers(ui, state);
            });

            let _ = ui.separator();

            // Bottom row: variable-height virtual_list + debounced search.
            let _ = ui.row(|ui| {
                render_feed(ui, state);
                render_search(ui, state);
            });

            let _ = ui.separator();
            render_inspector_hint(ui, state);
        });
}

/// Standalone `paginator`: pages over CATALOG, slicing with `page_bounds()`.
fn render_paginator(ui: &mut Context, state: &mut DemoState) {
    let pad = ui.spacing().xs();
    let _ = ui.container().fill().col(|ui| {
        let _ = ui
            .bordered(Border::Single)
            .title("paginator (Left/Right when focused)")
            .p(pad)
            .col(|ui| {
                let (start, end) = state.paginator.page_bounds();
                for &(sku, name) in &CATALOG[start..end] {
                    let _ = ui.container().gap(1).row(|ui| {
                        ui.text(sku).bold().fg(Color::Cyan);
                        ui.text(name).dim();
                    });
                }
                ui.text("").dim();
                let _ = ui.paginator(&mut state.paginator);
                ui.text(format!(
                    "page {}/{} — items {}..{} of {}",
                    state.paginator.page + 1,
                    state.paginator.total_pages(),
                    start,
                    end,
                    state.paginator.total_items
                ))
                .dim();
            });
    });
}

/// Numeric steppers: an integer quantity and a float price. `Response.changed`
/// is true on the frame the committed value moves.
fn render_steppers(ui: &mut Context, state: &mut DemoState) {
    let pad = ui.spacing().xs();
    let _ = ui.container().fill().col(|ui| {
        let _ = ui
            .bordered(Border::Single)
            .title("number_input (Up/Down · type + Enter)")
            .p(pad)
            .col(|ui| {
                let _ = ui.container().gap(1).row(|ui| {
                    ui.text("Qty  ").dim();
                    let r = ui.number_input(&mut state.qty);
                    if r.changed {
                        ui.text("← changed").fg(Color::Green);
                    }
                });
                let _ = ui.container().gap(1).row(|ui| {
                    ui.text("Price").dim();
                    let r = ui.number_input(&mut state.price);
                    if r.changed {
                        ui.text("← changed").fg(Color::Green);
                    }
                });
                ui.text("").dim();
                let total = state.qty.value * state.price.value;
                ui.text(format!(
                    "qty {} × ${:.2} = ${:.2}",
                    state.qty.value as i64, state.price.value, total
                ))
                .bold();
                if let Some(err) = &state.price.parse_error {
                    ui.text(format!("parse error: {err}")).fg(Color::Red);
                }
            });
    });
}

/// Variable-height `virtual_list_variable`: chat bubbles of differing heights,
/// only the visible range invokes the per-item closure.
fn render_feed(ui: &mut Context, state: &mut DemoState) {
    let pad = ui.spacing().xs();
    let _ = ui.container().fill().col(|ui| {
        let _ = ui
            .bordered(Border::Single)
            .title("virtual_list (variable height · Up/Down)")
            .p(pad)
            .col(|ui| {
                // Render at most ~8 rows of bubbles; heights come from
                // ListState::with_item_heights so a 4-row code block and a
                // 1-row reply pack correctly into the viewport. Snapshot the
                // cursor before the call — the closure borrows `feed` mutably.
                let cursor = state.feed.selected;
                let _ = ui.virtual_list_variable(&mut state.feed, 8, |ui, idx| {
                    let (text, _h) = FEED[idx];
                    let selected = idx == cursor;
                    let marker = if selected { "▸ " } else { "  " };
                    let color = if selected { Color::Cyan } else { Color::Reset };
                    // Multi-line bubbles render each row; this is the
                    // variable-height payload the widget reserves space for.
                    for (li, line) in text.split('\n').enumerate() {
                        let prefix = if li == 0 { marker } else { "  " };
                        ui.text(format!("{prefix}{line}")).fg(color);
                    }
                });
                ui.text(format!(
                    "selected bubble {} of {}",
                    state.feed.selected + 1,
                    FEED.len()
                ))
                .dim();
            });
    });
}

/// Debounced search: keystrokes set the `dirty` signal; `debounce` fires once
/// after a quiet window, the search-as-you-type primitive.
fn render_search(ui: &mut Context, state: &mut DemoState) {
    let pad = ui.spacing().xs();
    let _ = ui.container().fill().col(|ui| {
        let _ = ui
            .bordered(Border::Single)
            .title("debounce (scheduler · 300ms quiet)")
            .p(pad)
            .col(|ui| {
                let resp = ui.text_input(&mut state.search);
                // `resp.changed` is the per-keystroke dirty signal. The query
                // only "settles" after 300ms of no typing.
                if ui.debounce("v0210::search", Duration::from_millis(300), resp.changed) {
                    state.settled_query = state.search.value.clone();
                }
                ui.text("").dim();
                if state.settled_query.is_empty() {
                    ui.text("(no settled query yet — stop typing for 300ms)")
                        .dim();
                } else {
                    let _ = ui.container().gap(1).row(|ui| {
                        ui.text("settled →").dim();
                        ui.text(state.settled_query.as_str())
                            .bold()
                            .fg(Color::Green);
                    });
                }
            });
    });
}

/// Devtools inspector hint + state. The panel itself is drawn by the runtime
/// (Ctrl+F12 / `set_inspector`) on top of everything else.
fn render_inspector_hint(ui: &mut Context, state: &mut DemoState) {
    let _ = ui.container().gap(1).row(|ui| {
        let (label, color) = if state.inspector_on {
            ("inspector: ON", Color::Green)
        } else {
            ("inspector: off", Color::Reset)
        };
        ui.text(label).bold().fg(color);
        ui.text("press `i` (or Ctrl+F12) to toggle the resolved-style / focus-chain panel")
            .dim();
    });
}

// ── In-frame async note ───────────────────────────────────────────────────
//
// Context::spawn / poll (the in-frame async task registry) needs the `async`
// feature, which pulls tokio and a multi-threaded runtime. Demoing it in this
// same default-feature binary would force the whole example to be
// `required-features = ["async"]`, so it lives in `examples/async_demo.rs`
// instead. Everything in THIS file — the scheduler included — is wall-clock
// based and async-free, so it builds on the default feature set.
