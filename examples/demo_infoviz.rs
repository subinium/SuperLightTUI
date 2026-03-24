use slt::{
    Bar, BarDirection, BarGroup, Border, Candle, Color, Context, LegendPosition, Marker, TabsState,
    TreemapItem,
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

    // Candlestick data
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

    // Treemap data
    let treemap_items = vec![
        TreemapItem::new("Rust", 40.0, Color::Rgb(0, 150, 180)),
        TreemapItem::new("Go", 25.0, Color::Rgb(0, 120, 210)),
        TreemapItem::new("Python", 20.0, Color::Rgb(255, 200, 50)),
        TreemapItem::new("Java", 15.0, Color::Rgb(200, 60, 60)),
        TreemapItem::new("C++", 12.0, Color::Rgb(80, 160, 80)),
        TreemapItem::new("TypeScript", 10.0, Color::Rgb(48, 120, 214)),
        TreemapItem::new("Swift", 8.0, Color::Rgb(240, 80, 50)),
        TreemapItem::new("Kotlin", 6.0, Color::Rgb(150, 80, 200)),
        TreemapItem::new("Zig", 5.0, Color::Rgb(230, 160, 40)),
        TreemapItem::new("Lua", 3.0, Color::Rgb(60, 60, 160)),
    ];

    // Stacked bar data
    let stacked_groups = vec![
        BarGroup::new(
            "Q1",
            vec![
                Bar::new("Product", 45.0).color(Color::Cyan),
                Bar::new("Service", 30.0).color(Color::Yellow),
                Bar::new("License", 15.0).color(Color::Green),
            ],
        ),
        BarGroup::new(
            "Q2",
            vec![
                Bar::new("Product", 52.0).color(Color::Cyan),
                Bar::new("Service", 35.0).color(Color::Yellow),
                Bar::new("License", 20.0).color(Color::Green),
            ],
        ),
        BarGroup::new(
            "Q3",
            vec![
                Bar::new("Product", 48.0).color(Color::Cyan),
                Bar::new("Service", 40.0).color(Color::Yellow),
                Bar::new("License", 25.0).color(Color::Green),
            ],
        ),
        BarGroup::new(
            "Q4",
            vec![
                Bar::new("Product", 60.0).color(Color::Cyan),
                Bar::new("Service", 38.0).color(Color::Yellow),
                Bar::new("License", 28.0).color(Color::Green),
            ],
        ),
    ];

    let mut tabs = TabsState::new(vec![
        "Overview",
        "Lines",
        "Scatter",
        "Bars",
        "Heatmap",
        "Financial",
        "Treemap",
        "Canvas",
    ]);

    slt::run(|ui: &mut Context| {
        if ui.key('q') || ui.key_code(slt::KeyCode::Esc) {
            ui.quit();
        }

        let tw = ui.width() as u32;
        let th = ui.height() as u32;
        let grid_dim = slt::Style::new().fg(Color::Indexed(237));

        let _ = ui
            .bordered(Border::Rounded)
            .title("SLT Infoviz")
            .grow(1)
            .col(|ui| {
                let _ = ui.tabs(&mut tabs);

                match tabs.selected {
                    // ── Tab 0: Overview ──────────────────────────────────
                    0 => {
                        let cols4 = tw.saturating_sub(10) / 4;
                        let avail = th.saturating_sub(4);
                        let r1h = avail * 2 / 3;
                        let r23h = avail.saturating_sub(r1h);
                        let ch1 = r1h.saturating_sub(2).max(4);
                        let ch23 = r23h.saturating_sub(2).max(4);

                        let _ = ui.container().grow(2).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Multi-Series")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.chart(
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
                            let _ = ui.bordered(Border::Single).title("P&L").grow(1).col(|ui| {
                                let _ = ui.chart(
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
                            let _ = ui.bordered(Border::Single).title("Area").grow(1).col(|ui| {
                                let _ = ui.chart(
                                    |c| {
                                        c.area(&area_data).label("Growth").color(Color::Cyan);
                                        c.grid(true);
                                        c.grid_style(grid_dim);
                                    },
                                    cols4,
                                    ch1,
                                );
                            });
                            let _ =
                                ui.bordered(Border::Single)
                                    .title("Direction")
                                    .grow(1)
                                    .col(|ui| {
                                        let _ = ui.chart(
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
                        let _ =
                            ui.container().grow(1).row(|ui| {
                                let _ = ui.bordered(Border::Single).title("Bar Chart").grow(1).col(
                                    |ui| {
                                        let _ = ui.bar_chart_with(
                                            &bars,
                                            |c| {
                                                c.direction(BarDirection::Horizontal);
                                            },
                                            cols4.saturating_sub(14),
                                        );
                                    },
                                );
                                let _ = ui
                                    .bordered(Border::Single)
                                    .title("Candlestick HD")
                                    .grow(1)
                                    .col(|ui| {
                                        let _ = ui.candlestick_hd(
                                            &candles,
                                            Color::Rgb(38, 166, 91),
                                            Color::Rgb(234, 57, 67),
                                        );
                                    });
                                let _ =
                                    ui.bordered(Border::Single).title("Heatmap HD").grow(1).col(
                                        |ui| {
                                            let heat: Vec<Vec<f64>> = (0..ch23 as usize * 2)
                                                .map(|r| {
                                                    (0..cols4 as usize)
                                                        .map(|c| ((r * 3 + c * 7) % 20) as f64)
                                                        .collect()
                                                })
                                                .collect();
                                            let _ = ui.heatmap_halfblock(
                                                &heat,
                                                cols4,
                                                ch23,
                                                Color::Rgb(20, 20, 60),
                                                Color::Rgb(255, 100, 50),
                                            );
                                        },
                                    );
                                let _ = ui.bordered(Border::Single).title("Treemap").grow(1).col(
                                    |ui| {
                                        let _ = ui.treemap(&treemap_items);
                                    },
                                );
                            });
                    }

                    // ── Tab 1: Lines & Areas ─────────────────────────────
                    1 => {
                        let cols2 = tw.saturating_sub(8) / 2;
                        let cols3 = tw.saturating_sub(10) / 3;
                        let half = th.saturating_sub(4) / 2;
                        let ch_tall = half.saturating_sub(2).max(4);

                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Sine + Cosine (60 pts)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.chart(
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
                            let _ = ui
                                .bordered(Border::Single)
                                .title("P&L + Reference Lines")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.chart(
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
                        let _ =
                            ui.container().grow(1).row(|ui| {
                                let _ = ui.bordered(Border::Single).title("Area Fill").grow(1).col(
                                    |ui| {
                                        let _ = ui.chart(
                                            |c| {
                                                c.area(&area_data)
                                                    .label("Growth")
                                                    .color(Color::Cyan);
                                                c.xlabel("Week");
                                                c.grid(true);
                                                c.grid_style(grid_dim);
                                            },
                                            cols3,
                                            ch_tall,
                                        );
                                    },
                                );
                                let _ = ui
                                    .bordered(Border::Single)
                                    .title("Direction Coloring")
                                    .grow(1)
                                    .col(|ui| {
                                        let _ = ui.chart(
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
                                let _ = ui
                                    .bordered(Border::Single)
                                    .title("Custom Ticks")
                                    .grow(1)
                                    .col(|ui| {
                                        let _ = ui.chart(
                                            |c| {
                                                c.area(&cpu_data).color(Color::Cyan);
                                                c.line(&cpu_data).color(Color::White);
                                                c.xtick_labels(
                                                    &[0.0, 3.0, 6.0, 9.0, 11.0],
                                                    &["Jan", "Apr", "Jul", "Oct", "Dec"],
                                                );
                                                c.yticks(&[0.0, 25.0, 50.0, 75.0, 100.0]);
                                                c.xlabel("Month");
                                                c.grid_style(grid_dim);
                                                c.legend(LegendPosition::None);
                                            },
                                            cols3,
                                            ch_tall,
                                        );
                                    });
                            });
                    }

                    // ── Tab 2: Scatter ────────────────────────────────────
                    2 => {
                        let cols2 = tw.saturating_sub(8) / 2;
                        let cols3 = tw.saturating_sub(10) / 3;
                        let half = th.saturating_sub(4) / 2;
                        let ch_tall = half.saturating_sub(2).max(4);

                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Scatter + Trend (Braille)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.chart(
                                        |c| {
                                            c.scatter(&scatter_points)
                                                .label("Data")
                                                .color(Color::Yellow)
                                                .marker(Marker::Braille);
                                            c.line(&[(0.0, 10.0), (39.0, 68.5)])
                                                .label("Trend")
                                                .color(Color::Cyan);
                                            c.xlabel("x");
                                            c.ylabel("y");
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols2,
                                        ch_tall,
                                    );
                                });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Scatter (Dot marker)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.chart(
                                        |c| {
                                            c.scatter(&scatter_points)
                                                .label("Data")
                                                .color(Color::Magenta)
                                                .marker(Marker::Dot);
                                            c.xlabel("x");
                                            c.ylabel("y");
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols2,
                                        ch_tall,
                                    );
                                });
                        });
                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Scatter (Cross)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.chart(
                                        |c| {
                                            c.scatter(&scatter_points)
                                                .label("Data")
                                                .color(Color::Green)
                                                .marker(Marker::Cross);
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols3,
                                        ch_tall,
                                    );
                                });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Scatter (Circle)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.chart(
                                        |c| {
                                            c.scatter(&scatter_points)
                                                .label("Data")
                                                .color(Color::Red)
                                                .marker(Marker::Circle);
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols3,
                                        ch_tall,
                                    );
                                });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Multi-Series Scatter")
                                .grow(1)
                                .col(|ui| {
                                    let scatter2: Vec<(f64, f64)> = (0..40)
                                        .map(|i| {
                                            let x = i as f64;
                                            let noise = ((i * 13 + 7) % 9) as f64 - 4.0;
                                            (x, x * 0.8 + noise + 25.0)
                                        })
                                        .collect();
                                    let _ = ui.chart(
                                        |c| {
                                            c.scatter(&scatter_points)
                                                .label("Series A")
                                                .color(Color::Cyan)
                                                .marker(Marker::Braille);
                                            c.scatter(&scatter2)
                                                .label("Series B")
                                                .color(Color::Yellow)
                                                .marker(Marker::Dot);
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols3,
                                        ch_tall,
                                    );
                                });
                        });
                    }

                    // ── Tab 3: Bars & Distribution ───────────────────────
                    3 => {
                        let half = th.saturating_sub(4) / 2;
                        let ch_tall = half.saturating_sub(2).max(4);

                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Horizontal (gap=0)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.bar_chart_with(
                                        &bars,
                                        |config| {
                                            config.direction(BarDirection::Horizontal);
                                        },
                                        ch_tall,
                                    );
                                });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Horizontal (gap=1)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.bar_chart_with(
                                        &bars,
                                        |config| {
                                            config.direction(BarDirection::Horizontal).bar_gap(1);
                                        },
                                        ch_tall,
                                    );
                                });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Vertical (w=1)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.bar_chart_with(
                                        &bars,
                                        |config| {
                                            config.direction(BarDirection::Vertical).bar_width(1);
                                        },
                                        ch_tall,
                                    );
                                });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Vertical (w=3)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.bar_chart_with(
                                        &bars,
                                        |config| {
                                            config.direction(BarDirection::Vertical).bar_width(3);
                                        },
                                        ch_tall,
                                    );
                                });
                        });
                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Vertical (w=5)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.bar_chart_with(
                                        &bars,
                                        |config| {
                                            config.direction(BarDirection::Vertical).bar_width(5);
                                        },
                                        ch_tall,
                                    );
                                });
                            let _ =
                                ui.bordered(Border::Single)
                                    .title("Grouped")
                                    .grow(1)
                                    .col(|ui| {
                                        let _ = ui.bar_chart_grouped_with(
                                            &groups,
                                            |config| {
                                                config.group_gap(2);
                                            },
                                            ch_tall,
                                        );
                                    });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Stacked (w=5)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.bar_chart_stacked_with(
                                        &stacked_groups,
                                        |c| {
                                            c.bar_width(5).bar_gap(2);
                                        },
                                        ch_tall,
                                    );
                                });
                        });
                    }

                    // ── Tab 4: Heatmap ───────────────────────────────────
                    4 => {
                        let cols2 = tw.saturating_sub(8) / 2;
                        let half = th.saturating_sub(4) / 2;
                        let ch_tall = half.saturating_sub(2).max(4);

                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Standard Heatmap")
                                .grow(1)
                                .col(|ui| {
                                    let heat: Vec<Vec<f64>> = (0..ch_tall as usize)
                                        .map(|r| {
                                            (0..cols2 as usize)
                                                .map(|c| {
                                                    let dx = c as f64 - cols2 as f64 / 2.0;
                                                    let dy = r as f64 - ch_tall as f64 / 2.0;
                                                    100.0 - (dx * dx + dy * dy).sqrt() * 3.0
                                                })
                                                .collect()
                                        })
                                        .collect();
                                    let _ = ui.heatmap(
                                        &heat,
                                        cols2,
                                        ch_tall,
                                        Color::Rgb(10, 10, 40),
                                        Color::Rgb(255, 80, 30),
                                    );
                                });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Half-Block HD (2x res)")
                                .grow(1)
                                .col(|ui| {
                                    // Same data but with doubled virtual rows
                                    let heat: Vec<Vec<f64>> = (0..ch_tall as usize * 2)
                                        .map(|r| {
                                            (0..cols2 as usize)
                                                .map(|c| {
                                                    let dx = c as f64 - cols2 as f64 / 2.0;
                                                    let dy = r as f64 - ch_tall as f64;
                                                    100.0 - (dx * dx + dy * dy).sqrt() * 2.0
                                                })
                                                .collect()
                                        })
                                        .collect();
                                    let _ = ui.heatmap_halfblock(
                                        &heat,
                                        cols2,
                                        ch_tall,
                                        Color::Rgb(10, 10, 40),
                                        Color::Rgb(255, 80, 30),
                                    );
                                });
                        });
                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Cool Gradient")
                                .grow(1)
                                .col(|ui| {
                                    let heat: Vec<Vec<f64>> = (0..ch_tall as usize * 2)
                                        .map(|r| {
                                            (0..cols2 as usize)
                                                .map(|c| {
                                                    ((r as f64 * 0.3).sin()
                                                        + (c as f64 * 0.2).cos())
                                                        * 50.0
                                                        + 50.0
                                                })
                                                .collect()
                                        })
                                        .collect();
                                    let _ = ui.heatmap_halfblock(
                                        &heat,
                                        cols2,
                                        ch_tall,
                                        Color::Rgb(20, 0, 80),
                                        Color::Rgb(0, 255, 200),
                                    );
                                });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Warm Gradient")
                                .grow(1)
                                .col(|ui| {
                                    let heat: Vec<Vec<f64>> = (0..ch_tall as usize * 2)
                                        .map(|r| {
                                            (0..cols2 as usize)
                                                .map(|c| {
                                                    let cx = cols2 as f64 / 2.0;
                                                    let cy = ch_tall as f64;
                                                    let dx = c as f64 - cx;
                                                    let dy = r as f64 - cy;
                                                    let dist = (dx * dx + dy * dy).sqrt();
                                                    ((dist * 0.15).sin().abs()) * 100.0
                                                })
                                                .collect()
                                        })
                                        .collect();
                                    let _ = ui.heatmap_halfblock(
                                        &heat,
                                        cols2,
                                        ch_tall,
                                        Color::Rgb(40, 0, 0),
                                        Color::Rgb(255, 220, 100),
                                    );
                                });
                        });
                    }

                    // ── Tab 5: Financial ──────────────────────────────────
                    5 => {
                        let cols2 = tw.saturating_sub(8) / 2;
                        let half = th.saturating_sub(4) / 2;
                        let ch_tall = half.saturating_sub(2).max(4);

                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Candlestick (Standard)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.candlestick(
                                        &candles,
                                        Color::Rgb(38, 166, 91),
                                        Color::Rgb(234, 57, 67),
                                    );
                                });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Candlestick HD (Heavy + Halfblock)")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.candlestick_hd(
                                        &candles,
                                        Color::Rgb(38, 166, 91),
                                        Color::Rgb(234, 57, 67),
                                    );
                                });
                        });
                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Direction Line")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.chart(
                                        |c| {
                                            c.line(&direction_data)
                                                .label("Price")
                                                .color_by_direction(
                                                    Color::Rgb(38, 166, 91),
                                                    Color::Rgb(234, 57, 67),
                                                );
                                            c.axhline(
                                                50.0,
                                                slt::Style::new().fg(Color::Yellow).dim(),
                                            );
                                            c.grid(true);
                                            c.grid_style(grid_dim);
                                        },
                                        cols2,
                                        ch_tall,
                                    );
                                });
                            let _ =
                                ui.bordered(Border::Single)
                                    .title("Sparklines")
                                    .grow(1)
                                    .col(|ui| {
                                        ui.text("Market Trend").dim();
                                        let _ = ui.sparkline(&spark_data, cols2);
                                        ui.text("").dim();
                                        ui.text("Per-asset Colors").dim();
                                        let _ = ui.sparkline_styled(&colored_spark, cols2);
                                        ui.text("").dim();
                                        ui.text("Inverted").dim();
                                        let rev: Vec<f64> =
                                            spark_data.iter().rev().copied().collect();
                                        let _ = ui.sparkline(&rev, cols2);
                                    });
                        });
                    }

                    // ── Tab 6: Treemap ────────────────────────────────────
                    6 => {
                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Language Popularity")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.treemap(&treemap_items);
                                });
                            let _ =
                                ui.bordered(Border::Single)
                                    .title("Market Cap")
                                    .grow(1)
                                    .col(|ui| {
                                        let market = vec![
                                            TreemapItem::new(
                                                "AAPL",
                                                350.0,
                                                Color::Rgb(80, 180, 80),
                                            ),
                                            TreemapItem::new(
                                                "MSFT",
                                                310.0,
                                                Color::Rgb(60, 160, 60),
                                            ),
                                            TreemapItem::new(
                                                "NVDA",
                                                280.0,
                                                Color::Rgb(40, 200, 40),
                                            ),
                                            TreemapItem::new(
                                                "GOOG",
                                                200.0,
                                                Color::Rgb(200, 60, 60),
                                            ),
                                            TreemapItem::new(
                                                "AMZN",
                                                190.0,
                                                Color::Rgb(180, 40, 40),
                                            ),
                                            TreemapItem::new(
                                                "META",
                                                140.0,
                                                Color::Rgb(60, 100, 200),
                                            ),
                                            TreemapItem::new(
                                                "TSLA",
                                                100.0,
                                                Color::Rgb(200, 200, 60),
                                            ),
                                            TreemapItem::new("BRK", 80.0, Color::Rgb(160, 80, 160)),
                                        ];
                                        let _ = ui.treemap(&market);
                                    });
                        });

                        let _ = ui
                            .bordered(Border::Single)
                            .title("Disk Usage")
                            .grow(1)
                            .col(|ui| {
                                let disk = vec![
                                    TreemapItem::new(
                                        "node_modules",
                                        800.0,
                                        Color::Rgb(200, 50, 50),
                                    ),
                                    TreemapItem::new("target", 450.0, Color::Rgb(180, 100, 40)),
                                    TreemapItem::new("src", 120.0, Color::Rgb(50, 150, 200)),
                                    TreemapItem::new("docs", 60.0, Color::Rgb(100, 180, 100)),
                                    TreemapItem::new("tests", 40.0, Color::Rgb(140, 120, 200)),
                                    TreemapItem::new(".git", 200.0, Color::Rgb(150, 150, 60)),
                                    TreemapItem::new("assets", 90.0, Color::Rgb(60, 130, 160)),
                                    TreemapItem::new("dist", 180.0, Color::Rgb(170, 70, 120)),
                                    TreemapItem::new("config", 25.0, Color::Rgb(120, 160, 80)),
                                    TreemapItem::new("scripts", 15.0, Color::Rgb(200, 180, 100)),
                                ];
                                let _ = ui.treemap(&disk);
                            });
                    }

                    // ── Tab 7: Canvas ─────────────────────────────────────
                    _ => {
                        let cols2 = tw.saturating_sub(8) / 2;
                        let half = th.saturating_sub(4) / 2;
                        let ch_tall = half.saturating_sub(2).max(4);

                        let _ = ui.container().grow(1).row(|ui| {
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Shapes")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.canvas(cols2, ch_tall, |cv| {
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
                                            cy.saturating_sub(r),
                                            cx.saturating_sub(r),
                                            cy + r,
                                            cx + r,
                                            cy + r,
                                        );
                                        cv.layer();
                                        cv.set_color(Color::White);
                                        cv.print(2, 2, "SLT Canvas");
                                    });
                                });
                            let _ = ui
                                .bordered(Border::Single)
                                .title("Lines & Circles")
                                .grow(1)
                                .col(|ui| {
                                    let _ = ui.canvas(cols2, ch_tall, |cv| {
                                        cv.set_color(Color::Indexed(236));
                                        cv.filled_rect(0, 0, cv.width(), cv.height());
                                        cv.layer();
                                        // Draw radiating lines
                                        let cx = cv.width() / 2;
                                        let cy = cv.height() / 2;
                                        let colors = [
                                            Color::Red,
                                            Color::Yellow,
                                            Color::Green,
                                            Color::Cyan,
                                            Color::Blue,
                                            Color::Magenta,
                                        ];
                                        for (i, &color) in colors.iter().enumerate() {
                                            let angle = i as f64 * std::f64::consts::PI * 2.0 / 6.0;
                                            let ex = (cx as f64
                                                + angle.cos() * cv.height() as f64 * 0.4)
                                                as usize;
                                            let ey = (cy as f64
                                                + angle.sin() * cv.height() as f64 * 0.4)
                                                as usize;
                                            cv.set_color(color);
                                            cv.line(cx, cy, ex, ey);
                                        }
                                        cv.layer();
                                        cv.set_color(Color::White);
                                        cv.circle(cx, cy, cv.height() / 4);
                                        cv.circle(cx, cy, cv.height() / 6);
                                    });
                                });
                        });
                        let _ = ui
                            .bordered(Border::Single)
                            .title("Braille Drawing")
                            .grow(1)
                            .col(|ui| {
                                let cw = tw.saturating_sub(6);
                                let _ = ui.canvas(cw, ch_tall, |cv| {
                                    cv.set_color(Color::Cyan);
                                    // Draw a wave pattern
                                    let w = cv.width();
                                    let h = cv.height();
                                    for x in 0..w {
                                        let t = x as f64 / w as f64 * 4.0 * std::f64::consts::PI;
                                        let y1 =
                                            (h as f64 / 2.0 + t.sin() * h as f64 * 0.3) as usize;
                                        let y2 = (h as f64 / 2.0 + (t * 1.5).cos() * h as f64 * 0.2)
                                            as usize;
                                        if x > 0 {
                                            let prev_t = (x - 1) as f64 / w as f64
                                                * 4.0
                                                * std::f64::consts::PI;
                                            let py1 = (h as f64 / 2.0
                                                + prev_t.sin() * h as f64 * 0.3)
                                                as usize;
                                            let py2 = (h as f64 / 2.0
                                                + (prev_t * 1.5).cos() * h as f64 * 0.2)
                                                as usize;
                                            cv.set_color(Color::Cyan);
                                            cv.line(x - 1, py1, x, y1);
                                            cv.set_color(Color::Yellow);
                                            cv.line(x - 1, py2, x, y2);
                                        }
                                    }
                                });
                            });
                    }
                }

                let _ = ui.help(&[("q", "quit"), ("\u{2190}/\u{2192}", "tab"), ("Esc", "quit")]);
            });
    })
}
