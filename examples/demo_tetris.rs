use std::time::Duration;

use slt::{Border, Color, Context, KeyCode, Style, Theme};

const BOARD_W: usize = 10;
const BOARD_H: usize = 20;

const KICKS: [(i32, i32); 7] = [(0, 0), (-1, 0), (1, 0), (0, -1), (-2, 0), (2, 0), (0, 1)];

const PIECES: [[[(i32, i32); 4]; 4]; 7] = [
    [
        [(0, 1), (1, 1), (2, 1), (3, 1)],
        [(2, 0), (2, 1), (2, 2), (2, 3)],
        [(0, 2), (1, 2), (2, 2), (3, 2)],
        [(1, 0), (1, 1), (1, 2), (1, 3)],
    ],
    [
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (2, 1)],
    ],
    [
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (1, 2)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ],
    [
        [(1, 0), (2, 0), (0, 1), (1, 1)],
        [(1, 0), (1, 1), (2, 1), (2, 2)],
        [(1, 1), (2, 1), (0, 2), (1, 2)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ],
    [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(2, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (1, 2), (2, 2)],
        [(1, 0), (0, 1), (1, 1), (0, 2)],
    ],
    [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (2, 2)],
        [(1, 0), (1, 1), (0, 2), (1, 2)],
    ],
    [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (1, 2), (2, 2)],
        [(0, 1), (1, 1), (2, 1), (0, 2)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
    ],
];

#[derive(Clone, Copy)]
struct Active {
    kind: usize,
    rot: usize,
    x: i32,
    y: i32,
}

struct Game {
    board: [[Option<usize>; BOARD_W]; BOARD_H],
    active: Active,
    next_kind: usize,
    rng: u64,
    score: u64,
    lines: u32,
    level: u32,
    game_over: bool,
    paused: bool,
    last_drop_tick: u64,
}

impl Game {
    fn new(seed: u64, tick: u64) -> Self {
        let mut game = Self {
            board: [[None; BOARD_W]; BOARD_H],
            active: Active {
                kind: 0,
                rot: 0,
                x: 3,
                y: 0,
            },
            next_kind: 0,
            rng: seed.wrapping_mul(1664525).wrapping_add(1013904223),
            score: 0,
            lines: 0,
            level: 1,
            game_over: false,
            paused: false,
            last_drop_tick: tick,
        };
        game.active.kind = game.random_kind();
        game.next_kind = game.random_kind();
        game.active.rot = 0;
        game.active.x = 3;
        game.active.y = 0;
        if !game.is_valid(
            game.active.kind,
            game.active.rot,
            game.active.x,
            game.active.y,
        ) {
            game.game_over = true;
        }
        game
    }

    fn random_kind(&mut self) -> usize {
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.rng % 7) as usize
    }

    fn gravity_interval(&self) -> u64 {
        let level_speedup = self.level.min(18) as u64;
        let speed = 20_u64.saturating_sub(level_speedup);
        speed.max(2)
    }

    fn is_valid(&self, kind: usize, rot: usize, x: i32, y: i32) -> bool {
        for &(dx, dy) in &PIECES[kind][rot] {
            let px = x + dx;
            let py = y + dy;
            if px < 0 || px >= BOARD_W as i32 || py >= BOARD_H as i32 {
                return false;
            }
            if py >= 0 && self.board[py as usize][px as usize].is_some() {
                return false;
            }
        }
        true
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let nx = self.active.x + dx;
        let ny = self.active.y + dy;
        if self.is_valid(self.active.kind, self.active.rot, nx, ny) {
            self.active.x = nx;
            self.active.y = ny;
            return true;
        }
        false
    }

    fn rotate_cw(&mut self) {
        let new_rot = (self.active.rot + 1) % 4;
        for (kx, ky) in KICKS {
            let nx = self.active.x + kx;
            let ny = self.active.y + ky;
            if self.is_valid(self.active.kind, new_rot, nx, ny) {
                self.active.rot = new_rot;
                self.active.x = nx;
                self.active.y = ny;
                return;
            }
        }
    }

    fn soft_drop_step(&mut self) {
        if !self.try_move(0, 1) {
            self.lock_active();
            self.clear_lines();
            self.spawn_next();
        }
    }

    fn hard_drop(&mut self) {
        while self.try_move(0, 1) {}
        self.lock_active();
        self.clear_lines();
        self.spawn_next();
    }

    fn ghost_y(&self) -> i32 {
        let mut y = self.active.y;
        while self.is_valid(self.active.kind, self.active.rot, self.active.x, y + 1) {
            y += 1;
        }
        y
    }

    fn lock_active(&mut self) {
        for &(dx, dy) in &PIECES[self.active.kind][self.active.rot] {
            let px = self.active.x + dx;
            let py = self.active.y + dy;
            if py < 0 {
                self.game_over = true;
                continue;
            }
            if (0..BOARD_W as i32).contains(&px) && (0..BOARD_H as i32).contains(&py) {
                self.board[py as usize][px as usize] = Some(self.active.kind);
            }
        }
    }

    fn clear_lines(&mut self) {
        let mut new_board = [[None; BOARD_W]; BOARD_H];
        let mut write_y = BOARD_H as i32 - 1;
        let mut cleared = 0_u32;

        for y in (0..BOARD_H).rev() {
            let full = self.board[y].iter().all(Option::is_some);
            if full {
                cleared += 1;
            } else {
                new_board[write_y as usize] = self.board[y];
                write_y -= 1;
            }
        }

        self.board = new_board;

        if cleared > 0 {
            self.lines += cleared;
            self.level = self.lines / 10 + 1;
            self.score += match cleared {
                1 => 100,
                2 => 300,
                3 => 500,
                4 => 800,
                _ => 0,
            };
        }
    }

    fn spawn_next(&mut self) {
        self.active.kind = self.next_kind;
        self.active.rot = 0;
        self.active.x = 3;
        self.active.y = 0;
        self.next_kind = self.random_kind();
        if !self.is_valid(
            self.active.kind,
            self.active.rot,
            self.active.x,
            self.active.y,
        ) {
            self.game_over = true;
        }
    }

    fn restart(&mut self, seed: u64, tick: u64) {
        *self = Self::new(seed, tick);
    }
}

fn piece_color(kind: usize) -> Color {
    match kind {
        0 => Color::Cyan,
        1 => Color::Yellow,
        2 => Color::Magenta,
        3 => Color::Green,
        4 => Color::Red,
        5 => Color::Blue,
        _ => Color::Rgb(255, 165, 0),
    }
}

fn active_at(game: &Game, x: usize, y: usize) -> bool {
    for &(dx, dy) in &PIECES[game.active.kind][game.active.rot] {
        let px = game.active.x + dx;
        let py = game.active.y + dy;
        if px == x as i32 && py == y as i32 {
            return true;
        }
    }
    false
}

fn ghost_at(game: &Game, ghost_y: i32, x: usize, y: usize) -> bool {
    for &(dx, dy) in &PIECES[game.active.kind][game.active.rot] {
        let px = game.active.x + dx;
        let py = ghost_y + dy;
        if px == x as i32 && py == y as i32 {
            return true;
        }
    }
    false
}

fn next_preview_at(kind: usize, x: usize, y: usize) -> bool {
    for &(px, py) in &PIECES[kind][0] {
        if px == x as i32 && py == y as i32 {
            return true;
        }
    }
    false
}

fn format_score(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(b as char);
    }
    out
}

fn render_board(ui: &mut Context, game: &Game) {
    let ghost_y = game.ghost_y();
    for y in 0..BOARD_H {
        ui.row(|ui| {
            for x in 0..BOARD_W {
                if let Some(kind) = game.board[y][x] {
                    ui.styled("██", Style::new().fg(piece_color(kind)));
                } else if active_at(game, x, y) {
                    ui.styled("██", Style::new().fg(piece_color(game.active.kind)));
                } else if ghost_y != game.active.y && ghost_at(game, ghost_y, x, y) {
                    ui.styled("░░", Style::new().fg(piece_color(game.active.kind)).dim());
                } else {
                    ui.styled("· ", Style::new().fg(Color::Rgb(50, 50, 65)));
                }
            }
        });
    }
}

fn render_next(ui: &mut Context, kind: usize) {
    let color = piece_color(kind);
    for y in 0..4 {
        ui.row(|ui| {
            for x in 0..4 {
                if next_preview_at(kind, x, y) {
                    ui.styled("██", Style::new().fg(color));
                } else {
                    ui.text("  ");
                }
            }
        });
    }
}

fn main() -> std::io::Result<()> {
    let mut dark_mode = true;
    let mut game = Game::new(1, 0);

    slt::run_with(
        slt::RunConfig {
            mouse: false,
            tick_rate: Duration::from_millis(50),
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key('q') {
                ui.quit();
            }
            if ui.key('t') {
                dark_mode = !dark_mode;
            }
            ui.set_theme(if dark_mode {
                Theme::dark()
            } else {
                Theme::light()
            });

            let tick = ui.tick();

            if ui.key('r') {
                game.restart(tick.wrapping_mul(7919).wrapping_add(game.rng), tick);
            }

            if ui.key('p') && !game.game_over {
                game.paused = !game.paused;
                game.last_drop_tick = tick;
            }

            if !game.paused && !game.game_over {
                if ui.key_code(KeyCode::Left) {
                    game.try_move(-1, 0);
                }
                if ui.key_code(KeyCode::Right) {
                    game.try_move(1, 0);
                }
                if ui.key_code(KeyCode::Up) {
                    game.rotate_cw();
                }
                if ui.key_code(KeyCode::Down) {
                    game.soft_drop_step();
                    game.last_drop_tick = tick;
                }
                if ui.key(' ') {
                    game.hard_drop();
                    game.last_drop_tick = tick;
                }
                if tick.saturating_sub(game.last_drop_tick) >= game.gravity_interval() {
                    game.soft_drop_step();
                    game.last_drop_tick = tick;
                }
            }

            let game_w = 45_u32;
            let game_h = 24_u32;
            let left = ui.width().saturating_sub(game_w) / 2;
            let top = ui.height().saturating_sub(game_h) / 2;

            ui.bordered(Border::Rounded)
                .title_styled(" T E T R I S ", Style::new().bold().fg(Color::Cyan))
                .w(game_w)
                .ml(left)
                .mt(top)
                .col(|ui| {
                    ui.row_gap(1, |ui| {
                        ui.bordered(Border::Single)
                            .border_style(Style::new().fg(Color::Rgb(60, 62, 80)))
                            .col(|ui| {
                                render_board(ui, &game);
                            });

                        ui.container().w(20).col(|ui| {
                            ui.bordered(Border::Rounded)
                                .title("NEXT")
                                .border_style(Style::new().fg(Color::Rgb(60, 62, 80)))
                                .px(2)
                                .py(1)
                                .col(|ui| {
                                    render_next(ui, game.next_kind);
                                });

                            ui.bordered(Border::Rounded)
                                .title("SCORE")
                                .border_style(Style::new().fg(Color::Rgb(60, 62, 80)))
                                .px(1)
                                .col(|ui| {
                                    ui.text(format_score(game.score)).bold().fg(Color::White);
                                    ui.row(|ui| {
                                        ui.text("LEVEL").dim();
                                        ui.spacer();
                                        ui.text(format!("{}", game.level)).bold().fg(Color::Cyan);
                                    });
                                    ui.row(|ui| {
                                        ui.text("LINES").dim();
                                        ui.spacer();
                                        ui.text(format!("{}", game.lines)).bold().fg(Color::Yellow);
                                    });
                                });

                            ui.spacer();

                            if game.game_over {
                                ui.text(" GAME OVER").bold().fg(Color::Red);
                                ui.text(format!(" Score: {}", format_score(game.score)))
                                    .dim();
                                ui.text(" [R] Restart").dim();
                            } else if game.paused {
                                ui.text(" PAUSED").bold().fg(Color::Yellow);
                                ui.text(" [P] Resume").dim();
                            }

                            ui.separator();
                            ui.text(" ←→↑↓ Move/Rotate").dim();
                            ui.text(" SPC Drop  P Pause").dim();
                            ui.text(" R Reset   Q Quit").dim();
                        });
                    });
                });
        },
    )
}
