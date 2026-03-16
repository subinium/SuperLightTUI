use slt::{Bar, BarDirection, BarGroup, Border, Color, Context, LegendPosition, Marker};

fn main() -> std::io::Result<()> {
    let cpu_data: Vec<(f64, f64)> = vec![
        (0.0, 32.0),
        (1.0, 45.0),
        (2.0, 38.0),
        (3.0, 52.0),
        (4.0, 61.0),
        (5.0, 55.0),
        (6.0, 68.0),
        (7.0, 72.0),
        (8.0, 65.0),
        (9.0, 78.0),
        (10.0, 85.0),
        (11.0, 80.0),
    ];
    let mem_data: Vec<(f64, f64)> = vec![
        (0.0, 40.0),
        (2.0, 48.0),
        (4.0, 55.0),
        (6.0, 62.0),
        (8.0, 70.0),
        (10.0, 75.0),
    ];
    let profit_data: Vec<(f64, f64)> = vec![
        (0.0, -15.0),
        (1.0, 8.0),
        (2.0, -3.0),
        (3.0, 25.0),
        (4.0, 18.0),
        (5.0, -10.0),
        (6.0, 30.0),
        (7.0, 42.0),
    ];
    let histogram_data = [
        2.1, 3.5, 4.2, 2.8, 5.1, 6.3, 4.8, 3.9, 5.5, 7.2, 6.1, 4.0, 3.2, 5.8, 6.7, 4.5, 3.1, 5.3,
        7.0, 6.5, 4.3, 2.9, 5.9, 6.8, 4.1, 3.7, 5.0, 6.4, 4.6, 3.4,
    ];
    let bars = vec![
        Bar::new("Rust", 72.0).color(Color::Cyan),
        Bar::new("Go", 58.0).color(Color::Blue),
        Bar::new("Python", 45.0).color(Color::Yellow),
        Bar::new("Java", 38.0).color(Color::Red),
        Bar::new("C++", 52.0).color(Color::Green),
    ];
    let groups = vec![
        BarGroup::new(
            "2023",
            vec![
                Bar::new("Rev", 100.0).color(Color::Cyan),
                Bar::new("Cost", 60.0).color(Color::Red),
                Bar::new("Profit", 40.0).color(Color::Green),
            ],
        ),
        BarGroup::new(
            "2024",
            vec![
                Bar::new("Rev", 140.0).color(Color::Cyan),
                Bar::new("Cost", 80.0).color(Color::Red),
                Bar::new("Profit", 60.0).color(Color::Green),
            ],
        ),
    ];
    let spark_data = [
        12.0, 18.0, 16.0, 21.0, 19.0, 25.0, 28.0, 26.0, 31.0, 34.0, 30.0, 37.0,
    ];
    let colored_spark: Vec<(f64, Option<Color>)> = vec![
        (12.0, Some(Color::Green)),
        (9.0, Some(Color::Red)),
        (14.0, Some(Color::Green)),
        (f64::NAN, None),
        (18.0, Some(Color::Cyan)),
        (22.0, Some(Color::Yellow)),
        (17.0, Some(Color::Red)),
        (24.0, Some(Color::Green)),
        (26.0, Some(Color::Cyan)),
        (f64::NAN, None),
        (23.0, Some(Color::Yellow)),
        (29.0, Some(Color::Green)),
    ];

    slt::run(|ui: &mut Context| {
        if ui.key_mod('q', slt::KeyModifiers::CONTROL) || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }

        ui.bordered(Border::Rounded)
            .title("Data Visualization")
            .pad(1)
            .grow(1)
            .col(|ui| {
                ui.text("SuperLightTUI Infoviz").bold().fg(Color::Cyan);
                ui.separator();

                ui.container().grow(1).row(|ui| {
                    ui.bordered(Border::Single)
                        .title("Multi-Series")
                        .grow(1)
                        .col(|ui| {
                            ui.chart(
                                |c| {
                                    c.xlabel("Time (s)");
                                    c.ylabel("(%)");
                                    c.line(&cpu_data).label("CPU").color(Color::Cyan);
                                    c.scatter(&mem_data)
                                        .label("Mem")
                                        .color(Color::Yellow)
                                        .marker(Marker::Dot);
                                    c.legend(LegendPosition::TopRight);
                                    c.grid(true);
                                },
                                38,
                                10,
                            );
                        });

                    ui.bordered(Border::Single)
                        .title("Profit/Loss")
                        .grow(1)
                        .col(|ui| {
                            ui.chart(
                                |c| {
                                    c.xlabel("Q");
                                    c.ylabel("$K");
                                    c.line(&profit_data).label("P&L").color(Color::Green);
                                    c.grid(true);
                                },
                                30,
                                10,
                            );
                        });
                });

                ui.container().grow(1).row(|ui| {
                    ui.bordered(Border::Single)
                        .title("Histogram")
                        .grow(1)
                        .col(|ui| {
                            ui.histogram_with(
                                &histogram_data,
                                |h| {
                                    h.bins(6).color(Color::Magenta);
                                },
                                28,
                                8,
                            );
                        });

                    ui.bordered(Border::Single)
                        .title("Bar Chart")
                        .grow(1)
                        .col(|ui| {
                            ui.bar_chart_styled(&bars, 16, BarDirection::Horizontal);
                        });

                    ui.bordered(Border::Single)
                        .title("Sparklines")
                        .grow(1)
                        .col(|ui| {
                            ui.text("Trend").dim();
                            ui.sparkline(&spark_data, 20);
                            ui.text("Styled").dim();
                            ui.sparkline_styled(&colored_spark, 20);
                        });
                });

                ui.container().grow(1).row(|ui| {
                    ui.bordered(Border::Single)
                        .title("Grouped Bars")
                        .grow(1)
                        .col(|ui| {
                            ui.bar_chart_grouped(&groups, 14);
                        });

                    ui.bordered(Border::Single)
                        .title("Canvas")
                        .grow(1)
                        .col(|ui| {
                            ui.canvas(36, 6, |cv| {
                                cv.set_color(Color::Indexed(236));
                                cv.filled_rect(0, 0, cv.width(), cv.height());

                                cv.layer();
                                cv.set_color(Color::Cyan);
                                cv.filled_circle(14, 12, 10);
                                cv.set_color(Color::Yellow);
                                cv.filled_circle(36, 12, 8);
                                cv.set_color(Color::Green);
                                cv.filled_triangle(56, 2, 46, 22, 66, 22);
                                cv.set_color(Color::Magenta);
                                cv.circle(14, 12, 10);

                                cv.layer();
                                cv.set_color(Color::White);
                                cv.print(2, 2, "SLT");
                            });
                        });
                });

                ui.separator();
                ui.help(&[("Ctrl+Q", "quit")]);
            });
    })
}
