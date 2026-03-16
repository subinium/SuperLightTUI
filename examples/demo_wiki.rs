use slt::{Border, Color, Context, KeyCode, RunConfig, ScrollState, Style, TabsState, Theme};

struct MemberProfile {
    tab: &'static str,
    name: &'static str,
    born: &'static str,
    position: &'static str,
    nationality: &'static str,
    note: &'static str,
}

const MEMBERS: [MemberProfile; 4] = [
    MemberProfile {
        tab: "Jisoo",
        name: "Kim Jisoo (김지수)",
        born: "1995-01-03",
        position: "Lead Vocal",
        nationality: "South Korea",
        note: "Eldest member, actress in Snowdrop",
    },
    MemberProfile {
        tab: "Jennie",
        name: "Jennie Kim (제니)",
        born: "1996-01-16",
        position: "Main Rapper & Lead Vocal",
        nationality: "South Korea",
        note: "Solo debut 2018, 6 years trainee",
    },
    MemberProfile {
        tab: "Rosé",
        name: "Park Chaeyoung (로제/박채영)",
        born: "1997-02-11",
        position: "Main Vocal & Lead Dancer",
        nationality: "South Korea/New Zealand",
        note: "Born in NZ, raised in Australia",
    },
    MemberProfile {
        tab: "Lisa",
        name: "Lalisa Manobal (리사)",
        born: "1997-03-27",
        position: "Main Dancer & Lead Rapper",
        nationality: "Thailand",
        note: "Most followed K-pop idol on Instagram",
    },
];

fn main() -> std::io::Result<()> {
    let mut tabs = TabsState::new(vec![
        "Jisoo",
        "Jennie",
        "Rosé",
        "Lisa",
        "Group",
        "Discography",
    ]);
    let mut scroll = ScrollState::new();

    let member_images: Vec<Vec<u8>> = vec![
        gen_member_image(200, 300, [255, 105, 180], "JISOO"),
        gen_member_image(200, 300, [220, 20, 60], "JENNIE"),
        gen_member_image(200, 300, [255, 127, 80], "ROSE"),
        gen_member_image(200, 300, [148, 103, 189], "LISA"),
    ];

    slt::run_with(
        RunConfig {
            mouse: true,
            theme: Theme::dark(),
            ..Default::default()
        },
        move |ui: &mut Context| {
            let quit_q = ui.key('q');
            let quit_esc = ui.key_code(KeyCode::Esc);
            if quit_q || quit_esc {
                ui.quit();
            }

            ui.bordered(Border::Single).col(|ui| {
                ui.container().bg(Color::Rgb(28, 30, 34)).p(1).row(|ui| {
                    ui.text("BLACKPINK").bold().fg(Color::Rgb(255, 105, 180));
                    ui.text("  블랙핑크").fg(Color::White);
                    ui.spacer();
                    ui.text("나무위키 스타일").fg(Color::Rgb(126, 211, 33));
                });

                let tab_resp = ui.tabs(&mut tabs);
                if tab_resp.changed {
                    scroll.offset = 0;
                }

                ui.scrollable(&mut scroll)
                    .grow(1)
                    .col(|ui| match tabs.selected {
                        0..=3 => render_member(ui, tabs.selected, &member_images),
                        4 => render_group(ui),
                        _ => render_discography(ui),
                    });

                ui.help(&[("← →", "tab"), ("q", "quit"), ("↑ ↓", "scroll")]);
            });
        },
    )
}

fn render_member(ui: &mut Context, idx: usize, member_images: &[Vec<u8>]) {
    let p = &MEMBERS[idx];
    ui.bordered(Border::Single).p(1).row(|ui| {
        ui.bordered(Border::Single)
            .title(format!("{} Photo", p.tab))
            .w(34)
            .p(1)
            .col(|ui| {
                ui.kitty_image(&member_images[idx], 200, 300, 30, 18);
            });

        ui.bordered(Border::Single)
            .title("Profile Info")
            .grow(1)
            .p(1)
            .col(|ui| {
                info_row(ui, "Name", p.name);
                info_row(ui, "Born", p.born);
                info_row(ui, "Position", p.position);
                info_row(ui, "Nationality", p.nationality);
                info_row(ui, "Note", p.note);
            });
    });
}

fn render_group(ui: &mut Context) {
    ui.bordered(Border::Single).title("Group").p(1).col(|ui| {
        info_row(ui, "Debut", "2016-08-08");
        info_row(ui, "Agency", "YG Entertainment");
        info_row(ui, "Fandom", "BLINK");
        info_row(ui, "Note", "First K-pop girl group at Coachella");
    });
}

fn render_discography(ui: &mut Context) {
    ui.bordered(Border::Single)
        .title("Discography")
        .p(1)
        .col(|ui| {
            ui.text("SQUARE ONE (2016)");
            ui.text("SQUARE UP (2018)");
            ui.text("THE ALBUM (2020)");
            ui.text("BORN PINK (2022)");
        });
}

fn info_row(ui: &mut Context, key: &str, value: &str) {
    ui.line(|ui| {
        ui.styled(
            format!("{key}: "),
            Style::new().fg(Color::Indexed(250)).bold(),
        );
        ui.text(value);
    });
}

fn gen_member_image(w: u32, h: u32, base: [u8; 3], label: &str) -> Vec<u8> {
    let mut rgba = gen_gradient(w, h, base);
    draw_label_band(&mut rgba, w, h);
    draw_text_5x7(&mut rgba, w, h, label, Color::Rgb(245, 245, 245), 3);
    rgba
}

fn gen_gradient(w: u32, h: u32, base: [u8; 3]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let tx = x as f32 / w as f32;
            let ty = y as f32 / h as f32;
            let glow = (0.08 * tx).min(0.08);
            let r = (base[0] as f32 * (1.0 - ty * 0.52) + 255.0 * glow).clamp(0.0, 255.0) as u8;
            let g = (base[1] as f32 * (1.0 - ty * 0.30) + 220.0 * glow).clamp(0.0, 255.0) as u8;
            let b = (base[2] as f32 * (1.0 - ty * 0.50) + 255.0 * glow).clamp(0.0, 255.0) as u8;
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    rgba
}

fn draw_label_band(rgba: &mut [u8], w: u32, h: u32) {
    let y_start = h.saturating_sub(86);
    for y in y_start..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            rgba[idx] = ((rgba[idx] as u16 * 38) / 100) as u8;
            rgba[idx + 1] = ((rgba[idx + 1] as u16 * 38) / 100) as u8;
            rgba[idx + 2] = ((rgba[idx + 2] as u16 * 38) / 100) as u8;
        }
    }
}

fn draw_text_5x7(rgba: &mut [u8], w: u32, h: u32, text: &str, color: Color, scale: u32) {
    let color = match color {
        Color::Rgb(r, g, b) => [r, g, b],
        _ => [245, 245, 245],
    };

    let glyph_w = 5 * scale;
    let spacing = scale;
    let count = text.chars().count() as u32;
    let total_w = count
        .saturating_mul(glyph_w + spacing)
        .saturating_sub(spacing);
    let start_x = w.saturating_sub(total_w) / 2;
    let start_y = h.saturating_sub(68);

    for (i, ch) in text.chars().enumerate() {
        let Some(rows) = glyph_rows(ch) else {
            continue;
        };
        let ox = start_x + (i as u32) * (glyph_w + spacing);
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..5 {
                if (bits >> (4 - col)) & 1 == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let x = ox + col as u32 * scale + sx;
                        let y = start_y + row as u32 * scale + sy;
                        if x >= w || y >= h {
                            continue;
                        }
                        let idx = ((y * w + x) * 4) as usize;
                        rgba[idx] = color[0];
                        rgba[idx + 1] = color[1];
                        rgba[idx + 2] = color[2];
                        rgba[idx + 3] = 255;
                    }
                }
            }
        }
    }
}

fn glyph_rows(ch: char) -> Option<[u8; 7]> {
    match ch {
        'A' => Some([
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ]),
        'E' => Some([
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ]),
        'I' => Some([
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ]),
        'J' => Some([
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ]),
        'L' => Some([
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ]),
        'N' => Some([
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ]),
        'O' => Some([
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ]),
        'R' => Some([
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ]),
        'S' => Some([
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ]),
        _ => None,
    }
}
