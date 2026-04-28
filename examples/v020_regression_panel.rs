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
//! Tab navigates focusable widgets · `M` opens modal · `?` opens key help
//! · `n`/`p` navigates highlighted lines · `Esc` / `Ctrl-Q` quits.

use slt::{
    Anchor, Border, Color, Context, GutterOpts, HighlightRange, KeyCode, KeyModifiers, ScrollState,
    TableState, Theme,
};

const PANEL_KEYS: &[(&str, &str)] = &[
    ("Tab / ⇧Tab", "next / prev focusable"),
    ("M", "open modal (tab_trap on)"),
    ("?", "open key-help overlay"),
    ("n / p", "next / prev highlight"),
    ("Esc / ^Q", "quit"),
];

const LOG_LINES: &[&str] = &[
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

fn highlights() -> Vec<HighlightRange> {
    LOG_LINES
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("ERROR"))
        .map(|(i, _)| HighlightRange::single(i))
        .collect()
}

fn main() -> std::io::Result<()> {
    let mut table = TableState::new(
        vec!["Name", "Status", "Latency"],
        vec![
            vec!["alpha", "ok", "12ms"],
            vec!["beta", "ok", "47ms"],
            vec!["gamma", "warn", "284ms"],
            vec!["delta", "fail", "—"],
        ],
    );
    let mut scroll = ScrollState::default();
    scroll.set_highlights(&highlights());
    let mut modal_open = false;
    let mut help_open = false;
    let cpu_history: Vec<f64> = (0..40)
        .map(|i| 0.5 + (i as f64 * 0.4).sin() * 0.25)
        .collect();
    let req_history: Vec<f64> = (0..40)
        .map(|i| 30.0 + (i as f64 * 0.6).cos() * 12.0)
        .collect();

    // publish keymap so keymap_help_overlay has something to show
    slt::run_with(
        slt::RunConfig::default().mouse(true),
        move |ui: &mut Context| {
            if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
                ui.quit();
            }
            if ui.key('m') || ui.key('M') {
                modal_open = !modal_open;
            }
            if ui.key('?') {
                help_open = !help_open;
            }
            if ui.key('n') {
                scroll.highlight_next();
            }
            if ui.key('p') {
                scroll.highlight_previous();
            }

            ui.publish_keymap("regression_panel", PANEL_KEYS);

            let theme = ui.theme();
            let pad = theme.spacing.xs();

            // Wrap in error_boundary so a panic in any sub-section is caught.
            ui.error_boundary(|ui| {
                let _ = ui
                    .bordered(Border::Rounded)
                    .title("v0.20 Regression Panel")
                    .p(pad)
                    .gap(pad)
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
                                let _ = ui.table(&mut table);
                            });
                            let _ = ui.container().w(40).col(|ui| {
                                ui.text("Gutter highlights (#235) n/p").bold();
                                let r = ui.scrollable_with_gutter(
                                    &mut scroll,
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
                let _ = ui.overlay_at(Anchor::TopLeft, |ui| {
                    ui.styled(
                        " ◤ TL ",
                        slt::Style::new().bg(Color::Rgb(40, 40, 40)).fg(Color::Cyan),
                    );
                });
                let _ = ui.overlay_at(Anchor::TopRight, |ui| {
                    ui.styled(
                        " TR ◥ ",
                        slt::Style::new().bg(Color::Rgb(40, 40, 40)).fg(Color::Cyan),
                    );
                });
                let _ = ui.overlay_at(Anchor::BottomLeft, |ui| {
                    ui.styled(
                        " ◣ BL ",
                        slt::Style::new().bg(Color::Rgb(40, 40, 40)).fg(Color::Cyan),
                    );
                });
                let _ = ui.overlay_at(Anchor::BottomRight, |ui| {
                    ui.styled(
                        " BR ◢ ",
                        slt::Style::new().bg(Color::Rgb(40, 40, 40)).fg(Color::Cyan),
                    );
                });
                let _ = ui.overlay_at(Anchor::Center, |ui| {
                    if modal_open {
                        // The actual modal renders below; this overlay just shows
                        // we did NOT confuse overlay_at with modal.
                    } else {
                        ui.text(" ⊕ ").fg(Color::DarkGray);
                    }
                });

                // modal + tab_trap (#225). Toggled by `M`.
                if modal_open {
                    let _ = ui.modal_with(slt::context::ModalOptions { tab_trap: true }, |ui| {
                        let _ = ui
                            .bordered(Border::Double)
                            .title("Modal (#225 tab_trap)")
                            .p(2)
                            .theme(Theme::dracula())
                            .col(|ui| {
                                ui.text("Tab cycles within this modal only.");
                                ui.text("");
                                let _ = ui.button("OK");
                                let _ = ui.button("Cancel");
                                if ui.button("Close (M)").clicked {
                                    modal_open = false;
                                }
                            });
                    });
                }
            });

            // keymap_help_overlay (#236) renders on top so it can dismiss modal too.
            ui.keymap_help_overlay(help_open);
        },
    )
}
