//! v0.20.0 spacing-scale demo — three density presets side-by-side, all
//! widgets identical, only `theme.spacing` differs.
//!
//! Demonstrates: #227.
//!
//! Run: `cargo run --example v020_spacing_scale`
//!
//! Keys:
//!   q / Esc / Ctrl-Q — quit
//!
//! Layout:
//!   ┌─────────── outer frame ─────────────────────────────────┐
//!   │ ┌── compact ──┐ ┌── comfortable ──┐ ┌── spacious ──┐    │
//!   │ │ help row    │ │ help row        │ │ help row     │    │
//!   │ │ [Click me]  │ │ [Click me]      │ │ [Click me]   │    │
//!   │ │ code block  │ │ code block      │ │ code block   │    │
//!   │ └─────────────┘ └─────────────────┘ └──────────────┘    │
//!   └─────────────────────────────────────────────────────────┘

use slt::{Border, Context, KeyCode, KeyModifiers, RunConfig, Theme};

/// Shared body. The outer frame uses the OUTER theme's spacing scale;
/// each panel re-establishes its own spacing via `container().theme(...)`.
fn body(ui: &mut Context) {
    // Outer-frame spacing comes from the active (default) theme. The inner
    // panels each pick up their own scale via the per-subtree theme override.
    let sp = ui.spacing();
    let _ = ui
        .bordered(Border::Rounded)
        .title("SLT v0.20: Density presets")
        .p(sp.xs())
        .grow(1)
        .col(|ui| {
            ui.text("Same widgets, three Theme presets — note the widening padding.")
                .dim();
            ui.text("compact = base 1, comfortable = base 2, spacious = base 3.")
                .dim();

            let _ = ui.container().gap(sp.sm()).row(|ui| {
                panel(ui, "compact", Theme::compact());
                panel(ui, "comfortable", Theme::comfortable());
                panel(ui, "spacious", Theme::spacious());
            });
        });
}

/// Per-panel render. Padding/gap inside the panel resolve against the
/// PANEL's theme — not the outer frame's — so the visual density step
/// between panels is proportional to `Theme::*::spacing.base`.
fn panel(ui: &mut Context, label: &str, theme: Theme) {
    // Capture the panel's spacing BEFORE entering the closure so the
    // padding helpers below see the override theme, not the outer one.
    let inner_sp = theme.spacing;
    let _ = ui
        .container()
        .theme(theme)
        .border(Border::Rounded)
        .title(label)
        .p(inner_sp.xs())
        .grow(1)
        .col(|ui| {
            let _ = ui.help(&[("Tab", "next"), ("Enter", "ok"), ("Esc", "cancel")]);
            ui.text("");
            let _ = ui.button("Click me");
            ui.text("");
            let _ = ui.code_block("fn main() {\n    println!(\"hi\");\n}");
        });
}

/// One-frame deterministic render entry point used by snapshot tests
/// (`tests/v020_theme_modal_demos.rs`). Equivalent to the live loop's
/// view with no events processed.
pub fn render(ui: &mut Context) {
    body(ui);
}

fn main() -> std::io::Result<()> {
    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        // macOS Ctrl-C is bound to copy in many terminals — bind quit to plain
        // `q`, Esc, and Ctrl-Q so the demo is escape-able under every setup.
        if ui.key('q') || ui.key_code(KeyCode::Esc) || ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
        }
        body(ui);
    })
}
