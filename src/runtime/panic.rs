//! Panic hook installation and recoverable-panic scopes for terminal sessions.

#[cfg(feature = "crossterm")]
use crate::terminal;

#[cfg(feature = "crossterm")]
type PanicHook = Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static>;

std::thread_local! {
    static RECOVERABLE_PANICS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TERMINAL_PANIC_OWNER: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

struct PanicScopeGuard(&'static std::thread::LocalKey<std::cell::Cell<usize>>);

impl PanicScopeGuard {
    fn enter(slot: &'static std::thread::LocalKey<std::cell::Cell<usize>>) -> Self {
        slot.with(|depth| depth.set(depth.get().saturating_add(1)));
        Self(slot)
    }
}

impl Drop for PanicScopeGuard {
    fn drop(&mut self) {
        self.0
            .with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

pub(crate) fn with_recoverable_panics<T>(f: impl FnOnce() -> T) -> T {
    let _scope = PanicScopeGuard::enter(&RECOVERABLE_PANICS);
    f()
}

pub(crate) fn catch_recoverable_unwind<T>(f: impl FnOnce() -> T) -> std::thread::Result<T> {
    with_recoverable_panics(|| std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)))
}

#[cfg(feature = "crossterm")]
pub(super) struct PanicHookState {
    pub(super) active_sessions: usize,
    pub(super) installed: bool,
    pub(super) previous: Option<PanicHook>,
}

#[cfg(feature = "crossterm")]
pub(super) static PANIC_HOOK_STATE: std::sync::Mutex<PanicHookState> =
    std::sync::Mutex::new(PanicHookState {
        active_sessions: 0,
        installed: false,
        previous: None,
    });

#[cfg(feature = "crossterm")]
pub(super) struct PanicHookGuard;

#[cfg(feature = "crossterm")]
impl Drop for PanicHookGuard {
    fn drop(&mut self) {
        let mut state = PANIC_HOOK_STATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.active_sessions = state.active_sessions.saturating_sub(1);

        // `take_hook` panics while the current thread is unwinding. In that
        // case the dispatcher remains installed but inactive and forwards
        // directly to the previous hook. A later normal session restores it.
        if state.active_sessions == 0 && state.installed && !std::thread::panicking() {
            let dispatcher = std::panic::take_hook();
            drop(dispatcher);
            if let Some(previous) = state.previous.take() {
                std::panic::set_hook(previous);
            }
            state.installed = false;
        }
    }
}

#[expect(
    clippy::print_stderr,
    reason = "the panic hook reports to stderr after restoring the terminal"
)]
#[cfg(feature = "crossterm")]
fn slt_panic_hook(panic_info: &std::panic::PanicHookInfo<'_>) {
    if cfg!(panic = "unwind") && RECOVERABLE_PANICS.with(|depth| depth.get() > 0) {
        return;
    }
    let active = PANIC_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .active_sessions
        > 0;
    let restored = active
        && (cfg!(panic = "abort") || TERMINAL_PANIC_OWNER.with(|depth| depth.get() > 0))
        && terminal::cleanup_after_panic();

    if restored {
        eprintln!("\n\x1b[1;31m━━━ SLT Panic ━━━\x1b[0m\n");
        if let Some(location) = panic_info.location() {
            eprintln!(
                "\x1b[90m{}:{}:{}\x1b[0m",
                location.file(),
                location.line(),
                location.column()
            );
        }
        if let Some(msg) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("\x1b[1m{msg}\x1b[0m");
        } else if let Some(msg) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("\x1b[1m{msg}\x1b[0m");
        }
        eprintln!(
            "\n\x1b[90mTerminal state restored. Report bugs at https://github.com/subinium/SuperLightTUI/issues\x1b[0m\n"
        );
    }

    let state = PANIC_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(previous) = state.previous.as_ref() {
        previous(panic_info);
    }
}

#[cfg(feature = "crossterm")]
pub(super) fn install_panic_hook() -> PanicHookGuard {
    let mut state = PANIC_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !state.installed {
        state.previous = Some(std::panic::take_hook());
        std::panic::set_hook(Box::new(slt_panic_hook));
        state.installed = true;
    }
    state.active_sessions = state.active_sessions.saturating_add(1);
    PanicHookGuard
}

#[cfg(feature = "crossterm")]
pub(super) fn with_session_panic_hook<T>(f: impl FnOnce() -> T) -> T {
    let guard = install_panic_hook();
    let owner = PanicScopeGuard::enter(&TERMINAL_PANIC_OWNER);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    drop(owner);
    drop(guard);
    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}
