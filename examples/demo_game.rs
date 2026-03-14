use std::collections::VecDeque;
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

const SNAKE_W: i32 = 20;
const SNAKE_H: i32 = 15;

const MINE_W: usize = 16;
const MINE_H: usize = 16;
const MINE_COUNT: usize = 40;

#[derive(Clone, Copy)]
enum ActiveGame {
    Tetris,
    Snake,
    Minesweeper,
}

#[derive(Clone, Copy)]
struct Active {
    kind: usize,
    rot: usize,
    x: i32,
    y: i32,
}

struct TetrisGame {
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

impl TetrisGame {
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
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
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

    fn sync_tick(&mut self, tick: u64) {
        self.last_drop_tick = tick;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

struct SnakeGame {
    snake: VecDeque<(i32, i32)>,
    dir: Direction,
    queued_dir: Direction,
    food: (i32, i32),
    rng: u64,
    score: u32,
    game_over: bool,
    paused: bool,
    last_move_tick: u64,
}

impl SnakeGame {
    fn new(seed: u64, tick: u64) -> Self {
        let mut snake = VecDeque::new();
        snake.push_back((7, 7));
        snake.push_back((6, 7));
        snake.push_back((5, 7));
        let mut game = Self {
            snake,
            dir: Direction::Right,
            queued_dir: Direction::Right,
            food: (0, 0),
            rng: seed.wrapping_mul(1664525).wrapping_add(1013904223),
            score: 0,
            game_over: false,
            paused: false,
            last_move_tick: tick,
        };
        game.spawn_food();
        game
    }

    fn next_rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn move_interval(&self) -> u64 {
        let base = 8_u64;
        let speedup = (self.score / 4) as u64;
        base.saturating_sub(speedup).max(2)
    }

    fn is_opposite(a: Direction, b: Direction) -> bool {
        matches!(
            (a, b),
            (Direction::Up, Direction::Down)
                | (Direction::Down, Direction::Up)
                | (Direction::Left, Direction::Right)
                | (Direction::Right, Direction::Left)
        )
    }

    fn set_direction(&mut self, next: Direction) {
        if !Self::is_opposite(self.dir, next) {
            self.queued_dir = next;
        }
    }

    fn spawn_food(&mut self) {
        loop {
            let x = (self.next_rand() % SNAKE_W as u64) as i32;
            let y = (self.next_rand() % SNAKE_H as u64) as i32;
            if !self.snake.iter().any(|&(sx, sy)| sx == x && sy == y) {
                self.food = (x, y);
                break;
            }
        }
    }

    fn step(&mut self) {
        if self.game_over || self.paused {
            return;
        }

        self.dir = self.queued_dir;
        let (hx, hy) = self.snake.front().copied().unwrap_or((0, 0));
        let (nx, ny) = match self.dir {
            Direction::Up => (hx, hy - 1),
            Direction::Down => (hx, hy + 1),
            Direction::Left => (hx - 1, hy),
            Direction::Right => (hx + 1, hy),
        };

        if !(0..SNAKE_W).contains(&nx) || !(0..SNAKE_H).contains(&ny) {
            self.game_over = true;
            return;
        }

        let will_grow = (nx, ny) == self.food;
        let tail = self.snake.back().copied();
        if self.snake.iter().any(|&(x, y)| {
            if will_grow {
                x == nx && y == ny
            } else if let Some((tx, ty)) = tail {
                if x == tx && y == ty {
                    false
                } else {
                    x == nx && y == ny
                }
            } else {
                x == nx && y == ny
            }
        }) {
            self.game_over = true;
            return;
        }

        self.snake.push_front((nx, ny));
        if will_grow {
            self.score += 1;
            if self.snake.len() < (SNAKE_W * SNAKE_H) as usize {
                self.spawn_food();
            }
        } else {
            let _ = self.snake.pop_back();
        }
    }

    fn restart(&mut self, seed: u64, tick: u64) {
        *self = Self::new(seed, tick);
    }

    fn sync_tick(&mut self, tick: u64) {
        self.last_move_tick = tick;
    }
}

#[derive(Clone, Copy)]
struct MineCell {
    mine: bool,
    revealed: bool,
    flagged: bool,
    adjacent: u8,
}

impl MineCell {
    fn empty() -> Self {
        Self {
            mine: false,
            revealed: false,
            flagged: false,
            adjacent: 0,
        }
    }
}

struct MinesweeperGame {
    board: [[MineCell; MINE_W]; MINE_H],
    rng: u64,
    cursor_x: usize,
    cursor_y: usize,
    first_reveal: bool,
    game_over: bool,
    won: bool,
}

impl MinesweeperGame {
    fn new(seed: u64) -> Self {
        let mut game = Self {
            board: [[MineCell::empty(); MINE_W]; MINE_H],
            rng: seed.wrapping_mul(1664525).wrapping_add(1013904223),
            cursor_x: 0,
            cursor_y: 0,
            first_reveal: true,
            game_over: false,
            won: false,
        };
        game.generate_mines(None);
        game
    }

    fn next_rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    fn generate_mines(&mut self, avoid: Option<(usize, usize)>) {
        self.board = [[MineCell::empty(); MINE_W]; MINE_H];
        let mut placed = 0;
        while placed < MINE_COUNT {
            let x = (self.next_rand() % MINE_W as u64) as usize;
            let y = (self.next_rand() % MINE_H as u64) as usize;
            if let Some((ax, ay)) = avoid {
                if x == ax && y == ay {
                    continue;
                }
            }
            if self.board[y][x].mine {
                continue;
            }
            self.board[y][x].mine = true;
            placed += 1;
        }
        self.compute_adjacency();
    }

    fn compute_adjacency(&mut self) {
        for y in 0..MINE_H {
            for x in 0..MINE_W {
                if self.board[y][x].mine {
                    self.board[y][x].adjacent = 0;
                    continue;
                }
                let mut count = 0_u8;
                for ny in y.saturating_sub(1)..=((y + 1).min(MINE_H - 1)) {
                    for nx in x.saturating_sub(1)..=((x + 1).min(MINE_W - 1)) {
                        if nx == x && ny == y {
                            continue;
                        }
                        if self.board[ny][nx].mine {
                            count = count.saturating_add(1);
                        }
                    }
                }
                self.board[y][x].adjacent = count;
            }
        }
    }

    fn reveal_current(&mut self) {
        self.reveal(self.cursor_x, self.cursor_y);
    }

    fn reveal(&mut self, x: usize, y: usize) {
        if self.game_over || self.won {
            return;
        }
        if self.board[y][x].flagged || self.board[y][x].revealed {
            return;
        }

        if self.first_reveal {
            if self.board[y][x].mine {
                self.generate_mines(Some((x, y)));
            }
            self.first_reveal = false;
        }

        if self.board[y][x].mine {
            self.board[y][x].revealed = true;
            self.game_over = true;
            self.reveal_all_mines();
            return;
        }

        self.flood_reveal(x, y);
        self.check_win();
    }

    fn flood_reveal(&mut self, x: usize, y: usize) {
        let mut queue = VecDeque::new();
        queue.push_back((x, y));

        while let Some((cx, cy)) = queue.pop_front() {
            if self.board[cy][cx].revealed || self.board[cy][cx].flagged {
                continue;
            }

            self.board[cy][cx].revealed = true;
            if self.board[cy][cx].adjacent != 0 {
                continue;
            }

            for ny in cy.saturating_sub(1)..=((cy + 1).min(MINE_H - 1)) {
                for nx in cx.saturating_sub(1)..=((cx + 1).min(MINE_W - 1)) {
                    if nx == cx && ny == cy {
                        continue;
                    }
                    if !self.board[ny][nx].revealed && !self.board[ny][nx].mine {
                        queue.push_back((nx, ny));
                    }
                }
            }
        }
    }

    fn toggle_flag_current(&mut self) {
        if self.game_over || self.won {
            return;
        }
        let cell = &mut self.board[self.cursor_y][self.cursor_x];
        if !cell.revealed {
            cell.flagged = !cell.flagged;
        }
    }

    fn reveal_all_mines(&mut self) {
        for y in 0..MINE_H {
            for x in 0..MINE_W {
                if self.board[y][x].mine {
                    self.board[y][x].revealed = true;
                }
            }
        }
    }

    fn check_win(&mut self) {
        for y in 0..MINE_H {
            for x in 0..MINE_W {
                if !self.board[y][x].mine && !self.board[y][x].revealed {
                    return;
                }
            }
        }
        self.won = true;
    }

    fn flags_count(&self) -> usize {
        let mut n = 0;
        for y in 0..MINE_H {
            for x in 0..MINE_W {
                if self.board[y][x].flagged {
                    n += 1;
                }
            }
        }
        n
    }

    fn mines_remaining(&self) -> i32 {
        MINE_COUNT as i32 - self.flags_count() as i32
    }

    fn restart(&mut self, seed: u64) {
        *self = Self::new(seed);
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

fn active_at(game: &TetrisGame, x: usize, y: usize) -> bool {
    for &(dx, dy) in &PIECES[game.active.kind][game.active.rot] {
        let px = game.active.x + dx;
        let py = game.active.y + dy;
        if px == x as i32 && py == y as i32 {
            return true;
        }
    }
    false
}

fn ghost_at(game: &TetrisGame, ghost_y: i32, x: usize, y: usize) -> bool {
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

fn render_tetris_board(ui: &mut Context, game: &TetrisGame, theme: Theme) {
    let ghost_y = game.ghost_y();
    for y in 0..BOARD_H {
        ui.row(|ui| {
            for x in 0..BOARD_W {
                if let Some(kind) = game.board[y][x] {
                    ui.styled("██", Style::new().fg(piece_color(kind)));
                } else if active_at(game, x, y) {
                    ui.styled("██", Style::new().fg(piece_color(game.active.kind)));
                } else if ghost_y != game.active.y && ghost_at(game, ghost_y, x, y) {
                    ui.styled("░░", Style::new().fg(theme.text_dim));
                } else {
                    ui.styled("· ", Style::new().fg(theme.text_dim));
                }
            }
        });
    }
}

fn render_tetris_next(ui: &mut Context, kind: usize) {
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

fn render_snake_board(ui: &mut Context, game: &SnakeGame, theme: Theme) {
    for y in 0..SNAKE_H {
        ui.row(|ui| {
            for x in 0..SNAKE_W {
                if (x, y) == game.food {
                    ui.styled("● ", Style::new().fg(theme.accent));
                } else if let Some(&(hx, hy)) = game.snake.front() {
                    if (x, y) == (hx, hy) {
                        ui.styled("██", Style::new().fg(theme.success));
                    } else if game.snake.iter().any(|&(sx, sy)| (sx, sy) == (x, y)) {
                        ui.styled("██", Style::new().fg(theme.primary));
                    } else {
                        ui.styled("· ", Style::new().fg(theme.text_dim));
                    }
                } else {
                    ui.styled("· ", Style::new().fg(theme.text_dim));
                }
            }
        });
    }
}

fn mine_number_color(n: u8) -> Color {
    match n {
        1 => Color::Blue,
        2 => Color::Green,
        3 => Color::Red,
        4 => Color::Rgb(0, 0, 139),
        5 => Color::Rgb(139, 0, 0),
        6 => Color::Cyan,
        7 => Color::Black,
        8 => Color::Rgb(128, 128, 128),
        _ => Color::White,
    }
}

fn render_mine_board(ui: &mut Context, game: &MinesweeperGame, theme: Theme) {
    for y in 0..MINE_H {
        ui.row(|ui| {
            for x in 0..MINE_W {
                let cell = game.board[y][x];
                let mut style = Style::new();
                let content = if cell.revealed {
                    if cell.mine {
                        style = style.fg(theme.error);
                        "*"
                    } else if cell.adjacent == 0 {
                        style = style.fg(theme.text_dim);
                        " "
                    } else {
                        style = style.fg(mine_number_color(cell.adjacent));
                        match cell.adjacent {
                            1 => "1",
                            2 => "2",
                            3 => "3",
                            4 => "4",
                            5 => "5",
                            6 => "6",
                            7 => "7",
                            _ => "8",
                        }
                    }
                } else if cell.flagged {
                    style = style.fg(theme.warning);
                    "⚑"
                } else {
                    style = style.fg(theme.text_dim);
                    "·"
                };

                if x == game.cursor_x && y == game.cursor_y {
                    style = style.reversed();
                }
                ui.styled(format!("{} ", content), style);
            }
        });
    }
}

fn render_header(ui: &mut Context, active: ActiveGame, theme: Theme, theme_name: &str) {
    ui.container()
        .bg(theme.surface)
        .border(Border::Rounded)
        .border_style(Style::new().fg(theme.border))
        .px(2)
        .py(1)
        .row(|ui| {
            let tetris_style = if matches!(active, ActiveGame::Tetris) {
                Style::new().fg(theme.primary).bold()
            } else {
                Style::new().fg(theme.text_dim)
            };
            let snake_style = if matches!(active, ActiveGame::Snake) {
                Style::new().fg(theme.primary).bold()
            } else {
                Style::new().fg(theme.text_dim)
            };
            let mine_style = if matches!(active, ActiveGame::Minesweeper) {
                Style::new().fg(theme.primary).bold()
            } else {
                Style::new().fg(theme.text_dim)
            };

            ui.styled("[1] Tetris", tetris_style);
            ui.text("  ").fg(theme.surface_text);
            ui.styled("[2] Snake", snake_style);
            ui.text("  ").fg(theme.surface_text);
            ui.styled("[3] Minesweeper", mine_style);
            ui.spacer();
            ui.text(format!("Theme: {}", theme_name))
                .fg(theme.surface_text);
            ui.text("   t cycle   q quit").fg(theme.text_dim);
        });
}

fn render_tetris_screen(ui: &mut Context, game: &TetrisGame, theme: Theme) {
    let game_w = 45_u32;
    let left = ui.width().saturating_sub(game_w) / 2;

    ui.bordered(Border::Rounded)
        .title_styled(" T E T R I S ", Style::new().bold().fg(theme.primary))
        .border_style(Style::new().fg(theme.border))
        .bg(theme.surface)
        .w(game_w)
        .ml(left)
        .col(|ui| {
            ui.row_gap(1, |ui| {
                ui.bordered(Border::Single)
                    .border_style(Style::new().fg(theme.border))
                    .col(|ui| {
                        render_tetris_board(ui, game, theme);
                    });

                ui.container().w(20).col(|ui| {
                    ui.container()
                        .bg(theme.surface)
                        .border(Border::Rounded)
                        .title("NEXT")
                        .border_style(Style::new().fg(theme.border))
                        .px(2)
                        .py(1)
                        .col(|ui| {
                            render_tetris_next(ui, game.next_kind);
                        });

                    ui.container()
                        .bg(theme.surface)
                        .border(Border::Rounded)
                        .title("SCORE")
                        .border_style(Style::new().fg(theme.border))
                        .px(1)
                        .col(|ui| {
                            ui.text(format_score(game.score))
                                .bold()
                                .fg(theme.surface_text);
                            ui.row(|ui| {
                                ui.text("LEVEL").fg(theme.text_dim);
                                ui.spacer();
                                ui.text(format!("{}", game.level)).bold().fg(theme.primary);
                            });
                            ui.row(|ui| {
                                ui.text("LINES").fg(theme.text_dim);
                                ui.spacer();
                                ui.text(format!("{}", game.lines)).bold().fg(theme.accent);
                            });
                        });

                    ui.spacer();

                    if game.game_over {
                        ui.text(" GAME OVER").bold().fg(theme.error);
                        ui.text(format!(" Score: {}", format_score(game.score)))
                            .fg(theme.text_dim);
                        ui.text(" [R] Restart").fg(theme.text_dim);
                    } else if game.paused {
                        ui.text(" PAUSED").bold().fg(theme.warning);
                        ui.text(" [P] Resume").fg(theme.text_dim);
                    }

                    ui.separator();
                    ui.text(" ←→↑↓ Move/Rotate").fg(theme.text_dim);
                    ui.text(" SPC Drop  P Pause").fg(theme.text_dim);
                    ui.text(" R Reset   Q Quit").fg(theme.text_dim);
                });
            });
        });
}

fn render_snake_screen(ui: &mut Context, game: &SnakeGame, theme: Theme) {
    let game_w = 58_u32;
    let left = ui.width().saturating_sub(game_w) / 2;

    ui.container()
        .bg(theme.surface)
        .border(Border::Rounded)
        .title_styled(" S N A K E ", Style::new().bold().fg(theme.primary))
        .border_style(Style::new().fg(theme.border))
        .w(game_w)
        .ml(left)
        .col(|ui| {
            ui.row_gap(1, |ui| {
                ui.bordered(Border::Single)
                    .border_style(Style::new().fg(theme.border))
                    .col(|ui| {
                        render_snake_board(ui, game, theme);
                    });

                ui.container()
                    .bg(theme.surface)
                    .border(Border::Rounded)
                    .border_style(Style::new().fg(theme.border))
                    .w(14)
                    .px(1)
                    .col(|ui| {
                        ui.text("SCORE").bold().fg(theme.surface_text);
                        ui.text(format!("{}", game.score)).bold().fg(theme.primary);
                        ui.text("SPEED").bold().fg(theme.surface_text);
                        ui.text(format!("{}", 10_u64.saturating_sub(game.move_interval())))
                            .fg(theme.accent);
                        ui.separator();
                        if game.game_over {
                            ui.text("GAME OVER").bold().fg(theme.error);
                        } else if game.paused {
                            ui.text("PAUSED").bold().fg(theme.warning);
                        }
                        ui.separator();
                        ui.text("←→↑↓ Move").fg(theme.text_dim);
                        ui.text("P Pause").fg(theme.text_dim);
                        ui.text("R Restart").fg(theme.text_dim);
                    });
            });
        });
}

fn render_minesweeper_screen(ui: &mut Context, game: &MinesweeperGame, theme: Theme) {
    let game_w = 56_u32;
    let left = ui.width().saturating_sub(game_w) / 2;

    ui.container()
        .bg(theme.surface)
        .border(Border::Rounded)
        .title_styled(
            " M I N E S W E E P E R ",
            Style::new().bold().fg(theme.primary),
        )
        .border_style(Style::new().fg(theme.border))
        .w(game_w)
        .ml(left)
        .col(|ui| {
            ui.row_gap(1, |ui| {
                ui.bordered(Border::Single)
                    .border_style(Style::new().fg(theme.border))
                    .col(|ui| {
                        render_mine_board(ui, game, theme);
                    });

                ui.container()
                    .bg(theme.surface)
                    .border(Border::Rounded)
                    .border_style(Style::new().fg(theme.border))
                    .w(18)
                    .px(1)
                    .col(|ui| {
                        ui.text("MINES").bold().fg(theme.surface_text);
                        ui.text(format!("{}", game.mines_remaining()))
                            .bold()
                            .fg(theme.primary);
                        ui.text("FLAGS").bold().fg(theme.surface_text);
                        ui.text(format!("{}", game.flags_count())).fg(theme.accent);
                        ui.separator();
                        if game.game_over {
                            ui.text("GAME OVER").bold().fg(theme.error);
                        } else if game.won {
                            ui.text("YOU WIN").bold().fg(theme.success);
                        }
                        ui.separator();
                        ui.text("Arrows Move").fg(theme.text_dim);
                        ui.text("Enter/Space Reveal").fg(theme.text_dim);
                        ui.text("F Flag").fg(theme.text_dim);
                        ui.text("R Restart").fg(theme.text_dim);
                    });
            });
        });
}

fn main() -> std::io::Result<()> {
    let themes: [fn() -> Theme; 7] = [
        Theme::dark,
        Theme::light,
        Theme::dracula,
        Theme::catppuccin,
        Theme::nord,
        Theme::solarized_dark,
        Theme::tokyo_night,
    ];
    let theme_names = [
        "Dark",
        "Light",
        "Dracula",
        "Catppuccin",
        "Nord",
        "Solarized",
        "Tokyo Night",
    ];

    let mut theme_idx: usize = 0;
    let mut active = ActiveGame::Tetris;

    let mut tetris = TetrisGame::new(1, 0);
    let mut snake = SnakeGame::new(2, 0);
    let mut mines = MinesweeperGame::new(3);

    slt::run_with(
        slt::RunConfig {
            mouse: true,
            tick_rate: Duration::from_millis(50),
            ..Default::default()
        },
        |ui: &mut Context| {
            if ui.key('q') {
                ui.quit();
            }

            if ui.key('t') {
                theme_idx = (theme_idx + 1) % themes.len();
            }

            ui.set_theme(themes[theme_idx]());
            let theme = *ui.theme();
            let tick = ui.tick();

            let mut switched = false;
            if ui.key('1') {
                active = ActiveGame::Tetris;
                switched = true;
            }
            if ui.key('2') {
                active = ActiveGame::Snake;
                switched = true;
            }
            if ui.key('3') {
                active = ActiveGame::Minesweeper;
                switched = true;
            }
            if switched {
                tetris.sync_tick(tick);
                snake.sync_tick(tick);
            }

            match active {
                ActiveGame::Tetris => {
                    if ui.key('r') {
                        tetris.restart(tick.wrapping_mul(7919).wrapping_add(tetris.rng), tick);
                    }

                    if ui.key('p') && !tetris.game_over {
                        tetris.paused = !tetris.paused;
                        tetris.last_drop_tick = tick;
                    }

                    if !tetris.paused && !tetris.game_over {
                        if ui.key_code(KeyCode::Left) {
                            tetris.try_move(-1, 0);
                        }
                        if ui.key_code(KeyCode::Right) {
                            tetris.try_move(1, 0);
                        }
                        if ui.key_code(KeyCode::Up) {
                            tetris.rotate_cw();
                        }
                        if ui.key_code(KeyCode::Down) {
                            tetris.soft_drop_step();
                            tetris.last_drop_tick = tick;
                        }
                        if ui.key(' ') {
                            tetris.hard_drop();
                            tetris.last_drop_tick = tick;
                        }
                        if tick.saturating_sub(tetris.last_drop_tick) >= tetris.gravity_interval() {
                            tetris.soft_drop_step();
                            tetris.last_drop_tick = tick;
                        }
                    }
                }
                ActiveGame::Snake => {
                    if ui.key('r') {
                        snake.restart(tick.wrapping_mul(7919).wrapping_add(snake.rng), tick);
                    }
                    if ui.key('p') && !snake.game_over {
                        snake.paused = !snake.paused;
                        snake.last_move_tick = tick;
                    }

                    if ui.key_code(KeyCode::Left) {
                        snake.set_direction(Direction::Left);
                    }
                    if ui.key_code(KeyCode::Right) {
                        snake.set_direction(Direction::Right);
                    }
                    if ui.key_code(KeyCode::Up) {
                        snake.set_direction(Direction::Up);
                    }
                    if ui.key_code(KeyCode::Down) {
                        snake.set_direction(Direction::Down);
                    }

                    if !snake.game_over
                        && !snake.paused
                        && tick.saturating_sub(snake.last_move_tick) >= snake.move_interval()
                    {
                        snake.step();
                        snake.last_move_tick = tick;
                    }
                }
                ActiveGame::Minesweeper => {
                    if ui.key('r') {
                        mines.restart(tick.wrapping_mul(7919).wrapping_add(mines.rng));
                    }

                    if ui.key_code(KeyCode::Left) {
                        mines.cursor_x = mines.cursor_x.saturating_sub(1);
                    }
                    if ui.key_code(KeyCode::Right) {
                        mines.cursor_x = (mines.cursor_x + 1).min(MINE_W - 1);
                    }
                    if ui.key_code(KeyCode::Up) {
                        mines.cursor_y = mines.cursor_y.saturating_sub(1);
                    }
                    if ui.key_code(KeyCode::Down) {
                        mines.cursor_y = (mines.cursor_y + 1).min(MINE_H - 1);
                    }

                    if ui.key('f') {
                        mines.toggle_flag_current();
                    }
                    if ui.key(' ') || ui.key_code(KeyCode::Enter) {
                        mines.reveal_current();
                    }
                }
            }

            ui.container().grow(1).col(|ui| {
                ui.spacer();
                render_header(ui, active, theme, theme_names[theme_idx]);

                match active {
                    ActiveGame::Tetris => render_tetris_screen(ui, &tetris, theme),
                    ActiveGame::Snake => render_snake_screen(ui, &snake, theme),
                    ActiveGame::Minesweeper => render_minesweeper_screen(ui, &mines, theme),
                }
                ui.spacer();
            });
        },
    )
}
