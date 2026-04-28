//! v0.20.0 #235 — scrollable_with_gutter + highlight_next/previous.
//!
//! Demo: grep-style log viewer. Type to filter; n/N navigates between
//! matching lines (search-result style). The gutter renders 1-based line
//! numbers; matching lines are bolded and the current match has an accent
//! background.

use slt::{Border, Color, Context, HighlightRange, KeyCode, ScrollState};

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

fn main() -> std::io::Result<()> {
    let mut state = ScrollState::new();
    let mut query = String::from("ERROR");

    // Initial highlight set.
    refresh_highlights(&mut state, &query);

    slt::run_with(slt::RunConfig::default().mouse(true), |ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.key('n') {
            state.highlight_next();
        }
        if ui.key('N') {
            state.highlight_previous();
        }
        if ui.key('1') {
            query = "ERROR".into();
            refresh_highlights(&mut state, &query);
        }
        if ui.key('2') {
            query = "WARN".into();
            refresh_highlights(&mut state, &query);
        }
        if ui.key('3') {
            query = "INFO".into();
            refresh_highlights(&mut state, &query);
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("v0.20.0 #235 — scrollable_with_gutter")
            .p(1)
            .gap(1)
            .grow(1)
            .col(|ui| {
                ui.text(format!(
                    "Filter: {:?}    n=next  N=prev    1=ERROR 2=WARN 3=INFO    Ctrl+Q quits",
                    query
                ))
                .fg(Color::Cyan);

                let r = ui.scrollable_with_gutter(
                    &mut state,
                    SAMPLE_LOG.len(),
                    12,
                    |idx| format!("{:>3}", idx + 1),
                    |ui, abs| {
                        if let Some(line) = SAMPLE_LOG.get(abs) {
                            let style_color = if line.contains("ERROR") {
                                Color::Red
                            } else if line.contains("WARN") {
                                Color::Yellow
                            } else {
                                Color::Reset
                            };
                            ui.text(*line).fg(style_color);
                        }
                    },
                );
                if let Some(idx) = r.current_highlight {
                    ui.text(format!(
                        "Match {}/{}    (press n/N to navigate)",
                        idx + 1,
                        r.total_highlights
                    ))
                    .dim();
                } else {
                    ui.text("No matches").dim();
                }
            });
    })
}

fn refresh_highlights(state: &mut ScrollState, query: &str) {
    let mut hits: Vec<HighlightRange> = Vec::new();
    if !query.is_empty() {
        for (i, line) in SAMPLE_LOG.iter().enumerate() {
            if line.contains(query) {
                hits.push(HighlightRange::line(i));
            }
        }
    }
    state.set_highlights(&hits);
}
