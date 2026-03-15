use slt::{Border, Buffer, Color, Context, KeyCode, Rect, RunConfig, Style};
use std::time::Duration;

fn main() {
    let mut tick_offset: u64 = 0;

    let _ = slt::run_with(
        RunConfig {
            tick_rate: Duration::from_millis(16),
            max_fps: Some(60),
            ..RunConfig::default()
        },
        move |ui: &mut Context| {
            if ui.key('q') || ui.key_code(KeyCode::Esc) {
                ui.quit();
                return;
            }

            tick_offset = ui.tick();

            ui.bordered(Border::Rounded)
                .title("draw_raw demo")
                .pad(1)
                .gap(1)
                .col(|ui| {
                    ui.text("Direct buffer access via ContainerBuilder::draw()")
                        .bold();
                    ui.text("Press q to quit").dim();

                    ui.row(|ui| {
                        ui.bordered(Border::Single)
                            .title("Gradient")
                            .w(34)
                            .h(12)
                            .draw(|buf: &mut Buffer, rect: Rect| {
                                for y in rect.y..rect.bottom() {
                                    for x in rect.x..rect.right() {
                                        let r =
                                            ((x - rect.x) as f32 / rect.width as f32 * 255.0) as u8;
                                        let b = ((y - rect.y) as f32 / rect.height as f32 * 255.0)
                                            as u8;
                                        buf.set_char(
                                            x,
                                            y,
                                            '█',
                                            Style::new().fg(Color::Rgb(r, 80, b)),
                                        );
                                    }
                                }
                            });

                        ui.bordered(Border::Single)
                            .title("Plasma")
                            .w(34)
                            .h(12)
                            .draw(move |buf: &mut Buffer, rect: Rect| {
                                let t = tick_offset as f64 * 0.05;
                                for y in rect.y..rect.bottom() {
                                    for x in rect.x..rect.right() {
                                        let fx = (x - rect.x) as f64 * 0.15;
                                        let fy = (y - rect.y) as f64 * 0.3;
                                        let v = ((fx + t).sin()
                                            + (fy + t * 0.7).cos()
                                            + ((fx + fy + t * 0.5).sin()))
                                            / 3.0;
                                        let n = ((v + 1.0) * 0.5 * 255.0) as u8;
                                        let r = n;
                                        let g = 255 - n;
                                        let b = ((n as u16 + 128) % 256) as u8;
                                        buf.set_char(
                                            x,
                                            y,
                                            '▓',
                                            Style::new().fg(Color::Rgb(r, g, b)),
                                        );
                                    }
                                }
                            });

                        ui.bordered(Border::Single)
                            .title("Box Drawing")
                            .w(20)
                            .h(12)
                            .draw(|buf: &mut Buffer, rect: Rect| {
                                let chars = ['┌', '─', '┐', '│', ' ', '│', '└', '─', '┘'];
                                let w = rect.width.min(18);
                                let h = rect.height.min(10);
                                for dy in 0..h {
                                    for dx in 0..w {
                                        let ci = if dy == 0 {
                                            if dx == 0 {
                                                0
                                            } else if dx == w - 1 {
                                                2
                                            } else {
                                                1
                                            }
                                        } else if dy == h - 1 {
                                            if dx == 0 {
                                                6
                                            } else if dx == w - 1 {
                                                8
                                            } else {
                                                7
                                            }
                                        } else if dx == 0 {
                                            3
                                        } else if dx == w - 1 {
                                            5
                                        } else {
                                            4
                                        };
                                        buf.set_char(
                                            rect.x + dx,
                                            rect.y + dy,
                                            chars[ci],
                                            Style::new().fg(Color::Cyan),
                                        );
                                    }
                                }
                            });
                    });

                    ui.help(&[("q", "quit")]);
                });
        },
    );
}
