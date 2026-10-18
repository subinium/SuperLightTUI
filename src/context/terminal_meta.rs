//! Terminal metadata requests recorded by [`Context`]: window title, taskbar
//! progress, program status and cursor shape (issue #468), plus the
//! clipboard delivery status of the previous frame (issue #446).
//!
//! Setters only record the desired state. The built-in terminal backends
//! compare it with what they last emitted and write escape sequences only on
//! change, so an idle frame adds no bytes. Custom backends and
//! [`crate::TestBackend`] never emit anything; the getters below expose the
//! recorded state for tests.

use super::Context;

/// Key in `named_states` holding the desired [`TerminalMeta`].
pub(crate) const TERMINAL_META_KEY: &str = "__slt_terminal_meta";
/// Key in `named_states` holding the previous frame's [`ClipboardStatus`].
pub(crate) const CLIPBOARD_STATUS_KEY: &str = "__slt_clipboard_status";

/// Taskbar / tab progress indicator, reported through `OSC 9;4`.
///
/// Fractions are clamped to `0.0..=1.0` and rounded to whole percent; `NaN`
/// is treated as `0.0`. Emitted only on terminals known to implement the
/// sequence (Windows Terminal, Ghostty, ConEmu, iTerm2 3.6+) or when
/// `SLT_FORCE_PROGRESS=1` is set, because older terminals treat `OSC 9` as a
/// desktop notification.
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// ui.set_progress(Some(slt::Progress::Normal(0.4)));
/// # });
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Progress {
    /// Normal progress at the given fraction (`OSC 9;4;1;<percent>`).
    Normal(f32),
    /// Error state at the given fraction (`OSC 9;4;2;<percent>`).
    Error(f32),
    /// Busy without a known fraction (`OSC 9;4;3`).
    Indeterminate,
    /// Paused / warning state at the given fraction (`OSC 9;4;4;<percent>`).
    Paused(f32),
}

impl Progress {
    /// `(state, percent)` parameters of the `OSC 9;4` sequence.
    #[cfg(feature = "crossterm")]
    pub(crate) fn osc_params(self) -> (u8, u8) {
        fn percent(fraction: f32) -> u8 {
            let fraction = if fraction.is_nan() { 0.0 } else { fraction };
            (fraction.clamp(0.0, 1.0) * 100.0).round() as u8
        }
        match self {
            Progress::Normal(f) => (1, percent(f)),
            Progress::Error(f) => (2, percent(f)),
            Progress::Indeterminate => (3, 0),
            Progress::Paused(f) => (4, percent(f)),
        }
    }
}

/// Hardware cursor shape, set through DECSCUSR (`CSI <n> SP q`).
///
/// [`CursorShape::Default`] restores the terminal's configured shape; it is
/// also what SLT writes when the session ends.
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// ui.set_cursor_shape(slt::CursorShape::SteadyBar);
/// # });
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CursorShape {
    /// The terminal's configured default shape (`CSI 0 SP q`).
    #[default]
    Default,
    /// Blinking block (`CSI 1 SP q`).
    BlinkingBlock,
    /// Steady block (`CSI 2 SP q`).
    SteadyBlock,
    /// Blinking underline (`CSI 3 SP q`).
    BlinkingUnderline,
    /// Steady underline (`CSI 4 SP q`).
    SteadyUnderline,
    /// Blinking vertical bar (`CSI 5 SP q`).
    BlinkingBar,
    /// Steady vertical bar (`CSI 6 SP q`).
    SteadyBar,
}

impl CursorShape {
    /// DECSCUSR parameter for this shape.
    #[cfg(feature = "crossterm")]
    pub(crate) fn decscusr(self) -> u8 {
        match self {
            CursorShape::Default => 0,
            CursorShape::BlinkingBlock => 1,
            CursorShape::SteadyBlock => 2,
            CursorShape::BlinkingUnderline => 3,
            CursorShape::SteadyUnderline => 4,
            CursorShape::BlinkingBar => 5,
            CursorShape::SteadyBar => 6,
        }
    }
}

/// State reported through the program-status protocol (`OSC 7501`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgramState {
    /// Waiting for input or work.
    Idle,
    /// Actively working; may carry a progress percentage.
    Working,
    /// Blocked on the user (permission, question, authentication).
    Blocked,
    /// Finished successfully. The terminal keeps this record after exit.
    Done,
    /// Finished with an error. The terminal keeps this record after exit.
    Error,
}

impl ProgramState {
    #[cfg(feature = "crossterm")]
    pub(crate) fn wire(self) -> &'static str {
        match self {
            ProgramState::Idle => "idle",
            ProgramState::Working => "working",
            ProgramState::Blocked => "blocked",
            ProgramState::Done => "done",
            ProgramState::Error => "error",
        }
    }
}

/// Program status record for the `OSC 7501` protocol
/// (<https://www.superlogical.com/rex/docs/build/program-status>).
///
/// SLT reports the root record only. Terminals that do not implement the
/// protocol ignore the sequence; it is never sent under tmux / screen /
/// zellij or to `TERM=dumb`.
///
/// # Example
///
/// ```no_run
/// use slt::{ProgramState, ProgramStatus};
/// # slt::run(|ui: &mut slt::Context| {
/// ui.set_status(Some(
///     ProgramStatus::new(ProgramState::Working)
///         .progress(40)
///         .message("compiling"),
/// ));
/// # });
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProgramStatus {
    /// Reported state.
    pub state: ProgramState,
    /// Percentage `0..=100`, meaningful with [`ProgramState::Working`] and
    /// [`ProgramState::Blocked`]. `None` means indeterminate.
    pub progress: Option<u8>,
    /// One human-readable line. Control characters are replaced and the text
    /// is truncated to the protocol's 2048-byte limit before encoding.
    pub message: Option<String>,
}

impl ProgramStatus {
    /// Create a status record with no progress and no message.
    pub fn new(state: ProgramState) -> Self {
        Self {
            state,
            progress: None,
            message: None,
        }
    }

    /// Attach a progress percentage (clamped to `100`).
    #[must_use]
    pub fn progress(mut self, percent: u8) -> Self {
        self.progress = Some(percent.min(100));
        self
    }

    /// Attach a one-line message.
    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Outcome of the most recent clipboard write (`OSC 52`), visible for one
/// frame through [`Context::clipboard_status`].
///
/// `OSC 52` has no acknowledgement: [`ClipboardStatus::Sent`] means the
/// sequence was written to the terminal, not that the host clipboard
/// received it (tmux with `set-clipboard external`, for example, drops it).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardStatus {
    /// The sequence was written and flushed.
    Sent,
    /// The UTF-8 payload exceeded [`crate::RunConfig::clipboard_max_bytes`]
    /// and nothing was written.
    TooLarge {
        /// Payload size in bytes.
        bytes: usize,
        /// Configured limit in bytes.
        max: usize,
    },
    /// Writing or flushing the sequence failed.
    WriteFailed(std::io::ErrorKind),
}

/// Desired terminal metadata recorded by the `Context` setters.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TerminalMeta {
    pub(crate) title: Option<String>,
    pub(crate) progress: Option<Progress>,
    pub(crate) status: Option<ProgramStatus>,
    pub(crate) cursor_shape: CursorShape,
    /// OSC 22 pointer shape for this frame, `None` for the default pointer
    /// (#452). Derived from the hovered slot, not a sticky setter.
    pub(crate) pointer_shape: Option<crate::PointerShape>,
}

type NamedStates = std::collections::HashMap<&'static str, Box<dyn std::any::Any>>;

/// Read the desired metadata recorded in `named_states`, if any.
pub(crate) fn terminal_meta(named_states: &NamedStates) -> Option<&TerminalMeta> {
    named_states
        .get(TERMINAL_META_KEY)
        .and_then(|boxed| boxed.downcast_ref::<TerminalMeta>())
}

/// Replace the clipboard status surfaced to the next frame.
pub(crate) fn store_clipboard_status(
    named_states: &mut NamedStates,
    status: Option<ClipboardStatus>,
) {
    match status {
        Some(status) => {
            named_states.insert(CLIPBOARD_STATUS_KEY, Box::new(status));
        }
        None => {
            named_states.remove(CLIPBOARD_STATUS_KEY);
        }
    }
}

impl Context {
    fn terminal_meta_mut(&mut self) -> &mut TerminalMeta {
        let entry = self
            .named_states
            .entry(TERMINAL_META_KEY)
            .or_insert_with(|| Box::new(TerminalMeta::default()) as Box<dyn std::any::Any>);
        if !entry.is::<TerminalMeta>() {
            *entry = Box::new(TerminalMeta::default());
        }
        entry
            .downcast_mut::<TerminalMeta>()
            .expect("terminal meta slot holds TerminalMeta")
    }

    /// Set the terminal window / tab title (`OSC 2`).
    ///
    /// The title persists until changed and replaces
    /// [`RunConfig::title`](crate::RunConfig::title). It is written only when it
    /// differs from the last emitted title, with control and C1 characters
    /// replaced. The original title is saved on the first change (XTWINOPS 22)
    /// and restored when the session ends (XTWINOPS 23) on terminals that
    /// keep a title stack.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.set_title("agent: running 3/10");
    /// # });
    /// ```
    pub fn set_title(&mut self, title: impl Into<String>) {
        let title = title.into();
        let meta = self.terminal_meta_mut();
        if meta.title.as_deref() != Some(title.as_str()) {
            meta.title = Some(title);
        }
    }

    /// Set or clear the taskbar / tab progress indicator (`OSC 9;4`).
    ///
    /// The value persists until changed; `None` removes the indicator. The
    /// indicator is also removed when the session ends. See [`Progress`] for
    /// which terminals receive the sequence.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.set_progress(Some(slt::Progress::Indeterminate));
    /// # });
    /// ```
    pub fn set_progress(&mut self, progress: Option<Progress>) {
        self.terminal_meta_mut().progress = progress;
    }

    /// Set or clear the program status record (`OSC 7501`).
    ///
    /// `None` clears the record. On exit a `working`, `blocked` or `idle`
    /// record is cleared; `done` and `error` records are left for the
    /// terminal to keep, as the protocol intends.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{ProgramState, ProgramStatus};
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.set_status(Some(ProgramStatus::new(ProgramState::Done)));
    /// # });
    /// ```
    pub fn set_status(&mut self, status: Option<ProgramStatus>) {
        let meta = self.terminal_meta_mut();
        if meta.status != status {
            meta.status = status;
        }
    }

    /// Set the hardware cursor shape (DECSCUSR).
    ///
    /// The shape persists until changed and is reset to
    /// [`CursorShape::Default`] when the session ends.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.set_cursor_shape(slt::CursorShape::BlinkingBar);
    /// # });
    /// ```
    pub fn set_cursor_shape(&mut self, shape: CursorShape) {
        self.terminal_meta_mut().cursor_shape = shape;
    }

    /// Title recorded by [`set_title`](Self::set_title), if any.
    pub fn terminal_title(&self) -> Option<&str> {
        terminal_meta(&self.named_states).and_then(|meta| meta.title.as_deref())
    }

    /// Progress recorded by [`set_progress`](Self::set_progress).
    pub fn terminal_progress(&self) -> Option<Progress> {
        terminal_meta(&self.named_states).and_then(|meta| meta.progress)
    }

    /// Status recorded by [`set_status`](Self::set_status).
    pub fn program_status(&self) -> Option<&ProgramStatus> {
        terminal_meta(&self.named_states).and_then(|meta| meta.status.as_ref())
    }

    /// Cursor shape recorded by [`set_cursor_shape`](Self::set_cursor_shape).
    pub fn cursor_shape(&self) -> CursorShape {
        terminal_meta(&self.named_states)
            .map(|meta| meta.cursor_shape)
            .unwrap_or_default()
    }

    /// Outcome of the clipboard write performed after the previous frame.
    ///
    /// `Some` for exactly one frame after a [`copy_to_clipboard`] call or a
    /// mouse-selection copy, `None` otherwise. Custom backends and
    /// [`crate::TestBackend`] never write the clipboard, so this stays `None`
    /// there.
    ///
    /// [`copy_to_clipboard`]: Self::copy_to_clipboard
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// if let Some(slt::ClipboardStatus::TooLarge { .. }) = ui.clipboard_status() {
    ///     ui.notify("selection too large to copy", slt::ToastLevel::Warning);
    /// }
    /// # });
    /// ```
    pub fn clipboard_status(&self) -> Option<ClipboardStatus> {
        self.named_states
            .get(CLIPBOARD_STATUS_KEY)
            .and_then(|boxed| boxed.downcast_ref::<ClipboardStatus>())
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestBackend;

    #[test]
    fn issue_468_setters_are_observable_on_test_backend() {
        let mut tb = TestBackend::new(10, 2);
        tb.render(|ui| {
            assert_eq!(ui.terminal_title(), None);
            assert_eq!(ui.cursor_shape(), CursorShape::Default);
            ui.set_title("one");
            ui.set_progress(Some(Progress::Normal(0.4)));
            ui.set_status(Some(ProgramStatus::new(ProgramState::Working).progress(7)));
            ui.set_cursor_shape(CursorShape::SteadyBar);
        });
        tb.render(|ui| {
            assert_eq!(ui.terminal_title(), Some("one"));
            assert_eq!(ui.terminal_progress(), Some(Progress::Normal(0.4)));
            assert_eq!(
                ui.program_status().map(|s| (s.state, s.progress)),
                Some((ProgramState::Working, Some(7)))
            );
            assert_eq!(ui.cursor_shape(), CursorShape::SteadyBar);
            assert_eq!(ui.clipboard_status(), None);
            ui.set_progress(None);
            ui.set_status(None);
        });
        tb.render(|ui| {
            assert_eq!(ui.terminal_progress(), None);
            assert!(ui.program_status().is_none());
        });
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn issue_468_progress_params_clamp_and_round() {
        assert_eq!(Progress::Normal(0.4).osc_params(), (1, 40));
        assert_eq!(Progress::Error(2.0).osc_params(), (2, 100));
        assert_eq!(Progress::Paused(-1.0).osc_params(), (4, 0));
        assert_eq!(Progress::Normal(f32::NAN).osc_params(), (1, 0));
        assert_eq!(Progress::Indeterminate.osc_params(), (3, 0));
        assert_eq!(
            ProgramStatus::new(ProgramState::Idle)
                .progress(250)
                .progress,
            Some(100)
        );
    }
}
