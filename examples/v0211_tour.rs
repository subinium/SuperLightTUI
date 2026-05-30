//! v0.21.1 Tour — a single-screen showcase of the additive APIs shipped in
//! 0.21.1: ergonomic `Color` (HSL / hex parse / hue rotation), multi-stop and
//! background text gradients, spinner presets, the reorderable list, intrinsic
//! `measure_text`, declarative `KeyMap` dispatch, programmatic focus traversal,
//! and the new `Response` interaction signals (`submitted` / `double_clicked` /
//! `scroll_delta`) with callback chaining.
//!
//! Run: `cargo run --example v0211_tour`
//!
//! Keys:
//!   Tab / Shift-Tab  — cycle focus
//!   n                — focus_next() (programmatic, same as Tab)
//!   Shift+Up/Down    — reorder the selected list item
//!   Enter            — submit the search box (reported via Response.submitted)
//!   r                — reset counters (routed through KeyMap::matched)
//!   q / Esc / Ctrl-Q — quit
//!
//! Mouse: click / double-click / scroll-wheel over the "target" button to see
//! `Response.clicked` / `.double_clicked` / `.scroll_delta` live.

use slt::widgets::{ListState, SpinnerState, TextInputState};
use slt::{Color, Context, KeyCode, KeyMap, KeyModifiers, RunConfig, SpinnerPreset};

const ITEMS: &[&str] = &[
    "Reorder me with Shift+Up/Down",
    "flexbox layout",
    "double-buffer diff",
    "tree-sitter syntax",
    "sixel + kitty images",
];

const SAMPLE: &str = "measure_text returns the (width, rows) a string occupies under the layout \
     engine's own wrap kernel — handy for sizing a tooltip or panel before you \
     draw it.";

/// Persistent across frames via `run_with`'s `move` closure.
pub struct TourState {
    query: TextInputState,
    last_submit: Option<String>,
    items: ListState,
    last_reorder: Option<(usize, usize)>,
    clicks: u32,
    double_clicks: u32,
    wheel: i64,
}

impl Default for TourState {
    fn default() -> Self {
        Self {
            query: TextInputState::with_placeholder("type then press Enter…"),
            last_submit: None,
            items: ListState::new(ITEMS.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
            last_reorder: None,
            clicks: 0,
            double_clicks: 0,
            wheel: 0,
        }
    }
}

/// The seven named spinner presets added in 0.21.1.
const PRESETS: &[(&str, SpinnerPreset)] = &[
    ("moon", SpinnerPreset::Moon),
    ("bounce", SpinnerPreset::Bounce),
    ("circle", SpinnerPreset::Circle),
    ("points", SpinnerPreset::Points),
    ("arc", SpinnerPreset::Arc),
    ("toggle", SpinnerPreset::Toggle),
    ("arrow", SpinnerPreset::Arrow),
];

pub fn render(ui: &mut Context, state: &mut TourState) {
    // ── Title: a multi-stop foreground gradient + a background-gradient banner.
    let _ = ui
        .text("SuperLightTUI · v0.21.1 tour")
        .bold()
        .gradient_stops(&[
            (0.0, Color::from_hsl(200.0, 0.9, 0.6)),
            (0.5, Color::from_hsl(280.0, 0.9, 0.65)),
            (1.0, Color::from_hsl(330.0, 0.9, 0.6)),
        ]);
    let _ = ui.text(" additive APIs, no breaking changes ").bg_gradient(
        Color::from_hsl(210.0, 0.7, 0.35),
        Color::from_hsl(330.0, 0.7, 0.35),
    );

    // ── Color ergonomics: HSL swatch row, a parsed hex, and rotate_hue.
    let _ = ui.container().gap(1).row(|ui| {
        ui.text("Color:").dim();
        for i in 0..12 {
            let hue = i as f32 / 12.0 * 360.0;
            ui.text("██").fg(Color::from_hsl(hue, 0.85, 0.6));
        }
    });
    let _ = ui.container().gap(1).row(|ui| {
        let parsed: Color = "#ff6b6b".parse().unwrap_or(Color::Reset);
        ui.text("\"#ff6b6b\".parse()").dim();
        ui.text("██").fg(parsed);
        ui.text("rotate_hue(180)").dim();
        ui.text("██").fg(parsed.rotate_hue(180.0));
    });

    // ── Spinner presets.
    let _ = ui.container().gap(2).row(|ui| {
        ui.text("Spinners:").dim();
        for (name, preset) in PRESETS {
            let _ = ui.container().gap(0).row(|ui| {
                let _ = ui.spinner(&SpinnerState::preset(*preset));
                ui.text(*name).dim();
            });
        }
    });

    // ── Reorderable list (Shift+Up/Down moves the selected item).
    let r = ui.list_reorderable(&mut state.items);
    if let Some(mv) = r.reordered {
        state.last_reorder = Some(mv);
    }
    if let Some((from, to)) = state.last_reorder {
        ui.text(format!("last reorder: {from} → {to}"))
            .fg(Color::Green);
    }

    // ── Search box: Enter reports Response.submitted (chained via on_submit).
    let submitted = ui.text_input(&mut state.query).submitted;
    if submitted {
        state.last_submit = Some(state.query.value.clone());
    }
    if let Some(q) = &state.last_submit {
        ui.text(format!("submitted: {q:?}")).fg(Color::Cyan);
    }

    // ── A mouse "target": clicked / double_clicked / scroll_delta in one Response.
    let target = ui.button("◎ click / double-click / scroll me");
    if target.clicked {
        state.clicks += 1;
    }
    if target.double_clicked {
        state.double_clicks += 1;
    }
    state.wheel += target.scroll_delta as i64;
    ui.text(format!(
        "clicks: {}   double: {}   wheel: {}",
        state.clicks, state.double_clicks, state.wheel
    ))
    .dim();

    // ── Intrinsic measurement.
    let (w, h) = ui.measure_text(SAMPLE, Some(46));
    ui.text(format!("measure_text(SAMPLE, 46) = {w}×{h} cells"))
        .dim();

    ui.text("Tab/n: focus · Shift+Up/Down: reorder · r: reset · q: quit")
        .dim();
}

fn main() -> std::io::Result<()> {
    let mut state = TourState::default();
    slt::run_with(
        RunConfig::default()
            .mouse(true)
            .tick_rate(std::time::Duration::from_millis(80)),
        move |ui: &mut Context| {
            if ui.key_mod('q', KeyModifiers::CONTROL) {
                ui.quit();
                return;
            }

            // Declarative KeyMap dispatch: 'r' resets the counters.
            let km = KeyMap::new()
                .bind('r', "reset counters")
                .bind('n', "focus next");
            if let Some(binding) = ui.keymap_match(&km) {
                match binding.key {
                    KeyCode::Char('r') => {
                        state.clicks = 0;
                        state.double_clicks = 0;
                        state.wheel = 0;
                        state.last_reorder = None;
                    }
                    KeyCode::Char('n') => ui.focus_next(),
                    _ => {}
                }
            }

            render(ui, &mut state);

            if ui.key_code(KeyCode::Esc) || ui.key('q') {
                ui.quit();
            }
        },
    )
}
