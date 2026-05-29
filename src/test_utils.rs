//! Headless testing utilities.
//!
//! [`TestBackend`] renders a UI closure to an in-memory buffer without a real
//! terminal. [`EventBuilder`] constructs event sequences for simulating user
//! input. Together they enable snapshot and assertion-based UI testing.

use crate::buffer::Buffer;
use crate::context::Context;
use crate::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseKind,
};
use crate::rect::Rect;
use crate::style::Style;
use crate::{run_frame_kernel, FrameState, RunConfig};

/// Builder for constructing a sequence of input [`Event`]s.
///
/// Chain calls to [`key`](EventBuilder::key), [`click`](EventBuilder::click),
/// [`scroll_up`](EventBuilder::scroll_up), etc., then call
/// [`build`](EventBuilder::build) to get the final `Vec<Event>`.
///
/// # Example
///
/// ```
/// use slt::EventBuilder;
/// use slt::KeyCode;
///
/// let events = EventBuilder::new()
///     .key('a')
///     .key_code(KeyCode::Enter)
///     .build();
/// assert_eq!(events.len(), 2);
/// ```
pub struct EventBuilder {
    events: Vec<Event>,
}

impl EventBuilder {
    /// Create an empty event builder.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Append a character key-press event.
    pub fn key(mut self, c: char) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
        }));
        self
    }

    /// Append a special key-press event (arrows, Enter, Esc, etc.).
    pub fn key_code(mut self, code: KeyCode) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
        }));
        self
    }

    /// Append a key-press event with modifier keys (Ctrl, Shift, Alt).
    pub fn key_with(mut self, code: KeyCode, modifiers: KeyModifiers) -> Self {
        self.events.push(Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
        }));
        self
    }

    /// Append a left mouse click at terminal position `(x, y)`.
    pub fn click(mut self, x: u32, y: u32) -> Self {
        self.events.push(Event::Mouse(MouseEvent {
            kind: MouseKind::Down(MouseButton::Left),
            x,
            y,
            modifiers: KeyModifiers::NONE,
            pixel_x: None,
            pixel_y: None,
        }));
        self
    }

    /// Append a left mouse button release at terminal position `(x, y)`.
    pub fn mouse_up(mut self, x: u32, y: u32) -> Self {
        self.events.push(Event::mouse_up(x, y));
        self
    }

    /// Append a mouse drag (movement with the left button held) at `(x, y)`.
    pub fn drag(mut self, x: u32, y: u32) -> Self {
        self.events.push(Event::mouse_drag(x, y));
        self
    }

    /// Append a key-release event for character `c`.
    ///
    /// Only meaningful on terminals that emit release events
    /// (e.g. with the Kitty keyboard protocol enabled).
    pub fn key_release(mut self, c: char) -> Self {
        self.events.push(Event::key_release(c));
        self
    }

    /// Append a terminal focus-gained event.
    pub fn focus_gained(mut self) -> Self {
        self.events.push(Event::FocusGained);
        self
    }

    /// Append a terminal focus-lost event.
    pub fn focus_lost(mut self) -> Self {
        self.events.push(Event::FocusLost);
        self
    }

    /// Append a scroll-up event at `(x, y)`.
    pub fn scroll_up(mut self, x: u32, y: u32) -> Self {
        self.events.push(Event::Mouse(MouseEvent {
            kind: MouseKind::ScrollUp,
            x,
            y,
            modifiers: KeyModifiers::NONE,
            pixel_x: None,
            pixel_y: None,
        }));
        self
    }

    /// Append a scroll-down event at `(x, y)`.
    pub fn scroll_down(mut self, x: u32, y: u32) -> Self {
        self.events.push(Event::Mouse(MouseEvent {
            kind: MouseKind::ScrollDown,
            x,
            y,
            modifiers: KeyModifiers::NONE,
            pixel_x: None,
            pixel_y: None,
        }));
        self
    }

    /// Append a bracketed-paste event.
    pub fn paste(mut self, text: impl Into<String>) -> Self {
        self.events.push(Event::Paste(text.into()));
        self
    }

    /// Append a terminal resize event.
    pub fn resize(mut self, width: u32, height: u32) -> Self {
        self.events.push(Event::Resize(width, height));
        self
    }

    /// Consume the builder and return the event sequence.
    pub fn build(self) -> Vec<Event> {
        self.events
    }
}

impl Default for EventBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Headless rendering backend for tests.
///
/// Renders a UI closure to an in-memory [`Buffer`] without a real terminal.
/// Use [`render`](TestBackend::render) to run one frame, then inspect the
/// output with [`line`](TestBackend::line), [`assert_contains`](TestBackend::assert_contains),
/// or [`to_string_trimmed`](TestBackend::to_string_trimmed).
/// Session state persists across renders, so multi-frame tests can exercise
/// hooks, focus, and previous-frame hit testing.
///
/// # Example
///
/// ```
/// use slt::TestBackend;
///
/// let mut backend = TestBackend::new(40, 10);
/// backend.render(|ui| {
///     ui.text("hello");
/// });
/// backend.assert_contains("hello");
/// ```
pub struct TestBackend {
    buffer: Buffer,
    width: u32,
    height: u32,
    frame_state: FrameState,
    /// Frame history. `None` = recording disabled (zero overhead).
    /// `Some(_)` = recording enabled — every [`render`](TestBackend::render)
    /// call appends a [`FrameRecord`].
    frames: Option<Vec<FrameRecord>>,
}

/// Snapshot of a single rendered frame, captured by
/// [`TestBackend::record_frames`].
///
/// Stores the styled snapshot string (via [`Buffer::snapshot_format`]) plus a
/// per-row trimmed text view for ergonomic substring assertions. Both are
/// produced from the same buffer and are guaranteed to refer to the same
/// frame.
///
/// Cheap to clone; useful for replaying a failing test by inspecting
/// intermediate frames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameRecord {
    /// Styled snapshot of the buffer at this frame, in the stable
    /// [`Buffer::snapshot_format`] vocabulary.
    pub snapshot: String,
    /// Plain-text view of each buffer row, trailing spaces trimmed.
    /// Mirrors [`TestBackend::line`] for every row.
    pub lines: Vec<String>,
}

impl FrameRecord {
    /// Return the frame as a multi-line string (rows joined with `\n`,
    /// trailing empty rows preserved). Mirrors [`TestBackend::to_string_trimmed`]
    /// on the originating buffer.
    pub fn to_string_trimmed(&self) -> String {
        let mut lines = self.lines.clone();
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }

    /// Return the trimmed text of row `y` from this frame, or empty if `y`
    /// is past the buffer height.
    pub fn line(&self, y: u32) -> &str {
        self.lines
            .get(y as usize)
            .map(|s| s.as_str())
            .unwrap_or_default()
    }

    /// Assert any row in this frame contains `expected`. Panics with a
    /// row-by-row dump on failure.
    pub fn assert_contains(&self, expected: &str) {
        for line in &self.lines {
            if line.contains(expected) {
                return;
            }
        }
        let mut detail = String::new();
        for (y, line) in self.lines.iter().enumerate() {
            detail.push_str(&format!("  {y}: {line}\n"));
        }
        panic!("FrameRecord does not contain {expected:?}.\nFrame:\n{detail}");
    }
}

impl TestBackend {
    /// Create a test backend with the given terminal dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        let area = Rect::new(0, 0, width, height);
        Self {
            buffer: Buffer::empty(area),
            width,
            height,
            frame_state: FrameState::default(),
            frames: None,
        }
    }

    /// Enable frame recording.
    ///
    /// After this call, every subsequent [`render`](TestBackend::render),
    /// [`render_with_events`](TestBackend::render_with_events), and
    /// [`run_with_events`](TestBackend::run_with_events) call appends a
    /// [`FrameRecord`] to the internal history. Disabled by default so tests
    /// that don't need history pay zero memory overhead.
    ///
    /// Returns `self` for chaining.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::TestBackend;
    ///
    /// let mut tb = TestBackend::new(20, 3).record_frames();
    /// for n in 0..3 {
    ///     tb.render(|ui| {
    ///         ui.text(format!("frame {n}"));
    ///     });
    /// }
    /// assert_eq!(tb.frames().len(), 3);
    /// tb.frames()[0].assert_contains("frame 0");
    /// tb.frames()[2].assert_contains("frame 2");
    /// ```
    pub fn record_frames(mut self) -> Self {
        if self.frames.is_none() {
            self.frames = Some(Vec::new());
        }
        self
    }

    /// Return all captured frame snapshots in chronological order.
    ///
    /// Returns an empty slice if [`record_frames`](TestBackend::record_frames)
    /// was never called on this backend.
    pub fn frames(&self) -> &[FrameRecord] {
        self.frames.as_deref().unwrap_or(&[])
    }

    /// Capture the current buffer state into the recording, if enabled.
    ///
    /// No-op when recording is off — keeps the hot path allocation-free
    /// for the common case.
    fn capture_frame(&mut self) {
        if let Some(frames) = self.frames.as_mut() {
            let snapshot = self.buffer.snapshot_format();
            let mut lines = Vec::with_capacity(self.height as usize);
            for y in 0..self.height {
                let mut s = String::new();
                for x in 0..self.width {
                    s.push_str(&self.buffer.get(x, y).symbol);
                }
                lines.push(s.trim_end().to_string());
            }
            frames.push(FrameRecord { snapshot, lines });
        }
    }

    fn render_frame(
        &mut self,
        events: Vec<Event>,
        setup_state: impl FnOnce(&mut FrameState),
        f: impl FnOnce(&mut Context),
    ) {
        setup_state(&mut self.frame_state);

        self.buffer.reset();
        let mut once = Some(f);
        let mut render = |ui: &mut Context| {
            if let Some(f) = once.take() {
                f(ui);
            } else {
                panic!("render closure called twice");
            }
        };
        let _ = run_frame_kernel(
            &mut self.buffer,
            &mut self.frame_state,
            &RunConfig::default(),
            (self.width, self.height),
            events,
            false,
            &mut render,
        );
        self.capture_frame();
    }

    /// Run a UI closure for one frame and render to the internal buffer.
    pub fn render(&mut self, f: impl FnOnce(&mut Context)) {
        self.render_frame(Vec::new(), |_| {}, f);
    }

    /// Render with injected events and focus state for interaction testing.
    pub fn render_with_events(
        &mut self,
        events: Vec<Event>,
        focus_index: usize,
        prev_focus_count: usize,
        f: impl FnOnce(&mut Context),
    ) {
        self.render_frame(
            events,
            |state| {
                state.focus.focus_index = focus_index;
                state.focus.prev_focus_count = prev_focus_count;
            },
            f,
        );
    }

    /// Convenience wrapper: render with events using default focus state.
    pub fn run_with_events(&mut self, events: Vec<Event>, f: impl FnOnce(&mut crate::Context)) {
        self.render_with_events(events, 0, 0, f);
    }

    /// Number of live frame-clock scheduler timer slots persisted after the
    /// most recent render (issue #248). Test-only — used to assert that
    /// abandoned timers are garbage-collected and `SchedulerState` does not
    /// grow without bound.
    #[cfg(test)]
    pub(crate) fn scheduler_slot_count(&self) -> usize {
        self.frame_state.scheduler.slot_count()
    }

    /// Get the rendered text content of row y (trimmed trailing spaces)
    pub fn line(&self, y: u32) -> String {
        let mut s = String::new();
        for x in 0..self.width {
            s.push_str(&self.buffer.get(x, y).symbol);
        }
        s.trim_end().to_string()
    }

    /// Assert that row y contains `expected` as a substring
    pub fn assert_line(&self, y: u32, expected: &str) {
        let line = self.line(y);
        assert_eq!(
            line, expected,
            "Line {y}: expected {expected:?}, got {line:?}"
        );
    }

    /// Assert that row y contains `expected` as a substring
    pub fn assert_line_contains(&self, y: u32, expected: &str) {
        let line = self.line(y);
        assert!(
            line.contains(expected),
            "Line {y}: expected to contain {expected:?}, got {line:?}"
        );
    }

    /// Assert that any line in the buffer contains `expected`
    pub fn assert_contains(&self, expected: &str) {
        for y in 0..self.height {
            if self.line(y).contains(expected) {
                return;
            }
        }
        let mut all_lines = String::new();
        for y in 0..self.height {
            all_lines.push_str(&format!("{}: {}\n", y, self.line(y)));
        }
        panic!("Buffer does not contain {expected:?}.\nBuffer:\n{all_lines}");
    }

    /// Access the underlying render buffer.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Terminal width used for this backend.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Terminal height used for this backend.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Return the full rendered buffer as a multi-line string.
    ///
    /// Each row is trimmed of trailing spaces and joined with newlines.
    /// Useful for snapshot testing with `insta::assert_snapshot!`.
    pub fn to_string_trimmed(&self) -> String {
        let mut lines = Vec::with_capacity(self.height as usize);
        for y in 0..self.height {
            lines.push(self.line(y));
        }
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines.join("\n")
    }

    // ---- Negative assertions (#232) ---------------------------------------

    /// Assert that no row in the buffer contains `expected` as a substring.
    ///
    /// Panics with the offending row indices and contents on failure.
    pub fn assert_not_contains(&self, expected: &str) {
        let mut offending: Vec<(u32, String)> = Vec::new();
        for y in 0..self.height {
            let line = self.line(y);
            if line.contains(expected) {
                offending.push((y, line));
            }
        }
        if !offending.is_empty() {
            let detail = offending
                .iter()
                .map(|(y, l)| format!("  row {y}: {l:?}"))
                .collect::<Vec<_>>()
                .join("\n");
            panic!("Buffer unexpectedly contains {expected:?}:\n{detail}");
        }
    }

    /// Assert that row `y` does NOT contain `expected` as a substring.
    pub fn assert_line_not_contains(&self, y: u32, expected: &str) {
        let line = self.line(y);
        assert!(
            !line.contains(expected),
            "Line {y}: expected NOT to contain {expected:?}, but got {line:?}"
        );
    }

    /// Assert that row `y` is entirely blank (contains no non-space content).
    ///
    /// Useful for verifying that cleared, padded, or overflow-suppressed rows
    /// render as empty.
    pub fn assert_empty_line(&self, y: u32) {
        let line = self.line(y);
        assert!(line.is_empty(), "Line {y}: expected empty, got {line:?}");
    }

    /// Assert that the cell at `(x, y)` carries exactly the `expected` style.
    ///
    /// Useful for focused color/modifier regression checks without committing
    /// to a full-buffer snapshot. Panics with `(x, y)`, the actual style, and
    /// the expected style on mismatch.
    pub fn assert_style_at(&self, x: u32, y: u32, expected: Style) {
        let actual = self.buffer.get(x, y).style;
        assert_eq!(
            actual, expected,
            "Style mismatch at ({x}, {y}): expected {expected:?}, got {actual:?}"
        );
    }

    // ---- Multi-step sequences + type_string (#230) ------------------------

    /// Begin building a multi-step interaction sequence.
    ///
    /// Each [`tick`](TestSequence::tick) (or [`key`](TestSequence::key))
    /// appends an event batch + render closure pair.
    /// [`run`](TestSequence::run) executes them in order, advancing
    /// `FrameState` naturally between steps so callers don't need to thread
    /// `focus_index` / `prev_focus_count` manually.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::{KeyCode, TestBackend};
    ///
    /// let mut tb = TestBackend::new(20, 3);
    /// tb.sequence()
    ///     .tick(|ui| { ui.text("ready"); })
    ///     .key(KeyCode::Esc, |ui| { ui.text("after esc"); })
    ///     .run();
    /// tb.assert_contains("after esc");
    /// ```
    pub fn sequence(&mut self) -> TestSequence<'_> {
        TestSequence {
            backend: self,
            steps: Vec::new(),
        }
    }

    /// Simulate typing `s` one character at a time, rendering with `render`
    /// between each character.
    ///
    /// Each character produces a [`KeyCode::Char`] event with no modifiers.
    /// Focus state is preserved across characters.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::TestBackend;
    ///
    /// let mut tb = TestBackend::new(20, 3);
    /// let mut typed = String::new();
    /// tb.type_string("hi", |ui| {
    ///     ui.text(&typed);
    /// });
    /// // 2 characters → 2 frames rendered.
    /// drop(typed);
    /// ```
    pub fn type_string(&mut self, s: &str, mut render: impl FnMut(&mut Context)) {
        for ch in s.chars() {
            let events = vec![Event::Key(KeyEvent {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
            })];
            // Use render_frame directly so frame recording is preserved and
            // FrameState advances naturally between characters.
            self.render_frame(events, |_| {}, &mut render);
        }
    }
}

/// A single step in a [`TestSequence`].
///
/// Holds the event batch to inject, plus a render closure to execute. Created
/// internally by [`TestSequence::tick`], [`TestSequence::key`],
/// [`TestSequence::events`], etc.
struct TestStep<'a> {
    events: Vec<Event>,
    render: Box<dyn FnOnce(&mut Context) + 'a>,
}

/// Builder returned by [`TestBackend::sequence`].
///
/// Chain step builders (`tick`, `key`, `type_string`, `events`) and finalize
/// with [`run`](TestSequence::run). Steps execute sequentially, advancing
/// `FrameState` between them so focus and hooks evolve naturally without the
/// caller having to thread state.
pub struct TestSequence<'a> {
    backend: &'a mut TestBackend,
    steps: Vec<TestStep<'a>>,
}

impl<'a> TestSequence<'a> {
    /// Append a step that renders without injecting any events.
    ///
    /// Equivalent to a single frame tick — useful for letting hooks /
    /// animations advance between input steps.
    pub fn tick(mut self, f: impl FnOnce(&mut Context) + 'a) -> Self {
        self.steps.push(TestStep {
            events: Vec::new(),
            render: Box::new(f),
        });
        self
    }

    /// Append a step that fires a single key-press event with no modifiers.
    pub fn key(mut self, code: KeyCode, f: impl FnOnce(&mut Context) + 'a) -> Self {
        let events = vec![Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
        })];
        self.steps.push(TestStep {
            events,
            render: Box::new(f),
        });
        self
    }

    /// Append a step that types `s` as a sequence of `KeyCode::Char` events
    /// **before** invoking `render`.
    ///
    /// Unlike [`TestBackend::type_string`], this collapses every typed
    /// character into a single render step — useful when the per-character
    /// frame state is not the assertion target. For per-keystroke rendering,
    /// chain individual `.key(...)` calls.
    pub fn type_string(mut self, s: &str, f: impl FnOnce(&mut Context) + 'a) -> Self {
        let events = s
            .chars()
            .map(|c| {
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press,
                })
            })
            .collect();
        self.steps.push(TestStep {
            events,
            render: Box::new(f),
        });
        self
    }

    /// Append a step with an arbitrary event batch.
    ///
    /// Useful for mouse interactions, paste events, or sequences built
    /// with [`EventBuilder`].
    pub fn events(mut self, events: Vec<Event>, f: impl FnOnce(&mut Context) + 'a) -> Self {
        self.steps.push(TestStep {
            events,
            render: Box::new(f),
        });
        self
    }

    /// Execute every queued step in order. Returns control to the caller
    /// (the [`TestBackend`] is borrowed mutably for the lifetime of the
    /// sequence builder). Use [`TestBackend::buffer`] / `.frames()` /
    /// `.assert_*` after `run()` returns.
    pub fn run(self) {
        let backend = self.backend;
        for step in self.steps {
            let TestStep { events, render } = step;
            // Adapt FnOnce(&mut Context) into the &mut FnMut(&mut Context)
            // shape that render_frame's internal trampoline already expects.
            let mut once = Some(render);
            let f = move |ui: &mut Context| {
                if let Some(f) = once.take() {
                    f(ui);
                }
            };
            backend.render_frame(events, |_| {}, f);
        }
    }
}

impl std::fmt::Display for TestBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_trimmed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{KeyEventKind, MouseKind};

    /// Regression test for issue #131: `mouse_up` produces `MouseKind::Up(Left)`.
    #[test]
    fn event_builder_mouse_up_produces_up_event() {
        let events = EventBuilder::new().mouse_up(5, 3).build();
        assert_eq!(events.len(), 1);
        match &events[0] {
            Event::Mouse(m) => {
                assert!(matches!(m.kind, MouseKind::Up(MouseButton::Left)));
                assert_eq!(m.x, 5);
                assert_eq!(m.y, 3);
            }
            _ => panic!("expected mouse event"),
        }
    }

    /// Regression test for issue #131: `drag` produces a drag mouse event.
    #[test]
    fn event_builder_drag_produces_drag_event() {
        let events = EventBuilder::new().drag(10, 5).build();
        assert_eq!(events.len(), 1);
        match &events[0] {
            Event::Mouse(m) => {
                assert!(matches!(m.kind, MouseKind::Drag(MouseButton::Left)));
                assert_eq!(m.x, 10);
                assert_eq!(m.y, 5);
            }
            _ => panic!("expected mouse event"),
        }
    }

    /// Regression test for issue #131: `key_release` produces a release key event.
    #[test]
    fn event_builder_key_release_produces_release_event() {
        let events = EventBuilder::new().key_release('a').build();
        assert_eq!(events.len(), 1);
        match &events[0] {
            Event::Key(k) => {
                assert_eq!(k.code, KeyCode::Char('a'));
                assert!(matches!(k.kind, KeyEventKind::Release));
            }
            _ => panic!("expected key event"),
        }
    }

    /// Regression test for issue #131: focus_gained / focus_lost chain through builder.
    #[test]
    fn event_builder_focus_events_chaining() {
        let events = EventBuilder::new().focus_lost().focus_gained().build();
        assert_eq!(events, vec![Event::FocusLost, Event::FocusGained]);
    }

    // ---- #229 record_frames -------------------------------------------------

    #[test]
    fn record_frames_disabled_returns_empty_slice() {
        let mut tb = TestBackend::new(10, 2);
        tb.render(|ui| {
            ui.text("hi");
        });
        assert!(tb.frames().is_empty());
    }

    #[test]
    fn record_frames_captures_each_render() {
        let mut tb = TestBackend::new(20, 2).record_frames();
        for n in 0..3 {
            tb.render(|ui| {
                ui.text(format!("frame {n}"));
            });
        }
        assert_eq!(tb.frames().len(), 3);
        tb.frames()[0].assert_contains("frame 0");
        tb.frames()[1].assert_contains("frame 1");
        tb.frames()[2].assert_contains("frame 2");
    }

    #[test]
    fn record_frames_stores_styled_snapshot() {
        let mut tb = TestBackend::new(10, 1).record_frames();
        tb.render(|ui| {
            ui.text("hi").bold();
        });
        let frame = &tb.frames()[0];
        // Styled snapshot should encode the bold modifier somewhere.
        assert!(
            frame.snapshot.contains("bold"),
            "snapshot missing bold marker: {:?}",
            frame.snapshot
        );
    }

    #[test]
    fn record_frames_idempotent_when_called_twice() {
        // record_frames() called twice must not wipe prior history.
        let tb = TestBackend::new(10, 1).record_frames();
        let mut tb = tb.record_frames();
        tb.render(|ui| {
            ui.text("a");
        });
        assert_eq!(tb.frames().len(), 1);
    }

    #[test]
    fn frame_record_to_string_trimmed_drops_trailing_blank_rows() {
        let mut tb = TestBackend::new(10, 4).record_frames();
        tb.render(|ui| {
            ui.text("hello");
        });
        let frame = &tb.frames()[0];
        // The frame should have all 4 rows recorded.
        assert_eq!(frame.lines.len(), 4);
        // to_string_trimmed drops the trailing empty rows like TestBackend.
        let s = frame.to_string_trimmed();
        assert!(!s.ends_with('\n'));
        assert!(s.starts_with("hello"));
    }

    // ---- #230 sequence + type_string ----------------------------------------

    #[test]
    fn sequence_runs_multiple_steps_in_order() {
        let mut tb = TestBackend::new(20, 2).record_frames();
        tb.sequence()
            .tick(|ui| {
                ui.text("step-1");
            })
            .tick(|ui| {
                ui.text("step-2");
            })
            .tick(|ui| {
                ui.text("step-3");
            })
            .run();
        assert_eq!(tb.frames().len(), 3);
        tb.frames()[0].assert_contains("step-1");
        tb.frames()[1].assert_contains("step-2");
        tb.frames()[2].assert_contains("step-3");
    }

    #[test]
    fn sequence_key_step_injects_event() {
        // We can't easily observe the key event without a stateful widget,
        // but we can confirm the sequence builder ran the render closure.
        let mut tb = TestBackend::new(20, 2);
        tb.sequence()
            .key(KeyCode::Esc, |ui| {
                ui.text("after-esc");
            })
            .run();
        tb.assert_contains("after-esc");
    }

    #[test]
    fn sequence_type_string_collapses_into_single_step() {
        let mut tb = TestBackend::new(20, 2).record_frames();
        tb.sequence()
            .type_string("abc", |ui| {
                ui.text("done");
            })
            .run();
        // Sequence's type_string is one step → one frame, not three.
        assert_eq!(tb.frames().len(), 1);
        tb.frames()[0].assert_contains("done");
    }

    #[test]
    fn sequence_events_step_takes_arbitrary_batch() {
        let mut tb = TestBackend::new(20, 2);
        let events = EventBuilder::new()
            .key('a')
            .key_code(KeyCode::Enter)
            .build();
        tb.sequence()
            .events(events, |ui| {
                ui.text("ran");
            })
            .run();
        tb.assert_contains("ran");
    }

    #[test]
    fn type_string_renders_one_frame_per_char() {
        let mut tb = TestBackend::new(20, 2).record_frames();
        tb.type_string("abc", |ui| {
            ui.text("char");
        });
        assert_eq!(tb.frames().len(), 3);
    }

    #[test]
    fn type_string_handles_empty_input() {
        let mut tb = TestBackend::new(20, 2).record_frames();
        tb.type_string("", |ui| {
            ui.text("never-called");
        });
        assert_eq!(tb.frames().len(), 0);
    }

    // ---- #232 negative assertions ------------------------------------------

    #[test]
    fn assert_not_contains_passes_when_absent() {
        let mut tb = TestBackend::new(20, 2);
        tb.render(|ui| {
            ui.text("hello world");
        });
        tb.assert_not_contains("error");
    }

    #[test]
    #[should_panic(expected = "Buffer unexpectedly contains")]
    fn assert_not_contains_panics_when_present() {
        let mut tb = TestBackend::new(20, 2);
        tb.render(|ui| {
            ui.text("error: fail");
        });
        tb.assert_not_contains("error");
    }

    #[test]
    fn assert_line_not_contains_passes_when_other_row_has_substring() {
        let mut tb = TestBackend::new(20, 3);
        tb.render(|ui| {
            let _ = ui.col(|ui| {
                ui.text("first");
                ui.text("second");
            });
        });
        // Line 0 has "first" but not "second".
        tb.assert_line_not_contains(0, "second");
    }

    #[test]
    #[should_panic(expected = "Line 0: expected NOT to contain")]
    fn assert_line_not_contains_panics_when_present() {
        let mut tb = TestBackend::new(20, 1);
        tb.render(|ui| {
            ui.text("hello");
        });
        tb.assert_line_not_contains(0, "ello");
    }

    #[test]
    fn assert_empty_line_passes_for_blank_row() {
        let mut tb = TestBackend::new(20, 2);
        tb.render(|ui| {
            ui.text("only-row-0");
        });
        // Row 1 is untouched after rendering one text → blank.
        tb.assert_empty_line(1);
    }

    #[test]
    #[should_panic(expected = "Line 0: expected empty")]
    fn assert_empty_line_panics_when_non_blank() {
        let mut tb = TestBackend::new(20, 2);
        tb.render(|ui| {
            ui.text("not-empty");
        });
        tb.assert_empty_line(0);
    }

    #[test]
    fn assert_style_at_passes_for_matching_style() {
        use crate::style::{Color, Modifiers};
        let mut tb = TestBackend::new(10, 1);
        tb.render(|ui| {
            ui.text("x").fg(Color::Red);
        });
        let expected = Style {
            fg: Some(Color::Red),
            bg: None,
            modifiers: Modifiers::NONE,
        };
        tb.assert_style_at(0, 0, expected);
    }

    #[test]
    #[should_panic(expected = "Style mismatch")]
    fn assert_style_at_panics_on_mismatch() {
        use crate::style::Color;
        let mut tb = TestBackend::new(10, 1);
        tb.render(|ui| {
            ui.text("x").fg(Color::Red);
        });
        let expected = Style::new().fg(Color::Blue);
        tb.assert_style_at(0, 0, expected);
    }
}
