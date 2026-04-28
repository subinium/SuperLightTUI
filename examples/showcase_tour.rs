//! Showcase Tour — seven domain `demo_*` examples integrated into a
//! single Tabs-driven tour. Each tab embeds the corresponding standalone
//! demo's `pub fn render(...)` so what you see is exactly what the
//! standalone binary renders.
//!
//! Run: `cargo run --example showcase_tour`
//!
//! Keys:
//!   Left / Right     - switch tab (when the tabs bar is focused; Tab to focus)
//!   Tab / Shift-Tab  - cycle focus (tabs bar -> demo)
//!   q / Esc / Ctrl-Q - quit (Esc only when no demo's modal is open;
//!                      see notes below)
//!
//! Tabs:
//!   1. Intro       - overview + navigation help (description-only)
//!   2. Dashboard   - system dashboard layout
//!   3. Design      - typography / colors / spacing showcase
//!   4. Infoviz     - chart / heatmap / treemap / canvas patterns
//!   5. Trading     - finance/trading dashboard mockup
//!   6. Spreadsheet - editable cell grid
//!   7. Table       - searchable + sortable data table
//!   8. Website     - website-style layout with multiple sub-pages
//!
//! All embedded demos are **Standard** archetype (full-canvas, no
//! overlay, no scrollback) so per §5 C1 they coexist cleanly in the
//! tabbed shell. The trading and infoviz demos own private modals
//! (none) and tabs (own `TabsState`); their internal tabs receive
//! Left/Right *after* the tour's outer tabs widget consumes them when
//! focused, so users can focus the inner widgets to switch.

use slt::widgets::{ScrollState, TabsState};
use slt::{Border, Color, Context, KeyCode, KeyModifiers, RunConfig};

// Each `#[path = ...] mod ...;` re-includes a single-feature demo so the
// tour can call its `pub fn render(...)` directly. The demos' own `fn
// main()` and helpers are unused in this build, hence the blanket
// `#[allow(dead_code)]` on every include.
#[allow(dead_code)]
#[path = "demo_dashboard.rs"]
mod dashboard;
#[allow(dead_code)]
#[path = "demo_design_system.rs"]
mod design_system;
#[allow(dead_code)]
#[path = "demo_infoviz.rs"]
mod infoviz;
#[allow(dead_code)]
#[path = "demo_spreadsheet.rs"]
mod spreadsheet;
#[allow(dead_code)]
#[path = "demo_table.rs"]
mod table;
#[allow(dead_code)]
#[path = "demo_trading.rs"]
mod trading;
#[allow(dead_code)]
#[path = "demo_website.rs"]
mod website;

/// Aggregated state for every embedded demo. Each field is the
/// `DemoState` from the corresponding domain demo.
struct TourState {
    tabs: TabsState,
    /// Scroll offset for the active tab body. The Design tab in particular
    /// stacks typography / colours / spacing samples that overflow the
    /// viewport; a tour-level scrollable keeps the lower content reachable.
    tab_scroll: ScrollState,
    dashboard: dashboard::DemoState,
    design_system: design_system::DemoState,
    infoviz: infoviz::DemoState,
    trading: trading::DemoState,
    spreadsheet: spreadsheet::DemoState,
    table: table::DemoState,
    website: website::DemoState,
}

impl Default for TourState {
    fn default() -> Self {
        Self {
            tabs: TabsState::new(vec![
                "Intro",
                "Dashboard",
                "Design",
                "Infoviz",
                "Trading",
                "Spreadsheet",
                "Table",
                "Website",
            ]),
            tab_scroll: ScrollState::new(),
            dashboard: dashboard::DemoState::new(),
            design_system: design_system::DemoState::new(),
            infoviz: infoviz::DemoState::new(),
            trading: trading::DemoState::new(),
            spreadsheet: spreadsheet::DemoState::new(),
            table: table::DemoState::new(),
            website: website::DemoState::new(),
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut state = TourState::default();
    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        // Tour-level quit: Ctrl-Q always at the top of the frame. We do
        // NOT consume Esc here so embedded demos can use it for their
        // own escape paths (e.g. `demo_website` clears `blog_view` on
        // Esc, `demo_spreadsheet` exits edit mode on Esc).
        if ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
            return;
        }

        let pad = ui.spacing().xs();
        let _ = ui
            .bordered(Border::Rounded)
            .title("Showcase Tour: domain examples")
            .p(pad)
            .grow(1)
            .col(|ui| {
                let _ = ui.tabs(&mut state.tabs);
                ui.separator();

                // Wrap the tab body in a vertical scrollable so tabs whose
                // demo content overflows the viewport (Design's stacked
                // typography / colour / spacing showcase, in particular)
                // stay fully reachable. Mouse wheel outside any inner
                // scroll region scrolls the whole tab body; when the
                // content fits the viewport this is a no-op.
                let _ = ui.scrollable(&mut state.tab_scroll).grow(1).col(|ui| {
                    match state.tabs.selected {
                        0 => render_intro(ui),
                        1 => render_dashboard(ui, &mut state),
                        2 => render_design(ui, &mut state),
                        3 => render_infoviz(ui, &mut state),
                        4 => render_trading(ui, &mut state),
                        5 => render_spreadsheet(ui, &mut state),
                        6 => render_table(ui, &mut state),
                        7 => render_website(ui, &mut state),
                        _ => {}
                    }
                });
            });

        // 'q' and Esc are checked AFTER demos render so a focused
        // text_input (Design's input, Table's filter, Trading's order
        // form, Spreadsheet's editor, Website's email/contact form)
        // consumes them as text first.
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
    })
}

/// Tab 1: Intro. Pure overview - no embedded demo.
fn render_intro(ui: &mut Context) {
    let _ = ui.col(|ui| {
        let pad = ui.spacing().xs();
        ui.text("Welcome to the showcase tour.").bold();
        ui.text("");
        ui.text("Each tab embeds a domain example from examples/demo_*.rs")
            .dim();
        ui.text("without changes - what you see in a tab is exactly the").dim();
        ui.text("standalone demo's render path.").dim();
        ui.text("");
        let _ = ui
            .bordered(Border::Single)
            .title("Domain examples at a glance")
            .p(pad)
            .col(|ui| {
                row_pair(ui, "Dashboard", "metric cards, processes, log stream, toasts");
                row_pair(
                    ui,
                    "Design",
                    "typography, ThemeColor, Spacing, ContainerStyle extends",
                );
                row_pair(
                    ui,
                    "Infoviz",
                    "line / scatter / bars / heatmap / candlestick / treemap / canvas",
                );
                row_pair(
                    ui,
                    "Trading",
                    "BTC/USDT order book, candles, order form, positions",
                );
                row_pair(
                    ui,
                    "Spreadsheet",
                    "editable cell grid with cursor, formula bar, edit mode",
                );
                row_pair(
                    ui,
                    "Table",
                    "searchable + sortable data table with footer status",
                );
                row_pair(
                    ui,
                    "Website",
                    "multi-page layout (Home / Docs / Blog / Pricing / Contact)",
                );
            });
        ui.text("");
        ui.text(
            "Navigation: Left/Right arrows switch tabs (Tab to focus the bar). q / Ctrl-Q quits.",
        )
        .fg(Color::Cyan);
        ui.text("Esc quits everywhere except inside Spreadsheet edit mode and Website blog-view, where it dismisses.")
            .dim();
    });
}

/// One label/description row for the intro example list.
fn row_pair(ui: &mut Context, label: &str, desc: &str) {
    let _ = ui.container().gap(1).row(|ui| {
        ui.text(format!("{label:<12}")).bold().fg(Color::Cyan);
        ui.text(desc).dim();
    });
}

/// Tab 2: Dashboard. Spinner phase, log scroll, table cursor, theme
/// toggle, and toast queue all live in TourState.
fn render_dashboard(ui: &mut Context, state: &mut TourState) {
    dashboard::render_with_state(ui, &mut state.dashboard);
}

/// Tab 3: Design system. Theme cursor, theme-browser toggle, input
/// value, list cursor, and counter persist across tab switches.
fn render_design(ui: &mut Context, state: &mut TourState) {
    design_system::render(ui, &mut state.design_system);
}

/// Tab 4: Infoviz. Selected chart tab persists across tab switches.
fn render_infoviz(ui: &mut Context, state: &mut TourState) {
    infoviz::render_with_state(ui, &mut state.infoviz);
}

/// Tab 5: Trading. The random-walk price feed, candle history, order
/// book, recent trades, and order-form inputs all keep ticking across
/// tab switches because the `St` wrapped in `DemoState` is owned here.
fn render_trading(ui: &mut Context, state: &mut TourState) {
    trading::render(ui, &mut state.trading);
}

/// Tab 6: Spreadsheet. Cursor position, edit-mode flag, typed edit
/// value, and scroll offset survive tab switches.
fn render_spreadsheet(ui: &mut Context, state: &mut TourState) {
    spreadsheet::render(ui, &mut state.spreadsheet);
}

/// Tab 7: Table. Filter text, sort column, table cursor persist.
fn render_table(ui: &mut Context, state: &mut TourState) {
    table::render(ui, &mut state.table);
}

/// Tab 8: Website. Sub-page nav, scroll, theme cursor, blog view,
/// modal flags, and contact form all persist.
fn render_website(ui: &mut Context, state: &mut TourState) {
    website::render(ui, &mut state.website);
}
