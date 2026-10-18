//! Ctrl+Z suspend / resume handling for native Unix sessions.

use std::io;

use crate::terminal;

/// RAII guard owning the unix suspend/resume (`SIGTSTP`/`SIGCONT`) handler
/// thread for the duration of a run loop (issue #263).
///
/// Dropping the guard closes the `signal-hook` registration so the background
/// thread breaks out of `Signals::forever()` and is joined, leaving no signal
/// handlers installed after the loop exits.
#[cfg(all(feature = "crossterm", unix))]
pub(super) struct SuspendGuard {
    handle: signal_hook::iterator::Handle,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(all(feature = "crossterm", unix))]
impl Drop for SuspendGuard {
    fn drop(&mut self) {
        // Closing the handle wakes `Signals::forever()` so the thread returns.
        self.handle.close();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Install the unix job-control suspend/resume handler for one run loop.
///
/// Spawns a `signal-hook` background thread that, on `SIGTSTP`, restores the
/// terminal and re-raises the default-disposition stop, and on `SIGCONT`
/// re-enters the session and flags a full redraw. Uses only signal-hook's safe
/// API, preserving `#![forbid(unsafe_code)]`. Returns the guard that owns the
/// thread; dropping it uninstalls the handler.
#[cfg(all(feature = "crossterm", unix))]
pub(super) fn install_suspend_handler(
    snapshot: terminal::SessionSnapshot,
) -> io::Result<SuspendGuard> {
    use signal_hook::consts::{SIGCONT, SIGTSTP};
    use signal_hook::iterator::Signals;

    let mut signals = Signals::new([SIGTSTP, SIGCONT])?;
    let handle = signals.handle();
    let thread = std::thread::Builder::new()
        .name("slt-suspend".to_string())
        .spawn(move || {
            // `has_terminal` tracks whether the TUI session is currently
            // entered, so a stray SIGCONT (no prior SIGTSTP) or a repeated
            // SIGTSTP cannot double-leave / double-enter (idempotency).
            let mut has_terminal = true;
            for signal in &mut signals {
                match signal {
                    SIGTSTP if has_terminal => {
                        terminal::suspend_to_shell(&snapshot);
                        has_terminal = false;
                        // Genuinely stop the process now that the terminal is
                        // restored; control returns to the shell.
                        let _ = signal_hook::low_level::emulate_default_handler(SIGTSTP);
                    }
                    SIGCONT if !has_terminal => {
                        terminal::resume_from_shell(&snapshot);
                        has_terminal = true;
                    }
                    // Repeated SIGTSTP/SIGCONT or out-of-order delivery is a
                    // no-op — the `has_terminal` guard keeps enter/leave
                    // balanced (idempotency, issue #263).
                    _ => {}
                }
            }
        })?;

    Ok(SuspendGuard {
        handle,
        thread: Some(thread),
    })
}

#[cfg(all(feature = "crossterm", unix))]
pub(super) fn suspend_current_session(snapshot: terminal::SessionSnapshot) -> io::Result<()> {
    terminal::suspend_to_shell(&snapshot);
    let result = signal_hook::low_level::emulate_default_handler(signal_hook::consts::SIGTSTP);
    terminal::resume_from_shell(&snapshot);
    result?;
    Ok(())
}

/// Consume the pending full-redraw request raised by a `SIGCONT` resume and, if
/// set, clear + repaint the whole frame (issue #263).
///
/// Called at the top of each run-loop iteration. No-op on non-unix builds.
/// Returns whether a repaint was requested, so an idle-skipping loop renders
/// the frame (#490).
#[cfg(all(feature = "crossterm", unix))]
pub(super) fn drain_resume_redraw(
    handle_resize: &mut impl FnMut() -> io::Result<()>,
) -> io::Result<bool> {
    use std::sync::atomic::Ordering;
    if terminal::NEEDS_FULL_REDRAW.swap(false, Ordering::SeqCst) {
        handle_resize()?;
        return Ok(true);
    }
    Ok(false)
}
