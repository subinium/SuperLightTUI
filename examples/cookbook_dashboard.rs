//! Cookbook: simulated real-time dashboard with line chart and sparklines.
//!
//! Demonstrates:
//! - storing a rolling history (`VecDeque<f64>` capped at 60 points)
//! - `ui.chart(..)` with a line dataset
//! - two `ui.sparkline` rows for secondary metrics
//! - stat tiles with `ui.stat` / `ui.stat_colored`
//! - `q` to quit

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

fn main() -> std::io::Result<()> {
    let mut cpu_hist: VecDeque<f64> = VecDeque::with_capacity(MAX_POINTS);
    let mut mem_hist: VecDeque<f64> = VecDeque::with_capacity(MAX_POINTS);
    let mut req_hist: VecDeque<f64> = VecDeque::with_capacity(MAX_POINTS);

    slt::run(|ui: &mut Context| {
        if ui.key('q') || ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        let tick = ui.tick();
        let m = tick_metrics(tick);
        push(&mut cpu_hist, m.cpu);
        push(&mut mem_hist, m.mem);
        push(&mut req_hist, m.req_per_s);

        let chart_w = ui.width().saturating_sub(6).max(20);
        let chart_h = ui.height().saturating_sub(18).max(8);
        let cpu_points: Vec<(f64, f64)> = cpu_hist
            .iter()
            .enumerate()
            .map(|(i, v)| (i as f64, *v))
            .collect();
        let mem_slice: Vec<f64> = mem_hist.iter().copied().collect();
        let req_slice: Vec<f64> = req_hist.iter().copied().collect();

        let _ = ui
            .bordered(Border::Rounded)
            .title("Cookbook — Dashboard")
            .pad(1)
            .gap(1)
            .grow(1)
            .col(|ui| {
                let _ = ui.row_gap(2, |ui| {
                    let _ = ui.bordered(Border::Single).pad(1).grow(1).col(|ui| {
                        let _ = ui.stat_colored("CPU", &format!("{:.1}%", m.cpu), Color::Cyan);
                    });
                    let _ = ui.bordered(Border::Single).pad(1).grow(1).col(|ui| {
                        let _ = ui.stat_colored("Memory", &format!("{:.1}%", m.mem), Color::Yellow);
                    });
                    let _ = ui.bordered(Border::Single).pad(1).grow(1).col(|ui| {
                        let _ =
                            ui.stat_colored("Req/s", &format!("{:.0}", m.req_per_s), Color::Green);
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
    })
}
