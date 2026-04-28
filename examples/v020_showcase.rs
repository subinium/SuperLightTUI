//! v0.20.0 showcase — every major v0.20 feature on a single screen.
//!
//! Demonstrates the **new builder APIs** introduced by the v0.20.0 API
//! consistency pass: chainable `gauge` / `line_gauge` / `breadcrumb`,
//! `GutterOpts::line_numbers`, `HighlightRange::line`, plus widthspec /
//! theme override / animate_bool / on_hover / named_focus that ship in
//! v0.20.
//!
//! Layout (80×30 minimum):
//!
//! ```text
//! ┌─ v0.20 Showcase ──────────────────────────────────────────────┐
//! │ Home › Project › src › lib.rs                  ← breadcrumb   │
//! ├──────────── WidthSpec sampler ─────────────┬─ Theme subtree ──┤
//! │ Fixed(20)  ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒              │ Hover me [btn]   │
//! │ Pct(40)    ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒        │ animate_bool fade│
//! │ Ratio(1,3) ▒▒▒▒▒▒▒▒▒▒                      │ Dracula colors   │
//! │ MinMax     ▒▒▒▒▒▒▒▒▒▒▒▒                    │                  │
//! │ Auto       (content-sized)                  │                  │
//! ├─────────────── Gauges (color tiers) ────────┴──────────────────┤
//! │ CPU      ━━━━━━━━━━━━━━━━━━━━━━━━━━━─────  42%                 │
//! │ Memory   ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  78%                 │
//! │ Disk     ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  95%                 │
//! ├──────── named_focus + on_hover ────┬─── gutter highlights ─────┤
//! │ Name  [             ]              │  1 │ pub fn one()         │
//! │ Email [             ]              │  2 │ ERROR unresolved     │
//! │ [Save] [Cancel]                    │  3 │ pub fn two()         │
//! ├────────────────────────────────────┴───────────────────────────┤
//! │ Tab/⇧Tab focus · Space toggle · n/p highlight · ? help · ^Q    │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Run with: `cargo run --example v020_showcase`
//!
//! Demos: #209 on_hover, #210 animate_bool, #213 breadcrumb_response,
//! #217 named_focus, #224 gauge / line_gauge, #226 theme override,
//! #235 gutter highlights, #237 WidthSpec.

use slt::{
    Color, Constraints, Context, GutterOpts, HighlightRange, KeyCode, KeyModifiers, ScrollState,
    TextInputState, Theme, ToastLevel,
};

const BREADCRUMB: &[&str] = &["Home", "Project", "src", "lib.rs"];

const SAMPLE_LINES: &[&str] = &[
    "pub fn one() -> u32 { 1 }",
    "ERROR: unresolved import `super::missing`",
    "pub fn two() -> u32 { 2 }",
    "WARN: unused variable `x`",
    "pub fn three() -> u32 { 3 }",
    "INFO: build complete in 1.42s",
    "pub fn four() -> u32 { 4 }",
];

fn highlights() -> Vec<HighlightRange> {
    SAMPLE_LINES
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("ERROR") || line.starts_with("WARN"))
        .map(|(i, _)| HighlightRange::line(i))
        .collect()
}

fn main() -> std::io::Result<()> {
    let mut name = TextInputState::new();
    let mut email = TextInputState::new();
    let mut panel_open = true;
    let mut scroll = ScrollState::default();
    let hl = highlights();
    scroll.set_highlights(&hl);

    slt::run_with(slt::RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.key(' ') {
            panel_open = !panel_open;
        }
        if ui.key('n') {
            scroll.highlight_next();
        }
        if ui.key('p') {
            scroll.highlight_previous();
        }

        render(ui, &mut name, &mut email, panel_open, &mut scroll);
    })
}

/// Render one full showcase frame. Public so the snapshot test can pin it.
pub fn render(
    ui: &mut Context,
    name: &mut TextInputState,
    email: &mut TextInputState,
    panel_open: bool,
    scroll: &mut ScrollState,
) {
    let theme = ui.theme();
    let pad = theme.spacing.xs();
    let panel_alpha = ui.animate_bool("showcase::panel", panel_open);

    let _ = ui
        .bordered(slt::Border::Rounded)
        .title("v0.20 Showcase")
        .p(pad)
        .gap(pad)
        .col(|ui| {
            // Row 1 — breadcrumb (#213, builder API).
            // The new chainable form: `.separator(s).color(c)`. The Drop
            // renders without us having to capture the response.
            let crumb = ui
                .breadcrumb(BREADCRUMB)
                .separator(" › ")
                .color(Color::Cyan)
                .show();
            if let Some(idx) = crumb.clicked_segment {
                ui.notify(&format!("breadcrumb: clicked {idx}"), ToastLevel::Info);
            }

            // Row 2 — WidthSpec sampler (#237) | theme subtree (#226).
            let _ = ui.row(|ui| {
                let _ = ui.container().fill().gap(pad).col(|ui| {
                    ui.text("WidthSpec sampler (#237)").bold();
                    spec_row(ui, "Fixed(20)", Constraints::default().w(20));
                    spec_row(ui, "Pct(40)", Constraints::default().w_pct(40));
                    spec_row(ui, "Ratio(1,3)", Constraints::default().w_ratio(1, 3));
                    spec_row(
                        ui,
                        "MinMax(10,30)",
                        Constraints::default().min_w(10).max_w(30),
                    );
                    spec_row(ui, "Auto", Constraints::default());
                });

                // Theme subtree (#226). Switching theme on the builder
                // affects every widget rendered inside; the original theme
                // is restored automatically on exit (panic-safe).
                let _ = ui
                    .container()
                    .w(28)
                    .theme(Theme::dracula())
                    .border(slt::Border::Single)
                    .p(pad)
                    .gap(pad)
                    .col(|ui| {
                        ui.text("Theme: Dracula (#226)").bold();
                        let _ = ui
                            .button("Hover me")
                            .on_hover(ui, "on_hover tooltip — chained Response (#209)");
                        ui.text(format!("animate_bool α = {panel_alpha:.2}")).dim();
                        if panel_alpha > 0.0 {
                            let alpha_color = match panel_alpha {
                                a if a > 0.66 => Color::Green,
                                a if a > 0.33 => Color::Yellow,
                                _ => Color::DarkGray,
                            };
                            ui.text(format!("Fading panel ({:.0}%)", panel_alpha * 100.0))
                                .fg(alpha_color);
                        }
                    });
            });

            // Row 3 — gauges (#224, builder API).
            // Chainable: `ui.gauge(ratio).label(...).width(...)`,
            // `ui.line_gauge(ratio).label(...).width(...).filled(...)`.
            let _ = ui.bordered(slt::Border::Single).p(pad).gap(pad).col(|ui| {
                ui.text("Gauges (#224 — color tiers green / yellow / red)")
                    .bold();
                gauge_row(ui, "CPU   ", 0.42);
                gauge_row(ui, "Memory", 0.78);
                gauge_row(ui, "Disk  ", 0.95);
            });

            // Row 4 — named_focus (#217) | gutter highlights (#235).
            let _ = ui.row(|ui| {
                let _ = ui.container().fill().gap(pad).col(|ui| {
                    ui.text("named_focus + on_hover (#217 + #209)").bold();
                    ui.register_focusable_named("name");
                    let _ = ui.text_input(name);
                    ui.register_focusable_named("email");
                    let _ = ui.text_input(email);
                    let _ = ui.row(|ui| {
                        let save = ui.button("Save").on_hover(ui, "save form (Enter on Save)");
                        if save.clicked {
                            ui.notify("saved", ToastLevel::Success);
                        }
                        let _ = ui.button("Cancel").on_hover(ui, "discard changes");
                    });
                    if let Some(name) = ui.focused_name() {
                        ui.text(format!("focused: {name}")).dim();
                    }
                });

                let _ = ui.container().w(34).gap(pad).col(|ui| {
                    ui.text("Gutter highlights (#235)  n/p navigates").bold();
                    // GutterOpts::new takes the labeling closure; for the
                    // 90% line-number case use `GutterOpts::line_numbers`.
                    let r = ui.scrollable_with_gutter(
                        scroll,
                        GutterOpts::line_numbers(SAMPLE_LINES.len(), SAMPLE_LINES.len() as u32),
                        |ui, i| {
                            let line = SAMPLE_LINES[i];
                            let style = if line.starts_with("ERROR") {
                                Color::LightRed
                            } else if line.starts_with("WARN") {
                                Color::Yellow
                            } else {
                                Color::White
                            };
                            ui.styled(line, slt::Style::new().fg(style));
                        },
                    );
                    if let (Some(i), total) = (r.current_highlight, r.total_highlights) {
                        ui.text(format!("match {} of {}", i + 1, total)).dim();
                    }
                });
            });

            // Row 5 — footer (key bindings).
            ui.text("Tab/⇧Tab focus · Space toggle · n/p highlights · ? help · Ctrl-Q quit")
                .dim();
        });
}

fn spec_row(ui: &mut Context, label: &str, constraints: Constraints) {
    let _ = ui.row(|ui| {
        let _ = ui.container().w(14).col(|ui| {
            ui.text(label);
        });
        let _ = ui
            .bordered(slt::Border::Single)
            .constraints(constraints)
            .h(3)
            .col(|ui| {
                // f64 ratio. New builder.
                ui.gauge(0.55);
            });
    });
}

fn gauge_row(ui: &mut Context, label: &str, ratio: f64) {
    let _ = ui.row(|ui| {
        let _ = ui.container().w(8).col(|ui| {
            ui.text(label);
        });
        // Builder API: chained `.label().width().filled()`.
        ui.line_gauge(ratio).width(48).filled('━');
        ui.text(format!("{:>3.0}%", ratio * 100.0)).bold();
    });
}
