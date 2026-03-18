use slt::{Color, Context, KeyCode, RunConfig, Style};
use std::time::Duration;

const PALETTE_SIZE: usize = 37;

fn build_palette() -> [Color; PALETTE_SIZE] {
    let raw: [(u8, u8, u8); PALETTE_SIZE] = [
        (7, 7, 7),
        (31, 7, 7),
        (47, 15, 7),
        (71, 15, 7),
        (87, 23, 7),
        (103, 31, 7),
        (119, 31, 7),
        (143, 39, 7),
        (159, 47, 7),
        (175, 63, 7),
        (191, 71, 7),
        (199, 71, 7),
        (223, 79, 7),
        (223, 87, 7),
        (223, 87, 7),
        (215, 95, 7),
        (215, 95, 7),
        (215, 103, 15),
        (207, 111, 15),
        (207, 119, 15),
        (207, 127, 15),
        (207, 135, 23),
        (199, 135, 23),
        (199, 143, 23),
        (199, 151, 31),
        (191, 159, 31),
        (191, 159, 31),
        (191, 167, 39),
        (191, 167, 39),
        (191, 175, 47),
        (183, 175, 47),
        (183, 183, 47),
        (183, 183, 55),
        (207, 207, 111),
        (223, 223, 159),
        (239, 239, 199),
        (255, 255, 255),
    ];
    let mut palette = [Color::Rgb(0, 0, 0); PALETTE_SIZE];
    for (i, (r, g, b)) in raw.iter().enumerate() {
        palette[i] = Color::Rgb(*r, *g, *b);
    }
    palette
}

struct Fire {
    w: usize,
    h: usize,
    pixels: Vec<usize>,
    rng: u64,
}

impl Fire {
    fn new(w: usize, h: usize) -> Self {
        let mut pixels = vec![0usize; w * h];
        for x in 0..w {
            pixels[(h - 1) * w + x] = PALETTE_SIZE - 1;
        }
        Self {
            w,
            h,
            pixels,
            rng: 0xDEAD_BEEF_CAFE_1234,
        }
    }

    fn next_rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn step(&mut self) {
        for x in 0..self.w {
            for y in 1..self.h {
                let src = y * self.w + x;
                let rand_val = self.next_rand();
                let decay = (rand_val & 3) as usize;
                let wind = ((rand_val >> 2) & 1) as usize;
                let dst_x = x.saturating_sub(wind);
                let dst = (y - 1) * self.w + dst_x;
                self.pixels[dst] = self.pixels[src].saturating_sub(decay);
            }
        }
    }

    fn color_at(&self, x: usize, y: usize, palette: &[Color; PALETTE_SIZE]) -> Color {
        palette[self.pixels[y * self.w + x]]
    }
}

fn main() {
    let palette = build_palette();
    let mut fire: Option<Fire> = None;
    let mut paused = false;

    let _ = slt::run_with(
        RunConfig {
            tick_rate: Duration::from_millis(16),
            max_fps: Some(60),
            ..RunConfig::default()
        },
        move |ui: &mut Context| {
            let term_w = ui.width() as usize;
            let term_h = ui.height() as usize;

            let fire_w = term_w;
            let fire_h = term_h * 2;

            let fire = fire.get_or_insert_with(|| Fire::new(fire_w, fire_h));

            if fire.w != fire_w || fire.h != fire_h {
                *fire = Fire::new(fire_w, fire_h);
            }

            if ui.key('q') || ui.key_code(KeyCode::Esc) {
                ui.quit();
                return;
            }
            if ui.key(' ') {
                paused = !paused;
            }

            if !paused {
                for _ in 0..2 {
                    fire.step();
                }
            }

            let _ = ui.col(|ui| {
                for row in 0..term_h {
                    let top_y = row * 2;
                    let bot_y = top_y + 1;

                    let _ = ui.row(|ui| {
                        let mut run_start = 0;
                        let mut cur_top = fire.color_at(0, top_y, &palette);
                        let mut cur_bot = fire.color_at(0, bot_y, &palette);

                        for col in 1..=term_w {
                            let (t, b) = if col < term_w {
                                (
                                    fire.color_at(col, top_y, &palette),
                                    fire.color_at(col, bot_y, &palette),
                                )
                            } else {
                                (Color::Reset, Color::Reset)
                            };

                            if col == term_w || t != cur_top || b != cur_bot {
                                let len = col - run_start;
                                let s: String = (0..len).map(|_| '▀').collect();
                                ui.styled(s, Style::new().fg(cur_top).bg(cur_bot));
                                run_start = col;
                                cur_top = t;
                                cur_bot = b;
                            }
                        }
                    });
                }
            });
        },
    );
}
