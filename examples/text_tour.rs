//! Text Tour — every text / IME / CLI demo, switched via SLT's own `Tabs`
//! widget. Each tab dispatches to the matching `pub fn render(...)` from a
//! single-feature demo so behaviour matches the standalone binary 1:1.
//!
//! Run: `cargo run --example text_tour`
//!
//! Keys (tour-level):
//!   Left / Right     — switch tab (when the tabs bar is focused; Tab to focus)
//!   Tab / Shift-Tab  — cycle focus (tabs bar -> active demo)
//!   q / Esc / Ctrl-Q — quit
//!
//! Tabs:
//!   1. Intro    — overview + navigation help
//!   2. CJK      — Korean / Chinese / Japanese title and content rendering
//!   3. IME      — composition input across two text inputs and a textarea
//!   4. Pretext  — mouse-reactive text reflow (raw `draw` API)
//!   5. CLI      — cargo-style package manager UI
//!
//! Note on §7 of `docs/DEMO_GUIDE.md` (BMP ASCII titles): the audit V7
//! rule only scans `examples/v020_*.rs`, not this file, so the embedded
//! demos may legitimately use wide-character titles to exercise CJK
//! rendering. The tour's *own* wrapper title stays BMP ASCII so the outer
//! border alignment is guaranteed regardless of terminal width-reporting
//! quirks. Do not strip wide chars from the embedded demos — the wide
//! chars are the point.

use slt::widgets::ScrollState;
use slt::widgets::TabsState;
use slt::widgets::TextInputState;
use slt::{Border, Color, Context, KeyModifiers, RunConfig};

#[allow(dead_code)]
#[path = "demo_cjk.rs"]
mod demo_cjk;
#[allow(dead_code)]
#[path = "demo_cli.rs"]
mod demo_cli;
#[allow(dead_code)]
#[path = "demo_ime.rs"]
mod demo_ime;
#[allow(dead_code)]
#[path = "demo_pretext.rs"]
mod demo_pretext;

/// Aggregated state for every embedded demo. Each field is the
/// `DemoState` from the corresponding feature demo (or, for `demo_cjk`,
/// the two `TextInputState`s its `render_frame` accepts).
struct TourState {
    tabs: TabsState,
    /// Scroll offset for the active tab body. Mouse-wheel events outside
    /// any inner scrollable scroll the whole tab so wide-character /
    /// CLI scrollback content stays reachable on small terminals.
    tab_scroll: ScrollState,
    cjk_name: TextInputState,
    cjk_tag: TextInputState,
    ime: demo_ime::DemoState,
    pretext: demo_pretext::DemoState,
    cli: demo_cli::DemoState,
}

impl Default for TourState {
    fn default() -> Self {
        Self {
            tabs: TabsState::new(vec!["Intro", "CJK", "IME", "Pretext", "CLI"]),
            tab_scroll: ScrollState::new(),
            cjk_name: TextInputState::with_placeholder("name (CJK ok)"),
            cjk_tag: TextInputState::with_placeholder("tag"),
            ime: Default::default(),
            pretext: Default::default(),
            cli: Default::default(),
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut state = TourState::default();
    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        // Tour-level quit: Ctrl-Q always, Esc ONLY at the top of the
        // frame. We intentionally do NOT consume `q` here — the IME and
        // CLI tabs both host text inputs that need to receive `q` as
        // composition / search input, so plain `q` is checked at the
        // bottom of the frame after the active demo has had a chance to
        // claim it.
        if ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
            return;
        }

        let pad = ui.spacing().xs();
        let _ = ui
            .bordered(Border::Rounded)
            .title("Text Tour: i18n and text input")
            .p(pad)
            .grow(1)
            .col(|ui| {
                let _ = ui.tabs(&mut state.tabs);
                ui.separator();

                // Wrap the tab body in a vertical scrollable so the
                // intro's long help text and any overflowing demo
                // content stay reachable. Mouse wheel outside any inner
                // scroll region scrolls the whole tab; when the body
                // fits the viewport this is a no-op.
                let _ = ui.scrollable(&mut state.tab_scroll).grow(1).col(|ui| {
                    match state.tabs.selected {
                        0 => render_intro(ui),
                        1 => demo_cjk::render_frame(ui, &mut state.cjk_name, &mut state.cjk_tag),
                        2 => demo_ime::render(ui, &mut state.ime),
                        3 => demo_pretext::render(ui, &mut state.pretext),
                        4 => demo_cli::render(ui, &mut state.cli),
                        _ => {}
                    }
                });
            });

        // `q` is checked AFTER demos render so a focused text_input
        // (CJK, IME, or CLI tabs) consumes it as text first. `Esc` is
        // also deferred — `demo_cli`'s own Esc handler clears its
        // install state, and we only want tour-level quit when no demo
        // claimed the key.
        if ui.key('q') || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }
    })
}

/// Tab 1: Intro. Pure overview — no embedded demo.
fn render_intro(ui: &mut Context) {
    let _ = ui.col(|ui| {
        let pad = ui.spacing().xs();
        ui.text("Welcome to the Text Tour.").bold();
        ui.text("");
        ui.text("Each tab embeds the corresponding single-feature demo from")
            .dim();
        ui.text("examples/demo_*.rs without modification — what you see in")
            .dim();
        ui.text("a tab is exactly the standalone demo's render path.")
            .dim();
        ui.text("");
        let _ = ui
            .bordered(Border::Single)
            .title("Tabs at a glance")
            .p(pad)
            .col(|ui| {
                row_pair(
                    ui,
                    "CJK",
                    "Korean / Chinese / Japanese title and content rendering with mouse cards",
                );
                row_pair(
                    ui,
                    "IME",
                    "composition input across two text inputs and a textarea, live filter",
                );
                row_pair(
                    ui,
                    "Pretext",
                    "raw-draw text reflow around a mouse-tracking caterpillar trail",
                );
                row_pair(
                    ui,
                    "CLI",
                    "cargo-style package manager: search, install, scrolling output log",
                );
            });
        ui.text("");
        ui.text("Navigation: click a tab, or focus the bar with Tab and use Left/Right.")
            .fg(Color::Cyan);
        ui.text("Quit: q / Esc / Ctrl-Q (a focused text input claims `q` as input first).")
            .fg(Color::Cyan);
        ui.text("");
        ui.text(
            "Note: the CJK and IME tabs intentionally render wide-character\
             content to test fullwidth glyph handling. The tour's own wrapper\
             title stays BMP ASCII so the outer border alignment is guaranteed.",
        )
        .dim()
        .wrap();
    });
}

/// One label/description row for the intro feature list.
fn row_pair(ui: &mut Context, label: &str, desc: &str) {
    let _ = ui.container().gap(1).row(|ui| {
        ui.text(format!("{label:<8}")).bold().fg(Color::Cyan);
        ui.text(desc).dim();
    });
}
