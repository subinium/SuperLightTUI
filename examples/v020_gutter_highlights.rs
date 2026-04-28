//! v0.20.0 scrollable_with_gutter demo — grep-style log viewer with search-
//! result highlights.
//!
//! Demonstrates: #235 (scrollable_with_gutter via `GutterOpts`, `HighlightRange`,
//! `ScrollState::highlight_next` / `highlight_previous`).
//!
//! Run: `cargo run --example v020_gutter_highlights`
//!
//! Keys:
//!   n            — jump to the next matching line
//!   N            — jump to the previous matching line
//!   1            — filter by ERROR
//!   2            — filter by WARN
//!   3            — filter by INFO
//!   Ctrl-Q / Esc — quit
//!
//! Layout:
//!   ┌── filter status / match counter ────────────┐
//!   │                                              │
//!   │  1 │ INFO  app starting up                   │
//!   │  2 │ DEBUG loaded config from ./config.toml  │
//!   │  3 │ INFO  listening on :8080                │
//!   │ … │ …                                        │
//!   └──────────────────────────────────────────────┘

use slt::{
    Border, Color, Context, GutterOpts, HighlightRange, KeyCode, KeyModifiers, RunConfig,
    ScrollState,
};

const SAMPLE_LOG: &[&str] = &[
    "INFO  app starting up",
    "DEBUG loaded config from ./config.toml",
    "INFO  listening on :8080",
    "DEBUG accepted connection from 127.0.0.1",
    "WARN  rate limit nearing for /api/heavy",
    "ERROR upstream timeout after 30s",
    "DEBUG retry attempt 1 of 3",
    "DEBUG retry attempt 2 of 3",
    "ERROR upstream timeout after 30s",
    "INFO  switching to fallback service",
    "DEBUG cache hit for key=user.42",
    "INFO  request 200 OK in 12ms",
    "WARN  slow query detected (412ms)",
    "ERROR database connection lost",
    "INFO  reconnecting...",
    "INFO  reconnected to db1",
    "DEBUG cache miss for key=user.99",
    "INFO  request 200 OK in 8ms",
    "WARN  rate limit nearing for /api/light",
    "INFO  graceful shutdown begin",
    "INFO  graceful shutdown complete",
];

const VIEWPORT_HEIGHT: u32 = 12;

fn main() -> std::io::Result<()> {
    let mut state = DemoState::new();

    slt::run_with(RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.key('n') {
            state.scroll.highlight_next();
        }
        if ui.key('N') {
            state.scroll.highlight_previous();
        }
        if ui.key('1') {
            state.set_query("ERROR");
        }
        if ui.key('2') {
            state.set_query("WARN");
        }
        if ui.key('3') {
            state.set_query("INFO");
        }
        render(ui, &mut state);
    })
}

/// Demo state — scroll position, the active filter query, and the matching
/// highlights derived from it.
pub struct DemoState {
    /// Scroll position + active highlight ranges live here.
    pub scroll: ScrollState,
    /// Current filter substring shown in the status row.
    pub query: String,
}

impl DemoState {
    /// Construct with `ERROR` selected so the snapshot test sees a non-empty
    /// match set on first frame.
    pub fn new() -> Self {
        let mut s = Self {
            scroll: ScrollState::new(),
            query: String::new(),
        };
        s.set_query("ERROR");
        s
    }

    /// Replace the active filter and rebuild the highlight set.
    pub fn set_query(&mut self, query: &str) {
        self.query.clear();
        self.query.push_str(query);
        let hits: Vec<HighlightRange> = if query.is_empty() {
            Vec::new()
        } else {
            SAMPLE_LOG
                .iter()
                .enumerate()
                .filter_map(|(i, line)| line.contains(query).then_some(HighlightRange::line(i)))
                .collect()
        };
        self.scroll.set_highlights(&hits);
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
        .title("v0.20.0 #235 — scrollable_with_gutter")
        .p(sp.xs())
        .gap(sp.xs())
        .grow(1)
        .col(|ui| {
            ui.text(format!(
                "Filter: {:?}    n=next  N=prev    1=ERROR 2=WARN 3=INFO",
                state.query,
            ))
            .fg(Color::Cyan);

            let r = ui.scrollable_with_gutter(
                &mut state.scroll,
                GutterOpts::line_numbers(SAMPLE_LOG.len(), VIEWPORT_HEIGHT),
                |ui, abs| {
                    if let Some(line) = SAMPLE_LOG.get(abs) {
                        let fg = if line.contains("ERROR") {
                            Color::Red
                        } else if line.contains("WARN") {
                            Color::Yellow
                        } else {
                            Color::Reset
                        };
                        ui.text(*line).fg(fg);
                    }
                },
            );

            if let Some(idx) = r.current_highlight {
                ui.text(format!(
                    "Match {}/{}    (press n / N to navigate)",
                    idx + 1,
                    r.total_highlights
                ))
                .dim();
            } else {
                ui.text("No matches").dim();
            }
            ui.text("Ctrl-Q / Esc quits.").dim();
        });
}
