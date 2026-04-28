//! Cookbook: simulated real-time dashboard with line chart and sparklines.
//!
//! Archetype: **Standard** (full-canvas, no overlay, no scrollback).
//!
//! Demonstrates:
//! - storing a rolling history (`VecDeque<f64>` capped at 60 points)
//! - `ui.chart(..)` with a line dataset
//! - two `ui.sparkline` rows for secondary metrics
//! - stat tiles with `ui.stat` / `ui.stat_colored`
//! - `q` to quit
//!
//! §2 (Demo Guide): exposes `pub fn render(ui, &mut DemoState)` so a
//! composing demo (e.g. `cookbook_tour.rs`) can embed it without losing
//! the rolling histories every frame. The standalone `main()` is a thin
//! wrapper that owns the state.

use std::collections::VecDeque;

use slt::{Border, Color, Context, KeyCode, KeyModifiers};

const MAX_POINTS: usize = 60;

struct Metrics {
    cpu: f64,
    mem: f64,
    req_per_s: f64,
}

fn tick_metrics(t: u64) -> Metrics {
    let f = t as f64 * 0.1;
    Metrics {
        cpu: 45.0 + 30.0 * (f * 0.35).sin() + 8.0 * (f * 1.1).cos(),
        mem: 55.0 + 15.0 * (f * 0.22).sin(),
        req_per_s: 120.0 + 60.0 * (f * 0.45).sin() + 20.0 * (f * 1.3).cos(),
    }
}

fn push(buf: &mut VecDeque<f64>, v: f64) {
    if buf.len() == MAX_POINTS {
        buf.pop_front();
    }
    buf.push_back(v);
}

/// Persistent rolling histories. Bundled into a struct so a composing
/// demo can hold it across frames — keeping these as `main`-local locals
/// breaks the moment the tour re-enters this demo on every tab switch.
pub struct DemoState {
    cpu_hist: VecDeque<f64>,
    mem_hist: VecDeque<f64>,
    req_hist: VecDeque<f64>,
}

impl Default for DemoState {
    fn default() -> Self {
        // Pre-fill the rolling histories with the same deterministic
        // mock data the per-frame `tick_metrics` produces for ticks
        // 0..MAX_POINTS. Without this prefill the very first frame
        // renders an empty chart and three flat sparklines that look
        // broken to a first-time viewer; the real metrics only catch
        // up after MAX_POINTS frames of `push`. The standalone
        // `main()` and any composing tour inherit the populated
        // histories on cold start, then `render` continues with the
        // existing per-frame `push` (which `pop_front`s once full),
        // so behaviour after frame 0 is identical to the unprefilled
        // version.
        let mut s = Self {
            cpu_hist: VecDeque::with_capacity(MAX_POINTS),
            mem_hist: VecDeque::with_capacity(MAX_POINTS),
            req_hist: VecDeque::with_capacity(MAX_POINTS),
        };
        for t in 0..(MAX_POINTS as u64) {
            let m = tick_metrics(t);
            s.cpu_hist.push_back(m.cpu);
            s.mem_hist.push_back(m.mem);
            s.req_hist.push_back(m.req_per_s);
        }
        s
    }
}

/// Render one frame of the dashboard. Caller owns the rolling
/// histories so they survive across frames (and across tab switches in
/// a composing demo).
pub fn render(ui: &mut Context, state: &mut DemoState) {
    let tick = ui.tick();
    let m = tick_metrics(tick);
    push(&mut state.cpu_hist, m.cpu);
    push(&mut state.mem_hist, m.mem);
    push(&mut state.req_hist, m.req_per_s);

    let chart_w = ui.width().saturating_sub(6).max(20);
    let chart_h = ui.height().saturating_sub(18).max(8);
    let cpu_points: Vec<(f64, f64)> = state
        .cpu_hist
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64, *v))
        .collect();
    let mem_slice: Vec<f64> = state.mem_hist.iter().copied().collect();
    let req_slice: Vec<f64> = state.req_hist.iter().copied().collect();

    let _ = ui
        .bordered(Border::Rounded)
        .title("Cookbook: Dashboard")
        .p(1)
        .gap(1)
        .grow(1)
        .col(|ui| {
            let _ = ui.row_gap(2, |ui| {
                let _ = ui.bordered(Border::Single).p(1).grow(1).col(|ui| {
                    let _ = ui.stat_colored("CPU", &format!("{:.1}%", m.cpu), Color::Cyan);
                });
                let _ = ui.bordered(Border::Single).p(1).grow(1).col(|ui| {
                    let _ = ui.stat_colored("Memory", &format!("{:.1}%", m.mem), Color::Yellow);
                });
                let _ = ui.bordered(Border::Single).p(1).grow(1).col(|ui| {
                    let _ = ui.stat_colored("Req/s", &format!("{:.0}", m.req_per_s), Color::Green);
                });
            });

            ui.text("CPU history (last 60 ticks)").dim();
            let _ = ui.chart(
                |c| {
                    let _ = c.line(&cpu_points).color(Color::Cyan).label("cpu");
                    c.grid(true);
                },
                chart_w,
                chart_h,
            );

            let spark_w = ui.width().saturating_sub(14).max(20);
            let _ = ui.row_gap(2, |ui| {
                ui.text("Memory").dim();
                let _ = ui.sparkline(&mem_slice, spark_w);
            });
            let _ = ui.row_gap(2, |ui| {
                ui.text("Req/s ").dim();
                let _ = ui.sparkline(&req_slice, spark_w);
            });

            ui.text("q or Ctrl+Q to quit.").dim();
        });
}

fn main() -> std::io::Result<()> {
    let mut state = DemoState::default();
    slt::run(move |ui: &mut Context| {
        if ui.key('q') || ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        render(ui, &mut state);
    })
}
