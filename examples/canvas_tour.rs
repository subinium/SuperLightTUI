//! Canvas Tour — five canvas/animation demos in one Tabs-driven tour.
//!
//! Mirrors `examples/v020_tour.rs`: each tab embeds the rendering logic
//! of a single canvas/animation demo. Per-frame state (fire pixels,
//! game boards, animation primitives, scroll offsets) lives in the
//! `TourState` struct so switching tabs and back resumes where you left
//! off rather than re-initialising every frame.
//!
//! Run: `cargo run --example canvas_tour --all-features`
//!
//! Tabs:
//!   1. Intro      — overview + key reference
//!   2. Fire       — animated fire effect (top-half-block double-resolution)
//!   3. Game       — tetris/snake/minesweeper switcher
//!   4. Raw Draw   — `ContainerBuilder::draw(|buf, rect|)` primitives
//!   5. Kitty Image — kitty graphics protocol image gallery
//!   6. Anim       — Tween / Spring / Keyframes / Sequence / Stagger
//!
//! Top-level keys:
//!   Tab / Shift-Tab  — cycle focus (tabs bar ↔ demo)
//!   Left / Right     — switch tab when the tabs bar is focused
//!   q / Esc / Ctrl-Q — quit
//!
//! Per-tab keys (documented in each tab's status line):
//!   Fire     — Space pauses; resizes auto-rebuild the buffer
//!   Game     — 1/2/3 switch game; arrows + Space + r + p + t + f
//!   Anim     — Space retargets tween; j/k or arrows nudge spring; r restarts
//!   Kitty    — j/k or arrows scroll; q quits

use std::collections::VecDeque;
use std::time::Duration;

use slt::anim::{ease_in_out_cubic, ease_out_bounce, ease_out_quad};
use slt::widgets::{ScrollState, TabsState};
use slt::{
    Border, Buffer, Color, Context, KeyCode, KeyModifiers, Keyframes, LoopMode, Rect, RunConfig,
    Sequence, Spring, Stagger, Style, Theme, Tween,
};

// ─── Fire constants ─────────────────────────────────────────────────
const FIRE_PALETTE_SIZE: usize = 37;

fn build_fire_palette() -> [Color; FIRE_PALETTE_SIZE] {
    let raw: [(u8, u8, u8); FIRE_PALETTE_SIZE] = [
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
    let mut palette = [Color::Rgb(0, 0, 0); FIRE_PALETTE_SIZE];
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
        if h > 0 {
            for x in 0..w {
                pixels[(h - 1) * w + x] = FIRE_PALETTE_SIZE - 1;
            }
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

    fn color_at(&self, x: usize, y: usize, palette: &[Color; FIRE_PALETTE_SIZE]) -> Color {
        palette[self.pixels[y * self.w + x]]
    }
}

struct FireState {
    fire: Option<Fire>,
    palette: [Color; FIRE_PALETTE_SIZE],
    paused: bool,
}

impl Default for FireState {
    fn default() -> Self {
        Self {
            fire: None,
            palette: build_fire_palette(),
            paused: false,
        }
    }
}

// ─── Game (Tetris / Snake / Minesweeper) ───────────────────────────
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

struct GameState {
    active: ActiveGame,
    theme_idx: usize,
    tetris: TetrisGame,
    snake: SnakeGame,
    mines: MinesweeperGame,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            active: ActiveGame::Tetris,
            theme_idx: 0,
            tetris: TetrisGame::new(1, 0),
            snake: SnakeGame::new(2, 0),
            mines: MinesweeperGame::new(3),
        }
    }
}

// ─── Anim demo state ────────────────────────────────────────────────
struct AnimState {
    progress_target: f64,
    progress_tween: Tween,
    spring_target: f64,
    spring: Spring,
    kf: Keyframes,
    seq: Sequence,
    stagger: Stagger,
    anim_started: bool,
    cb_tween: Tween,
    cb_fired: bool,
}

impl Default for AnimState {
    fn default() -> Self {
        let progress_target = 0.2;
        let mut progress_tween =
            Tween::new(progress_target, progress_target, 1).easing(ease_in_out_cubic);
        progress_tween.reset(0);

        let kf = Keyframes::new(120)
            .stop(0.0, 0.0)
            .stop(0.3, 100.0)
            .stop(0.7, 20.0)
            .stop(1.0, 80.0)
            .loop_mode(LoopMode::PingPong);

        let seq = Sequence::new()
            .then(0.0, 80.0, 40, ease_out_quad)
            .then(80.0, 20.0, 30, ease_in_out_cubic)
            .then(20.0, 60.0, 20, ease_out_bounce)
            .loop_mode(LoopMode::Repeat);

        let stagger = Stagger::new(0.0, 1.0, 30)
            .delay(6)
            .easing(ease_out_quad)
            .items(5)
            .loop_mode(LoopMode::Repeat);

        Self {
            progress_target,
            progress_tween,
            spring_target: 0.0,
            spring: Spring::new(0.0, 0.15, 0.85),
            kf,
            seq,
            stagger,
            anim_started: false,
            cb_tween: Tween::new(0.0, 100.0, 120),
            cb_fired: false,
        }
    }
}

// ─── Tour state ─────────────────────────────────────────────────────
struct TourState {
    tabs: TabsState,
    /// Scroll offset for the active tab body. The Kitty Image tab owns
    /// its own inner scrollable for the gallery; mouse-wheel events
    /// inside that inner region are consumed there, while wheel events
    /// outside the inner gallery (or on tabs with overflowing content)
    /// scroll the tab body.
    tab_scroll: ScrollState,
    fire: FireState,
    game: GameState,
    raw_draw_tick_offset: u64,
    kitty_scroll: ScrollState,
    kitty_images: Vec<(String, Vec<u8>, u32, u32)>,
    anim: AnimState,
}

impl Default for TourState {
    fn default() -> Self {
        Self {
            tabs: TabsState::new(vec![
                "Intro",
                "Fire",
                "Game",
                "Raw Draw",
                "Kitty Image",
                "Anim",
            ]),
            tab_scroll: ScrollState::default(),
            fire: FireState::default(),
            game: GameState::default(),
            raw_draw_tick_offset: 0,
            kitty_scroll: ScrollState::default(),
            kitty_images: build_kitty_images(),
            anim: AnimState::default(),
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut state = TourState::default();
    slt::run_with(
        RunConfig::default()
            .mouse(true)
            .tick_rate(Duration::from_millis(33))
            .max_fps(60),
        move |ui: &mut Context| {
            // Top-of-frame Ctrl-Q quit. Esc and 'q' are NOT consumed
            // here so each tab can claim them locally — Fire/Anim use
            // Esc for quit, Game's `q` is handled inside its tab so
            // text-like keys (1/2/3/r/p/f) are not stolen.
            if ui.key_mod('q', KeyModifiers::CONTROL) {
                ui.quit();
                return;
            }

            let pad = ui.spacing().xs();
            let _ = ui
                .bordered(Border::Rounded)
                .title("Canvas Tour: drawing & animation")
                .p(pad)
                .grow(1)
                .col(|ui| {
                    let _ = ui.tabs(&mut state.tabs);
                    ui.separator();

                    // Wrap the tab body in a vertical scrollable so tabs
                    // with content that overflows the viewport stay
                    // reachable. The Kitty Image tab owns its own inner
                    // scrollable; the auto_scroll_nested logic inside
                    // `scrollable` makes the outer wrapper skip wheel
                    // events that land in the inner gallery, so the two
                    // do not fight each other.
                    let _ = ui.scrollable(&mut state.tab_scroll).grow(1).col(|ui| {
                        match state.tabs.selected {
                            0 => render_intro(ui),
                            1 => render_fire(ui, &mut state.fire),
                            2 => render_game(ui, &mut state.game),
                            3 => render_raw_draw(ui, &mut state.raw_draw_tick_offset),
                            4 => render_kitty(ui, &mut state.kitty_scroll, &state.kitty_images),
                            5 => render_anim(ui, &mut state.anim),
                            _ => {}
                        }
                    });
                });

            // After-dispatch quit — `q`/Esc only quit when the active
            // tab did not consume them. The Game tab claims `q` first
            // because text-style keys belong to it; on every other tab
            // these reach the top level and end the tour.
            if ui.key('q') || ui.key_code(KeyCode::Esc) {
                ui.quit();
            }
        },
    )
}

// ─── Intro tab ──────────────────────────────────────────────────────
fn render_intro(ui: &mut Context) {
    let _ = ui.col(|ui| {
        let pad = ui.spacing().xs();
        ui.text("Canvas / animation tour").bold();
        ui.text("");
        ui.text("Each tab embeds the matching standalone demo's render path.")
            .dim();
        ui.text("Per-frame state (fire pixels, game boards, anim primitives)")
            .dim();
        ui.text("lives in TourState so switching tabs and back resumes cleanly.")
            .dim();
        ui.text("");
        let _ = ui
            .bordered(Border::Single)
            .title("Demos at a glance")
            .p(pad)
            .col(|ui| {
                row_pair(ui, "Fire", "DOOM-style fire palette over half-block cells");
                row_pair(ui, "Game", "Tetris / Snake / Minesweeper switcher (1/2/3)");
                row_pair(
                    ui,
                    "Raw Draw",
                    "ContainerBuilder::draw(|buf, rect|) primitives",
                );
                row_pair(ui, "Kitty", "Kitty graphics protocol image gallery");
                row_pair(
                    ui,
                    "Anim",
                    "Tween / Spring / Keyframes / Sequence / Stagger",
                );
            });
        ui.text("");
        ui.text("Navigation: Tab focuses the bar, Left/Right switch tabs.")
            .fg(Color::Cyan);
        ui.text("Quit: q / Esc / Ctrl-Q (some tabs claim these locally).")
            .fg(Color::Cyan);
    });
}

fn row_pair(ui: &mut Context, label: &str, desc: &str) {
    let _ = ui.row_gap(1, |ui| {
        ui.text(format!("{label:<10}")).bold().fg(Color::Cyan);
        ui.text(desc).dim();
    });
}

// ─── Fire tab ───────────────────────────────────────────────────────
fn render_fire(ui: &mut Context, state: &mut FireState) {
    if ui.key(' ') {
        state.paused = !state.paused;
    }

    let term_w = ui.width() as usize;
    let term_h = ui.height() as usize;

    // Reserve a couple of cell-rows at the bottom for the help line so
    // the fire grid does not collide with the outer border footer.
    let help_rows = 2_usize;
    let canvas_h = term_h.saturating_sub(help_rows + 2).max(2);
    let canvas_w = term_w.saturating_sub(2).max(2);

    let fire_w = canvas_w;
    let fire_h = canvas_h * 2;

    let fire = state.fire.get_or_insert_with(|| Fire::new(fire_w, fire_h));
    if fire.w != fire_w || fire.h != fire_h {
        *fire = Fire::new(fire_w, fire_h);
    }

    if !state.paused {
        for _ in 0..2 {
            fire.step();
        }
    }

    let palette = &state.palette;

    let _ = ui.col(|ui| {
        for row in 0..canvas_h {
            let top_y = row * 2;
            let bot_y = top_y + 1;

            let _ = ui.row(|ui| {
                let mut run_start = 0;
                let mut cur_top = fire.color_at(0, top_y, palette);
                let mut cur_bot = fire.color_at(0, bot_y, palette);

                for col in 1..=canvas_w {
                    let (t, b) = if col < canvas_w {
                        (
                            fire.color_at(col, top_y, palette),
                            fire.color_at(col, bot_y, palette),
                        )
                    } else {
                        (Color::Reset, Color::Reset)
                    };

                    if col == canvas_w || t != cur_top || b != cur_bot {
                        let len = col - run_start;
                        let s: String = (0..len).map(|_| '\u{2580}').collect();
                        ui.styled(s, Style::new().fg(cur_top).bg(cur_bot));
                        run_start = col;
                        cur_top = t;
                        cur_bot = b;
                    }
                }
            });
        }
        ui.text(if state.paused {
            "PAUSED — Space resume | q/Esc quit"
        } else {
            "Space pause | q/Esc quit"
        })
        .dim()
        .fg(Color::Cyan);
    });
}

// ─── Game tab (Tetris / Snake / Minesweeper) ───────────────────────
fn render_game(ui: &mut Context, state: &mut GameState) {
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

    if ui.key('t') {
        state.theme_idx = (state.theme_idx + 1) % themes.len();
    }
    let theme = themes[state.theme_idx]();
    let theme_name = theme_names[state.theme_idx];
    let tick = ui.tick();

    let mut switched = false;
    if ui.key('1') {
        state.active = ActiveGame::Tetris;
        switched = true;
    }
    if ui.key('2') {
        state.active = ActiveGame::Snake;
        switched = true;
    }
    if ui.key('3') {
        state.active = ActiveGame::Minesweeper;
        switched = true;
    }
    if switched {
        state.tetris.sync_tick(tick);
        state.snake.sync_tick(tick);
    }

    match state.active {
        ActiveGame::Tetris => {
            if ui.key('r') {
                state
                    .tetris
                    .restart(tick.wrapping_mul(7919).wrapping_add(state.tetris.rng), tick);
            }

            if ui.key('p') && !state.tetris.game_over {
                state.tetris.paused = !state.tetris.paused;
                state.tetris.last_drop_tick = tick;
            }

            if !state.tetris.paused && !state.tetris.game_over {
                if ui.key_code(KeyCode::Left) {
                    state.tetris.try_move(-1, 0);
                }
                if ui.key_code(KeyCode::Right) {
                    state.tetris.try_move(1, 0);
                }
                if ui.key_code(KeyCode::Up) {
                    state.tetris.rotate_cw();
                }
                if ui.key_code(KeyCode::Down) {
                    state.tetris.soft_drop_step();
                    state.tetris.last_drop_tick = tick;
                }
                if ui.key(' ') {
                    state.tetris.hard_drop();
                    state.tetris.last_drop_tick = tick;
                }
                if tick.saturating_sub(state.tetris.last_drop_tick)
                    >= state.tetris.gravity_interval()
                {
                    state.tetris.soft_drop_step();
                    state.tetris.last_drop_tick = tick;
                }
            }
        }
        ActiveGame::Snake => {
            if ui.key('r') {
                state
                    .snake
                    .restart(tick.wrapping_mul(7919).wrapping_add(state.snake.rng), tick);
            }
            if ui.key('p') && !state.snake.game_over {
                state.snake.paused = !state.snake.paused;
                state.snake.last_move_tick = tick;
            }

            if ui.key_code(KeyCode::Left) {
                state.snake.set_direction(Direction::Left);
            }
            if ui.key_code(KeyCode::Right) {
                state.snake.set_direction(Direction::Right);
            }
            if ui.key_code(KeyCode::Up) {
                state.snake.set_direction(Direction::Up);
            }
            if ui.key_code(KeyCode::Down) {
                state.snake.set_direction(Direction::Down);
            }

            if !state.snake.game_over
                && !state.snake.paused
                && tick.saturating_sub(state.snake.last_move_tick) >= state.snake.move_interval()
            {
                state.snake.step();
                state.snake.last_move_tick = tick;
            }
        }
        ActiveGame::Minesweeper => {
            if ui.key('r') {
                state
                    .mines
                    .restart(tick.wrapping_mul(7919).wrapping_add(state.mines.rng));
            }

            if ui.key_code(KeyCode::Left) {
                state.mines.cursor_x = state.mines.cursor_x.saturating_sub(1);
            }
            if ui.key_code(KeyCode::Right) {
                state.mines.cursor_x = (state.mines.cursor_x + 1).min(MINE_W - 1);
            }
            if ui.key_code(KeyCode::Up) {
                state.mines.cursor_y = state.mines.cursor_y.saturating_sub(1);
            }
            if ui.key_code(KeyCode::Down) {
                state.mines.cursor_y = (state.mines.cursor_y + 1).min(MINE_H - 1);
            }

            if ui.key('f') {
                state.mines.toggle_flag_current();
            }
            if ui.key(' ') || ui.key_code(KeyCode::Enter) {
                state.mines.reveal_current();
            }
        }
    }

    render_game_header(ui, state.active, theme, theme_name);

    let _ = ui.container().grow(1).col(|ui| {
        ui.spacer();
        match state.active {
            ActiveGame::Tetris => render_tetris_screen(ui, &state.tetris, theme),
            ActiveGame::Snake => render_snake_screen(ui, &state.snake, theme),
            ActiveGame::Minesweeper => render_minesweeper_screen(ui, &state.mines, theme),
        }
        ui.spacer();
    });
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
        let _ = ui.row(|ui| {
            for x in 0..BOARD_W {
                if let Some(kind) = game.board[y][x] {
                    ui.styled("\u{2588}\u{2588}", Style::new().fg(piece_color(kind)));
                } else if active_at(game, x, y) {
                    ui.styled(
                        "\u{2588}\u{2588}",
                        Style::new().fg(piece_color(game.active.kind)),
                    );
                } else if ghost_y != game.active.y && ghost_at(game, ghost_y, x, y) {
                    ui.styled("\u{2591}\u{2591}", Style::new().fg(theme.text_dim));
                } else {
                    ui.styled("\u{00B7} ", Style::new().fg(theme.text_dim));
                }
            }
        });
    }
}

fn render_tetris_next(ui: &mut Context, kind: usize) {
    let color = piece_color(kind);
    for y in 0..4 {
        let _ = ui.row(|ui| {
            for x in 0..4 {
                if next_preview_at(kind, x, y) {
                    ui.styled("\u{2588}\u{2588}", Style::new().fg(color));
                } else {
                    ui.text("  ");
                }
            }
        });
    }
}

fn render_snake_board(ui: &mut Context, game: &SnakeGame, theme: Theme) {
    for y in 0..SNAKE_H {
        let _ = ui.row(|ui| {
            for x in 0..SNAKE_W {
                if (x, y) == game.food {
                    ui.styled("\u{25CF} ", Style::new().fg(theme.accent));
                } else if let Some(&(hx, hy)) = game.snake.front() {
                    if (x, y) == (hx, hy) {
                        ui.styled("\u{2588}\u{2588}", Style::new().fg(theme.success));
                    } else if game.snake.iter().any(|&(sx, sy)| (sx, sy) == (x, y)) {
                        ui.styled("\u{2588}\u{2588}", Style::new().fg(theme.primary));
                    } else {
                        ui.styled("\u{00B7} ", Style::new().fg(theme.text_dim));
                    }
                } else {
                    ui.styled("\u{00B7} ", Style::new().fg(theme.text_dim));
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
        let _ = ui.row(|ui| {
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
                    "\u{2691}"
                } else {
                    style = style.fg(theme.text_dim);
                    "\u{00B7}"
                };

                if x == game.cursor_x && y == game.cursor_y {
                    style = style.reversed();
                }
                ui.styled(format!("{} ", content), style);
            }
        });
    }
}

fn render_game_header(ui: &mut Context, active: ActiveGame, theme: Theme, theme_name: &str) {
    let _ = ui
        .container()
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

    let _ = ui
        .bordered(Border::Rounded)
        .title_styled(" T E T R I S ", Style::new().bold().fg(theme.primary))
        .border_style(Style::new().fg(theme.border))
        .bg(theme.surface)
        .w(game_w)
        .ml(left)
        .col(|ui| {
            let _ = ui.row_gap(1, |ui| {
                let _ = ui
                    .bordered(Border::Single)
                    .border_style(Style::new().fg(theme.border))
                    .col(|ui| {
                        render_tetris_board(ui, game, theme);
                    });

                let _ = ui.container().w(20).col(|ui| {
                    let _ = ui
                        .container()
                        .bg(theme.surface)
                        .border(Border::Rounded)
                        .title("NEXT")
                        .border_style(Style::new().fg(theme.border))
                        .px(2)
                        .py(1)
                        .col(|ui| {
                            render_tetris_next(ui, game.next_kind);
                        });

                    let _ = ui
                        .container()
                        .bg(theme.surface)
                        .border(Border::Rounded)
                        .title("SCORE")
                        .border_style(Style::new().fg(theme.border))
                        .px(1)
                        .col(|ui| {
                            ui.text(format_score(game.score))
                                .bold()
                                .fg(theme.surface_text);
                            let _ = ui.row(|ui| {
                                ui.text("LEVEL").fg(theme.text_dim);
                                ui.spacer();
                                ui.text(format!("{}", game.level)).bold().fg(theme.primary);
                            });
                            let _ = ui.row(|ui| {
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
                    ui.text(" Arrows Move/Rotate").fg(theme.text_dim);
                    ui.text(" SPC Drop  P Pause").fg(theme.text_dim);
                    ui.text(" R Reset   Q Quit").fg(theme.text_dim);
                });
            });
        });
}

fn render_snake_screen(ui: &mut Context, game: &SnakeGame, theme: Theme) {
    let game_w = 58_u32;
    let left = ui.width().saturating_sub(game_w) / 2;

    let _ = ui
        .container()
        .bg(theme.surface)
        .border(Border::Rounded)
        .title_styled(" S N A K E ", Style::new().bold().fg(theme.primary))
        .border_style(Style::new().fg(theme.border))
        .w(game_w)
        .ml(left)
        .col(|ui| {
            let _ = ui.row_gap(1, |ui| {
                let _ = ui
                    .bordered(Border::Single)
                    .border_style(Style::new().fg(theme.border))
                    .col(|ui| {
                        render_snake_board(ui, game, theme);
                    });

                let _ = ui
                    .container()
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
                        ui.text("Arrows Move").fg(theme.text_dim);
                        ui.text("P Pause").fg(theme.text_dim);
                        ui.text("R Restart").fg(theme.text_dim);
                    });
            });
        });
}

fn render_minesweeper_screen(ui: &mut Context, game: &MinesweeperGame, theme: Theme) {
    let game_w = 56_u32;
    let left = ui.width().saturating_sub(game_w) / 2;

    let _ = ui
        .container()
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
            let _ = ui.row_gap(1, |ui| {
                let _ = ui
                    .bordered(Border::Single)
                    .border_style(Style::new().fg(theme.border))
                    .col(|ui| {
                        render_mine_board(ui, game, theme);
                    });

                let _ = ui
                    .container()
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

// ─── Raw Draw tab ───────────────────────────────────────────────────
fn render_raw_draw(ui: &mut Context, tick_offset: &mut u64) {
    *tick_offset = ui.tick();
    let t_offset = *tick_offset;

    let _ = ui
        .bordered(Border::Rounded)
        .title("draw_raw demo")
        .p(1)
        .gap(1)
        .grow(1)
        .col(|ui| {
            ui.text("Direct buffer access via ContainerBuilder::draw()")
                .bold();
            ui.text("Each tile owns its rect; closures run at flush time.")
                .dim();

            let _ = ui.row(|ui| {
                ui.bordered(Border::Single)
                    .title("Gradient")
                    .w(34)
                    .h(12)
                    .draw(|buf: &mut Buffer, rect: Rect| {
                        for y in rect.y..rect.bottom() {
                            for x in rect.x..rect.right() {
                                let r = ((x - rect.x) as f32 / rect.width as f32 * 255.0) as u8;
                                let b = ((y - rect.y) as f32 / rect.height as f32 * 255.0) as u8;
                                buf.set_char(
                                    x,
                                    y,
                                    '\u{2588}',
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
                        let t = t_offset as f64 * 0.05;
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
                                    '\u{2593}',
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
                        let chars = [
                            '\u{250C}', '\u{2500}', '\u{2510}', '\u{2502}', ' ', '\u{2502}',
                            '\u{2514}', '\u{2500}', '\u{2518}',
                        ];
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

            ui.text("q/Esc quit").dim().fg(Color::Cyan);
        });
}

// ─── Kitty Image tab ────────────────────────────────────────────────
fn render_kitty(
    ui: &mut Context,
    scroll: &mut ScrollState,
    images: &[(String, Vec<u8>, u32, u32)],
) {
    if ui.key('j') || ui.key_code(KeyCode::Down) {
        scroll.offset = scroll.offset.saturating_add(2);
    }
    if ui.key('k') || ui.key_code(KeyCode::Up) {
        scroll.offset = scroll.offset.saturating_sub(2);
    }

    let _ = ui
        .bordered(Border::Rounded)
        .title("Kitty Image Gallery")
        .grow(1)
        .col(|ui| {
            let _ = ui.row(|ui| {
                ui.text("j/k or Up/Down scroll | q/Esc quit").dim();
                ui.spacer();
                ui.text(format!("offset: {}", scroll.offset)).dim();
            });

            let _ = ui.scrollable(scroll).grow(1).col(|ui| {
                for (i, (label, rgba, w, h)) in images.iter().enumerate() {
                    ui.text(format!("{}. {}", i + 1, label))
                        .bold()
                        .fg(Color::Yellow);
                    let _ = ui.kitty_image(rgba, *w, *h, 30, 8);
                    if i < images.len() - 1 {
                        ui.separator();
                    }
                }
                ui.text("");
                ui.text("--- End of gallery ---").dim();
            });
        });
}

fn build_kitty_images() -> Vec<(String, Vec<u8>, u32, u32)> {
    vec![
        gradient_image("Red-Blue", 120, 60, (255, 60, 60), (60, 60, 255)),
        gradient_image("Green-Yellow", 120, 60, (60, 255, 60), (255, 255, 60)),
        checkerboard_image("Checkerboard", 120, 60, 12),
        gradient_image("Cyan-Magenta", 120, 60, (60, 255, 255), (255, 60, 255)),
        stripe_image("Rainbow", 120, 60),
        gradient_image("White-Black", 120, 60, (240, 240, 240), (20, 20, 20)),
        gradient_image("Orange-Purple", 120, 60, (255, 140, 0), (128, 0, 128)),
        checkerboard_image("Fine Grid", 120, 60, 6),
        gradient_image("Teal-Rose", 120, 60, (0, 128, 128), (255, 100, 130)),
        stripe_image("Rainbow 2", 120, 60),
    ]
}

fn gradient_image(
    label: &str,
    width: u32,
    height: u32,
    from: (u8, u8, u8),
    to: (u8, u8, u8),
) -> (String, Vec<u8>, u32, u32) {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let t = y as f64 / height.max(1) as f64;
        let r = lerp(from.0, to.0, t);
        let g = lerp(from.1, to.1, t);
        let b = lerp(from.2, to.2, t);
        for _x in 0..width {
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    (label.to_string(), rgba, width, height)
}

fn checkerboard_image(
    label: &str,
    width: u32,
    height: u32,
    cell_size: u32,
) -> (String, Vec<u8>, u32, u32) {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let is_dark = ((x / cell_size) + (y / cell_size)) % 2 == 0;
            let v = if is_dark { 40u8 } else { 200u8 };
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
    }
    (label.to_string(), rgba, width, height)
}

fn stripe_image(label: &str, width: u32, height: u32) -> (String, Vec<u8>, u32, u32) {
    let colors: [(u8, u8, u8); 7] = [
        (255, 0, 0),
        (255, 127, 0),
        (255, 255, 0),
        (0, 255, 0),
        (0, 0, 255),
        (75, 0, 130),
        (148, 0, 211),
    ];
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    let stripe_h = height / colors.len() as u32;
    for y in 0..height {
        let idx = ((y / stripe_h.max(1)) as usize).min(colors.len() - 1);
        let (r, g, b) = colors[idx];
        for _x in 0..width {
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    (label.to_string(), rgba, width, height)
}

fn lerp(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).clamp(0.0, 255.0) as u8
}

// ─── Anim tab ───────────────────────────────────────────────────────
fn render_anim(ui: &mut Context, state: &mut AnimState) {
    if ui.key(' ') {
        let current = state.progress_tween.value(ui.tick());
        state.progress_target = if state.progress_target < 0.5 {
            0.9
        } else {
            0.1
        };
        state.progress_tween =
            Tween::new(current, state.progress_target, 12).easing(ease_in_out_cubic);
        state.progress_tween.reset(ui.tick());
    }

    if ui.key('r') {
        let t = ui.tick();
        state.kf.reset(t);
        state.seq.reset(t);
        state.stagger.reset(t);
        state.anim_started = true;
        state.progress_tween = Tween::new(0.1, 0.9, 12).easing(ease_in_out_cubic);
        state.progress_tween.reset(t);
        state.spring_target = 0.0;
        state.spring = Spring::new(0.0, 0.15, 0.85);
    }

    if !state.anim_started {
        let t = ui.tick();
        state.kf.reset(t);
        state.seq.reset(t);
        state.stagger.reset(t);
        state.anim_started = true;
    }

    if ui.key_code(KeyCode::Up) || ui.key('k') {
        state.spring_target += 10.0;
        state.spring.set_target(state.spring_target);
    }
    if ui.key_code(KeyCode::Down) || ui.key('j') {
        state.spring_target -= 10.0;
        state.spring.set_target(state.spring_target);
    }

    state.spring.tick();
    let progress = state.progress_tween.value(ui.tick());

    let _ = ui
        .bordered(Border::Rounded)
        .title("Animation Primitives")
        .p(1)
        .gap(1)
        .grow(1)
        .col(|ui| {
            let _ = ui
                .bordered(Border::Single)
                .title("Tween")
                .p(1)
                .gap(1)
                .col(|ui| {
                    ui.text("Press Space to retarget");
                    let _ = ui.progress(progress);
                    ui.text(format!(
                        "value {:.2} -> target {:.2} | done {}",
                        progress,
                        state.progress_target,
                        state.progress_tween.is_done()
                    ));
                });

            let _ = ui
                .bordered(Border::Single)
                .title("Spring")
                .p(1)
                .gap(1)
                .col(|ui| {
                    ui.text("Up/k +10, Down/j -10");
                    ui.text(format!(
                        "value {:.2} | target {:.2} | settled {}",
                        state.spring.value(),
                        state.spring_target,
                        state.spring.is_settled()
                    ));
                });

            let _ = ui
                .bordered(Border::Single)
                .title("Keyframes")
                .p(1)
                .gap(1)
                .col(|ui| {
                    let kf_val = state.kf.value(ui.tick());
                    let _ = ui.progress(kf_val / 100.0);
                    ui.text(format!(
                        "value {:.1} | done {} | mode PingPong",
                        kf_val,
                        state.kf.is_done()
                    ));
                    ui.text("4 stops: 0->100->20->80").dim();
                });

            let _ = ui
                .bordered(Border::Single)
                .title("Sequence")
                .p(1)
                .gap(1)
                .col(|ui| {
                    let seq_val = state.seq.value(ui.tick());
                    let _ = ui.progress(seq_val / 100.0);
                    ui.text(format!(
                        "value {:.1} | done {} | mode Repeat",
                        seq_val,
                        state.seq.is_done()
                    ));
                    ui.text("3 chained: 0->80->20->60").dim();
                });

            let _ = ui
                .bordered(Border::Single)
                .title("Stagger")
                .p(1)
                .gap(1)
                .col(|ui| {
                    let labels = ["Item A", "Item B", "Item C", "Item D", "Item E"];
                    for (i, label) in labels.iter().enumerate() {
                        let val = state.stagger.value(ui.tick(), i);
                        let _ = ui.row(|ui| {
                            ui.text(format!("{label}:"));
                            let _ = ui.progress(val);
                        });
                    }
                    ui.text("5 items, 6-tick delay each").dim();
                });

            let accent = ui.theme().accent;
            ui.text("Callback").bold().fg(accent);
            let val = state.cb_tween.value(ui.tick());
            let _ = ui.progress(val / 100.0);
            if state.cb_tween.is_done() && !state.cb_fired {
                state.cb_fired = true;
            }
            let _ = ui.row_gap(1, |ui| {
                if state.cb_fired {
                    ui.text("on_complete fired!").fg(Color::Green);
                }
                if ui.button("Restart").clicked {
                    state.cb_tween.reset(ui.tick());
                    state.cb_fired = false;
                }
            });

            ui.text("space tween | up/down spring | r restart all | q/Esc quit")
                .dim()
                .fg(Color::Cyan);
        });
}
