//! Native crossterm run loops (`run`, `run_inline`, `run_static`), terminal
//! endpoint validation and event polling.

use super::*;

use std::io;
use std::io::IsTerminal;
use std::time::Duration;

use crate::clock::Instant;
use crate::{
    Color, ColorDepth, Context, Event, InlineTerminal, KeyCode, KeyModifiers, StaticOutput,
    Terminal, event,
};

/// Run the TUI loop with default configuration.
///
/// Enters alternate screen mode, runs `f` each frame, and exits cleanly on
/// Ctrl+C or when [`Context::quit`] is called.
///
/// # Raw mode is handled for you
///
/// SLT enters raw mode automatically inside [`run`] / [`run_with`] /
/// [`run_inline`] / [`run_async`]. Wrapping these with manual
/// `crossterm::terminal::enable_raw_mode()` and `disable_raw_mode()` is
/// **redundant** — the calls are idempotent so no harm comes of it, but it
/// suggests a misunderstood lifecycle. Drop the wrapper calls:
///
/// ```no_run
/// // Don't do this — it's already handled internally:
/// // crossterm::terminal::enable_raw_mode()?;
/// slt::run(|ui| { ui.text("hi"); })?;
/// // crossterm::terminal::disable_raw_mode()?;
/// # Ok::<_, std::io::Error>(())
/// ```
///
/// # Ctrl+C opt-out (issue #238)
///
/// By default, Ctrl+C exits the loop cleanly — matching the v0.19 contract
/// and the convention most TUIs follow. To match RataTUI's raw-mode
/// semantics (Ctrl+C delivered as a regular `Event::Key`), set
/// [`RunConfig::handle_ctrl_c(false)`](RunConfig::handle_ctrl_c) and decide
/// inside the frame closure whether to call [`Context::quit`]:
///
/// ```no_run
/// use slt::{KeyModifiers, RunConfig};
///
/// slt::run_with(RunConfig::default().handle_ctrl_c(false), |ui| {
///     if ui.key_mod('c', KeyModifiers::CONTROL) {
///         // e.g. clear input, prompt to save, then quit:
///         ui.quit();
///     }
/// })?;
/// # Ok::<_, std::io::Error>(())
/// ```
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
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run(f: impl FnMut(&mut Context)) -> io::Result<()> {
    run_with(RunConfig::default(), f)
}

#[cfg(feature = "crossterm")]
pub(super) fn validate_terminal_endpoints(
    stdin_is_terminal: bool,
    stdout_is_terminal: bool,
) -> io::Result<()> {
    if stdin_is_terminal && stdout_is_terminal {
        return Ok(());
    }

    let unavailable = match (stdin_is_terminal, stdout_is_terminal) {
        (false, false) => "stdin and stdout are not terminals",
        (false, true) => "stdin is not a terminal",
        (true, false) => "stdout is not a terminal",
        (true, true) => unreachable!("validated above"),
    };
    Err(io::Error::new(
        io::ErrorKind::NotConnected,
        format!("interactive SLT runtime unavailable: {unavailable}"),
    ))
}

#[cfg(feature = "crossterm")]
pub(super) fn ensure_interactive_terminal() -> io::Result<()> {
    validate_terminal_endpoints(io::stdin().is_terminal(), io::stdout().is_terminal())
}

#[cfg(feature = "crossterm")]
fn validate_inline_height(height: u32) -> io::Result<()> {
    if height == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "inline terminal height must be greater than zero",
        ));
    }
    Ok(())
}

#[cfg(feature = "crossterm")]
pub(crate) fn sanitize_terminal_text(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_control() || ('\u{80}'..='\u{9f}').contains(&ch) {
                '?'
            } else {
                ch
            }
        })
        .collect()
}

/// Run the TUI loop with custom configuration.
///
/// Like [`run`], but accepts a [`RunConfig`] to control tick rate, mouse
/// support, and theming.
///
/// Returns [`io::ErrorKind::NotConnected`] when stdin or stdout is not a
/// terminal. Headless and remote renderers should drive [`frame`](crate::frame) with a custom
/// [`Backend`](crate::Backend) instead.
///
/// # Example
///
/// ```no_run
/// use slt::{RunConfig, Theme};
///
/// fn main() -> std::io::Result<()> {
///     slt::run_with(
///         RunConfig::default().theme(Theme::light()),
///         |ui| {
///             ui.text("Light theme!");
///         },
///     )
/// }
/// ```
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run_with(config: RunConfig, f: impl FnMut(&mut Context)) -> io::Result<()> {
    ensure_interactive_terminal()?;
    with_session_panic_hook(|| run_with_inner(config, f))
}

#[cfg(feature = "crossterm")]
fn run_with_inner(config: RunConfig, mut f: impl FnMut(&mut Context)) -> io::Result<()> {
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = Terminal::new(
        config.mouse,
        config.kitty_keyboard,
        config.report_all_keys,
        color_depth,
    )?;
    term.apply_run_config(&config);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    // Issue #263: install the unix Ctrl+Z / `fg` suspend handler for the loop.
    #[cfg(unix)]
    let _suspend_guard = if config.handle_suspend {
        Some(install_suspend_handler(term.session_snapshot())?)
    } else {
        None
    };
    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        // Issue #263: after a SIGCONT resume, repaint the whole frame.
        #[cfg(unix)]
        drain_resume_redraw(&mut || term.handle_resize())?;
        let (w, h) = term.size();
        if w > 0 && h > 0 {
            if !run_frame(
                &mut term,
                &mut state,
                &config,
                std::mem::take(&mut events),
                true,
                &mut f,
            )? {
                break;
            }
            // Issue #233: full-screen mode has no scrollback channel — warn and
            // drop any `ui.static_log(...)` lines so they do not leak into the
            // next frame's named_states.
            discard_static_log(&mut state, "full-screen run()");
            events = std::mem::take(&mut state.events_buf);
        }

        #[cfg(unix)]
        let suspend_snapshot = term.session_snapshot();
        #[cfg(unix)]
        let mut on_suspend = || suspend_current_session(suspend_snapshot);
        #[cfg(not(unix))]
        let mut on_suspend = || Ok(());

        let input_wait = input_poll_wait(&state, term.size(), config.tick_rate);
        if !poll_events(
            &mut events,
            &mut state,
            true,
            input_wait,
            &mut || term.handle_resize(),
            &config,
            &mut on_suspend,
        )? {
            break;
        }

        sleep_for_fps_cap(config.max_fps, frame_start.elapsed());
    }

    Ok(())
}

#[cfg(feature = "crossterm")]
pub(crate) fn localize_inline_events(
    term: &InlineTerminal,
    state: &mut FrameState,
    events: &mut Vec<Event>,
) {
    let origin = term.origin_row();
    if let Some(previous) = state.layout_feedback.origin_row {
        state.pending_input.retain_mut(|event| match event {
            Event::Mouse(mouse) => match mouse.y.checked_add(previous).and_then(|y| {
                let mut physical = mouse.clone();
                physical.y = y;
                term.localize_mouse(physical)
            }) {
                Some(local) => {
                    *mouse = local;
                    true
                }
                None => false,
            },
            _ => true,
        });
    }
    if let Some(previous) = state.layout_feedback.origin_row
        && previous != origin
    {
        state.layout_feedback.last_mouse_pos =
            state.layout_feedback.last_mouse_pos.and_then(|(x, y)| {
                y.checked_add(previous).and_then(|y| {
                    term.localize_mouse(event::MouseEvent::new(
                        event::MouseKind::Moved,
                        x,
                        y,
                        KeyModifiers::NONE,
                        None,
                        None,
                    ))
                    .map(|mouse| (mouse.x, mouse.y))
                })
            });
        state.layout_feedback.last_click_at = None;
        state.layout_feedback.last_click_pos = None;
        state.selection.clear();
    }
    state.layout_feedback.origin_row = Some(origin);
    events.retain_mut(|event| match event {
        Event::Mouse(mouse) => match term.localize_mouse(mouse.clone()) {
            Some(local) => {
                *mouse = local;
                true
            }
            None => false,
        },
        _ => true,
    });
}

/// Run the TUI in inline mode with default configuration.
///
/// Renders `height` rows directly below the current cursor position without
/// entering alternate screen mode. Useful for CLI tools that want a small
/// interactive widget below the prompt.
///
/// `height` is the reserved inline render area in terminal rows.
/// The rest of the terminal stays in normal scrollback mode.
/// A zero height returns [`io::ErrorKind::InvalidInput`]; non-terminal stdin or
/// stdout returns [`io::ErrorKind::NotConnected`].
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
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run_inline(height: u32, f: impl FnMut(&mut Context)) -> io::Result<()> {
    run_inline_with(height, RunConfig::default(), f)
}

/// Run the TUI in inline mode with custom configuration.
///
/// Like [`run_inline`], but accepts a [`RunConfig`] to control tick rate,
/// mouse support, and theming.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run_inline_with(
    height: u32,
    config: RunConfig,
    f: impl FnMut(&mut Context),
) -> io::Result<()> {
    validate_inline_height(height)?;
    ensure_interactive_terminal()?;
    with_session_panic_hook(|| run_inline_with_inner(height, config, f))
}

#[cfg(feature = "crossterm")]
fn run_inline_with_inner(
    height: u32,
    config: RunConfig,
    mut f: impl FnMut(&mut Context),
) -> io::Result<()> {
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = InlineTerminal::new(
        height,
        config.mouse,
        config.kitty_keyboard,
        config.report_all_keys,
        color_depth,
    )?;
    term.apply_run_config(&config);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    // Issue #263: install the unix Ctrl+Z / `fg` suspend handler for the loop.
    #[cfg(unix)]
    let _suspend_guard = if config.handle_suspend {
        Some(install_suspend_handler(term.session_snapshot())?)
    } else {
        None
    };
    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        // Issue #263: after a SIGCONT resume, repaint the whole frame.
        #[cfg(unix)]
        drain_resume_redraw(&mut || term.handle_resize())?;
        let (w, h) = term.size();
        if w > 0 && h > 0 {
            localize_inline_events(&term, &mut state, &mut events);
            if !run_frame(
                &mut term,
                &mut state,
                &config,
                std::mem::take(&mut events),
                true,
                &mut f,
            )? {
                break;
            }
            // Issue #233: inline mode without `StaticOutput` has no scrollback
            // channel either — warn and drop any pending lines.
            discard_static_log(&mut state, "run_inline()");
            events = std::mem::take(&mut state.events_buf);
        }

        #[cfg(unix)]
        let suspend_snapshot = term.session_snapshot();
        #[cfg(unix)]
        let mut on_suspend = || suspend_current_session(suspend_snapshot);
        #[cfg(not(unix))]
        let mut on_suspend = || Ok(());

        let input_wait = input_poll_wait(&state, term.size(), config.tick_rate);
        if !poll_events(
            &mut events,
            &mut state,
            true,
            input_wait,
            &mut || term.handle_resize(),
            &config,
            &mut on_suspend,
        )? {
            break;
        }

        sleep_for_fps_cap(config.max_fps, frame_start.elapsed());
    }

    Ok(())
}

/// Run the TUI in static-output mode.
///
/// Static lines written through [`StaticOutput`] are printed into terminal
/// scrollback, while the interactive UI stays rendered in a fixed-height inline
/// area at the bottom.
///
/// Use this when you want a log-style output stream above a live inline UI.
/// A zero dynamic height returns [`io::ErrorKind::InvalidInput`]; non-terminal
/// stdin or stdout returns [`io::ErrorKind::NotConnected`].
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run_static(
    output: &mut StaticOutput,
    dynamic_height: u32,
    f: impl FnMut(&mut Context),
) -> io::Result<()> {
    run_static_with(output, dynamic_height, RunConfig::default(), f)
}

/// Run the TUI in static-output mode with custom configuration.
///
/// Like [`run_static`] but accepts a [`RunConfig`] for theme, mouse, tick rate,
/// and other settings.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn run_static_with(
    output: &mut StaticOutput,
    dynamic_height: u32,
    config: RunConfig,
    f: impl FnMut(&mut Context),
) -> io::Result<()> {
    validate_inline_height(dynamic_height)?;
    ensure_interactive_terminal()?;
    with_session_panic_hook(|| run_static_with_inner(output, dynamic_height, config, f))
}

#[cfg(feature = "crossterm")]
fn run_static_with_inner(
    output: &mut StaticOutput,
    dynamic_height: u32,
    config: RunConfig,
    mut f: impl FnMut(&mut Context),
) -> io::Result<()> {
    let color_depth = config.color_depth.unwrap_or_else(ColorDepth::detect);
    let mut term = InlineTerminal::new(
        dynamic_height,
        config.mouse,
        config.kitty_keyboard,
        config.report_all_keys,
        color_depth,
    )?;
    term.write_scrollback(&output.drain_new())?;
    term.apply_run_config(&config);
    if config.theme.bg != Color::Reset {
        term.theme_bg = Some(config.theme.bg);
    }
    // Issue #263: install the unix Ctrl+Z / `fg` suspend handler for the loop.
    #[cfg(unix)]
    let _suspend_guard = if config.handle_suspend {
        Some(install_suspend_handler(term.session_snapshot())?)
    } else {
        None
    };

    let mut events: Vec<Event> = Vec::new();
    let mut state = FrameState::default();

    loop {
        let frame_start = Instant::now();
        // Issue #263: after a SIGCONT resume, repaint the whole frame.
        #[cfg(unix)]
        drain_resume_redraw(&mut || term.handle_resize())?;
        let (w, h) = term.size();
        term.write_scrollback(&output.drain_new())?;
        if w > 0 && h > 0 {
            localize_inline_events(&term, &mut state, &mut events);
            let keep_running = run_frame(
                &mut term,
                &mut state,
                &config,
                std::mem::take(&mut events),
                true,
                &mut f,
            )?;
            // Issue #233: drain any `ui.static_log(...)` lines queued during the
            // frame closure into `output`, then flush them before any exit path.
            for line in drain_static_log(&mut state) {
                output.println(line);
            }
            term.write_scrollback(&output.drain_new())?;
            if !keep_running {
                break;
            }
            events = std::mem::take(&mut state.events_buf);
        }

        #[cfg(unix)]
        let suspend_snapshot = term.session_snapshot();
        #[cfg(unix)]
        let mut on_suspend = || suspend_current_session(suspend_snapshot);
        #[cfg(not(unix))]
        let mut on_suspend = || Ok(());

        let input_wait = input_poll_wait(&state, term.size(), config.tick_rate);
        if !poll_events(
            &mut events,
            &mut state,
            true,
            input_wait,
            &mut || term.handle_resize(),
            &config,
            &mut on_suspend,
        )? {
            break;
        }

        sleep_for_fps_cap(config.max_fps, frame_start.elapsed());
    }

    Ok(())
}

#[cfg(all(feature = "crossterm", test))]
pub(super) fn write_static_lines_to(
    stdout: &mut impl io::Write,
    lines: &[String],
) -> io::Result<()> {
    for line in lines {
        let safe = sanitize_terminal_text(line);
        stdout.write_all(safe.as_bytes())?;
        stdout.write_all(b"\r\n")?;
    }
    stdout.flush()
}

/// Drain any [`Context::static_log`] lines accumulated during the most recent
/// frame from the persisted [`FrameState`] (issue #233).
///
/// After [`run_frame_kernel`] returns, `state.named_states` owns the buffer.
/// This helper drains it back to a `Vec<String>` so the runtime can flush
/// the lines through whichever scrollback mechanism is appropriate
/// (`run_static_with` writes them above the inline region; other run modes
/// drop them with a debug warning).
#[cfg(feature = "crossterm")]
pub(crate) fn drain_static_log(state: &mut FrameState) -> Vec<String> {
    if let Some(boxed) = state.named_states.get_mut(STATIC_LOG_NAMED_STATE_KEY)
        && let Some(buf) = boxed.downcast_mut::<Vec<String>>()
    {
        return std::mem::take(buf);
    }
    Vec::new()
}

/// Discard any [`Context::static_log`] lines that accumulated during the
/// most recent frame and emit a debug warning (issue #233).
///
/// Used by run modes that have no scrollback channel (full-screen,
/// inline-without-static, async). Release builds silently drop the buffer.
#[cfg(feature = "crossterm")]
pub(super) fn discard_static_log(state: &mut FrameState, mode: &str) {
    let drained = drain_static_log(state);
    #[cfg(debug_assertions)]
    if !drained.is_empty() {
        #[allow(clippy::print_stderr)]
        {
            eprintln!(
                "[slt] {} static_log lines were dropped: {} runtime has no scrollback channel; use slt::run_static for streaming output",
                drained.len(),
                mode
            );
        }
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = (drained, mode);
    }
}

/// Number of `on_resize` invocations a batch of events should trigger.
///
/// v0.21.1 resize coalescing: a single poll batch may deliver a burst of
/// `Event::Resize` events while a user drags the window edge. Each
/// [`Terminal::handle_resize`](crate::terminal::Terminal::handle_resize) does a
/// `terminal::size()` syscall, two buffer reallocations, and a `Clear(All)`, so
/// firing it per-event is pure waste — only the *final* geometry matters and
/// `handle_resize` always reads the live terminal size, not the per-event
/// payload. This helper returns `1` if the batch contains any resize and `0`
/// otherwise, so the caller can collapse the burst into one end-of-batch call.
///
/// Kept as a pure function (no I/O) so the coalescing rule is unit-testable
/// without a real crossterm event source.
#[cfg(feature = "crossterm")]
#[inline]
pub(super) fn resize_invocations_for_batch(events: &[Event]) -> usize {
    usize::from(events.iter().any(|e| matches!(e, Event::Resize(_, _))))
}

#[cfg(feature = "crossterm")]
fn input_poll_wait(state: &FrameState, size: (u32, u32), idle: Duration) -> Duration {
    if size.0 > 0 && size.1 > 0 && !state.pending_input.is_empty() {
        Duration::ZERO
    } else {
        idle
    }
}

/// Poll for terminal events, handling resize, Ctrl-C, F12 debug toggle,
/// and layout cache invalidation. Returns `Ok(false)` when the loop should exit.
///
/// `handle_ctrl_c` controls whether Ctrl+C exits the loop (`true`, default
/// v0.19 behavior) or is delivered to the frame closure as a regular key
/// event (`false`, RataTUI parity, issue #238).
///
/// v0.21.1: resize events within one poll batch are *coalesced* — `on_resize`
/// is invoked at most once, after the whole batch is drained, using the final
/// terminal size (`handle_resize` re-reads `terminal::size()`). Dragging a
/// window edge can emit dozens of `Event::Resize` per poll; firing the
/// `Clear(All)` + double realloc + `size()` syscall for each is wasted work
/// when only the last geometry survives. The SIGCONT/resume redraw path in
/// [`run_with`] is unaffected — it calls `handle_resize` directly, outside this
/// function.
#[cfg(feature = "crossterm")]
pub(super) fn poll_events(
    events: &mut Vec<Event>,
    state: &mut FrameState,
    accept_input: bool,
    tick_rate: Duration,
    on_resize: &mut impl FnMut() -> io::Result<()>,
    config: &RunConfig,
    on_suspend: &mut impl FnMut() -> io::Result<()>,
) -> io::Result<bool> {
    let mut has_resize = false;
    let batch_start = events.len();

    fn process_ev(ev: &Event, has_resize: &mut bool) {
        *has_resize |= matches!(ev, Event::Resize(_, _));
    }

    if event::poll(tick_rate)? {
        let raw = event::read()?;
        let mut raw_events = 1;
        if let Some(ev) = event::from_crossterm(raw) {
            if config.handle_ctrl_c && is_ctrl_c(&ev) {
                return Ok(false);
            }
            if config.handle_suspend && is_ctrl_z(&ev) {
                on_suspend()?;
                return Ok(true);
            }
            // Resize is recorded (via `has_resize`) but not yet acted on — the
            // single `on_resize` call is deferred to end-of-batch so a burst
            // collapses into one geometry sync.
            process_ev(&ev, &mut has_resize);
            if accept_input {
                events.push(ev);
            }
        }

        // Unmapped native events (for example media keys) also consume the
        // budget, otherwise a filtered-event flood can starve UI frames.
        while raw_events < 256 && events.len() < 256 && event::poll(Duration::ZERO)? {
            let raw = event::read()?;
            raw_events += 1;
            if let Some(ev) = event::from_crossterm(raw) {
                if config.handle_ctrl_c && is_ctrl_c(&ev) {
                    return Ok(false);
                }
                if config.handle_suspend && is_ctrl_z(&ev) {
                    on_suspend()?;
                    return Ok(true);
                }
                process_ev(&ev, &mut has_resize);
                if accept_input {
                    events.push(ev);
                }
            }
        }
    }

    // Coalesced resize: fire `on_resize` exactly once for the whole batch,
    // after every event has been read, so it picks up the final terminal size.
    // `has_resize` is the per-batch "saw a resize" flag set by `process_ev`.
    if accept_input {
        debug_assert_eq!(
            usize::from(has_resize),
            resize_invocations_for_batch(&events[batch_start..]),
            "has_resize must agree with the coalescing helper"
        );
    }
    if has_resize {
        on_resize()?;
    }

    if has_resize {
        clear_frame_layout_cache(state);
    }

    Ok(true)
}

#[cfg(feature = "crossterm")]
pub(super) fn is_ctrl_c(ev: &Event) -> bool {
    matches!(
        ev,
        Event::Key(event::KeyEvent {
            code: KeyCode::Char('c'),
            modifiers,
            kind: event::KeyEventKind::Press,
        }) if modifiers.contains(KeyModifiers::CONTROL)
    )
}

#[cfg(feature = "crossterm")]
pub(super) fn is_ctrl_z(ev: &Event) -> bool {
    matches!(
        ev,
        Event::Key(event::KeyEvent {
            code: KeyCode::Char('z'),
            modifiers,
            kind: event::KeyEventKind::Press,
        }) if modifiers.contains(KeyModifiers::CONTROL)
    )
}

#[cfg(feature = "crossterm")]
fn sleep_for_fps_cap(max_fps: Option<u32>, loop_elapsed: Duration) {
    if let Some(remaining) = fps_sleep_duration(max_fps, loop_elapsed) {
        std::thread::sleep(remaining);
    }
}

#[cfg(feature = "crossterm")]
pub(super) fn fps_sleep_duration(max_fps: Option<u32>, loop_elapsed: Duration) -> Option<Duration> {
    let fps = max_fps.filter(|fps| *fps > 0)?;
    let target = Duration::from_secs_f64(1.0 / fps as f64);
    (loop_elapsed < target).then(|| target - loop_elapsed)
}
