//! v0.20.0 progress / spinner Response demo — hover both widgets to confirm
//! they now return a real `Response`.
//!
//! Demonstrates: #212 (`Context::progress` and `Context::spinner` upgraded
//! from `&mut Self` to `Response`, enabling hover / tooltip wiring).
//!
//! Run: `cargo run --example v020_progress_response`
//!
//! Keys:
//!   Ctrl-Q / Esc — quit
//!
//! Layout:
//!   ┌── v0.20.0 #212: Response from progress / spinner ──┐
//!   │  ⠋  Loading...                                       │
//!   │  ████████░░░░░░░░░░░░    ratio = 42%                 │
//!   └──────────────────────────────────────────────────────┘

use slt::widgets::SpinnerState;
use slt::{Border, Color, Context, KeyCode, KeyModifiers, RunConfig};

/// Per-frame ratio step. Negated when ratio reaches the [0.0, 1.0] bounds so
/// the bar pingpongs forever without runaway accumulation.
const RATIO_STEP: f64 = 0.01;

fn main() -> std::io::Result<()> {
    let mut state = DemoState::new();

    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        state.advance();
        render(ui, &mut state);
    })
}

/// Demo state — animated progress ratio plus the spinner glyph cycle.
pub struct DemoState {
    /// Spinner phase (held by `SpinnerState::dots()`).
    pub spinner: SpinnerState,
    /// Current progress in `0.0..=1.0`.
    pub ratio: f64,
    /// Direction of motion. Flipped at each endpoint.
    pub step: f64,
}

impl DemoState {
    /// Construct with the spinner at frame 0 and the bar at the start.
    pub fn new() -> Self {
        Self {
            spinner: SpinnerState::dots(),
            ratio: 0.0,
            step: RATIO_STEP,
        }
    }

    /// Advance the ratio by one tick, reversing direction at each endpoint.
    pub fn advance(&mut self) {
        self.ratio += self.step;
        if self.ratio >= 1.0 {
            self.ratio = 1.0;
            self.step = -self.step;
        } else if self.ratio <= 0.0 {
            self.ratio = 0.0;
            self.step = -self.step;
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

    let _ = ui
        .bordered(Border::Rounded)
        .title("v0.20.0 #212: Response from progress / spinner")
        .p(sp.xs())
        .gap(sp.xs())
        .col(|ui| {
            ui.text("Hover the spinner or the progress bar to see Response in action.")
                .fg(Color::Cyan);

            let _ = ui.row(|ui| {
                let s = ui.spinner(&state.spinner);
                ui.text(" Loading...").dim();
                if s.hovered {
                    ui.text("  (spinner hovered!)").fg(Color::Yellow);
                }
            });

            let pr = ui.progress(state.ratio);
            ui.text(format!("ratio = {:.0}%", state.ratio * 100.0))
                .dim();
            if pr.hovered {
                ui.text("  Progress hovered — click would trigger a scrubber")
                    .fg(Color::Yellow);
            }

            ui.text("Ctrl-Q / Esc quits.").dim();
        });
}
