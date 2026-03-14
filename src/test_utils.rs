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
use crate::layout;
use crate::rect::Rect;
use crate::style::Theme;

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
        }));
        self
    }

    /// Append a scroll-up event at `(x, y)`.
    pub fn scroll_up(mut self, x: u32, y: u32) -> Self {
        self.events.push(Event::Mouse(MouseEvent {
            kind: MouseKind::ScrollUp,
            x,
            y,
            modifiers: KeyModifiers::NONE,
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
}

impl TestBackend {
    /// Create a test backend with the given terminal dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        let area = Rect::new(0, 0, width, height);
        Self {
            buffer: Buffer::empty(area),
            width,
            height,
        }
    }

    /// Run a UI closure for one frame and render to the internal buffer.
    pub fn render(&mut self, f: impl FnOnce(&mut Context)) {
        let mut ctx = Context::new(
            Vec::new(),
            self.width,
            self.height,
            0,
            0,
            0,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
            Theme::dark(),
            None,
            false,
        );
        f(&mut ctx);
        let mut tree = layout::build_tree(&ctx.commands);
        let area = Rect::new(0, 0, self.width, self.height);
        layout::compute(&mut tree, area);
        self.buffer.reset();
        layout::render(&tree, &mut self.buffer);
    }

    /// Render with injected events and focus state for interaction testing.
    pub fn render_with_events(
        &mut self,
        events: Vec<Event>,
        focus_index: usize,
        prev_focus_count: usize,
        f: impl FnOnce(&mut Context),
    ) {
        let mut ctx = Context::new(
            events,
            self.width,
            self.height,
            0,
            focus_index,
            prev_focus_count,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
            Theme::dark(),
            None,
            false,
        );
        ctx.process_focus_keys();
        f(&mut ctx);
        let mut tree = layout::build_tree(&ctx.commands);
        let area = Rect::new(0, 0, self.width, self.height);
        layout::compute(&mut tree, area);
        self.buffer.reset();
        layout::render(&tree, &mut self.buffer);
    }

    /// Convenience wrapper: render with events using default focus state.
    pub fn run_with_events(&mut self, events: Vec<Event>, f: impl FnOnce(&mut crate::Context)) {
        self.render_with_events(events, 0, 0, f);
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
}

impl std::fmt::Display for TestBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_trimmed())
    }
}
