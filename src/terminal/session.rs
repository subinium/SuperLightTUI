//! Terminal session lifecycle: active-session registry and guard,
//! enter / exit / cleanup writers, panic cleanup and suspend / resume.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerminalSessionMode {
    Fullscreen,
    Inline,
}

/// Immutable terminal-session state used by panic and suspend cleanup.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SessionSnapshot {
    pub(super) mode: TerminalSessionMode,
    pub(super) mouse_mode: crate::MouseMode,
    pub(super) kitty_keyboard: bool,
    #[cfg_attr(
        not(unix),
        expect(
            dead_code,
            reason = "read only when resuming from a unix job-control suspend"
        )
    )]
    pub(super) report_all_keys: bool,
    pub(super) raw_mode_owned: bool,
}

#[derive(Debug, Clone, Copy)]
struct ActiveSession {
    id: u64,
    owner: std::thread::ThreadId,
    snapshot: SessionSnapshot,
}

static ACTIVE_SESSIONS: std::sync::Mutex<Vec<ActiveSession>> = std::sync::Mutex::new(Vec::new());

static NEXT_SESSION_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(super) fn register_active_session(snapshot: SessionSnapshot) -> u64 {
    use std::sync::atomic::Ordering;
    let id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    ACTIVE_SESSIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(ActiveSession {
            id,
            owner: std::thread::current().id(),
            snapshot,
        });
    id
}

pub(super) fn update_active_session(id: u64, snapshot: SessionSnapshot) {
    if let Some(session) = ACTIVE_SESSIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter_mut()
        .rfind(|session| session.id == id)
    {
        session.snapshot = snapshot;
    }
}

pub(super) fn unregister_active_session(id: u64) {
    let mut sessions = ACTIVE_SESSIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(index) = sessions.iter().rposition(|session| session.id == id) {
        sessions.remove(index);
    }
    if sessions.is_empty() {
        crate::event::reply::release_input();
    }
}

pub(super) fn active_session_snapshot() -> Option<SessionSnapshot> {
    ACTIVE_SESSIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .rev()
        .find(|session| session.owner == std::thread::current().id())
        .map(|session| session.snapshot)
}

#[derive(Debug)]
pub(super) struct TerminalSessionGuard {
    pub(super) mode: TerminalSessionMode,
    pub(super) mouse_mode: crate::MouseMode,
    pub(super) kitty_keyboard: bool,
    pub(super) report_all_keys: bool,
    pub(super) raw_mode_owned: bool,
    pub(super) registry_id: Option<u64>,
    pub(super) restored: std::sync::atomic::AtomicBool,
    /// When `true`, the guard never touched real raw-mode / terminal state
    /// (PTY test harness path). `restore` then becomes a no-op so dropping a
    /// captured-sink `Terminal` does not call `disable_raw_mode` or emit
    /// teardown escapes into the byte capture. Always `false` on the
    /// production `enter` path.
    pub(super) harness: bool,
}

impl TerminalSessionGuard {
    pub(super) fn enter(
        mode: TerminalSessionMode,
        stdout: &mut impl Write,
        mouse_mode: crate::MouseMode,
        kitty_keyboard: bool,
        report_all_keys: bool,
    ) -> io::Result<Self> {
        let kitty_requested = kitty_keyboard && terminal_kitty_keyboard_allowed();
        *CELL_PIXEL_CACHE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = CellPixelCache::default();
        let raw_mode_owned = raw_mode_is_acquired(terminal::is_raw_mode_enabled()?);
        if raw_mode_owned {
            terminal::enable_raw_mode()?;
        }

        // Issue #448: the Kitty keyboard push waits for the capability probe
        // below, so the guard starts without it.
        let mut guard = Self {
            mode,
            mouse_mode,
            kitty_keyboard: false,
            report_all_keys,
            raw_mode_owned,
            registry_id: None,
            restored: std::sync::atomic::AtomicBool::new(false),
            harness: false,
        };
        guard.registry_id = Some(register_active_session(guard.snapshot()));
        if let Err(err) = write_session_enter(stdout, &guard) {
            guard.restore(stdout, false);
            return Err(err);
        }

        // Issue #264: run the one-shot DA1/DA2/XTGETTCAP capability probe at
        // session enter, while raw mode is active so the replies are readable.
        // `capabilities()` caches in a `OnceLock`, so the resume re-enter path
        // never re-probes. Never runs on the PTY-harness path (`harness` is
        // always `false` here, but resume/harness re-entries go through
        // `write_session_enter` directly, not `enter`).
        let caps = capabilities();

        // Issue #448: push the Kitty keyboard protocol only where it will take
        // effect: confirmed by `CSI ? u`, forced, or (when queries cannot run)
        // allowed by multiplexer policy. The registry snapshot is updated
        // before the push so panic / suspend cleanup pops exactly what was
        // pushed.
        if kitty_keyboard_push_decision(
            kitty_requested,
            true,
            automatic_terminal_queries_allowed(),
            caps.kitty_keyboard,
            force_env_enabled("SLT_FORCE_KITTY_KEYBOARD"),
        ) {
            guard.kitty_keyboard = true;
            if let Some(id) = guard.registry_id {
                update_active_session(id, guard.snapshot());
            }
            if let Err(err) = write_kitty_keyboard_push(stdout, report_all_keys) {
                guard.restore(stdout, false);
                return Err(err);
            }
            stdout.flush()?;
        }

        crate::event::reply::prepare_input()?;

        Ok(guard)
    }

    pub(super) fn restore(&self, stdout: &mut impl Write, inline_reserved: bool) {
        if self
            .restored
            .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return;
        }
        // PTY harness guard: nothing was ever entered, so nothing to restore.
        if self.harness {
            return;
        }
        let _ = write_session_exit(
            stdout,
            self.mode,
            inline_reserved,
            self.mouse_mode,
            self.kitty_keyboard,
        );
        if self.raw_mode_owned {
            let _ = terminal::disable_raw_mode();
        }
        if let Some(id) = self.registry_id {
            unregister_active_session(id);
        }
    }

    fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            mode: self.mode,
            mouse_mode: self.mouse_mode,
            kitty_keyboard: self.kitty_keyboard,
            report_all_keys: self.report_all_keys,
            raw_mode_owned: self.raw_mode_owned,
        }
    }
}

impl Drop for TerminalSessionGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        self.restore(&mut stdout, false);
    }
}

pub(super) fn raw_mode_is_acquired(raw_mode_was_enabled: bool) -> bool {
    !raw_mode_was_enabled
}

pub(super) fn write_session_enter(
    stdout: &mut impl Write,
    session: &TerminalSessionGuard,
) -> io::Result<()> {
    match session.mode {
        TerminalSessionMode::Fullscreen => {
            execute!(
                stdout,
                terminal::EnterAlternateScreen,
                cursor::Hide,
                EnableBracketedPaste
            )?;
        }
        TerminalSessionMode::Inline => {
            execute!(stdout, cursor::Hide, EnableBracketedPaste)?;
        }
    }

    // Focus-change reporting is independent of mouse capture — callers
    // routinely pause animations or clear hover state on focus loss even
    // without mouse support. Enabling it unconditionally matches modern
    // TUI conventions (zellij, helix, yazi) and the cost is one extra SGR
    // per session.
    execute!(stdout, EnableFocusChange)?;
    write_mouse_enable(stdout, session.mouse_mode)?;
    if session.kitty_keyboard {
        let _ = write_kitty_keyboard_push(stdout, session.report_all_keys);
    }

    Ok(())
}

/// DEC private modes requested for each [`crate::MouseMode`], in enable order
/// (#444). SGR (`?1006`) is always last on enable and first on disable.
/// Windows consoles use crossterm's console-API capture instead.
#[cfg(not(windows))]
pub(super) fn mouse_mode_sequences(mode: crate::MouseMode) -> &'static [u16] {
    match mode {
        crate::MouseMode::Off => &[],
        crate::MouseMode::Click => &[1000, 1006],
        crate::MouseMode::Drag => &[1000, 1002, 1006],
        crate::MouseMode::Hover => &[1000, 1003, 1006],
    }
}

/// Enable mouse reporting at `mode`'s level. Writes the DEC modes directly
/// (crossterm's `EnableMouseCapture` always requests any-motion `?1003`).
/// Windows consoles get crossterm's console-API capture instead, and the run
/// loop filters events down to `mode`.
pub(super) fn write_mouse_enable(
    stdout: &mut impl Write,
    mode: crate::MouseMode,
) -> io::Result<()> {
    if mode == crate::MouseMode::Off {
        return Ok(());
    }
    #[cfg(windows)]
    {
        execute!(stdout, EnableMouseCapture)
    }
    #[cfg(not(windows))]
    {
        for code in mouse_mode_sequences(mode) {
            write!(stdout, "\x1b[?{code}h")?;
        }
        stdout.flush()
    }
}

/// Disable mouse reporting enabled by [`write_mouse_enable`], in reverse order.
pub(super) fn write_mouse_disable(
    stdout: &mut impl Write,
    mode: crate::MouseMode,
) -> io::Result<()> {
    if mode == crate::MouseMode::Off {
        return Ok(());
    }
    #[cfg(windows)]
    {
        execute!(stdout, DisableMouseCapture)
    }
    #[cfg(not(windows))]
    {
        for code in mouse_mode_sequences(mode).iter().rev() {
            write!(stdout, "\x1b[?{code}l")?;
        }
        stdout.flush()
    }
}

/// Assemble the Kitty keyboard enhancement flags to push.
///
/// Always sets `DISAMBIGUATE_ESCAPE_CODES | REPORT_EVENT_TYPES`. When
/// `report_all_keys` is `true`, also OR-es in
/// `REPORT_ALL_KEYS_AS_ESCAPE_CODES`, which is the only mechanism by which a
/// spec-compliant terminal emits a bare modifier as a key event.
///
/// This is a pure helper so the flag assembly can be unit-tested without
/// touching stdout.
pub(super) fn kitty_flags(report_all_keys: bool) -> crossterm::event::KeyboardEnhancementFlags {
    use crossterm::event::KeyboardEnhancementFlags;
    let mut flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES;
    if report_all_keys {
        flags |= KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
    }
    flags
}

/// Write Kitty keyboard protocol commands directly instead of routing them
/// through crossterm's legacy Windows API, where these commands are reported
/// as unsupported even when the terminal host accepts ANSI sequences.
fn write_kitty_keyboard_push(stdout: &mut impl Write, report_all_keys: bool) -> io::Result<()> {
    write!(stdout, "\x1b[>{}u", kitty_flags(report_all_keys).bits())
}

fn write_kitty_keyboard_pop(stdout: &mut impl Write) -> io::Result<()> {
    stdout.write_all(b"\x1b[<1u")
}

pub(super) fn write_session_cleanup(
    stdout: &mut impl Write,
    mode: TerminalSessionMode,
    inline_reserved: bool,
) -> io::Result<()> {
    execute!(
        stdout,
        ResetColor,
        SetAttribute(Attribute::Reset),
        cursor::Show,
        DisableBracketedPaste
    )?;

    match mode {
        TerminalSessionMode::Fullscreen => {
            execute!(stdout, terminal::LeaveAlternateScreen)?;
        }
        TerminalSessionMode::Inline => {
            if inline_reserved {
                execute!(
                    stdout,
                    cursor::MoveToColumn(0),
                    cursor::MoveDown(1),
                    cursor::MoveToColumn(0),
                    Print("\n")
                )?;
            } else {
                execute!(stdout, Print("\n"))?;
            }
        }
    }

    Ok(())
}

pub(super) fn write_session_exit(
    stdout: &mut impl Write,
    mode: TerminalSessionMode,
    inline_reserved: bool,
    mouse_mode: crate::MouseMode,
    kitty_keyboard: bool,
) -> io::Result<()> {
    // Issue #468: undo title / progress / status / cursor-shape changes made
    // by this process (no bytes when nothing was emitted).
    let _ = write_terminal_meta_restore(stdout);
    if kitty_keyboard {
        write_kitty_keyboard_pop(stdout)?;
    }
    // On Windows this uses crossterm's process-global console mode state,
    // which may not have been initialized when panic cleanup runs. Mouse
    // restoration is best-effort so it cannot prevent the core cursor,
    // paste, and screen cleanup below.
    let _ = write_mouse_disable(stdout, mouse_mode);
    let _ = execute!(stdout, DisableFocusChange);
    write_session_cleanup(stdout, mode, inline_reserved)
}

pub(super) fn panic_cleanup_snapshots(aborting: bool) -> Vec<SessionSnapshot> {
    if aborting {
        ACTIVE_SESSIONS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .rev()
            .map(|session| session.snapshot)
            .collect()
    } else {
        active_session_snapshot().into_iter().collect()
    }
}

/// Unwind restores this thread's session; process abort restores every active
/// session because neither other threads nor their guards will run again.
#[cfg(feature = "crossterm")]
pub(crate) fn cleanup_after_panic() -> bool {
    let snapshots = panic_cleanup_snapshots(cfg!(panic = "abort"));
    if snapshots.is_empty() {
        return false;
    }
    let mut stdout = io::stdout();
    for snapshot in &snapshots {
        let _ = write_session_exit(
            &mut stdout,
            snapshot.mode,
            false,
            snapshot.mouse_mode,
            snapshot.kitty_keyboard,
        );
    }
    if snapshots.iter().any(|snapshot| snapshot.raw_mode_owned) {
        let _ = terminal::disable_raw_mode();
    }
    let _ = stdout.flush();
    true
}

#[cfg(test)]
pub(super) fn write_panic_cleanup(stdout: &mut impl Write) -> io::Result<()> {
    write_session_exit(
        stdout,
        TerminalSessionMode::Fullscreen,
        false,
        crate::MouseMode::Hover,
        true,
    )
}

// ---------------------------------------------------------------------------
// Unix job-control suspend/resume (Ctrl+Z / `fg`) — issue #263
// ---------------------------------------------------------------------------
//
// On Unix, SIGTSTP stops the process in kernel space with no Rust code on the
// stack, so neither `Drop` nor the panic hook can restore the terminal. The
// run loops install a `signal-hook` background thread that, on SIGTSTP, runs
// the same teardown the session guard would (`disable_raw_mode`, leave alt
// screen, show cursor, disable paste/focus/mouse/kitty) and then re-raises
// SIGTSTP to genuinely stop; on SIGCONT it re-enters the session and flags a
// full redraw. The whole feature is `#[cfg(unix)]` and uses only signal-hook's
// safe API, preserving `#![forbid(unsafe_code)]`.

/// Set by the SIGCONT handler and consumed once at the top of each run-loop
/// iteration to force a full clear + repaint after resuming from suspend.
#[cfg(unix)]
pub(crate) static NEEDS_FULL_REDRAW: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(unix)]
impl Terminal {
    /// Capture the session state the suspend/resume handler needs to restore
    /// and re-enter this fullscreen terminal across Ctrl+Z / `fg`.
    pub(crate) fn session_snapshot(&self) -> SessionSnapshot {
        self.session.snapshot()
    }
}

#[cfg(unix)]
impl InlineTerminal {
    /// Capture the session state the suspend/resume handler needs to restore
    /// and re-enter this inline terminal across Ctrl+Z / `fg`.
    pub(crate) fn session_snapshot(&self) -> SessionSnapshot {
        self.session.snapshot()
    }
}

/// Write the escape sequences that tear down the TUI session in preparation
/// for SIGTSTP (the inverse of [`write_session_enter`]).
///
/// `inline_reserved` is passed `false` to [`write_session_cleanup`] to avoid
/// emitting the inline trailing-newline dance mid-session; the reserved region
/// is repainted on resume via the forced full redraw. Pure byte output, no
/// raw-mode toggle — split out so it can be unit-tested against a `Vec<u8>`.
#[cfg(unix)]
pub(super) fn write_suspend_sequence(
    stdout: &mut impl Write,
    snapshot: &SessionSnapshot,
) -> io::Result<()> {
    write_session_exit(
        stdout,
        snapshot.mode,
        false,
        snapshot.mouse_mode,
        snapshot.kitty_keyboard,
    )
}

/// Restore the terminal to cooked/non-TUI state in preparation for the process
/// being stopped by SIGTSTP.
///
/// Mirrors [`TerminalSessionGuard::restore`] but writes directly to
/// `io::stdout()` (the handler runs on a background thread that does not own
/// the buffered terminal stdout).
#[cfg(unix)]
pub(crate) fn suspend_to_shell(snapshot: &SessionSnapshot) {
    let mut out = io::stdout();
    let _ = write_suspend_sequence(&mut out, snapshot);
    if snapshot.raw_mode_owned {
        let _ = terminal::disable_raw_mode();
    }
    let _ = out.flush();
}

/// Re-enter the TUI session after a SIGCONT (resume via `fg`), matching the
/// original [`SessionSnapshot`], and flag a full redraw for the next frame.
///
/// Mirrors [`TerminalSessionGuard::enter`] but writes directly to
/// `io::stdout()`. Sets [`NEEDS_FULL_REDRAW`] so the next loop iteration clears
/// the front buffer and repaints every cell.
#[cfg(unix)]
pub(crate) fn resume_from_shell(snapshot: &SessionSnapshot) {
    let mut out = io::stdout();
    if snapshot.raw_mode_owned {
        let _ = terminal::enable_raw_mode();
    }
    let _ = resume_from_shell_with_writer(&mut out, snapshot);
}

#[cfg(unix)]
pub(super) fn resume_from_shell_with_writer(
    out: &mut impl Write,
    snapshot: &SessionSnapshot,
) -> io::Result<()> {
    let guard = TerminalSessionGuard {
        mode: snapshot.mode,
        mouse_mode: snapshot.mouse_mode,
        kitty_keyboard: snapshot.kitty_keyboard,
        report_all_keys: snapshot.report_all_keys,
        raw_mode_owned: snapshot.raw_mode_owned,
        registry_id: None,
        restored: std::sync::atomic::AtomicBool::new(false),
        harness: true,
    };
    write_session_enter(out, &guard)?;
    out.flush()?;
    NEEDS_FULL_REDRAW.store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// Construct a [`SessionSnapshot`] for tests without a live terminal.
#[cfg(all(unix, test))]
pub(super) fn test_snapshot(
    mode: TerminalSessionMode,
    mouse: bool,
    kitty: bool,
) -> SessionSnapshot {
    SessionSnapshot {
        mode,
        mouse_mode: crate::MouseMode::from(mouse),
        kitty_keyboard: kitty,
        report_all_keys: false,
        raw_mode_owned: true,
    }
}

/// Construct a fullscreen [`SessionSnapshot`] for crate-level tests that drive
/// the suspend handler without a live terminal (issue #263).
#[cfg(all(unix, test))]
pub(crate) fn test_session_snapshot() -> SessionSnapshot {
    SessionSnapshot {
        mode: TerminalSessionMode::Fullscreen,
        mouse_mode: crate::MouseMode::Off,
        kitty_keyboard: false,
        report_all_keys: false,
        raw_mode_owned: true,
    }
}
