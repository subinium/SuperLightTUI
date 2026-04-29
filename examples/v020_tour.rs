//! v0.20.0 Tour — every interactive v0.20 demo, grouped by category and
//! switched via SLT's own `Tabs` widget. Renders the meta-view: each tab
//! reuses the existing `pub fn render(...)` from a single-feature demo.
//!
//! Run: `cargo run --example v020_tour`
//!
//! Keys:
//!   Left / Right     — switch tab (when the tabs bar is focused; Tab to focus)
//!   Tab / Shift-Tab  — cycle focus (tabs bar → demo)
//!   q / Esc / Ctrl-Q — quit
//!
//! Categories:
//!   1. Intro    — overview + navigation help
//!   2. Hooks    — use_state_keyed + use_effect + named_focus
//!   3. Theme    — theme_subtree + spacing_scale
//!   4. Modal    — modal_trap
//!   5. Layout   — split_pane + widthspec
//!   6. Widgets  — gauge + progress + breadcrumb + gutter
//!   7. Util     — ctrl_c_passthrough + keymap_help + static_log + dx_shortcuts

use slt::widgets::{ScrollState, TabsState};
use slt::{Border, Color, Context, KeyModifiers, RunConfig};

// Each `#[path = ...] mod ...;` re-includes a single-feature demo so the
// tour can call its `pub fn render(...)` directly. The demos' own `fn
// main()` and helpers are unused in this build, hence the blanket
// `#[allow(dead_code)]` on every include.
#[allow(dead_code)]
#[path = "v020_breadcrumb_response.rs"]
mod breadcrumb_response;
#[allow(dead_code)]
#[path = "v020_ctrl_c_passthrough.rs"]
mod ctrl_c_passthrough;
#[allow(dead_code)]
#[path = "v020_dx_shortcuts.rs"]
mod dx_shortcuts;
#[allow(dead_code)]
#[path = "v020_gauge.rs"]
mod gauge_demo;
#[allow(dead_code)]
#[path = "v020_gutter_highlights.rs"]
mod gutter_highlights;
#[allow(dead_code)]
#[path = "v020_keymap_help.rs"]
mod keymap_help;
#[allow(dead_code)]
#[path = "v020_modal_trap.rs"]
mod modal_trap;
#[allow(dead_code)]
#[path = "v020_named_focus.rs"]
mod named_focus;
#[allow(dead_code)]
#[path = "v020_progress_response.rs"]
mod progress_response;
#[allow(dead_code)]
#[path = "v020_spacing_scale.rs"]
mod spacing_scale;
#[allow(dead_code)]
#[path = "v020_split_pane.rs"]
mod split_pane_demo;
#[allow(dead_code)]
#[path = "v020_static_log.rs"]
mod static_log;
#[allow(dead_code)]
#[path = "v020_theme_subtree.rs"]
mod theme_subtree;
#[allow(dead_code)]
#[path = "v020_use_effect.rs"]
mod use_effect;
#[allow(dead_code)]
#[path = "v020_use_state_keyed.rs"]
mod use_state_keyed;
#[allow(dead_code)]
#[path = "v020_widthspec.rs"]
mod widthspec;

/// Aggregated state for every embedded demo. Each field is the
/// `DemoState` from the corresponding feature demo.
struct TourState {
    tabs: TabsState,
    /// Scroll offset for the active tab body. Mouse-wheel events outside any
    /// inner scrollable scroll the whole tab, so tall tabs (notably Hooks
    /// with three stacked demos) stay reachable on small terminals.
    tab_scroll: ScrollState,
    use_state_keyed: use_state_keyed::DemoState,
    use_effect: use_effect::DemoState,
    named_focus: named_focus::DemoState,
    modal: modal_trap::State,
    split_pane: split_pane_demo::DemoState,
    gauge: gauge_demo::DemoState,
    progress: progress_response::DemoState,
    gutter: gutter_highlights::DemoState,
    breadcrumb: breadcrumb_response::DemoState,
    keymap_help: keymap_help::DemoState,
    ctrl_c_passthrough: ctrl_c_passthrough::DemoState,
    dx_shortcuts: dx_shortcuts::DemoState,
}

impl Default for TourState {
    fn default() -> Self {
        Self {
            tabs: TabsState::new(vec![
                "Intro", "Hooks", "Theme", "Density", "Modal", "Layout", "Widgets", "Help", "Log",
                "Ctrl", "DX",
            ]),
            tab_scroll: ScrollState::new(),
            use_state_keyed: Default::default(),
            use_effect: Default::default(),
            named_focus: Default::default(),
            modal: Default::default(),
            split_pane: Default::default(),
            gauge: Default::default(),
            progress: Default::default(),
            gutter: Default::default(),
            breadcrumb: Default::default(),
            keymap_help: Default::default(),
            ctrl_c_passthrough: Default::default(),
            dx_shortcuts: Default::default(),
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut state = TourState::default();
    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        // Tour-level quit: Ctrl-Q only at the top of the frame. We
        // intentionally do NOT consume Esc here — embedded demos like
        // `modal_trap` rely on Esc to dismiss their own modals, and any
        // demo that wants Esc-to-quit handles it inside its own
        // `render(...)`.
        if ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
            return;
        }

        let pad = ui.spacing().xs();
        let _ = ui
            .bordered(Border::Rounded)
            .title("SLT v0.20 Tour: every feature, one demo")
            .p(pad)
            .grow(1)
            .col(|ui| {
                let _ = ui.tabs(&mut state.tabs);
                ui.separator();

                // Wrap the tab body in a vertical scrollable so tabs that
                // stack multiple sub-demos (notably Hooks: three demos in
                // one tab) stay reachable on small terminals. Mouse wheel
                // outside any inner scroll region scrolls the tab body;
                // when the body fits the viewport this is a no-op.
                let _ = ui.scrollable(&mut state.tab_scroll).grow(1).col(|ui| {
                    match state.tabs.selected {
                        0 => render_intro(ui),
                        1 => render_hooks(ui, &mut state),
                        2 => render_theme(ui),
                        3 => render_density(ui),
                        4 => render_modal(ui, &mut state),
                        5 => render_layout(ui, &mut state),
                        6 => render_widgets(ui, &mut state),
                        7 => keymap_help::render(ui, &mut state.keymap_help),
                        8 => render_log(ui),
                        9 => ctrl_c_passthrough::render(ui, &mut state.ctrl_c_passthrough),
                        10 => dx_shortcuts::render(ui, &mut state.dx_shortcuts),
                        _ => {}
                    }
                });
            });

        // 'q' is checked AFTER demos render so a focused text_input
        // (e.g. on the Hooks tab) consumes it as text first.
        if ui.key('q') {
            ui.quit();
        }
    })
}

/// Tab 1: Intro. Pure overview — no embedded demo.
fn render_intro(ui: &mut Context) {
    let _ = ui.col(|ui| {
        let pad = ui.spacing().xs();
        ui.text("Welcome to the v0.20 tour.").bold();
        ui.text("");
        ui.text("Each tab embeds the corresponding single-feature demo from").dim();
        ui.text("examples/v020_*.rs without modification — what you see in").dim();
        ui.text("a tab is exactly the standalone demo's render path.").dim();
        ui.text("");
        let _ = ui
            .bordered(Border::Single)
            .title("v0.20 features at a glance")
            .p(pad)
            .col(|ui| {
                row_pair(ui, "Hooks",   "use_state_keyed · use_effect · register_focusable_named");
                row_pair(ui, "Theme",   "per-subtree theme override (Dark/Light/Dracula/Nord)");
                row_pair(ui, "Density", "compact / comfortable / spacious spacing presets");
                row_pair(ui, "Modal",   "tab-trap focus locking inside modals");
                row_pair(ui, "Layout",  "split_pane / vsplit_pane · WidthSpec / HeightSpec");
                row_pair(ui, "Widgets", "gauge / line_gauge builders · progress / spinner Response · breadcrumb · gutter highlights");
                row_pair(ui, "Help",    "keymap_help — `?` opens a centered keyboard-shortcut overlay");
                row_pair(ui, "Log",     "static_log — append once-only scrollback lines without re-rendering");
                row_pair(ui, "Ctrl",    "ctrl_c passthrough (Ctrl-G alternative on macOS where Ctrl-C is copy)");
                row_pair(ui, "DX",      "provide / use_context / use_state_named / with_if shortcuts");
            });
        ui.text("");
        ui.text("Navigation: Left/Right arrows switch tabs (Tab to focus the bar). q / Esc / Ctrl-Q quits.")
            .fg(Color::Cyan);
    });
}

/// One label/description row for the intro feature list.
fn row_pair(ui: &mut Context, label: &str, desc: &str) {
    let _ = ui.container().gap(1).row(|ui| {
        ui.text(format!("{label:<8}")).bold().fg(Color::Cyan);
        ui.text(desc).dim();
    });
}

/// Tab 2: Hooks. Three demos in one tab — keyed counters | effect log,
/// with named_focus inputs across the bottom.
fn render_hooks(ui: &mut Context, state: &mut TourState) {
    let _ = ui.col(|ui| {
        let _ = ui.row(|ui| {
            let _ = ui.container().fill().col(|ui| {
                use_state_keyed::render(ui, &mut state.use_state_keyed);
            });
            let _ = ui.container().fill().col(|ui| {
                use_effect::render(ui, &mut state.use_effect);
            });
        });
        let _ = ui.container().fill().col(|ui| {
            named_focus::render(ui, &mut state.named_focus);
        });
    });
}

/// Tab 3: Theme. Same widgets rendered with four different `Theme`
/// presets via `container().theme(...)`. The point is that the inner
/// panels override the colour palette without leaking back to the
/// outer scope.
fn render_theme(ui: &mut Context) {
    theme_subtree::render(ui);
}

/// Tab 4: Density. Same widgets rendered with three `Theme.spacing`
/// presets (compact/comfortable/spacious). The point is the shared
/// `theme.spacing` scale — padding, gap, and margin all widen
/// proportionally without per-widget overrides.
fn render_density(ui: &mut Context) {
    spacing_scale::render(ui);
}

/// Tab 4: Modal. The embedded demo handles M-to-open and Esc-to-dismiss
/// internally. We pass a persistent `state.modal` so clicks on Yes/No
/// settle and don't get reset next frame.
fn render_modal(ui: &mut Context, state: &mut TourState) {
    modal_trap::render(ui, &mut state.modal);
}

/// Tab 5: Layout. split_pane on the left half, widthspec on the right.
fn render_layout(ui: &mut Context, state: &mut TourState) {
    let _ = ui.row(|ui| {
        let _ = ui.container().fill().col(|ui| {
            split_pane_demo::render(ui, &mut state.split_pane);
        });
        let _ = ui.container().fill().col(|ui| {
            widthspec::render(ui);
        });
    });
}

/// Tab 6: Widgets. 2x2 grid — gauge, progress, breadcrumb, gutter.
fn render_widgets(ui: &mut Context, state: &mut TourState) {
    let _ = ui.col(|ui| {
        let _ = ui.row(|ui| {
            let _ = ui.container().fill().col(|ui| {
                gauge_demo::render(ui, &mut state.gauge);
            });
            let _ = ui.container().fill().col(|ui| {
                progress_response::render(ui, &mut state.progress);
            });
        });
        let _ = ui.row(|ui| {
            let _ = ui.container().fill().col(|ui| {
                breadcrumb_response::render(ui, &mut state.breadcrumb);
            });
            let _ = ui.container().fill().col(|ui| {
                gutter_highlights::render(ui, &mut state.gutter);
            });
        });
    });
}

// Tabs 8-11 dispatch directly to the embedded demo's `render` (see the
// match arms in `main`). They were originally combined under a single
// "Util" tab as a 2x2 grid, but each demo claims overlay space and
// owns its own keymap — `keymap_help` opens a centered overlay that
// covers neighbouring cells, `static_log` appends to the terminal
// scrollback unboundedly, `dx_shortcuts` and `ctrl_c_passthrough` both
// register quit keys — so combining them produced overlapping
// overlays, infinite log spam, and key conflicts. One tab per demo
// gives each its own full canvas without duplicate work.

/// Tab 9: Log. Description-only page for `static_log`. The real demo
/// would call `ui.static_log(...)` on every frame, which writes to the
/// terminal scrollback and visibly corrupts the tour's bordered frame
/// (each push moves the inline buffer down a row). Run the standalone
/// demo to see the actual scrollback effect.
fn render_log(ui: &mut Context) {
    let pad = ui.spacing().xs();
    let _ = ui
        .bordered(Border::Rounded)
        .title("v0.20 #233: static_log (append-only scrollback)")
        .p(pad)
        .grow(1)
        .col(|ui| {
            ui.text(
                "ui.static_log(line) prints `line` once into the terminal's",
            )
            .dim();
            ui.text(
                "scrollback above the inline TUI buffer, then never re-renders",
            )
            .dim();
            ui.text("it. Use for cumulative event logs that must survive tear-down")
                .dim();
            ui.text("and re-render cycles without flicker.").dim();
            ui.text("");
            let _ = ui
                .bordered(Border::Single)
                .title("typical usage")
                .p(pad)
                .col(|ui| {
                    let _ = ui.code_block_lang(
                        "if frame % 5 == 0 {\n    ui.static_log(format!(\"[tick] count = {count}\"));\n}",
                        "rust",
                    );
                });
            ui.text("");
            ui.text("This page is description-only because calling static_log on")
                .fg(Color::Yellow);
            ui.text("every frame would push a new line into scrollback each tick,")
                .fg(Color::Yellow);
            ui.text("visibly corrupting the tour's bordered frame above.")
                .fg(Color::Yellow);
            ui.text("");
            ui.text("To see the actual scrollback effect, run the standalone demo:")
                .dim();
            ui.text("    cargo run --example v020_static_log").fg(Color::Cyan);
        });
}
