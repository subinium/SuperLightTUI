//! Cookbook Tour — five real-world recipes from `examples/cookbook_*.rs`,
//! switched via SLT's own `Tabs` widget. Each tab embeds the corresponding
//! standalone cookbook demo's `pub fn render(ui, &mut DemoState)` so what
//! you see here is exactly what the standalone demo renders.
//!
//! Run: `cargo run --example cookbook_tour`
//!
//! Keys:
//!   Left / Right     — switch tab (when the tabs bar is focused; Tab to focus)
//!   Tab / Shift-Tab  — cycle focus (tabs bar -> demo)
//!   q / Esc / Ctrl-Q — quit (Esc only when no modal is open; see notes
//!                      below)
//!
//! Tabs:
//!   1. Intro       — overview + navigation help (description-only)
//!   2. Dashboard   — rolling line chart + stat tiles + sparklines
//!   3. Picker      — file picker with side-by-side text preview
//!   4. Login       — text inputs + validation + welcome state
//!   5. Modal+Toast — confirmation modal driving a toast notification
//!   6. Table       — searchable + sortable data table

use slt::widgets::{ScrollState, TabsState};
use slt::{Border, Color, Context, KeyModifiers, RunConfig};

// Each `#[path = ...] mod ...;` re-includes a single-feature demo so the
// tour can call its `pub fn render(...)` directly. The demos' own `fn
// main()` and helpers are unused in this build, hence the blanket
// `#[allow(dead_code)]` on every include.
#[allow(dead_code)]
#[path = "cookbook_dashboard.rs"]
mod dashboard;
#[allow(dead_code)]
#[path = "cookbook_file_picker.rs"]
mod file_picker;
#[allow(dead_code)]
#[path = "cookbook_login.rs"]
mod login;
#[allow(dead_code)]
#[path = "cookbook_modal_toast.rs"]
mod modal_toast;
#[allow(dead_code)]
#[path = "cookbook_table.rs"]
mod table;

/// Aggregated state for every embedded demo. Each field is the
/// `DemoState` from the corresponding cookbook recipe.
struct TourState {
    tabs: TabsState,
    /// Scroll offset for the active tab body. Mouse-wheel events outside any
    /// inner scrollable scroll the whole tab so long intros / overflowing
    /// content stay reachable on small terminals.
    tab_scroll: ScrollState,
    dashboard: dashboard::DemoState,
    file_picker: file_picker::DemoState,
    login: login::DemoState,
    modal_toast: modal_toast::DemoState,
    table: table::DemoState,
}

impl Default for TourState {
    fn default() -> Self {
        Self {
            tabs: TabsState::new(vec![
                "Intro",
                "Dashboard",
                "Picker",
                "Login",
                "Modal+Toast",
                "Table",
            ]),
            tab_scroll: ScrollState::new(),
            dashboard: Default::default(),
            file_picker: file_picker::DemoState::new(),
            login: Default::default(),
            modal_toast: Default::default(),
            table: table::DemoState::new(),
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut state = TourState::default();
    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        // Tour-level quit: Ctrl-Q always, plain `q` after demos render
        // (so a focused text_input on Login/Table consumes it as text
        // first). We intentionally do NOT consume Esc here — the
        // Modal+Toast tab relies on Esc to dismiss its own modal, and
        // each demo's standalone `main()` already handles Esc-to-quit
        // when used standalone.
        if ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
            return;
        }

        let pad = ui.spacing().xs();
        let _ = ui
            .bordered(Border::Rounded)
            .title("Cookbook Tour: real-world patterns")
            .p(pad)
            .grow(1)
            .col(|ui| {
                let _ = ui.tabs(&mut state.tabs);
                ui.separator();

                // Wrap the tab body in a vertical scrollable so the intro's
                // long help text and any overflowing recipe stay reachable
                // on small terminals. Mouse wheel outside any inner scroll
                // region scrolls the whole tab; when the body fits the
                // viewport this is a no-op.
                let _ = ui.scrollable(&mut state.tab_scroll).grow(1).col(|ui| {
                    match state.tabs.selected {
                        0 => render_intro(ui),
                        1 => render_dashboard(ui, &mut state),
                        2 => render_picker(ui, &mut state),
                        3 => render_login(ui, &mut state),
                        4 => render_modal_toast(ui, &mut state),
                        5 => render_table(ui, &mut state),
                        _ => {}
                    }
                });
            });

        // 'q' is checked AFTER demos render so a focused text_input
        // (Login fields, Table search box) consumes it as text first.
        if ui.key('q') {
            ui.quit();
        }
    })
}

/// Tab 1: Intro. Pure overview — no embedded demo.
fn render_intro(ui: &mut Context) {
    let _ = ui.col(|ui| {
        let pad = ui.spacing().xs();
        ui.text("Welcome to the cookbook tour.").bold();
        ui.text("");
        ui.text("Each tab embeds the corresponding standalone recipe from")
            .dim();
        ui.text("examples/cookbook_*.rs without changes -- what you see in")
            .dim();
        ui.text("a tab is exactly the standalone demo's render path.")
            .dim();
        ui.text("");
        let _ = ui
            .bordered(Border::Single)
            .title("Recipes at a glance")
            .p(pad)
            .col(|ui| {
                row_pair(
                    ui,
                    "Dashboard",
                    "rolling line chart + sparklines + stat tiles",
                );
                row_pair(
                    ui,
                    "Picker",
                    "FilePickerState with side-by-side text preview",
                );
                row_pair(
                    ui,
                    "Login",
                    "two text inputs, validation, masked password, welcome state",
                );
                row_pair(
                    ui,
                    "Modal+Toast",
                    "confirmation modal driving toast notifications",
                );
                row_pair(
                    ui,
                    "Table",
                    "searchable + sortable table with footer status bar",
                );
            });
        ui.text("");
        ui.text(
            "Navigation: Left/Right arrows switch tabs (Tab to focus the bar). q / Ctrl-Q quits.",
        )
        .fg(Color::Cyan);
        ui.text("Esc quits everywhere except inside the Modal+Toast modal, where it dismisses.")
            .dim();
    });
}

/// One label/description row for the intro recipe list.
fn row_pair(ui: &mut Context, label: &str, desc: &str) {
    let _ = ui.row_gap(1, |ui| {
        ui.text(format!("{label:<12}")).bold().fg(Color::Cyan);
        ui.text(desc).dim();
    });
}

/// Tab 2: Dashboard. Rolling histories live in TourState so tab
/// switches don't clear the chart.
fn render_dashboard(ui: &mut Context, state: &mut TourState) {
    dashboard::render(ui, &mut state.dashboard);
}

/// Tab 3: File picker. The picker's selected directory and preview
/// cache survive across tab switches because the state is owned here.
fn render_picker(ui: &mut Context, state: &mut TourState) {
    file_picker::render(ui, &mut state.file_picker);
}

/// Tab 4: Login. The typed username / password and `logged_in` flag
/// are kept in TourState so a successful submit sticks even if the
/// user navigates away and back.
fn render_login(ui: &mut Context, state: &mut TourState) {
    login::render(ui, &mut state.login);
}

/// Tab 5: Modal + Toast. The embedded demo handles Delete-to-open,
/// Yes/No clicks, and Esc-to-dismiss internally. We pass a persistent
/// `state.modal_toast` so clicks settle and the items counter
/// decrements correctly.
fn render_modal_toast(ui: &mut Context, state: &mut TourState) {
    modal_toast::render(ui, &mut state.modal_toast);
}

/// Tab 6: Table. The search filter, sort column, and table cursor
/// live in TourState so the user's filter doesn't reset on tab
/// switch.
fn render_table(ui: &mut Context, state: &mut TourState) {
    table::render(ui, &mut state.table);
}
