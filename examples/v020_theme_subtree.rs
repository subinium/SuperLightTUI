//! v0.20.0 theme-subtree demo — per-subtree theme override.
//!
//! Demonstrates: #226
//!
//! Four side-by-side panels render the same widgets under four different
//! themes. Each panel uses `ContainerBuilder::theme(...)` to scope the
//! theme change to its own subtree — the outer container keeps its
//! parent theme, so nothing leaks across panel boundaries.
//!
//! Run: `cargo run --example v020_tour`
//!
//! Keys:
//!   q / Esc / Ctrl-Q — quit
//!
//! Layout:
//!   ┌── SLT v0.20: Per-subtree theme override ──────────────────┐
//!   │ ┌── Dark ──┐ ┌── Light ──┐ ┌── Dracula ──┐ ┌── Nord ──┐    │
//!   │ │ body…    │ │ body…     │ │ body…       │ │ body…    │    │
//!   │ │ [Press]  │ │ [Press]   │ │ [Press]     │ │ [Press]  │    │
//!   │ │ alert    │ │ alert     │ │ alert       │ │ alert    │    │
//!   │ │ code     │ │ code      │ │ code        │ │ code     │    │
//!   │ └──────────┘ └───────────┘ └─────────────┘ └──────────┘    │
//!   └────────────────────────────────────────────────────────────┘

use slt::widgets::AlertLevel;
use slt::{Border, Context, KeyCode, KeyModifiers, RunConfig, Theme};

/// Theme bench: label + theme constructor pairs rendered in panel order.
/// Centralised so the layout and the test render the same set.
pub fn theme_bench() -> [(&'static str, Theme); 4] {
    [
        ("Dark (default)", Theme::dark()),
        ("Light", Theme::light()),
        ("Dracula", Theme::dracula()),
        ("Nord", Theme::nord()),
    ]
}

fn main() -> std::io::Result<()> {
    slt::run_with(RunConfig::default().mouse(true), render)
}

/// Render one frame of the theme-subtree demo.
///
/// Public so snapshot tests can compare per-theme renders against fixed
/// markers without re-deriving the panel layout in each test.
pub fn render(ui: &mut Context) {
    // macOS Ctrl-C is bound to copy in many terminals — bind quit to plain `q`,
    // Esc, and Ctrl-Q so the demo is escape-able under every common setup.
    if ui.key('q') || ui.key_code(KeyCode::Esc) || ui.key_mod('q', KeyModifiers::CONTROL) {
        ui.quit();
        return;
    }

    let pad = ui.spacing().xs();
    let panel_gap = ui.spacing().sm();

    let _ = ui
        .bordered(Border::Rounded)
        .title("SLT v0.20: Per-subtree theme override")
        .p(pad)
        .grow(1)
        .col(|ui| {
            ui.text("Each panel below uses a different Theme via container().theme(...).")
                .dim();
            ui.text("Outer scope keeps its parent theme — nothing leaks across panels.")
                .dim();

            let _ = ui.container().gap(panel_gap).row(|ui| {
                for (label, theme) in theme_bench() {
                    panel(ui, label, theme);
                }
            });
        });
}

/// Render a single themed panel. All widgets inside the closure resolve
/// their colours against `theme`, not the parent's theme — that's the
/// invariant #226 added to `ContainerBuilder`.
fn panel(ui: &mut Context, label: &str, theme: Theme) {
    let pad = ui.spacing().xs();
    let _ = ui
        .container()
        .theme(theme)
        .border(Border::Rounded)
        .p(pad)
        .grow(1)
        .col(|ui| {
            ui.text(label).bold();
            ui.text("body text").dim();
            let _ = ui.button("Press me");
            // Two alert levels exercise distinct theme tokens (info vs warning).
            let _ = ui.alert("info banner", AlertLevel::Info);
            let _ = ui.alert("warning", AlertLevel::Warning);
            let _ = ui.code_block("let x = 1;");
        });
}
