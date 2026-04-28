//! System Tour — runtime modes and system-archetype demos grouped by tab.
//!
//! Run: `cargo run --example system_tour --features async`
//!
//! Keys:
//!   Left / Right     — switch tab (Tab to focus the tabs bar)
//!   Tab / Shift-Tab  — cycle focus
//!   q / Esc / Ctrl-Q — quit
//!
//! Tabs:
//!   1. Intro          — overview + navigation help
//!   2. Async          — description-only (`run_async` + tokio runtime)
//!   3. Error Boundary — live: `error_boundary_with` recovers from panics
//!   4. Inline (info)  — description-only (`run_inline` / InlineTerminal)
//!   5. Overlay Anchor — live: `overlay_at` / `overlay_at_offset`
//!
//! Why some tabs are description-only:
//! - **Async**: `async_demo` uses `slt::run_async` (a separate entry point
//!   with a `Vec<Message>` parameter) and a `#[tokio::main]` producer
//!   task. It cannot compose into a sync `slt::run_with` tour without
//!   reframing the whole binary as async, so this tab is a code-snippet
//!   description per DEMO_GUIDE §5 C2.
//! - **Inline (info)**: `inline.rs` uses `slt::run_inline`, which renders
//!   below the cursor without an alternate screen. The tour itself runs
//!   in alternate-screen mode (`slt::run_with`); the two terminal modes
//!   are mutually exclusive within one binary, so this tab is a
//!   description page (DEMO_GUIDE §9 macOS quirks + §5 C2).

use slt::widgets::{ScrollState, TabsState};
use slt::{Border, ButtonVariant, Color, Context, KeyCode, KeyModifiers, RunConfig};

// Each `#[path = ...] mod ...;` re-includes a single-feature demo so the
// tour can call its `pub fn render(...)` directly. Demos whose state is
// owned in their own `fn main()` are not re-included here — their
// description-only tabs are rendered inline in this file.
#[allow(dead_code)]
#[path = "demo_overlay_anchor.rs"]
mod overlay_anchor;

/// Aggregated state for every embedded demo. Each field is the persistent
/// state for one tab; constructing them here (not in the render closure)
/// keeps the state alive across frames per DEMO_GUIDE §5 C3.
struct TourState {
    tabs: TabsState,
    /// Scroll offset for the active tab body. Description-only tabs
    /// (Async / Inline) include code blocks that overflow short
    /// terminals; the wrapper keeps the tail reachable via mouse wheel.
    tab_scroll: ScrollState,
    error_boundary: ErrorBoundaryState,
}

/// State for the live Error Boundary tab. Mirrors the locals from
/// `examples/error_boundary_demo.rs::main` so clicks on the panic button
/// persist their effect into `panic_count`.
#[derive(Default)]
struct ErrorBoundaryState {
    panic_count: u32,
}

impl Default for TourState {
    fn default() -> Self {
        Self {
            tabs: TabsState::new(vec![
                "Intro",
                "Async",
                "Error Boundary",
                "Inline (info)",
                "Overlay Anchor",
            ]),
            tab_scroll: ScrollState::new(),
            error_boundary: ErrorBoundaryState::default(),
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut state = TourState::default();
    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        // Tour-level quit: Ctrl-Q only at the top of the frame. We
        // intentionally do NOT consume Esc here so embedded demos can
        // route their own Esc handling.
        if ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
            return;
        }

        let pad = ui.spacing().xs();
        let _ = ui
            .bordered(Border::Rounded)
            .title("System Tour: runtime modes")
            .p(pad)
            .grow(1)
            .col(|ui| {
                let _ = ui.tabs(&mut state.tabs);
                ui.separator();

                // Wrap the tab body in a vertical scrollable so
                // description-only tabs (Async / Inline code blocks)
                // and overflowing live tabs stay reachable on small
                // terminals. Mouse wheel outside any inner scroll
                // region scrolls the whole tab; no-op when content
                // fits.
                let _ = ui.scrollable(&mut state.tab_scroll).grow(1).col(|ui| {
                    match state.tabs.selected {
                        0 => render_intro(ui),
                        1 => render_async(ui),
                        2 => render_error_boundary(ui, &mut state.error_boundary),
                        3 => render_inline(ui),
                        4 => overlay_anchor::render(ui),
                        _ => {}
                    }
                });
            });

        // 'q' / 'Esc' handled AFTER demos render so embedded interactive
        // tabs (e.g. Error Boundary) get first crack at the keystrokes.
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
    })
}

/// Tab 1: Intro. Pure overview — no embedded demo.
fn render_intro(ui: &mut Context) {
    let _ = ui.col(|ui| {
        let pad = ui.spacing().xs();
        ui.text("System Tour — runtime modes and system-archetype demos.")
            .bold();
        ui.text("");
        ui.text("Two of the four source demos use a runtime mode that")
            .dim();
        ui.text("does not compose into a fullscreen alternate-screen tour.")
            .dim();
        ui.text("Those tabs render a code-snippet description and a")
            .dim();
        ui.text("`cargo run --example <name>` pointer for the standalone form.")
            .dim();
        ui.text("");
        let _ = ui
            .bordered(Border::Single)
            .title("Tabs at a glance")
            .p(pad)
            .col(|ui| {
                row_pair(ui, "Async", "run_async + tokio task (description-only)");
                row_pair(
                    ui,
                    "Boundary",
                    "error_boundary_with — panic recovery (live)",
                );
                row_pair(
                    ui,
                    "Inline",
                    "run_inline / InlineTerminal (description-only)",
                );
                row_pair(ui, "Overlay", "overlay_at + overlay_at_offset (live)");
            });
        ui.text("");
        ui.text("Navigation: Left/Right switch tabs (Tab focuses the tabs bar).")
            .fg(Color::Cyan);
        ui.text("q / Esc / Ctrl-Q quits.").fg(Color::Cyan);
    });
}

/// One label/description row for the intro feature list.
fn row_pair(ui: &mut Context, label: &str, desc: &str) {
    let _ = ui.row_gap(1, |ui| {
        ui.text(format!("{label:<9}")).bold().fg(Color::Cyan);
        ui.text(desc).dim();
    });
}

/// Tab 2: Async. Description-only page for `async_demo`.
///
/// The standalone demo uses `slt::run_async` (a distinct entry point that
/// receives `&mut Vec<Message>` of channel messages) and a `#[tokio::main]`
/// producer task that pushes status updates every 2 seconds. Composing it
/// into a sync `slt::run_with` tour would require rewriting the tour as
/// async and forwarding the producer's channel through the tour state —
/// out of scope. Run the standalone binary to see the actual async flow.
fn render_async(ui: &mut Context) {
    let pad = ui.spacing().xs();
    let _ = ui
        .bordered(Border::Rounded)
        .title("async_demo: run_async + tokio task")
        .p(pad)
        .grow(1)
        .col(|ui| {
            ui.text("slt::run_async spawns the render loop on the current tokio")
                .dim();
            ui.text("runtime and returns an `mpsc::Sender<M>` so background tasks")
                .dim();
            ui.text("can push messages into the render closure without polling.")
                .dim();
            ui.text("");
            let _ = ui
                .bordered(Border::Single)
                .title("typical usage")
                .p(pad)
                .col(|ui| {
                    let _ = ui.code_block_lang(
                        "#[tokio::main(flavor = \"current_thread\")]\nasync fn main() -> std::io::Result<()> {\n    let tx = slt::run_async(move |ui, messages: &mut Vec<String>| {\n        for m in messages.drain(..) { /* ... */ }\n        ui.text(\"...\");\n    })?;\n    tokio::spawn(async move {\n        loop { tx.send(\"tick\".into()).await.ok(); }\n    }).await.ok();\n    Ok(())\n}",
                        "rust",
                    );
                });
            ui.text("");
            ui.text("This page is description-only because the tour binary uses")
                .fg(Color::Yellow);
            ui.text("sync `slt::run_with`. Embedding `run_async` would require")
                .fg(Color::Yellow);
            ui.text("an async tour entry point and forwarding the channel.")
                .fg(Color::Yellow);
            ui.text("");
            ui.text("To see the actual async flow, run the standalone demo:")
                .dim();
            ui.text("    cargo run --example async_demo --features async")
                .fg(Color::Cyan);
        });
}

/// Tab 3: Error Boundary. Live render — `error_boundary_with` already
/// wraps its child in `std::panic::catch_unwind`, so a panic from the
/// inner closure is recovered without affecting the surrounding tour.
///
/// Mirrors `examples/error_boundary_demo.rs::main` body verbatim, but
/// pulls `panic_count` out of a local and into tour-owned state per
/// DEMO_GUIDE §5 C3 (otherwise the count would reset every frame).
fn render_error_boundary(ui: &mut Context, state: &mut ErrorBoundaryState) {
    let pad = ui.spacing().xs();
    let _ = ui
        .bordered(Border::Rounded)
        .title("error_boundary_with: panic recovery in widgets")
        .p(pad)
        .gap(1)
        .grow(1)
        .col(|ui| {
            ui.text("Trigger panic inside error boundary.").bold();
            ui.text("Press button or key 'p'. Esc/q/Ctrl-Q to quit the tour.")
                .dim();
            ui.text(format!("Recovered panics: {}", state.panic_count))
                .fg(Color::Cyan);

            let trigger_panic = ui
                .button_with("Panic in boundary", ButtonVariant::Danger)
                .clicked
                || ui.key('p');

            ui.error_boundary_with(
                |ui| {
                    if trigger_panic {
                        panic!("demo panic from error boundary");
                    }
                    ui.text("No panic this frame").fg(Color::Green);
                },
                |ui, _msg| {
                    state.panic_count = state.panic_count.saturating_add(1);
                    ui.text("Recovered from panic").bold().fg(Color::Yellow);
                },
            );
        });
}

/// Tab 4: Inline. Description-only page for `inline.rs`.
///
/// The standalone demo uses `slt::run_inline(height, ...)` which renders
/// in InlineTerminal mode below the cursor — no alternate screen. The
/// tour binary already entered alternate-screen mode via `slt::run_with`,
/// so switching to inline mode mid-frame would corrupt the terminal
/// state. Description-only per DEMO_GUIDE §9 macOS quirks + §5 C2.
fn render_inline(ui: &mut Context) {
    let pad = ui.spacing().xs();
    let _ = ui
        .bordered(Border::Rounded)
        .title("inline: run_inline / InlineTerminal mode")
        .p(pad)
        .grow(1)
        .col(|ui| {
            ui.text("slt::run_inline(height, render) renders a fixed-height TUI")
                .dim();
            ui.text("below the user's cursor without entering the alternate")
                .dim();
            ui.text("screen. The shell scrollback above the inline buffer is")
                .dim();
            ui.text("preserved; the inline region updates in place each frame.")
                .dim();
            ui.text("");
            let _ = ui
                .bordered(Border::Single)
                .title("typical usage")
                .p(pad)
                .col(|ui| {
                    let _ = ui.code_block_lang(
                        "fn main() -> std::io::Result<()> {\n    let mut count: i32 = 0;\n    slt::run_inline(4, |ui| {\n        if ui.key('k') { count += 1; }\n        if ui.key('j') { count -= 1; }\n        ui.text(format!(\"Inline count: {count}\"));\n    })\n}",
                        "rust",
                    );
                });
            ui.text("");
            ui.text("This page is description-only because run_inline uses a")
                .fg(Color::Yellow);
            ui.text("different terminal mode (InlineTerminal, no alternate")
                .fg(Color::Yellow);
            ui.text("screen) than the tour's `slt::run_with`. The two modes")
                .fg(Color::Yellow);
            ui.text("cannot coexist within one binary frame.")
                .fg(Color::Yellow);
            ui.text("");
            ui.text("To see the inline render mode, run the standalone demo:")
                .dim();
            ui.text("    cargo run --example inline").fg(Color::Cyan);
        });
}
