//! v0.20.0 progress / spinner Response demo — hover both widgets to confirm
//! they now return a real `Response`.
//!
//! Demonstrates: #212 (`Context::progress` and `Context::spinner` upgraded
//! from `&mut Self` to `Response`, enabling hover / tooltip wiring).
//!
//! Run: `cargo run --example v020_progress_response`
//!
//! Keys:
//!   Space            — pause / resume the animation
//!   Left / Right     — nudge ratio by 5% (also pauses if running)
//!   Hover (mouse)    — highlights the spinner / progress bar
//!   q / Esc / Ctrl-Q — quit
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

/// Manual nudge applied by Left/Right arrows when the user takes over.
const MANUAL_STEP: f64 = 0.05;

fn main() -> std::io::Result<()> {
    let mut state = DemoState::new();

    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key('q') || ui.key_code(KeyCode::Esc) || ui.key_mod('q', KeyModifiers::CONTROL) {
            ui.quit();
        }
        if ui.key(' ') {
            state.paused = !state.paused;
        }
        if ui.key_code(KeyCode::Left) {
            state.nudge(-MANUAL_STEP);
        }
        if ui.key_code(KeyCode::Right) {
            state.nudge(MANUAL_STEP);
        }
        if !state.paused {
            state.advance();
        }
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
    /// `true` when the auto-advance is paused (manually nudged or Space-toggled).
    pub paused: bool,
}

impl DemoState {
    /// Construct with the spinner at frame 0 and the bar at the start.
    pub fn new() -> Self {
        Self {
            spinner: SpinnerState::dots(),
            ratio: 0.0,
            step: RATIO_STEP,
            paused: false,
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

    /// Manually shift the ratio by `delta`, clamped to `[0.0, 1.0]`. Pauses
    /// the auto-advance so subsequent frames don't immediately overwrite the
    /// user's intent.
    pub fn nudge(&mut self, delta: f64) {
        self.ratio = (self.ratio + delta).clamp(0.0, 1.0);
        self.paused = true;
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
        .grow(1)
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
            ui.text(format!(
                "ratio = {:.0}%    {}",
                state.ratio * 100.0,
                if state.paused {
                    "(paused)"
                } else {
                    "(running)"
                },
            ))
            .dim();
            if pr.hovered {
                ui.text("  Progress hovered — click would trigger a scrubber")
                    .fg(Color::Yellow);
            }

            ui.text("");
            ui.text("Static variants (different ratios, different widths):")
                .fg(Color::Cyan);

            let _ = ui.row(|ui| {
                ui.text("  0%  ");
                let _ = ui.progress(0.0);
            });
            let _ = ui.row(|ui| {
                ui.text(" 25%  ");
                let _ = ui.progress(0.25);
            });
            let _ = ui.row(|ui| {
                ui.text(" 50%  ");
                let _ = ui.progress(0.50);
            });
            let _ = ui.row(|ui| {
                ui.text(" 75%  ");
                let _ = ui.progress(0.75);
            });
            let _ = ui.row(|ui| {
                ui.text("100%  ");
                let _ = ui.progress(1.0);
            });

            ui.text("Space pauses; ←/→ nudges 5%. q / Esc / Ctrl-Q quits.")
                .dim();
        });
}
