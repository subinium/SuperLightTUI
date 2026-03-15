//! # SLT — Super Light TUI
//!
//! Immediate-mode terminal UI for Rust. Two dependencies. Zero `unsafe`.
//!
//! SLT gives you an egui-style API for terminals: your closure runs each frame,
//! you describe your UI, and SLT handles layout, diffing, and rendering.
//!
//! ## Quick Start
//!
//! ```no_run
//! fn main() -> std::io::Result<()> {
//!     slt::run(|ui| {
//!         ui.text("hello, world");
//!     })
//! }
//! ```
//!
//! ## Features
//!
//! - **Flexbox layout** — `row()`, `col()`, `gap()`, `grow()`
//! - **30+ built-in widgets** — input, textarea, table, list, tabs, button, checkbox, toggle, spinner, progress, toast, separator, help bar, scrollable, chart, bar chart, sparkline, histogram, canvas, grid, select, radio, multi-select, tree, virtual list, command palette, markdown
//! - **Styling** — bold, italic, dim, underline, 256 colors, RGB
//! - **Mouse** — click, hover, drag-to-scroll
//! - **Focus** — automatic Tab/Shift+Tab cycling
//! - **Theming** — dark/light presets or custom
//! - **Animation** — tween and spring primitives with 9 easing functions
//! - **Inline mode** — render below your prompt, no alternate screen
//! - **Async** — optional tokio integration via `async` feature
//! - **Layout debugger** — F12 to visualize container bounds
//!
//! ## Feature Flags
//!
//! | Flag | Description |
//! |------|-------------|
//! | `async` | Enable `run_async()` with tokio channel-based message passing |
//! | `serde` | Enable Serialize/Deserialize for Style, Color, Theme, and layout types |

pub mod anim;
pub mod buffer;
pub mod cell;
pub mod chart;
pub mod context;
pub mod event;
pub mod halfblock;
pub mod layout;
pub mod rect;
pub mod style;
mod terminal;
pub mod test_utils;
pub mod widgets;

use std::io;
use std::io::IsTerminal;
use std::sync::Once;
use std::time::{Duration, Instant};

use terminal::{InlineTerminal, Terminal, TerminalBackend};

pub use crate::test_utils::{EventBuilder, TestBackend};
pub use anim::{Keyframes, LoopMode, Sequence, Spring, Stagger, Tween};
pub use buffer::Buffer;
pub use chart::{
    Axis, ChartBuilder, ChartConfig, ChartRenderer, Dataset, DatasetEntry, GraphType,
    HistogramBuilder, LegendPosition, Marker,
};
pub use context::{Bar, BarDirection, BarGroup, CanvasContext, Context, Response, State, Widget};
pub use event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseKind};
pub use halfblock::HalfBlockImage;
pub use rect::Rect;
pub use style::{
    Align, Border, BorderSides, Breakpoint, Color, ColorDepth, Constraints, ContainerStyle,
    Justify, Margin, Modifiers, Padding, Style, Theme, ThemeBuilder,
};
pub use widgets::{
    AlertLevel, ApprovalAction, ButtonVariant, CommandPaletteState, ContextItem, FormField,
    FormState, ListState, MultiSelectState, PaletteCommand, RadioState, ScrollState, SelectState,
    SpinnerState, StreamingTextState, TableState, TabsState, TextInputState, TextareaState,
    ToastLevel, ToastMessage, ToastState, ToolApprovalState, TreeNode, TreeState, Trend,
};

static PANIC_HOOK_ONCE: Once = Once::new();

fn install_panic_hook() {
    PANIC_HOOK_ONCE.call_once(|| {
        let original = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = crossterm::terminal::disable_raw_mode();
            let mut stdout = io::stdout();
            let _ = crossterm::execute!(
                stdout,
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::cursor::Show,
                crossterm::event::DisableMouseCapture,
                crossterm::event::DisableBracketedPaste,
                crossterm::style::ResetColor,
                crossterm::style::SetAttribute(crossterm::style::Attribute::Reset)
            );

            // Print friendly panic header
            eprintln!("\n\x1b[1;31m━━━ SLT Panic ━━━\x1b[0m\n");

            // Print location if available
            if let Some(location) = panic_info.location() {
                eprintln!(
                    "\x1b[90m{}:{}:{}\x1b[0m",
                    location.file(),
                    location.line(),
                    location.column()
                );
            }

            // Print message
            if let Some(msg) = panic_info.payload().downcast_ref::<&str>() {
                eprintln!("\x1b[1m{}\x1b[0m", msg);
            } else if let Some(msg) = panic_info.payload().downcast_ref::<String>() {
                eprintln!("\x1b[1m{}\x1b[0m", msg);
            }

            eprintln!(
                "\n\x1b[90mTerminal state restored. Report bugs at https://github.com/subinium/SuperLightTUI/issues\x1b[0m\n"
            );

            original(panic_info);
        }));
    });
}

/// Configuration for a TUI run loop.
///
/// Pass to [`run_with`] or [`run_inline_with`] to customize behavior.
/// Use [`Default::default()`] for sensible defaults (16ms tick / 60fps, no mouse, dark theme).
///
/// # Example
///
/// ```no_run
/// use slt::{RunConfig, Theme};
/// use std::time::Duration;
///
/// let config = RunConfig {
///     tick_rate: Duration::from_millis(50),
///     mouse: true,
///     kitty_keyboard: false,
///     theme: Theme::light(),
///     color_depth: None,
///     max_fps: Some(60),
/// };
/// ```
#[must_use = "configure loop behavior before passing to run_with or run_inline_with"]
pub struct RunConfig {
    /// How long to wait for input before triggering a tick with no events.
    ///
    /// Lower values give smoother animations at the cost of more CPU usage.
    /// Defaults to 16ms (60fps).
    pub tick_rate: Duration,
    /// Whether to enable mouse event reporting.
    ///
    /// When `true`, the terminal captures mouse clicks, scrolls, and movement.
    /// Defaults to `false`.
    pub mouse: bool,
    /// Whether to enable the Kitty keyboard protocol for enhanced input.
    ///
    /// When `true`, enables disambiguated key events, key release events,
    /// and modifier-only key reporting on supporting terminals (kitty, Ghostty, WezTerm).
    /// Terminals that don't support it silently ignore the request.
    /// Defaults to `false`.
    pub kitty_keyboard: bool,
    /// The color theme applied to all widgets automatically.
    ///
    /// Defaults to [`Theme::dark()`].
    pub theme: Theme,
    /// Color depth override.
    ///
    /// `None` means auto-detect from `$COLORTERM` and `$TERM` environment
    /// variables. Set explicitly to force a specific color depth regardless
    /// of terminal capabilities.
    pub color_depth: Option<ColorDepth>,
    /// Optional maximum frame rate.
    ///
    /// `None` means unlimited frame rate. `Some(fps)` sleeps at the end of each
    /// loop iteration to target that frame time.
    pub max_fps: Option<u32>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            tick_rate: Duration::from_millis(16),
            mouse: false,
            kitty_keyboard: false,
            theme: Theme::dark(),
            color_depth: None,
            max_fps: Some(60),
        }
    }
}

pub(crate) struct FrameState {
    pub hook_states: Vec<Box<dyn std::any::Any>>,
    pub focus_index: usize,
    pub prev_focus_count: usize,
    pub tick: u64,
    pub prev_scroll_infos: Vec<(u32, u32)>,
    pub prev_scroll_rects: Vec<rect::Rect>,
    pub prev_hit_map: Vec<rect::Rect>,
    pub prev_group_rects: Vec<(String, rect::Rect)>,
    pub prev_content_map: Vec<(rect::Rect, rect::Rect)>,
    pub prev_focus_rects: Vec<(usize, rect::Rect)>,
    pub prev_focus_groups: Vec<Option<String>>,
    pub last_mouse_pos: Option<(u32, u32)>,
    pub prev_modal_active: bool,
    pub debug_mode: bool,
    pub fps_ema: f32,
    pub selection: terminal::SelectionState,
}

impl Default for FrameState {
    fn default() -> Self {
        Self {
            hook_states: Vec::new(),
            focus_index: 0,
            prev_focus_count: 0,
            tick: 0,
            prev_scroll_infos: Vec::new(),
            prev_scroll_rects: Vec::new(),
            prev_hit_map: Vec::new(),
            prev_group_rects: Vec::new(),
            prev_content_map: Vec::new(),
            prev_focus_rects: Vec::new(),
            prev_focus_groups: Vec::new(),
            last_mouse_pos: None,
            prev_modal_active: false,
            debug_mode: false,
            fps_ema: 0.0,
            selection: terminal::SelectionState::default(),
        }
    }
}

/// Run the TUI loop with default configuration.
///
/// Enters alternate screen mode, runs `f` each frame, and exits cleanly on
/// Ctrl+C or when [`Context::quit`] is called.
///
/// # Example
///
/// ```no_run
/// fn main() -> std::io::Result<()> {
///     slt::run(|ui| {
///         ui.text("Press Ctrl+C to exit");
///     })
/// }
/// ```
pub fn run(f: impl FnMut(&mut Context)) -> io::Result<()> {
    run_with(RunConfig::default(), f)
}

/// Run the TUI loop with custom configuration.
///
/// Like [`run`], but accepts a [`RunConfig`] to control tick rate, mouse
/// support, and theming.
///
/// # Example
///
/// ```no_run
/// use slt::{RunConfig, Theme};
///
/// fn main() -> std::io::Result<()> {
///     slt::run_with(
///         RunConfig { theme: Theme::light(), ..Default::default() },
///         |ui| {
///             ui.text("Light theme!");
///         },
///     )
/// }
/// ```
pub fn run_with(config: RunConfig, mut f: impl FnMut(&mut Context)) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    install_panic_hook();
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = Terminal::new(config.mouse, config.kitty_keyboard, color_depth)?;
    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start);
            continue;
        }

        if !run_frame(&mut term, &mut state, &config, &events, &mut f)? {
            break;
        }

        events.clear();
        if crossterm::event::poll(config.tick_rate)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if is_ctrl_c(&ev) {
                    break;
                }
                if let Event::Resize(_, _) = &ev {
                    TerminalBackend::handle_resize(&mut term)?;
                }
                events.push(ev);
            }

            while crossterm::event::poll(Duration::ZERO)? {
                let raw = crossterm::event::read()?;
                if let Some(ev) = event::from_crossterm(raw) {
                    if is_ctrl_c(&ev) {
                        return Ok(());
                    }
                    if let Event::Resize(_, _) = &ev {
                        TerminalBackend::handle_resize(&mut term)?;
                    }
                    events.push(ev);
                }
            }

            for ev in &events {
                if matches!(
                    ev,
                    Event::Key(event::KeyEvent {
                        code: KeyCode::F(12),
                        kind: event::KeyEventKind::Press,
                        ..
                    })
                ) {
                    state.debug_mode = !state.debug_mode;
                }
            }
        }

        update_last_mouse_pos(&mut state, &events);

        if events.iter().any(|e| matches!(e, Event::Resize(_, _))) {
            clear_frame_layout_cache(&mut state);
        }

        sleep_for_fps_cap(config.max_fps, frame_start);
    }

    Ok(())
}

/// Run the TUI loop asynchronously with default configuration.
///
/// Requires the `async` feature. Spawns the render loop in a blocking thread
/// and returns a [`tokio::sync::mpsc::Sender`] you can use to push messages
/// from async tasks into the UI closure.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "async")]
/// # async fn example() -> std::io::Result<()> {
/// let tx = slt::run_async::<String>(|ui, messages| {
///     for msg in messages.drain(..) {
///         ui.text(msg);
///     }
/// })?;
/// tx.send("hello from async".to_string()).await.ok();
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "async")]
pub fn run_async<M: Send + 'static>(
    f: impl FnMut(&mut Context, &mut Vec<M>) + Send + 'static,
) -> io::Result<tokio::sync::mpsc::Sender<M>> {
    run_async_with(RunConfig::default(), f)
}

/// Run the TUI loop asynchronously with custom configuration.
///
/// Requires the `async` feature. Like [`run_async`], but accepts a
/// [`RunConfig`] to control tick rate, mouse support, and theming.
///
/// Returns a [`tokio::sync::mpsc::Sender`] for pushing messages into the UI.
#[cfg(feature = "async")]
pub fn run_async_with<M: Send + 'static>(
    config: RunConfig,
    f: impl FnMut(&mut Context, &mut Vec<M>) + Send + 'static,
) -> io::Result<tokio::sync::mpsc::Sender<M>> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let handle =
        tokio::runtime::Handle::try_current().map_err(|err| io::Error::other(err.to_string()))?;

    handle.spawn_blocking(move || {
        let _ = run_async_loop(config, f, rx);
    });

    Ok(tx)
}

#[cfg(feature = "async")]
fn run_async_loop<M: Send + 'static>(
    config: RunConfig,
    mut f: impl FnMut(&mut Context, &mut Vec<M>) + Send,
    mut rx: tokio::sync::mpsc::Receiver<M>,
) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    install_panic_hook();
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = Terminal::new(config.mouse, config.kitty_keyboard, color_depth)?;
    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        let mut messages: Vec<M> = Vec::new();
        while let Ok(message) = rx.try_recv() {
            messages.push(message);
        }

        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start);
            continue;
        }

        let mut render = |ctx: &mut Context| {
            f(ctx, &mut messages);
        };
        if !run_frame(&mut term, &mut state, &config, &events, &mut render)? {
            break;
        }

        events.clear();
        if crossterm::event::poll(config.tick_rate)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if is_ctrl_c(&ev) {
                    break;
                }
                if let Event::Resize(_, _) = &ev {
                    TerminalBackend::handle_resize(&mut term)?;
                    clear_frame_layout_cache(&mut state);
                }
                events.push(ev);
            }

            while crossterm::event::poll(Duration::ZERO)? {
                let raw = crossterm::event::read()?;
                if let Some(ev) = event::from_crossterm(raw) {
                    if is_ctrl_c(&ev) {
                        return Ok(());
                    }
                    if let Event::Resize(_, _) = &ev {
                        TerminalBackend::handle_resize(&mut term)?;
                        clear_frame_layout_cache(&mut state);
                    }
                    events.push(ev);
                }
            }
        }

        update_last_mouse_pos(&mut state, &events);

        sleep_for_fps_cap(config.max_fps, frame_start);
    }

    Ok(())
}

/// Run the TUI in inline mode with default configuration.
///
/// Renders `height` rows directly below the current cursor position without
/// entering alternate screen mode. Useful for CLI tools that want a small
/// interactive widget below the prompt.
///
/// # Example
///
/// ```no_run
/// fn main() -> std::io::Result<()> {
///     slt::run_inline(3, |ui| {
///         ui.text("Inline TUI — no alternate screen");
///     })
/// }
/// ```
pub fn run_inline(height: u32, f: impl FnMut(&mut Context)) -> io::Result<()> {
    run_inline_with(height, RunConfig::default(), f)
}

/// Run the TUI in inline mode with custom configuration.
///
/// Like [`run_inline`], but accepts a [`RunConfig`] to control tick rate,
/// mouse support, and theming.
pub fn run_inline_with(
    height: u32,
    config: RunConfig,
    mut f: impl FnMut(&mut Context),
) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Ok(());
    }

    install_panic_hook();
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = InlineTerminal::new(height, config.mouse, color_depth)?;
    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start);
            continue;
        }

        if !run_frame(&mut term, &mut state, &config, &events, &mut f)? {
            break;
        }

        events.clear();
        if crossterm::event::poll(config.tick_rate)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if is_ctrl_c(&ev) {
                    break;
                }
                if let Event::Resize(_, _) = &ev {
                    TerminalBackend::handle_resize(&mut term)?;
                }
                events.push(ev);
            }

            while crossterm::event::poll(Duration::ZERO)? {
                let raw = crossterm::event::read()?;
                if let Some(ev) = event::from_crossterm(raw) {
                    if is_ctrl_c(&ev) {
                        return Ok(());
                    }
                    if let Event::Resize(_, _) = &ev {
                        TerminalBackend::handle_resize(&mut term)?;
                    }
                    events.push(ev);
                }
            }

            for ev in &events {
                if matches!(
                    ev,
                    Event::Key(event::KeyEvent {
                        code: KeyCode::F(12),
                        kind: event::KeyEventKind::Press,
                        ..
                    })
                ) {
                    state.debug_mode = !state.debug_mode;
                }
            }
        }

        update_last_mouse_pos(&mut state, &events);

        if events.iter().any(|e| matches!(e, Event::Resize(_, _))) {
            clear_frame_layout_cache(&mut state);
        }

        sleep_for_fps_cap(config.max_fps, frame_start);
    }

    Ok(())
}

fn run_frame<T: TerminalBackend>(
    term: &mut T,
    state: &mut FrameState,
    config: &RunConfig,
    events: &[event::Event],
    f: &mut dyn FnMut(&mut context::Context),
) -> io::Result<bool> {
    let frame_start = Instant::now();
    let (w, h) = term.size();
    let mut ctx = Context::new(events.to_vec(), w, h, state, config.theme);
    ctx.is_real_terminal = true;
    ctx.process_focus_keys();

    f(&mut ctx);

    if ctx.should_quit {
        return Ok(false);
    }
    state.prev_modal_active = ctx.modal_active;
    let clipboard_text = ctx.clipboard_text.take();

    let mut should_copy_selection = false;
    for ev in &ctx.events {
        if let Event::Mouse(mouse) = ev {
            match mouse.kind {
                event::MouseKind::Down(event::MouseButton::Left) => {
                    state
                        .selection
                        .mouse_down(mouse.x, mouse.y, &state.prev_content_map);
                }
                event::MouseKind::Drag(event::MouseButton::Left) => {
                    state
                        .selection
                        .mouse_drag(mouse.x, mouse.y, &state.prev_content_map);
                }
                event::MouseKind::Up(event::MouseButton::Left) => {
                    should_copy_selection = state.selection.active;
                }
                _ => {}
            }
        }
    }

    state.focus_index = ctx.focus_index;
    state.prev_focus_count = ctx.focus_count;

    let mut tree = layout::build_tree(&ctx.commands);
    let area = crate::rect::Rect::new(0, 0, w, h);
    layout::compute(&mut tree, area);
    let fd = layout::collect_all(&tree);
    state.prev_scroll_infos = fd.scroll_infos;
    state.prev_scroll_rects = fd.scroll_rects;
    state.prev_hit_map = fd.hit_areas;
    state.prev_group_rects = fd.group_rects;
    state.prev_content_map = fd.content_areas;
    state.prev_focus_rects = fd.focus_rects;
    state.prev_focus_groups = fd.focus_groups;
    layout::render(&tree, term.buffer_mut());
    let raw_rects = layout::collect_raw_draw_rects(&tree);
    for (draw_id, rect) in raw_rects {
        if let Some(cb) = ctx.deferred_draws.get_mut(draw_id).and_then(|c| c.take()) {
            let buf = term.buffer_mut();
            buf.push_clip(rect);
            cb(buf, rect);
            buf.pop_clip();
        }
    }
    state.hook_states = ctx.hook_states;

    let frame_time = frame_start.elapsed();
    let frame_time_us = frame_time.as_micros().min(u128::from(u64::MAX)) as u64;
    let frame_secs = frame_time.as_secs_f32();
    let inst_fps = if frame_secs > 0.0 {
        1.0 / frame_secs
    } else {
        0.0
    };
    state.fps_ema = if state.fps_ema == 0.0 {
        inst_fps
    } else {
        (state.fps_ema * 0.9) + (inst_fps * 0.1)
    };
    if state.debug_mode {
        layout::render_debug_overlay(&tree, term.buffer_mut(), frame_time_us, state.fps_ema);
    }

    if state.selection.active {
        terminal::apply_selection_overlay(
            term.buffer_mut(),
            &state.selection,
            &state.prev_content_map,
        );
    }
    if should_copy_selection {
        let text = terminal::extract_selection_text(
            term.buffer_mut(),
            &state.selection,
            &state.prev_content_map,
        );
        if !text.is_empty() {
            terminal::copy_to_clipboard(&mut io::stdout(), &text)?;
        }
        state.selection.clear();
    }

    term.flush()?;
    if let Some(text) = clipboard_text {
        let _ = terminal::copy_to_clipboard(&mut io::stdout(), &text);
    }
    state.tick = state.tick.wrapping_add(1);

    Ok(true)
}

fn update_last_mouse_pos(state: &mut FrameState, events: &[Event]) {
    for ev in events {
        match ev {
            Event::Mouse(mouse) => {
                state.last_mouse_pos = Some((mouse.x, mouse.y));
            }
            Event::FocusLost => {
                state.last_mouse_pos = None;
            }
            _ => {}
        }
    }
}

fn clear_frame_layout_cache(state: &mut FrameState) {
    state.prev_hit_map.clear();
    state.prev_group_rects.clear();
    state.prev_content_map.clear();
    state.prev_focus_rects.clear();
    state.prev_focus_groups.clear();
    state.prev_scroll_infos.clear();
    state.prev_scroll_rects.clear();
    state.last_mouse_pos = None;
}

fn is_ctrl_c(ev: &Event) -> bool {
    matches!(
        ev,
        Event::Key(event::KeyEvent {
            code: KeyCode::Char('c'),
            modifiers,
            kind: event::KeyEventKind::Press,
        }) if modifiers.contains(KeyModifiers::CONTROL)
    )
}

fn sleep_for_fps_cap(max_fps: Option<u32>, frame_start: Instant) {
    if let Some(fps) = max_fps.filter(|fps| *fps > 0) {
        let target = Duration::from_secs_f64(1.0 / fps as f64);
        let elapsed = frame_start.elapsed();
        if elapsed < target {
            std::thread::sleep(target - elapsed);
        }
    }
}
