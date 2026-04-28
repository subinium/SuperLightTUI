//! v0.20.0 BreadcrumbResponse demo — focusable navigation segments with a
//! compound `Response`.
//!
//! Demonstrates: #213 (`breadcrumb` builder, `BreadcrumbResponse: Deref<Response>`,
//! custom separator and link color).
//!
//! Run: `cargo run --example v020_breadcrumb_response`
//!
//! Keys:
//!   Tab / Shift-Tab     — focus a segment
//!   Enter / Space       — activate the focused segment (drops trailing crumbs)
//!   Mouse click         — same as Enter
//!   r                   — reset path back to the full chain
//!   q / Esc / Ctrl-Q    — quit
//!
//! Layout:
//!   ┌── v0.20.0 #213: BreadcrumbResponse ────────┐
//!   │  Home › Projects › SuperLightTUI › v0.20.0  │
//!   │  Current segment: v0.20.0                   │
//!   └─────────────────────────────────────────────┘

use slt::{Border, Color, Context, KeyCode, KeyModifiers, RunConfig};

/// Path rendered by the breadcrumb. The "current" cursor walks back along
/// these segments in response to `clicked_segment`.
const SEGMENTS: &[&str] = &["Home", "Projects", "SuperLightTUI", "v0.20.0"];

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();

    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key('q') || ui.key_code(KeyCode::Esc) || ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
        }
        if ui.key('r') {
            state.current = SEGMENTS.len() - 1;
        }
        render(ui, &mut state);
    })
}

/// Demo state — the index of the currently selected segment.
pub struct DemoState {
    /// Index into [`SEGMENTS`] for the rightmost (active) crumb. Defaults to
    /// the last segment so the demo opens fully expanded.
    pub current: usize,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            current: SEGMENTS.len() - 1,
        }
    }
}

/// Render one frame. Stable signature for snapshot tests.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let sp = ui.spacing();

    let _ = ui
        .bordered(Border::Rounded)
        .title("v0.20.0 #213: BreadcrumbResponse")
        .p(sp.xs())
        .gap(sp.xs())
        .grow(1)
        .col(|ui| {
            ui.text("Tab / Shift-Tab focuses a segment; Enter, Space, or click activates it. 'r' resets the path.")
                .fg(Color::Cyan);

            let visible: Vec<&str> = SEGMENTS.iter().take(state.current + 1).copied().collect();
            let r = ui
                .breadcrumb(&visible)
                .separator(" › ")
                .color(Color::Cyan)
                .show();

            if let Some(i) = r.clicked_segment {
                state.current = i;
            }
            // BreadcrumbResponse derefs to Response, so .hovered works directly.
            if r.hovered {
                ui.text("(whole bar hovered)").fg(Color::Yellow);
            }

            ui.text(format!(
                "Current segment: {}    (depth {} / {})",
                SEGMENTS[state.current],
                state.current + 1,
                SEGMENTS.len(),
            ))
            .dim();
            ui.text("q / Esc / Ctrl-Q quits.").dim();
        });
}
