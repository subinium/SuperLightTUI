use slt::{Border, Color, Context};

fn main() -> std::io::Result<()> {
    let bar_data = [("Q1", 32.0), ("Q2", 46.0), ("Q3", 28.0), ("Q4", 54.0)];
    let spark_data = [
        12.0, 18.0, 16.0, 21.0, 19.0, 25.0, 28.0, 26.0, 31.0, 34.0, 30.0, 37.0,
    ];
    let line_data = [
        8.0, 12.0, 10.0, 16.0, 14.0, 22.0, 19.0, 25.0, 21.0, 27.0, 24.0, 31.0,
    ];

    slt::run(|ui: &mut Context| {
        if ui.key('q') {
            ui.quit();
        }

        ui.bordered(Border::Rounded)
            .title("Infoviz Demo")
            .pad(1)
            .grow(1)
            .col(|ui| {
                ui.text("Data Visualization").bold().fg(Color::Cyan);
                ui.separator();

                ui.container().grow(1).row(|ui| {
                    ui.bordered(Border::Single)
                        .title("BarChart")
                        .pad(1)
                        .grow(1)
                        .col(|ui| {
                            ui.bar_chart(&bar_data, 20);
                        });

                    ui.bordered(Border::Single)
                        .title("Sparkline")
                        .pad(1)
                        .grow(1)
                        .col(|ui| {
                            ui.sparkline(&spark_data, 28);
                            ui.text("latest trend").dim();
                        });
                });

                ui.container().grow(1).row(|ui| {
                    ui.bordered(Border::Single)
                        .title("LineChart")
                        .pad(1)
                        .grow(1)
                        .col(|ui| {
                            ui.line_chart(&line_data, 28, 8);
                        });

                    ui.bordered(Border::Single)
                        .title("Canvas")
                        .pad(1)
                        .grow(1)
                        .col(|ui| {
                            ui.canvas(28, 8, |cv| {
                                let w = cv.width();
                                let h = cv.height();
                                cv.circle(w / 2, h / 2, h.min(w) / 4);
                                cv.line(0, 0, w.saturating_sub(1), h.saturating_sub(1));
                                cv.line(w.saturating_sub(1), 0, 0, h.saturating_sub(1));
                            });
                        });
                });

                ui.separator();
                ui.help(&[("q", "quit")]);
            });
    })
}
