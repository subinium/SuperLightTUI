//! Demo: system dashboard (metric cards, processes, log stream, toasts).
//!
//! Archetype: **Standard** (full-canvas, no overlay, no scrollback).
//!
//! §2 (Demo Guide): exposes `pub fn render(ui, &mut DemoState)` so a
//! composing demo (e.g. `examples/showcase_tour.rs`) can preserve the
//! spinner phase, log scroll position, process-table cursor, theme
//! toggle, and toast queue across tab switches. The legacy stateless
//! `pub fn render(ui)` (snapshot-style) is retained for visual snapshot
//! tests in `tests/visual_snapshots.rs`.

use slt::{
    Border, Color, Context, ScrollState, SpinnerState, Style, TableState, Theme, ToastState, Trend,
};

struct Metrics {
    cpu: f64,
    mem: f64,
    disk: f64,
    net_in: f64,
    net_out: f64,
    uptime_secs: u64,
    requests: u64,
    errors: u64,
}

fn main() -> std::io::Result<()> {
    let spinner = SpinnerState::dots();
    let mut log_scroll = ScrollState::new();
    let mut proc_table = make_proc_table();
    let mut dark_mode = true;
    let mut toasts = ToastState::new();
    let logs = make_logs();

    slt::run_with(slt::RunConfig::default().mouse(true), |ui: &mut Context| {
        render_frame(
            ui,
            &spinner,
            &mut log_scroll,
            &mut proc_table,
            &mut dark_mode,
            &mut toasts,
            &logs,
        );
    })
}

/// Render one frame with fresh, default state — used by visual snapshot tests
/// in `tests/visual_snapshots.rs`. The runtime example uses [`render_frame`]
/// directly so widget state can persist across frames.
pub fn render(ui: &mut Context) {
    let spinner = SpinnerState::dots();
    let mut log_scroll = ScrollState::new();
    let mut proc_table = make_proc_table();
    let mut dark_mode = true;
    let mut toasts = ToastState::new();
    let logs = make_logs();
    render_frame(
        ui,
        &spinner,
        &mut log_scroll,
        &mut proc_table,
        &mut dark_mode,
        &mut toasts,
        &logs,
    );
}

/// Persistent dashboard state. Owns the spinner phase, log scroll
/// position, process-table cursor, theme toggle, and the toast queue
/// so they all survive across frames in the showcase tour.
pub struct DemoState {
    pub spinner: SpinnerState,
    pub log_scroll: ScrollState,
    pub proc_table: TableState,
    pub dark_mode: bool,
    pub toasts: ToastState,
    pub logs: Vec<(&'static str, &'static str, &'static str)>,
}

impl DemoState {
    pub fn new() -> Self {
        Self {
            spinner: SpinnerState::dots(),
            log_scroll: ScrollState::new(),
            proc_table: make_proc_table(),
            dark_mode: true,
            toasts: ToastState::new(),
            logs: make_logs(),
        }
    }
}

impl Default for DemoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render one frame of the dashboard with caller-owned state — used by
/// the showcase tour so the spinner phase, log scroll, table cursor,
/// theme toggle, and toast queue survive across frames. Snapshot tests
/// use [`render`] (which constructs fresh state each call).
pub fn render_with_state(ui: &mut Context, state: &mut DemoState) {
    render_frame(
        ui,
        &state.spinner,
        &mut state.log_scroll,
        &mut state.proc_table,
        &mut state.dark_mode,
        &mut state.toasts,
        &state.logs,
    );
}

/// Render one frame of the dashboard demo into the supplied context.
///
/// Exposed so both `main` (which keeps state across frames) and the
/// visual snapshot test (which builds fresh state and renders once) can
/// share the same rendering logic.
pub fn render_frame(
    ui: &mut Context,
    spinner: &SpinnerState,
    log_scroll: &mut ScrollState,
    proc_table: &mut TableState,
    dark_mode: &mut bool,
    toasts: &mut ToastState,
    logs: &[(&'static str, &'static str, &'static str)],
) {
    if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
        ui.quit();
    }
    if ui.key_mod('t', slt::KeyModifiers::CONTROL) {
        *dark_mode = !*dark_mode;
    }
    ui.set_theme(if *dark_mode {
        Theme::dark()
    } else {
        Theme::light()
    });

    let tick = ui.tick();
    let metrics = sim_metrics(tick);

    let _ = ui
        .bordered(Border::Rounded)
        .title("System Dashboard")
        .p(1)
        .grow(1)
        .col(|ui| {
            let _ = ui.row(|ui| {
                let _ = ui.spinner(spinner);
                ui.text(" LIVE").bold().fg(Color::Green);
                ui.spacer();
                ui.text(format!(
                    "Uptime: {}d {}h {}m",
                    metrics.uptime_secs / 86400,
                    (metrics.uptime_secs % 86400) / 3600,
                    (metrics.uptime_secs % 3600) / 60,
                ))
                .dim();
            });
            let _ = ui.divider_text("System Metrics");
            let _ = ui.row(|ui| {
                metric_card(ui, "CPU", metrics.cpu, "%", Color::Cyan);
                metric_card(ui, "Memory", metrics.mem, "%", Color::Yellow);
                metric_card(
                    ui,
                    "Disk",
                    metrics.disk,
                    "%",
                    if metrics.disk > 80.0 {
                        Color::Red
                    } else {
                        Color::Green
                    },
                );
                metric_card(ui, "Net In", metrics.net_in, "MB/s", Color::Blue);
                metric_card(ui, "Net Out", metrics.net_out, "MB/s", Color::Magenta);
            });

            let _ = ui.divider_text("Key Metrics");
            let _ = ui.row(|ui| {
                let _ = ui.bordered(Border::Rounded).p(1).grow(1).col(|ui| {
                    let _ = ui.stat_trend("Requests", &format!("{}", metrics.requests), Trend::Up);
                });
                let _ = ui.bordered(Border::Rounded).p(1).grow(1).col(|ui| {
                    let _ = ui.stat_colored(
                        "Errors",
                        &format!("{}", metrics.errors),
                        if metrics.errors > 5 {
                            Color::Red
                        } else {
                            Color::Green
                        },
                    );
                });
                let _ = ui.bordered(Border::Rounded).p(1).grow(1).col(|ui| {
                    let _ = ui.stat_colored("P99", "45ms", Color::Yellow);
                });
                let _ = ui.bordered(Border::Rounded).p(1).grow(1).col(|ui| {
                    let _ = ui.stat_colored("Threads", "24", Color::Blue);
                });
            });

            let _ = ui.container().grow(1).row(|ui| {
                // process table
                let _ = ui
                    .bordered(Border::Rounded)
                    .title("Processes")
                    .p(1)
                    .grow(1)
                    .col(|ui| {
                        let _ = ui.table(proc_table);
                        ui.separator();
                        let _ = ui.row(|ui| {
                            if ui.button("Kill").clicked {
                                let row = proc_table.selected;
                                if let Some(name) = proc_table.rows.get(row).and_then(|r| r.get(1))
                                {
                                    toasts.warning(format!("Killed: {name}"), tick);
                                }
                            }
                            if ui.button("Restart").clicked {
                                let row = proc_table.selected;
                                if let Some(name) = proc_table.rows.get(row).and_then(|r| r.get(1))
                                {
                                    toasts.success(format!("Restarted: {name}"), tick);
                                }
                            }
                        });
                    });

                // log stream
                let _ = ui
                    .bordered(Border::Rounded)
                    .title("Logs")
                    .p(1)
                    .grow(1)
                    .col(|ui| {
                        let _ = ui.scrollable(log_scroll).grow(1).col(|ui| {
                            for &(time, level, msg) in logs {
                                let color = match level {
                                    "ERROR" => Color::Red,
                                    "WARN" => Color::Yellow,
                                    _ => Color::Indexed(245),
                                };
                                ui.styled(
                                    format!("{time} [{level:5}] {msg}"),
                                    Style::new().fg(color),
                                );
                            }
                        });
                    });
            });

            ui.toast(toasts);

            let _ = ui.divider_text("Controls");
            let _ = ui.help(&[
                ("Ctrl+Q", "quit"),
                ("Ctrl+T", "theme"),
                ("Tab", "focus"),
                ("j/k", "select"),
            ]);
        });
}

fn make_proc_table() -> TableState {
    TableState::new(
        vec!["PID", "Name", "CPU%", "Mem%", "Status"],
        vec![
            vec!["1", "systemd", "0.1", "0.3", "running"],
            vec!["142", "nginx", "2.4", "1.2", "running"],
            vec!["389", "postgres", "8.7", "12.4", "running"],
            vec!["412", "redis", "1.1", "3.8", "running"],
            vec!["501", "node", "15.3", "8.2", "running"],
            vec!["623", "python3", "4.2", "6.1", "running"],
            vec!["789", "go-api", "3.8", "2.9", "running"],
            vec!["834", "cron", "0.0", "0.1", "sleeping"],
        ],
    )
}

fn make_logs() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("12:04:01", "INFO", "Request GET /api/users 200 (12ms)"),
        ("12:04:03", "INFO", "Request POST /api/auth 200 (45ms)"),
        ("12:04:05", "WARN", "High memory usage: 82.4%"),
        ("12:04:07", "INFO", "Request GET /api/items 200 (8ms)"),
        ("12:04:08", "ERROR", "Connection timeout: db-replica-2"),
        ("12:04:10", "INFO", "Request GET /health 200 (1ms)"),
        ("12:04:12", "INFO", "Cache hit ratio: 94.2%"),
        (
            "12:04:15",
            "WARN",
            "Slow query: SELECT * FROM orders (320ms)",
        ),
        ("12:04:18", "INFO", "Request DELETE /api/sessions 204 (3ms)"),
        ("12:04:20", "INFO", "SSL cert renewal: 23 days remaining"),
        ("12:04:22", "INFO", "Request GET /api/dashboard 200 (18ms)"),
        ("12:04:25", "ERROR", "Rate limit exceeded: 203.0.113.42"),
        ("12:04:28", "INFO", "Backup completed: 2.4GB (42s)"),
        ("12:04:30", "INFO", "Request PATCH /api/users/5 200 (22ms)"),
        ("12:04:33", "WARN", "Disk usage above 75% on /var/log"),
        ("12:04:35", "INFO", "Request GET /api/metrics 200 (5ms)"),
        ("12:04:38", "INFO", "New deployment: v2.14.3 rolling out"),
        (
            "12:04:40",
            "INFO",
            "Request GET /api/search?q=rust 200 (31ms)",
        ),
        ("12:04:42", "ERROR", "Failed to send email: SMTP timeout"),
        ("12:04:45", "INFO", "Worker process recycled (PID 501)"),
    ]
}

fn metric_card(ui: &mut Context, label: &str, value: f64, unit: &str, color: Color) {
    let resp = ui.bordered(Border::Single).p(1).grow(1).col(|ui| {
        ui.text(label).dim();
        ui.text(format!("{value:.1}{unit}")).bold().fg(color);
        let bar_w = 10;
        let filled = ((value / 100.0).clamp(0.0, 1.0) * bar_w as f64) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_w - filled);
        ui.text(bar).fg(color);
        if value > 80.0 {
            let _ = ui.badge_colored("HIGH", Color::Red);
        }
    });
    let _ = resp;
}

fn sim_metrics(tick: u64) -> Metrics {
    let t = tick as f64 * 0.1;
    Metrics {
        cpu: 35.0 + 25.0 * (t * 0.3).sin() + 10.0 * (t * 0.7).cos(),
        mem: 62.0 + 15.0 * (t * 0.2).sin(),
        disk: 73.0 + 5.0 * (t * 0.05).sin(),
        net_in: (12.0 + 8.0 * (t * 0.4).sin()).max(0.1),
        net_out: (4.0 + 3.0 * (t * 0.5).cos()).max(0.1),
        uptime_secs: 345_612 + tick,
        requests: 1_847_293 + tick * 3,
        errors: ((tick as f64 * 0.1).sin().abs() * 8.0) as u64,
    }
}
