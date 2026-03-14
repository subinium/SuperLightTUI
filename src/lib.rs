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

use terminal::{InlineTerminal, Terminal};

pub use crate::test_utils::{EventBuilder, TestBackend};
pub use anim::{Keyframes, LoopMode, Sequence, Spring, Stagger, Tween};
pub use chart::{
    Axis, ChartBuilder, ChartConfig, ChartRenderer, Dataset, DatasetEntry, GraphType,
    HistogramBuilder, LegendPosition, Marker,
};
pub use context::{Bar, BarDirection, BarGroup, CanvasContext, Context, Response, Widget};
pub use event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseKind};
pub use style::{
    Align, Border, BorderSides, Color, Constraints, Justify, Margin, Modifiers, Padding, Style,
    Theme,
};
pub use widgets::{
    ButtonVariant, CommandPaletteState, FormField, FormState, ListState, MultiSelectState,
    PaletteCommand, RadioState, ScrollState, SelectState, SpinnerState, TableState, TabsState,
    TextInputState, TextareaState, ToastLevel, ToastMessage, ToastState, TreeNode, TreeState,
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
/// Use [`Default::default()`] for sensible defaults (100ms tick, no mouse, dark theme).
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
///     theme: Theme::light(),
///     max_fps: Some(60),
/// };
/// ```
#[must_use = "configure loop behavior before passing to run_with or run_inline_with"]
pub struct RunConfig {
    /// How long to wait for input before triggering a tick with no events.
    ///
    /// Lower values give smoother animations at the cost of more CPU usage.
    /// Defaults to 100ms.
    pub tick_rate: Duration,
    /// Whether to enable mouse event reporting.
    ///
    /// When `true`, the terminal captures mouse clicks, scrolls, and movement.
    /// Defaults to `false`.
    pub mouse: bool,
    /// The color theme applied to all widgets automatically.
    ///
    /// Defaults to [`Theme::dark()`].
    pub theme: Theme,
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
            theme: Theme::dark(),
            max_fps: Some(60),
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
    let mut term = Terminal::new(config.mouse)?;
    let mut events: Vec<Event> = Vec::new();
    let mut debug_mode: bool = false;
    let mut tick: u64 = 0;
    let mut focus_index: usize = 0;
    let mut prev_focus_count: usize = 0;
    let mut prev_scroll_infos: Vec<(u32, u32)> = Vec::new();
    let mut prev_hit_map: Vec<rect::Rect> = Vec::new();
    let mut prev_content_map: Vec<(rect::Rect, rect::Rect)> = Vec::new();
    let mut prev_focus_rects: Vec<(usize, rect::Rect)> = Vec::new();
    let mut last_mouse_pos: Option<(u32, u32)> = None;
    let mut prev_modal_active = false;
    let mut selection = terminal::SelectionState::default();

    loop {
        let frame_start = Instant::now();
        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start);
            continue;
        }
        let mut ctx = Context::new(
            std::mem::take(&mut events),
            w,
            h,
            tick,
            focus_index,
            prev_focus_count,
            std::mem::take(&mut prev_scroll_infos),
            std::mem::take(&mut prev_hit_map),
            std::mem::take(&mut prev_focus_rects),
            debug_mode,
            config.theme,
            last_mouse_pos,
            prev_modal_active,
        );
        ctx.process_focus_keys();

        f(&mut ctx);

        if ctx.should_quit {
            break;
        }
        prev_modal_active = ctx.modal_active;

        let mut should_copy_selection = false;
        for ev in ctx.events.iter() {
            if let Event::Mouse(mouse) = ev {
                match mouse.kind {
                    event::MouseKind::Down(event::MouseButton::Left) => {
                        selection.mouse_down(mouse.x, mouse.y, &prev_content_map);
                    }
                    event::MouseKind::Drag(event::MouseButton::Left) => {
                        selection.mouse_drag(mouse.x, mouse.y, &prev_content_map);
                    }
                    event::MouseKind::Up(event::MouseButton::Left) => {
                        should_copy_selection = selection.active;
                    }
                    _ => {}
                }
            }
        }

        focus_index = ctx.focus_index;
        prev_focus_count = ctx.focus_count;

        let mut tree = layout::build_tree(&ctx.commands);
        let area = crate::rect::Rect::new(0, 0, w, h);
        layout::compute(&mut tree, area);
        prev_scroll_infos = layout::collect_scroll_infos(&tree);
        prev_hit_map = layout::collect_hit_areas(&tree);
        prev_content_map = layout::collect_content_areas(&tree);
        prev_focus_rects = layout::collect_focus_rects(&tree);
        layout::render(&tree, term.buffer_mut());
        if debug_mode {
            layout::render_debug_overlay(&tree, term.buffer_mut());
        }

        if selection.active {
            terminal::apply_selection_overlay(term.buffer_mut(), &selection, &prev_content_map);
        }
        if should_copy_selection {
            let text =
                terminal::extract_selection_text(term.buffer_mut(), &selection, &prev_content_map);
            if !text.is_empty() {
                terminal::copy_to_clipboard(&mut io::stdout(), &text)?;
            }
            selection.clear();
        }

        term.flush()?;
        tick = tick.wrapping_add(1);

        events.clear();
        if crossterm::event::poll(config.tick_rate)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if is_ctrl_c(&ev) {
                    break;
                }
                if let Event::Resize(_, _) = &ev {
                    term.handle_resize()?;
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
                        term.handle_resize()?;
                    }
                    events.push(ev);
                }
            }

            for ev in &events {
                if matches!(
                    ev,
                    Event::Key(event::KeyEvent {
                        code: KeyCode::F(12),
                        ..
                    })
                ) {
                    debug_mode = !debug_mode;
                }
            }
        }

        for ev in &events {
            match ev {
                Event::Mouse(mouse) => {
                    last_mouse_pos = Some((mouse.x, mouse.y));
                }
                Event::FocusLost => {
                    last_mouse_pos = None;
                }
                _ => {}
            }
        }

        if events.iter().any(|e| matches!(e, Event::Resize(_, _))) {
            prev_hit_map.clear();
            prev_content_map.clear();
            prev_focus_rects.clear();
            prev_scroll_infos.clear();
            last_mouse_pos = None;
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
    let mut term = Terminal::new(config.mouse)?;
    let mut events: Vec<Event> = Vec::new();
    let mut tick: u64 = 0;
    let mut focus_index: usize = 0;
    let mut prev_focus_count: usize = 0;
    let mut prev_scroll_infos: Vec<(u32, u32)> = Vec::new();
    let mut prev_hit_map: Vec<rect::Rect> = Vec::new();
    let mut prev_content_map: Vec<(rect::Rect, rect::Rect)> = Vec::new();
    let mut prev_focus_rects: Vec<(usize, rect::Rect)> = Vec::new();
    let mut last_mouse_pos: Option<(u32, u32)> = None;
    let mut prev_modal_active = false;
    let mut selection = terminal::SelectionState::default();

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
        let mut ctx = Context::new(
            std::mem::take(&mut events),
            w,
            h,
            tick,
            focus_index,
            prev_focus_count,
            std::mem::take(&mut prev_scroll_infos),
            std::mem::take(&mut prev_hit_map),
            std::mem::take(&mut prev_focus_rects),
            false,
            config.theme,
            last_mouse_pos,
            prev_modal_active,
        );
        ctx.process_focus_keys();

        f(&mut ctx, &mut messages);

        if ctx.should_quit {
            break;
        }
        prev_modal_active = ctx.modal_active;

        let mut should_copy_selection = false;
        for ev in ctx.events.iter() {
            if let Event::Mouse(mouse) = ev {
                match mouse.kind {
                    event::MouseKind::Down(event::MouseButton::Left) => {
                        selection.mouse_down(mouse.x, mouse.y, &prev_content_map);
                    }
                    event::MouseKind::Drag(event::MouseButton::Left) => {
                        selection.mouse_drag(mouse.x, mouse.y, &prev_content_map);
                    }
                    event::MouseKind::Up(event::MouseButton::Left) => {
                        should_copy_selection = selection.active;
                    }
                    _ => {}
                }
            }
        }

        focus_index = ctx.focus_index;
        prev_focus_count = ctx.focus_count;

        let mut tree = layout::build_tree(&ctx.commands);
        let area = crate::rect::Rect::new(0, 0, w, h);
        layout::compute(&mut tree, area);
        prev_scroll_infos = layout::collect_scroll_infos(&tree);
        prev_hit_map = layout::collect_hit_areas(&tree);
        prev_content_map = layout::collect_content_areas(&tree);
        prev_focus_rects = layout::collect_focus_rects(&tree);
        layout::render(&tree, term.buffer_mut());

        if selection.active {
            terminal::apply_selection_overlay(term.buffer_mut(), &selection, &prev_content_map);
        }
        if should_copy_selection {
            let text =
                terminal::extract_selection_text(term.buffer_mut(), &selection, &prev_content_map);
            if !text.is_empty() {
                terminal::copy_to_clipboard(&mut io::stdout(), &text)?;
            }
            selection.clear();
        }

        term.flush()?;
        tick = tick.wrapping_add(1);

        events.clear();
        if crossterm::event::poll(config.tick_rate)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if is_ctrl_c(&ev) {
                    break;
                }
                if let Event::Resize(_, _) = &ev {
                    term.handle_resize()?;
                    prev_hit_map.clear();
                    prev_content_map.clear();
                    prev_focus_rects.clear();
                    prev_scroll_infos.clear();
                    last_mouse_pos = None;
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
                        term.handle_resize()?;
                        prev_hit_map.clear();
                        prev_content_map.clear();
                        prev_focus_rects.clear();
                        prev_scroll_infos.clear();
                        last_mouse_pos = None;
                    }
                    events.push(ev);
                }
            }
        }

        for ev in &events {
            match ev {
                Event::Mouse(mouse) => {
                    last_mouse_pos = Some((mouse.x, mouse.y));
                }
                Event::FocusLost => {
                    last_mouse_pos = None;
                }
                _ => {}
            }
        }

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
    let mut term = InlineTerminal::new(height, config.mouse)?;
    let mut events: Vec<Event> = Vec::new();
    let mut debug_mode: bool = false;
    let mut tick: u64 = 0;
    let mut focus_index: usize = 0;
    let mut prev_focus_count: usize = 0;
    let mut prev_scroll_infos: Vec<(u32, u32)> = Vec::new();
    let mut prev_hit_map: Vec<rect::Rect> = Vec::new();
    let mut prev_content_map: Vec<(rect::Rect, rect::Rect)> = Vec::new();
    let mut prev_focus_rects: Vec<(usize, rect::Rect)> = Vec::new();
    let mut last_mouse_pos: Option<(u32, u32)> = None;
    let mut prev_modal_active = false;
    let mut selection = terminal::SelectionState::default();

    loop {
        let frame_start = Instant::now();
        let (w, h) = term.size();
        if w == 0 || h == 0 {
            sleep_for_fps_cap(config.max_fps, frame_start);
            continue;
        }
        let mut ctx = Context::new(
            std::mem::take(&mut events),
            w,
            h,
            tick,
            focus_index,
            prev_focus_count,
            std::mem::take(&mut prev_scroll_infos),
            std::mem::take(&mut prev_hit_map),
            std::mem::take(&mut prev_focus_rects),
            debug_mode,
            config.theme,
            last_mouse_pos,
            prev_modal_active,
        );
        ctx.process_focus_keys();

        f(&mut ctx);

        if ctx.should_quit {
            break;
        }
        prev_modal_active = ctx.modal_active;

        let mut should_copy_selection = false;
        for ev in ctx.events.iter() {
            if let Event::Mouse(mouse) = ev {
                match mouse.kind {
                    event::MouseKind::Down(event::MouseButton::Left) => {
                        selection.mouse_down(mouse.x, mouse.y, &prev_content_map);
                    }
                    event::MouseKind::Drag(event::MouseButton::Left) => {
                        selection.mouse_drag(mouse.x, mouse.y, &prev_content_map);
                    }
                    event::MouseKind::Up(event::MouseButton::Left) => {
                        should_copy_selection = selection.active;
                    }
                    _ => {}
                }
            }
        }

        focus_index = ctx.focus_index;
        prev_focus_count = ctx.focus_count;

        let mut tree = layout::build_tree(&ctx.commands);
        let area = crate::rect::Rect::new(0, 0, w, h);
        layout::compute(&mut tree, area);
        prev_scroll_infos = layout::collect_scroll_infos(&tree);
        prev_hit_map = layout::collect_hit_areas(&tree);
        prev_content_map = layout::collect_content_areas(&tree);
        prev_focus_rects = layout::collect_focus_rects(&tree);
        layout::render(&tree, term.buffer_mut());
        if debug_mode {
            layout::render_debug_overlay(&tree, term.buffer_mut());
        }

        if selection.active {
            terminal::apply_selection_overlay(term.buffer_mut(), &selection, &prev_content_map);
        }
        if should_copy_selection {
            let text =
                terminal::extract_selection_text(term.buffer_mut(), &selection, &prev_content_map);
            if !text.is_empty() {
                terminal::copy_to_clipboard(&mut io::stdout(), &text)?;
            }
            selection.clear();
        }

        term.flush()?;
        tick = tick.wrapping_add(1);

        events.clear();
        if crossterm::event::poll(config.tick_rate)? {
            let raw = crossterm::event::read()?;
            if let Some(ev) = event::from_crossterm(raw) {
                if is_ctrl_c(&ev) {
                    break;
                }
                if let Event::Resize(_, _) = &ev {
                    term.handle_resize()?;
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
                        term.handle_resize()?;
                    }
                    events.push(ev);
                }
            }

            for ev in &events {
                if matches!(
                    ev,
                    Event::Key(event::KeyEvent {
                        code: KeyCode::F(12),
                        ..
                    })
                ) {
                    debug_mode = !debug_mode;
                }
            }
        }

        for ev in &events {
            match ev {
                Event::Mouse(mouse) => {
                    last_mouse_pos = Some((mouse.x, mouse.y));
                }
                Event::FocusLost => {
                    last_mouse_pos = None;
                }
                _ => {}
            }
        }

        if events.iter().any(|e| matches!(e, Event::Resize(_, _))) {
            prev_hit_map.clear();
            prev_content_map.clear();
            prev_focus_rects.clear();
            prev_scroll_infos.clear();
            last_mouse_pos = None;
        }

        sleep_for_fps_cap(config.max_fps, frame_start);
    }

    Ok(())
}

fn is_ctrl_c(ev: &Event) -> bool {
    matches!(
        ev,
        Event::Key(event::KeyEvent {
            code: KeyCode::Char('c'),
            modifiers,
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
