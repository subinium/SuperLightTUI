use slt::{
    Bar, BarDirection, BarGroup, Border, Candle, Color, Context, LegendPosition, Marker, TabsState,
};

fn main() -> std::io::Result<()> {
    // --- Shared data ---
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
    let area_data: Vec<(f64, f64)> = [
        10.0, 15.0, 12.0, 22.0, 18.0, 28.0, 25.0, 35.0, 30.0, 40.0, 38.0, 45.0, 42.0, 50.0, 48.0,
        55.0, 52.0, 58.0, 55.0, 60.0,
    ]
    .iter()
    .enumerate()
    .map(|(i, v)| (i as f64, *v))
    .collect();
    let direction_data: Vec<(f64, f64)> = vec![
        (0.0, 20.0),
        (1.0, 35.0),
        (2.0, 28.0),
        (3.0, 45.0),
        (4.0, 40.0),
        (5.0, 55.0),
        (6.0, 48.0),
        (7.0, 62.0),
        (8.0, 58.0),
        (9.0, 70.0),
        (10.0, 65.0),
        (11.0, 75.0),
    ];

    // Smooth sine/cosine for Lines tab
    let sine_data: Vec<(f64, f64)> = (0..60)
        .map(|i| {
            let x = i as f64 * 0.2;
            (x, (x * 0.8).sin() * 30.0 + 50.0)
        })
        .collect();
    let cosine_data: Vec<(f64, f64)> = (0..60)
        .map(|i| {
            let x = i as f64 * 0.2;
            (x, (x * 0.8).cos() * 25.0 + 45.0)
        })
        .collect();
    let scatter_points: Vec<(f64, f64)> = (0..40)
        .map(|i| {
            let x = i as f64;
            let noise = ((i * 17 + 3) % 11) as f64 - 5.0;
            (x, x * 1.5 + noise + 10.0)
        })
        .collect();

    // Bars tab data
    let bars = vec![
        Bar::new("Rust", 72.0).color(Color::Cyan).text_value("72%"),
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
    let histogram_data = [
        2.1, 3.5, 4.2, 2.8, 5.1, 6.3, 4.8, 3.9, 5.5, 7.2, 6.1, 4.0, 3.2, 5.8, 6.7, 4.5, 3.1, 5.3,
        7.0, 6.5, 4.3, 2.9, 5.9, 6.8,
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

    // Advanced tab data
    let candles = vec![
        Candle {
            open: 100.0,
            high: 108.0,
            low: 98.0,
            close: 105.0,
        },
        Candle {
            open: 105.0,
            high: 112.0,
            low: 103.0,
            close: 110.0,
        },
        Candle {
            open: 110.0,
            high: 115.0,
            low: 106.0,
            close: 107.0,
        },
        Candle {
            open: 107.0,
            high: 111.0,
            low: 101.0,
            close: 103.0,
        },
        Candle {
            open: 103.0,
            high: 109.0,
            low: 100.0,
            close: 108.0,
        },
        Candle {
            open: 108.0,
            high: 118.0,
            low: 107.0,
            close: 116.0,
        },
        Candle {
            open: 116.0,
            high: 120.0,
            low: 112.0,
            close: 113.0,
        },
        Candle {
            open: 113.0,
            high: 117.0,
            low: 110.0,
            close: 115.0,
        },
        Candle {
            open: 115.0,
            high: 122.0,
            low: 113.0,
            close: 120.0,
        },
        Candle {
            open: 120.0,
            high: 125.0,
            low: 118.0,
            close: 119.0,
        },
        Candle {
            open: 119.0,
            high: 123.0,
            low: 115.0,
            close: 121.0,
        },
        Candle {
            open: 121.0,
            high: 128.0,
            low: 119.0,
            close: 126.0,
        },
    ];

    let mut tabs = TabsState::new(vec!["Overview", "Lines", "Bars", "Advanced"]);

    slt::run(|ui: &mut Context| {
        if ui.key('q') || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }

        let tw = ui.width() as u32;
        let th = ui.height() as u32;
        let grid_dim = slt::Style::new().fg(Color::Indexed(237));

        ui.bordered(Border::Rounded)
            .title("SLT Infoviz")
            .grow(1)
            .col(|ui| {
                ui.tabs(&mut tabs);

                match tabs.selected {
                    // ── Tab 0: Overview ──────────────────────────────────
                    0 => {
                        let cols4 = tw.saturating_sub(10) / 4;
                        let avail = th.saturating_sub(4);
                        let r1h = avail * 2 / 3;
                        let r23h = avail.saturating_sub(r1h);
                        let ch1 = r1h.saturating_sub(2).max(4);
                        let ch23 = r23h.saturating_sub(2).max(4);

                        ui.container().grow(2).row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Multi-Series")
                                .grow(1)
                                .col(|ui| {
                                    ui.chart(
                                        |c| {
                                            c.line(&cpu_data).label("CPU").color(Color::Cyan);
                                            c.scatter(&mem_data)
                                                .label("Mem")
                                                .color(Color::Yellow)
                                                .marker(Marker::Dot);
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols4,
                                        ch1,
                                    );
                                });
                            ui.bordered(Border::Single).title("P&L").grow(1).col(|ui| {
                                ui.chart(
                                    |c| {
                                        c.line(&profit_data).label("P&L").color(Color::Green);
                                        c.axhline(0.0, slt::Style::new().fg(Color::Red).dim());
                                        c.grid(true);
                                        c.grid_style(grid_dim);
                                    },
                                    cols4,
                                    ch1,
                                );
                            });
                            ui.bordered(Border::Single).title("Area").grow(1).col(|ui| {
                                ui.chart(
                                    |c| {
                                        c.area(&area_data).label("Growth").color(Color::Cyan);
                                        c.grid(true);
                                        c.grid_style(grid_dim);
                                    },
                                    cols4,
                                    ch1,
                                );
                            });
                            ui.bordered(Border::Single)
                                .title("Direction")
                                .grow(1)
                                .col(|ui| {
                                    ui.chart(
                                        |c| {
                                            c.line(&direction_data)
                                                .label("Price")
                                                .color_by_direction(
                                                    Color::Rgb(38, 166, 91),
                                                    Color::Rgb(234, 57, 67),
                                                );
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols4,
                                        ch1,
                                    );
                                });
                        });
                        ui.container().grow(1).row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Bar Chart")
                                .grow(1)
                                .col(|ui| {
                                    ui.bar_chart_styled(
                                        &bars,
                                        cols4.saturating_sub(14),
                                        BarDirection::Horizontal,
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Candlestick")
                                .grow(1)
                                .col(|ui| {
                                    ui.candlestick(
                                        &candles,
                                        cols4,
                                        ch23,
                                        Color::Rgb(38, 166, 91),
                                        Color::Rgb(234, 57, 67),
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Heatmap")
                                .grow(1)
                                .col(|ui| {
                                    let heat: Vec<Vec<f64>> = (0..12)
                                        .map(|r| {
                                            (0..cols4 as usize)
                                                .map(|c| ((r * 3 + c * 7) % 20) as f64)
                                                .collect()
                                        })
                                        .collect();
                                    ui.heatmap(
                                        &heat,
                                        cols4,
                                        ch23,
                                        Color::Rgb(20, 20, 60),
                                        Color::Rgb(255, 100, 50),
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Sparklines")
                                .grow(1)
                                .col(|ui| {
                                    ui.text("Trend").dim();
                                    ui.sparkline(&spark_data, cols4);
                                    ui.text("Styled").dim();
                                    ui.sparkline_styled(&colored_spark, cols4);
                                });
                        });
                    }

                    // ── Tab 1: Lines & Areas ─────────────────────────────
                    1 => {
                        let cols2 = tw.saturating_sub(8) / 2;
                        let cols3 = tw.saturating_sub(10) / 3;
                        let half = th.saturating_sub(4) / 2;
                        let ch_tall = half.saturating_sub(2).max(4);

                        ui.container().grow(1).row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Sine + Cosine (60 pts)")
                                .grow(1)
                                .col(|ui| {
                                    ui.chart(
                                        |c| {
                                            c.line(&sine_data).label("sin").color(Color::Cyan);
                                            c.line(&cosine_data).label("cos").color(Color::Yellow);
                                            c.xlabel("x");
                                            c.ylabel("y");
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols2,
                                        ch_tall,
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("P&L + Reference Lines")
                                .grow(1)
                                .col(|ui| {
                                    ui.chart(
                                        |c| {
                                            c.line(&profit_data).label("P&L").color(Color::Green);
                                            c.axhline(0.0, slt::Style::new().fg(Color::Red).dim());
                                            c.axhline(
                                                20.0,
                                                slt::Style::new().fg(Color::Cyan).dim(),
                                            );
                                            c.axvline(
                                                3.5,
                                                slt::Style::new().fg(Color::Yellow).dim(),
                                            );
                                            c.xlabel("Quarter");
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols2,
                                        ch_tall,
                                    );
                                });
                        });
                        ui.container().grow(1).row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Area Fill")
                                .grow(1)
                                .col(|ui| {
                                    ui.chart(
                                        |c| {
                                            c.area(&area_data).label("Growth").color(Color::Cyan);
                                            c.xlabel("Week");
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols3,
                                        ch_tall,
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Direction Coloring")
                                .grow(1)
                                .col(|ui| {
                                    ui.chart(
                                        |c| {
                                            c.line(&direction_data)
                                                .label("Price")
                                                .color_by_direction(
                                                    Color::Rgb(38, 166, 91),
                                                    Color::Rgb(234, 57, 67),
                                                );
                                            c.xlabel("Day");
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols3,
                                        ch_tall,
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Scatter + Trend")
                                .grow(1)
                                .col(|ui| {
                                    ui.chart(
                                        |c| {
                                            c.scatter(&scatter_points)
                                                .label("Data")
                                                .color(Color::Yellow)
                                                .marker(Marker::Dot);
                                            c.line(&[(0.0, 10.0), (39.0, 68.5)])
                                                .label("Trend")
                                                .color(Color::Cyan);
                                            c.xlabel("x");
                                            c.ylabel("y");
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols3,
                                        ch_tall,
                                    );
                                });
                        });
                    }

                    // ── Tab 2: Bars & Distribution ───────────────────────
                    2 => {
                        let cols3 = tw.saturating_sub(10) / 3;
                        let half = th.saturating_sub(4) / 2;
                        let ch_tall = half.saturating_sub(2).max(4);

                        ui.container().grow(1).row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Horizontal Bars")
                                .grow(1)
                                .col(|ui| {
                                    ui.bar_chart_with(
                                        &bars,
                                        |config| {
                                            config.direction(BarDirection::Horizontal);
                                        },
                                        ch_tall,
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Grouped Bars")
                                .grow(1)
                                .col(|ui| {
                                    ui.bar_chart_grouped_with(
                                        &groups,
                                        |config| {
                                            config.group_gap(2);
                                        },
                                        ch_tall,
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Vertical Bars")
                                .grow(1)
                                .col(|ui| {
                                    ui.bar_chart_with(
                                        &bars,
                                        |config| {
                                            config.direction(BarDirection::Vertical).bar_width(3);
                                        },
                                        ch_tall,
                                    );
                                });
                        });
                        ui.container().grow(1).row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Histogram (6 bins)")
                                .grow(1)
                                .col(|ui| {
                                    ui.histogram_with(
                                        &histogram_data,
                                        |h| {
                                            h.bins(6).color(Color::Magenta);
                                        },
                                        cols3,
                                        ch_tall,
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Histogram (10 bins)")
                                .grow(1)
                                .col(|ui| {
                                    ui.histogram_with(
                                        &histogram_data,
                                        |h| {
                                            h.bins(10).color(Color::Cyan);
                                        },
                                        cols3,
                                        ch_tall,
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Sparklines")
                                .grow(1)
                                .col(|ui| {
                                    ui.text("Trend (plain)").dim();
                                    ui.sparkline(&spark_data, cols3);
                                    ui.text("").dim();
                                    ui.text("Per-point colors").dim();
                                    ui.sparkline_styled(&colored_spark, cols3);
                                    ui.text("").dim();
                                    ui.text("Reversed").dim();
                                    let rev: Vec<f64> = spark_data.iter().rev().copied().collect();
                                    ui.sparkline(&rev, cols3);
                                });
                        });
                    }

                    // ── Tab 3: Advanced ───────────────────────────────────
                    _ => {
                        let cols2 = tw.saturating_sub(8) / 2;
                        let half = th.saturating_sub(4) / 2;
                        let ch_tall = half.saturating_sub(2).max(4);

                        ui.container().grow(1).row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Candlestick (12 candles)")
                                .grow(1)
                                .col(|ui| {
                                    ui.candlestick(
                                        &candles,
                                        cols2,
                                        ch_tall,
                                        Color::Rgb(38, 166, 91),
                                        Color::Rgb(234, 57, 67),
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Heatmap")
                                .grow(1)
                                .col(|ui| {
                                    let heat: Vec<Vec<f64>> = (0..20)
                                        .map(|r| {
                                            (0..cols2 as usize)
                                                .map(|c| {
                                                    let dx = c as f64 - cols2 as f64 / 2.0;
                                                    let dy = r as f64 - 10.0;
                                                    100.0 - (dx * dx + dy * dy).sqrt() * 3.0
                                                })
                                                .collect()
                                        })
                                        .collect();
                                    ui.heatmap(
                                        &heat,
                                        cols2,
                                        ch_tall,
                                        Color::Rgb(10, 10, 40),
                                        Color::Rgb(255, 80, 30),
                                    );
                                });
                        });
                        ui.container().grow(1).row(|ui| {
                            ui.bordered(Border::Single)
                                .title("Custom Ticks + Ref Lines")
                                .grow(1)
                                .col(|ui| {
                                    ui.chart(
                                        |c| {
                                            c.area(&cpu_data).color(Color::Cyan);
                                            c.line(&cpu_data).color(Color::White);
                                            c.xtick_labels(
                                                &[0.0, 3.0, 6.0, 9.0, 11.0],
                                                &["Jan", "Apr", "Jul", "Oct", "Dec"],
                                            );
                                            c.yticks(&[0.0, 25.0, 50.0, 75.0, 100.0]);
                                            c.axhline(
                                                50.0,
                                                slt::Style::new().fg(Color::Yellow).dim(),
                                            );
                                            c.axhline(75.0, slt::Style::new().fg(Color::Red).dim());
                                            c.xlabel("Month");
                                            c.grid_style(grid_dim);
                                            c.legend(LegendPosition::None);
                                        },
                                        cols2,
                                        ch_tall,
                                    );
                                });
                            ui.bordered(Border::Single)
                                .title("Canvas Drawing")
                                .grow(1)
                                .col(|ui| {
                                    ui.canvas(cols2, ch_tall, |cv| {
                                        cv.set_color(Color::Indexed(236));
                                        cv.filled_rect(0, 0, cv.width(), cv.height());
                                        cv.layer();
                                        cv.set_color(Color::Cyan);
                                        cv.filled_circle(
                                            cv.width() / 4,
                                            cv.height() / 2,
                                            cv.height() / 3,
                                        );
                                        cv.set_color(Color::Yellow);
                                        cv.filled_circle(
                                            cv.width() / 2,
                                            cv.height() / 2,
                                            cv.height() / 4,
                                        );
                                        cv.set_color(Color::Green);
                                        let cx = cv.width() * 3 / 4;
                                        let cy = cv.height() / 2;
                                        let r = cv.height() / 3;
                                        cv.filled_triangle(
                                            cx,
                                            cy - r,
                                            cx - r,
                                            cy + r,
                                            cx + r,
                                            cy + r,
                                        );
                                        cv.layer();
                                        cv.set_color(Color::White);
                                        cv.print(2, 2, "SLT Canvas");
                                    });
                                });
                        });
                    }
                }

                ui.help(&[("q", "quit"), ("←/→", "tab"), ("Esc", "quit")]);
            });
    })
}
