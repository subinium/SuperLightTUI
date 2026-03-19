use slt::{Border, Color, Context, KeyCode, RunConfig, Style, TabsState, Theme};

struct MemberProfile {
    tab: &'static str,
    name: &'static str,
    born: &'static str,
    position: &'static str,
    nationality: &'static str,
    note: &'static str,
    image_path: &'static str,
    placeholder_color: [u8; 3],
}

struct MemberImage {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

const MEMBERS: [MemberProfile; 4] = [
    MemberProfile {
        tab: "Jisoo",
        name: "Kim Jisoo (김지수)",
        born: "1995-01-03",
        position: "Lead Vocal",
        nationality: "South Korea",
        note: "Eldest member, actress in Snowdrop",
        image_path: "assets/blackpink/jisoo.jpg",
        placeholder_color: [255, 105, 180],
    },
    MemberProfile {
        tab: "Jennie",
        name: "Jennie Kim (제니)",
        born: "1996-01-16",
        position: "Main Rapper & Lead Vocal",
        nationality: "South Korea",
        note: "Solo debut 2018, 6 years trainee",
        image_path: "assets/blackpink/jennie.jpg",
        placeholder_color: [220, 20, 60],
    },
    MemberProfile {
        tab: "Rosé",
        name: "Park Chaeyoung (로제/박채영)",
        born: "1997-02-11",
        position: "Main Vocal & Lead Dancer",
        nationality: "South Korea/New Zealand",
        note: "Born in NZ, raised in Australia",
        image_path: "assets/blackpink/rose.jpg",
        placeholder_color: [255, 127, 80],
    },
    MemberProfile {
        tab: "Lisa",
        name: "Lalisa Manobal (리사)",
        born: "1997-03-27",
        position: "Main Dancer & Lead Rapper",
        nationality: "Thailand",
        note: "Most followed K-pop idol on Instagram",
        image_path: "assets/blackpink/lisa.jpg",
        placeholder_color: [148, 103, 189],
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
    let member_images: Vec<MemberImage> = MEMBERS.iter().map(load_member_image).collect();

    slt::run_with(
        RunConfig::default().mouse(true).theme(Theme::dark()),
        move |ui: &mut Context| {
            let quit_key = ui.key('q');
            let esc_key = ui.key_code(KeyCode::Esc);
            if quit_key || esc_key {
                ui.quit();
            }

            let _ = ui.container().bg(Color::Rgb(28, 30, 34)).p(1).row(|ui| {
                ui.text("BLACKPINK").bold().fg(Color::Rgb(255, 105, 180));
                ui.text("  블랙핑크").fg(Color::White);
                ui.spacer();
                ui.text("나무위키 스타일").fg(Color::Rgb(126, 211, 33));
            });

            let _ = ui.tabs(&mut tabs);

            let selected = tabs.selected;
            let imgs = &member_images;
            match selected {
                0..=3 => render_member(ui, selected, imgs),
                4 => render_group(ui),
                _ => render_discography(ui),
            }

            let _ = ui.help(&[("← →", "tab"), ("q", "quit")]);
        },
    )
}

fn render_member(ui: &mut Context, idx: usize, member_images: &[MemberImage]) {
    let p = &MEMBERS[idx];
    let img = &member_images[idx];

    let _ = ui.bordered(Border::Single).p(1).row(|ui| {
        let _ = ui
            .bordered(Border::Single)
            .title(format!("{} Photo", p.tab))
            .col(|ui| {
                let _ = ui.kitty_image_fit(&img.rgba, img.width, img.height, 30);
            });

        let _ = ui
            .bordered(Border::Single)
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
    let _ = ui.bordered(Border::Single).title("Group").p(1).col(|ui| {
        info_row(ui, "Debut", "2016-08-08");
        info_row(ui, "Agency", "YG Entertainment");
        info_row(ui, "Fandom", "BLINK");
        info_row(ui, "Members", "Jisoo, Jennie, Rosé, Lisa");
        ui.separator();
        ui.text("Achievements").bold().fg(Color::Rgb(255, 105, 180));
        ui.text("• First K-pop girl group at Coachella (2019)");
        ui.text("• Highest-charting female K-pop group on Billboard Hot 100");
        ui.text("• Most-subscribed music group on YouTube");
    });
}

fn render_discography(ui: &mut Context) {
    let _ = ui
        .bordered(Border::Single)
        .title("Discography")
        .p(1)
        .col(|ui| {
            album_row(ui, "SQUARE ONE", "2016", "Digital Single");
            album_row(ui, "SQUARE UP", "2018", "Mini Album");
            album_row(ui, "THE ALBUM", "2020", "Studio Album");
            album_row(ui, "BORN PINK", "2022", "Studio Album");
            album_row(ui, "DEADLINE", "2026", "Mini Album");
        });
}

fn album_row(ui: &mut Context, title: &str, year: &str, kind: &str) {
    ui.line(|ui| {
        ui.text(format!("{year}  ")).fg(Color::Indexed(245));
        ui.text(title).bold().fg(Color::Rgb(255, 105, 180));
        ui.text(format!("  ({kind})")).fg(Color::Indexed(245));
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

fn load_member_image(member: &MemberProfile) -> MemberImage {
    if let Some(img) = try_load_image_file(member.image_path) {
        return img;
    }
    MemberImage {
        rgba: gen_gradient(200, 300, member.placeholder_color),
        width: 200,
        height: 300,
    }
}

fn try_load_image_file(path: &str) -> Option<MemberImage> {
    let path = std::path::Path::new(path);
    if !path.exists() {
        return None;
    }
    let data = std::fs::read(path).ok()?;
    let (w, h, rgb) = jpeg_decoder(&data)?;
    let mut rgba = Vec::with_capacity(w * h * 4);
    for chunk in rgb.chunks(3) {
        rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
    }
    Some(MemberImage {
        rgba,
        width: w as u32,
        height: h as u32,
    })
}

fn jpeg_decoder(data: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    if data.len() < 2 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    // Minimal approach: use image crate if available, otherwise use stb-style decode
    // For now, shell out to convert via sips (macOS) or ffmpeg
    let tmp_in = "/tmp/slt_wiki_input.jpg";
    let tmp_out = "/tmp/slt_wiki_output.bmp";
    std::fs::write(tmp_in, data).ok()?;

    let status = std::process::Command::new("sips")
        .args(["-s", "format", "bmp", tmp_in, "--out", tmp_out])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;

    if !status.success() {
        return None;
    }

    read_bmp_rgb(tmp_out)
}

fn read_bmp_rgb(path: &str) -> Option<(usize, usize, Vec<u8>)> {
    let data = std::fs::read(path).ok()?;
    if data.len() < 54 || data[0] != b'B' || data[1] != b'M' {
        return None;
    }
    let offset = u32::from_le_bytes([data[10], data[11], data[12], data[13]]) as usize;
    let w = i32::from_le_bytes([data[18], data[19], data[20], data[21]]) as usize;
    let h_raw = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
    let h = h_raw.unsigned_abs() as usize;
    let bpp = u16::from_le_bytes([data[28], data[29]]) as usize;

    if bpp != 24 && bpp != 32 {
        return None;
    }

    let bytes_per_pixel = bpp / 8;
    let row_size = (w * bytes_per_pixel).div_ceil(4) * 4;
    let mut rgb = Vec::with_capacity(w * h * 3);
    let bottom_up = h_raw > 0;

    for row in 0..h {
        let src_row = if bottom_up { h - 1 - row } else { row };
        let row_start = offset + src_row * row_size;
        for col in 0..w {
            let px = row_start + col * bytes_per_pixel;
            if px + 2 >= data.len() {
                rgb.extend_from_slice(&[0, 0, 0]);
                continue;
            }
            rgb.extend_from_slice(&[data[px + 2], data[px + 1], data[px]]);
        }
    }

    Some((w, h, rgb))
}

fn gen_gradient(w: u32, h: u32, base: [u8; 3]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for _x in 0..w {
            let t = y as f32 / h as f32;
            let r = (base[0] as f32 * (1.0 - t * 0.5)).clamp(0.0, 255.0) as u8;
            let g = (base[1] as f32 * (1.0 - t * 0.3)).clamp(0.0, 255.0) as u8;
            let b = (base[2] as f32 * (1.0 - t * 0.5)).clamp(0.0, 255.0) as u8;
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    rgba
}
