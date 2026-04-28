//! v0.20.0 split_pane / vsplit_pane demo — draggable two-pane container.
//!
//! Demonstrates: #223 (split_pane / vsplit_pane builder, mouse drag, focusable handle).
//!
//! Run: `cargo run --example v020_split_pane`
//!
//! Keys:
//!   Tab / Shift-Tab — focus the split handle
//!   Left / Right    — adjust ratio when horizontal handle is focused
//!   Up   / Down     — adjust ratio when vertical handle is focused
//!   v               — toggle horizontal / vertical orientation
//!   Ctrl-Q / Esc    — quit
//!
//! Layout:
//!   ┌── horizontal ──────────────────────────┐
//!   │ LEFT PANE        │ RIGHT PANE          │
//!   │                  │                     │
//!   │                  ↑ drag this handle    │
//!   └────────────────────────────────────────┘

use slt::{Border, Color, Context, KeyCode, KeyModifiers, RunConfig, SplitPaneState};

fn main() -> std::io::Result<()> {
    let mut state = DemoState::new();

    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.key('v') {
            state.vertical = !state.vertical;
        }
        render(ui, &mut state);
    })
}

/// Per-demo state captured in one place so `render` can be called from both
/// the runtime loop and the snapshot test in `tests/v020_widgets_demos.rs`.
pub struct DemoState {
    /// Backing state for the split widget (ratio + drag flag).
    pub split: SplitPaneState,
    /// `true` when the demo is in vertical orientation (`vsplit_pane`).
    pub vertical: bool,
}

impl DemoState {
    /// Construct with the same defaults the live demo opens with.
    pub fn new() -> Self {
        Self {
            split: SplitPaneState::new(0.4),
            vertical: false,
        }
    }
}

impl Default for DemoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render one frame. Stable signature for snapshot tests.
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let sp = ui.spacing();
    let title = if state.vertical {
        "v0.20.0 #223 — vsplit_pane (vertical)"
    } else {
        "v0.20.0 #223 — split_pane (horizontal)"
    };

    let _ = ui
        .bordered(Border::Rounded)
        .title(title)
        .p(sp.xs())
        .gap(sp.xs())
        .grow(1)
        .col(|ui| {
            ui.text("Tab focuses the handle, arrows adjust the ratio. 'v' toggles orientation.")
                .fg(Color::Cyan);

            let r = if state.vertical {
                ui.vsplit_pane(
                    &mut state.split,
                    |ui| {
                        ui.text("TOP PANE").bold();
                        ui.text("Drag the handle below or arrow-key it.");
                    },
                    |ui| {
                        ui.text("BOTTOM PANE").bold();
                        ui.text("Status: ratio updates live.");
                    },
                )
            } else {
                ui.split_pane(
                    &mut state.split,
                    |ui| {
                        ui.text("LEFT PANE").bold();
                        ui.text("Drag the handle right of this pane.");
                    },
                    |ui| {
                        ui.text("RIGHT PANE").bold();
                        ui.text("Or use the arrow keys when the handle is focused.");
                    },
                )
            };

            ui.text(format!(
                "ratio = {:.2}    drag_active = {}",
                r.ratio, r.drag_active
            ))
            .dim();
            ui.text("Ctrl-Q / Esc quits.").dim();
        });
}
