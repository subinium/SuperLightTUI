use slt::{
    Border, Color, Context, ScrollState, SpinnerState, Style, TableState, Theme, ToastState,
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
    let mut proc_table = TableState::new(
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
    );
    let mut dark_mode = true;
    let mut toasts = ToastState::new();

    let logs = vec![
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
    ];

    slt::run_with(
        slt::RunConfig {
            mouse: true,
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
                ui.quit();
            }
            if ui.key_mod('t', slt::KeyModifiers::CONTROL) {
                dark_mode = !dark_mode;
            }
            ui.set_theme(if dark_mode {
                Theme::dark()
            } else {
                Theme::light()
            });

            let tick = ui.tick();
            let metrics = sim_metrics(tick);

            ui.bordered(Border::Rounded)
                .title("System Dashboard")
                .pad(1)
                .grow(1)
                .col(|ui| {
                    ui.row(|ui| {
                        ui.spinner(&spinner);
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
                    ui.separator();

                    // metric cards
                    ui.row(|ui| {
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

                    ui.row(|ui| {
                        stat_pill(
                            ui,
                            "Requests",
                            &format!("{}", metrics.requests),
                            Color::Cyan,
                        );
                        stat_pill(
                            ui,
                            "Errors",
                            &format!("{}", metrics.errors),
                            if metrics.errors > 5 {
                                Color::Red
                            } else {
                                Color::Green
                            },
                        );
                        stat_pill(ui, "P99", "45ms", Color::Yellow);
                        stat_pill(ui, "Threads", "24", Color::Blue);
                    });

                    ui.container().grow(1).row(|ui| {
                        // process table
                        ui.bordered(Border::Rounded)
                            .title("Processes")
                            .pad(1)
                            .grow(1)
                            .col(|ui| {
                                ui.table(&mut proc_table);
                                ui.separator();
                                ui.row(|ui| {
                                    if ui.button("Kill") {
                                        let row = proc_table.selected;
                                        if let Some(name) =
                                            proc_table.rows.get(row).and_then(|r| r.get(1))
                                        {
                                            toasts.warning(format!("Killed: {name}"), tick);
                                        }
                                    }
                                    if ui.button("Restart") {
                                        let row = proc_table.selected;
                                        if let Some(name) =
                                            proc_table.rows.get(row).and_then(|r| r.get(1))
                                        {
                                            toasts.success(format!("Restarted: {name}"), tick);
                                        }
                                    }
                                });
                            });

                        // log stream
                        ui.bordered(Border::Rounded)
                            .title("Logs")
                            .pad(1)
                            .grow(1)
                            .col(|ui| {
                                ui.scrollable(&mut log_scroll).grow(1).col(|ui| {
                                    for &(time, level, msg) in &logs {
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

                    ui.toast(&mut toasts);

                    ui.separator();
                    ui.help(&[
                        ("q", "quit"),
                        ("t", "theme"),
                        ("Tab", "focus"),
                        ("j/k", "select"),
                    ]);
                });
        },
    )
}

fn metric_card(ui: &mut Context, label: &str, value: f64, unit: &str, color: Color) {
    let resp = ui.bordered(Border::Single).pad(1).grow(1).col(|ui| {
        ui.text(label).dim();
        ui.text(format!("{value:.1}{unit}")).bold().fg(color);
        let bar_w = 10;
        let filled = ((value / 100.0).clamp(0.0, 1.0) * bar_w as f64) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_w - filled);
        ui.text(bar).fg(color);
        if value > 80.0 {
            ui.text("⚠ HIGH").bold().fg(Color::Red);
        }
    });
    let _ = resp;
}

fn stat_pill(ui: &mut Context, label: &str, value: &str, color: Color) {
    ui.bordered(Border::Rounded).pad(1).grow(1).col(|ui| {
        ui.text(label).dim();
        ui.text(value).bold().fg(color);
    });
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
