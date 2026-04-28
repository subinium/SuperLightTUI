//! v0.20.0 cumulative regression panel — visual proof that v0.19 → v0.20
//! features still render correctly together on a single screen.
//!
//! Reviewers running this binary should confirm at a glance that:
//! - **#200 overlay_anchor** — corner / center anchors still pin correctly
//! - **#225 modal + tab_trap** — Tab/Shift-Tab cycles inside the modal only
//! - **chart / sparkline** — line + sparkline still render (regression check)
//! - **table + scrollable** — table state with movable selection
//! - **error_boundary** — caught panic in a child closure does not crash app
//! - **#236 keymap_help_overlay** — `?` opens an overlay with all bindings
//! - **#235 gutter highlights** — gutter + n/p navigation
//! - **#224 gauge / line_gauge** — both gauge variants render together
//!
//! Keys:
//!   Tab / Shift-Tab     — navigate focusable widgets
//!   ↑ / ↓ (j / k)       — move table selection (when table is focused)
//!   M                   — toggle the modal (tab_trap on, Esc dismisses)
//!   ?                   — toggle the key-help overlay
//!   n / p               — next / prev gutter highlight
//!   q / Esc / Ctrl-Q    — quit (Ctrl-C may be bound to copy on macOS)

use slt::{
    Anchor, Border, Color, Context, GutterOpts, HighlightRange, KeyCode, KeyModifiers, ScrollState,
    TableState, Theme,
};

/// Key bindings advertised by the demo. Pulled out of the live closure so
/// both [`render`] (snapshot) and `main` (interactive) publish the exact
/// same keymap, keeping the help overlay byte-identical between paths.
pub const PANEL_KEYS: &[(&str, &str)] = &[
    ("Tab / ⇧Tab", "next / prev focusable"),
    ("M", "open modal (tab_trap on)"),
    ("?", "open key-help overlay"),
    ("n / p", "next / prev highlight"),
    ("Esc / ^Q", "quit"),
];

/// Log lines fed to the gutter scrollable. Public so the [`render`]
/// fixture and `main` stay aligned without duplicating the strings.
pub const LOG_LINES: &[&str] = &[
    "INFO  app starting",
    "DEBUG loaded config",
    "INFO  listening on :8080",
    "ERROR upstream timeout",
    "INFO  retrying...",
    "ERROR database unavailable",
    "INFO  switched to fallback",
    "DEBUG cache hit",
    "INFO  request OK",
    "WARN  rate limit nearing",
];

/// Compute the highlight ranges marked on the scrollable's gutter.
pub fn highlights() -> Vec<HighlightRange> {
    LOG_LINES
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("ERROR"))
        .map(|(i, _)| HighlightRange::line(i))
        .collect()
}

/// All mutable widget state owned by the demo.
///
/// Held by `main` for the live loop and constructed fresh by the snapshot
/// test. Keeping the fields `pub` lets the snapshot flip `modal_open` /
/// `help_open` deterministically without re-running the event loop.
pub struct DemoState {
    /// Table widget state (selection cursor, scroll).
    pub table: TableState,
    /// Scrollable widget state with highlights pre-loaded.
    pub scroll: ScrollState,
    /// Whether the tab-trap modal is currently open.
    pub modal_open: bool,
    /// Whether the keymap help overlay is currently open.
    pub help_open: bool,
    /// 40-sample CPU sparkline history.
    pub cpu_history: Vec<f64>,
    /// 40-sample request-rate sparkline history.
    pub req_history: Vec<f64>,
}

impl DemoState {
    /// Build the state used by the live binary. Sparkline histories are
    /// deterministic (sin/cos of index) so `render` produces the same
    /// frame regardless of when the demo is launched.
    pub fn new() -> Self {
        let mut scroll = ScrollState::default();
        scroll.set_highlights(&highlights());
        let table = TableState::new(
            vec!["Name", "Status", "Latency"],
            vec![
                vec!["alpha", "ok", "12ms"],
                vec!["beta", "ok", "47ms"],
                vec!["gamma", "warn", "284ms"],
                vec!["delta", "fail", "—"],
            ],
        );
        let cpu_history: Vec<f64> = (0..40)
            .map(|i| 0.5 + (i as f64 * 0.4).sin() * 0.25)
            .collect();
        let req_history: Vec<f64> = (0..40)
            .map(|i| 30.0 + (i as f64 * 0.6).cos() * 12.0)
            .collect();
        Self {
            table,
            scroll,
            modal_open: false,
            help_open: false,
            cpu_history,
            req_history,
        }
    }
}

impl Default for DemoState {
    fn default() -> Self {
        Self::new()
    }
}

/// One-frame deterministic render entry point used by snapshot tests
/// (`tests/v020_regression_panel_demo.rs`).
///
/// Renders the full regression panel — gauges, sparklines, table, gutter
/// scrollable, anchored overlays, and (when toggled) the modal +
/// keymap-help overlays. `main` calls this from inside the live event
/// loop after consuming input, so interactive output and the snapshot are
/// pixel-identical for a given `state`.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    ui.publish_keymap("regression_panel", PANEL_KEYS);

    let theme = ui.theme();
    let pad = theme.spacing.xs();
    // Pull theme colors out of the Theme so closures don't need to
    // re-borrow `theme` — they capture the `Color` value directly.
    // Showcasing the theme system rather than hardcoding RGB demo values.
    let badge_bg = theme.surface;
    let badge_fg = theme.primary;
    let center_dim = theme.text_dim;

    // Wrap in error_boundary so a panic in any sub-section is caught.
    let cpu_history = state.cpu_history.clone();
    let req_history = state.req_history.clone();
    let table = &mut state.table;
    let scroll = &mut state.scroll;
    let modal_open = &mut state.modal_open;
    ui.error_boundary(|ui| {
        let _ = ui
            .bordered(Border::Rounded)
            .title("v0.20 Regression Panel")
            .p(pad)
            .gap(pad)
            .grow(1)
            .col(|ui| {
                // Row 1: gauges (#224 — builder API).
                let _ = ui.row(|ui| {
                    let _ = ui.container().fill().col(|ui| {
                        ui.text("Gauges (#224)").bold();
                        ui.gauge(0.42).label("CPU 42%").width(28);
                        ui.line_gauge(0.78).label("MEM 78%").width(28);
                    });
                    let _ = ui.container().w(36).col(|ui| {
                        ui.text("Sparkline + chart").bold();
                        let _ = ui.sparkline(&cpu_history, 30);
                        let _ = ui.sparkline(&req_history, 30);
                    });
                });

                // Row 2: table + log/gutter highlights (#235).
                let _ = ui.row(|ui| {
                    let _ = ui.container().fill().col(|ui| {
                        ui.text("Table (j/k or ↑/↓)").bold();
                        let _ = ui.table(table);
                    });
                    let _ = ui.container().w(40).col(|ui| {
                        ui.text("Gutter highlights (#235) n/p").bold();
                        let r = ui.scrollable_with_gutter(
                            scroll,
                            GutterOpts::line_numbers(LOG_LINES.len(), 8),
                            |ui, abs| {
                                let line = LOG_LINES[abs];
                                let color = if line.contains("ERROR") {
                                    Color::Red
                                } else if line.contains("WARN") {
                                    Color::Yellow
                                } else {
                                    Color::Reset
                                };
                                ui.styled(line, slt::Style::new().fg(color));
                            },
                        );
                        if let (Some(i), n) = (r.current_highlight, r.total_highlights) {
                            ui.text(format!("match {}/{}", i + 1, n)).dim();
                        }
                    });
                });

                ui.text("press M to open modal (tab_trap), ? for key-help, n/p navigates")
                    .dim();
            });

        // overlay_anchor (#200) — 4 corners + center, all visible at once.
        // Colors come from the active theme so the demo also showcases #226.
        let _ = ui.overlay_at(Anchor::TopLeft, |ui| {
            ui.styled(" ◤ TL ", slt::Style::new().bg(badge_bg).fg(badge_fg));
        });
        let _ = ui.overlay_at(Anchor::TopRight, |ui| {
            ui.styled(" TR ◥ ", slt::Style::new().bg(badge_bg).fg(badge_fg));
        });
        let _ = ui.overlay_at(Anchor::BottomLeft, |ui| {
            ui.styled(" ◣ BL ", slt::Style::new().bg(badge_bg).fg(badge_fg));
        });
        let _ = ui.overlay_at(Anchor::BottomRight, |ui| {
            ui.styled(" BR ◢ ", slt::Style::new().bg(badge_bg).fg(badge_fg));
        });
        let modal_is_open = *modal_open;
        let _ = ui.overlay_at(Anchor::Center, |ui| {
            if modal_is_open {
                // The actual modal renders below; this overlay just shows
                // we did NOT confuse overlay_at with modal.
            } else {
                ui.text(" ⊕ ").fg(center_dim);
            }
        });

        // modal + tab_trap (#225). Toggled by `M`.
        if *modal_open {
            let pad = ui.theme().spacing.sm();
            let _ = ui.modal_with(slt::context::ModalOptions { tab_trap: true }, |ui| {
                let _ = ui
                    .bordered(Border::Double)
                    .title("Modal (#225 tab_trap)")
                    .p(pad)
                    .theme(Theme::dracula())
                    .col(|ui| {
                        ui.text("Tab cycles within this modal only.");
                        ui.text("");
                        let _ = ui.button("OK");
                        let _ = ui.button("Cancel");
                        if ui.button("Close (M)").clicked {
                            *modal_open = false;
                        }
                    });
            });
        }
    });

    // keymap_help_overlay (#236) renders on top so it can dismiss modal too.
    ui.keymap_help_overlay(state.help_open);
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::new();

    // publish keymap so keymap_help_overlay has something to show
    slt::run_with(
        slt::RunConfig::default().mouse(true),
        move |ui: &mut Context| {
            // Standard exit-key policy: bare `q`, Esc, and Ctrl-Q. Ctrl-C is
            // intentionally NOT bound — many terminals (e.g. macOS Terminal,
            // iTerm2 with default copy-shortcut) intercept Ctrl-C before it
            // reaches the app. Quit only when no overlay/modal is intercepting
            // input — Esc inside the modal/help-overlay must dismiss it
            // first.
            let any_overlay = state.modal_open || state.help_open;
            if !any_overlay
                && (ui.key('q')
                    || ui.key_code(KeyCode::Esc)
                    || ui.key_mod('q', KeyModifiers::CONTROL))
            {
                ui.quit();
            }
            // M toggles the modal. When the modal is already open, the modal
            // guard inside `key()` blocks us — fall back to `raw_key_code`
            // so the same key still closes the modal.
            if !any_overlay && (ui.key('m') || ui.key('M')) {
                state.modal_open = true;
            } else if state.modal_open && ui.raw_key_code(KeyCode::Char('m')) {
                state.modal_open = false;
            }
            // `?` toggles the key-help overlay. Same `raw_*` fallback as M:
            // once the overlay is open it counts as a modal, so the plain
            // `key('?')` check is blocked by the overlay guard.
            if !any_overlay && ui.key('?') {
                state.help_open = true;
            } else if state.help_open && ui.raw_key_code(KeyCode::Char('?')) {
                state.help_open = false;
            }
            // Esc dismisses any open overlay (modal first, then help). Both
            // need raw_key_code because plain `key_code(Esc)` is blocked
            // while a modal is active.
            if state.modal_open && ui.raw_key_code(KeyCode::Esc) {
                state.modal_open = false;
            } else if state.help_open && ui.raw_key_code(KeyCode::Esc) {
                state.help_open = false;
            }
            if !any_overlay && ui.key('n') {
                state.scroll.highlight_next();
            }
            if !any_overlay && ui.key('p') {
                state.scroll.highlight_previous();
            }

            render(ui, &mut state);
        },
    )
}
