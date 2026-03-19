//! Terminal input events.
//!
//! This module defines the event types that SLT delivers to your UI closure
//! each frame: keyboard, mouse, resize, paste, and focus events. In most
//! cases you'll use the convenience methods on [`crate::Context`] (e.g.,
//! [`Context::key`](crate::Context::key),
//! [`Context::mouse_down`](crate::Context::mouse_down)) instead of matching
//! on these types directly.

#[cfg(feature = "crossterm")]
use crossterm::event as crossterm_event;

/// A terminal input event.
///
/// Produced each frame by the run loop and passed to your UI closure via
/// [`crate::Context`]. Use the helper methods on `Context` (e.g., `key()`,
/// `key_code()`) rather than matching on this type directly.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A keyboard event.
    Key(KeyEvent),
    /// A mouse event (requires `mouse: true` in [`crate::RunConfig`]).
    Mouse(MouseEvent),
    /// The terminal was resized to the given `(columns, rows)`.
    Resize(u32, u32),
    /// Pasted text (bracketed paste). May contain newlines.
    Paste(String),
    /// The terminal window gained focus.
    FocusGained,
    /// The terminal window lost focus. Used to clear hover state.
    FocusLost,
}

impl Event {
    /// Create a key press event for a character.
    pub fn key_char(c: char) -> Self {
        Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
        })
    }

    /// Create a key press event for a special key.
    pub fn key(code: KeyCode) -> Self {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
        })
    }

    /// Create a key press event with Ctrl modifier.
    pub fn key_ctrl(c: char) -> Self {
        Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
        })
    }

    /// Create a key press event with custom modifiers.
    pub fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
        })
    }

    /// Create a terminal resize event.
    pub fn resize(width: u32, height: u32) -> Self {
        Event::Resize(width, height)
    }

    /// Create a left mouse click event at (x, y).
    pub fn mouse_click(x: u32, y: u32) -> Self {
        Event::Mouse(MouseEvent {
            kind: MouseKind::Down(MouseButton::Left),
            x,
            y,
            modifiers: KeyModifiers::NONE,
            pixel_x: None,
            pixel_y: None,
        })
    }

    /// Create a mouse move event at the given position.
    pub fn mouse_move(x: u32, y: u32) -> Self {
        Event::Mouse(MouseEvent {
            kind: MouseKind::Moved,
            x,
            y,
            modifiers: KeyModifiers::NONE,
            pixel_x: None,
            pixel_y: None,
        })
    }

    /// Create a scroll up event at the given position.
    pub fn scroll_up(x: u32, y: u32) -> Self {
        Event::Mouse(MouseEvent {
            kind: MouseKind::ScrollUp,
            x,
            y,
            modifiers: KeyModifiers::NONE,
            pixel_x: None,
            pixel_y: None,
        })
    }

    /// Create a scroll down event at the given position.
    pub fn scroll_down(x: u32, y: u32) -> Self {
        Event::Mouse(MouseEvent {
            kind: MouseKind::ScrollDown,
            x,
            y,
            modifiers: KeyModifiers::NONE,
            pixel_x: None,
            pixel_y: None,
        })
    }

    /// Create a key release event for a character.
    pub fn key_release(c: char) -> Self {
        Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
        })
    }

    /// Create a paste event with the given text.
    pub fn paste(text: impl Into<String>) -> Self {
        Event::Paste(text.into())
    }

    /// Returns the key event data if this is a `Key` variant.
    pub fn as_key(&self) -> Option<&KeyEvent> {
        match self {
            Event::Key(k) => Some(k),
            _ => None,
        }
    }

    /// Returns the mouse event data if this is a `Mouse` variant.
    pub fn as_mouse(&self) -> Option<&MouseEvent> {
        match self {
            Event::Mouse(m) => Some(m),
            _ => None,
        }
    }

    /// Returns `(columns, rows)` if this is a `Resize` variant.
    pub fn as_resize(&self) -> Option<(u32, u32)> {
        match self {
            Event::Resize(w, h) => Some((*w, *h)),
            _ => None,
        }
    }

    /// Returns the pasted text if this is a `Paste` variant.
    pub fn as_paste(&self) -> Option<&str> {
        match self {
            Event::Paste(s) => Some(s),
            _ => None,
        }
    }

    /// Returns `true` if this is a `Key` event.
    pub fn is_key(&self) -> bool {
        matches!(self, Event::Key(_))
    }

    /// Returns `true` if this is a `Mouse` event.
    pub fn is_mouse(&self) -> bool {
        matches!(self, Event::Mouse(_))
    }
}

/// A keyboard event with key code and modifiers.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// The key that was pressed.
    pub code: KeyCode,
    /// Modifier keys held at the time of the press.
    pub modifiers: KeyModifiers,
    /// The type of key event. Always `Press` without Kitty keyboard protocol.
    pub kind: KeyEventKind,
}

impl KeyEvent {
    /// Returns `true` if this is a press of the given character (no modifiers).
    pub fn is_char(&self, c: char) -> bool {
        self.code == KeyCode::Char(c)
            && self.modifiers == KeyModifiers::NONE
            && self.kind == KeyEventKind::Press
    }

    /// Returns `true` if this is Ctrl+`c`.
    pub fn is_ctrl_char(&self, c: char) -> bool {
        self.code == KeyCode::Char(c)
            && self.modifiers == KeyModifiers::CONTROL
            && self.kind == KeyEventKind::Press
    }

    /// Returns `true` if this is a press of the given key code (no modifiers).
    pub fn is_code(&self, code: KeyCode) -> bool {
        self.code == code
            && self.modifiers == KeyModifiers::NONE
            && self.kind == KeyEventKind::Press
    }
}

/// The type of key event.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventKind {
    /// Key was pressed.
    Press,
    /// Key was released (requires Kitty keyboard protocol).
    Release,
    /// Key is being held/repeated (requires Kitty keyboard protocol).
    Repeat,
}

/// Key identifier.
///
/// Covers printable characters, control keys, arrow keys, function keys,
/// and navigation keys. Unrecognized keys are silently dropped by the
/// crossterm conversion layer.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyCode {
    /// A printable character (letter, digit, symbol, space, etc.).
    Char(char),
    /// Enter / Return key.
    Enter,
    /// Backspace key.
    Backspace,
    /// Tab key (forward tab).
    Tab,
    /// Shift+Tab (back tab).
    BackTab,
    /// Escape key.
    Esc,
    /// Up arrow key.
    Up,
    /// Down arrow key.
    Down,
    /// Left arrow key.
    Left,
    /// Right arrow key.
    Right,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page Up key.
    PageUp,
    /// Page Down key.
    PageDown,
    /// Delete (forward delete) key.
    Delete,
    /// Insert key.
    Insert,
    /// Null key (Ctrl+Space on some terminals).
    Null,
    /// Caps Lock key (Kitty keyboard protocol only).
    CapsLock,
    /// Scroll Lock key (Kitty keyboard protocol only).
    ScrollLock,
    /// Num Lock key (Kitty keyboard protocol only).
    NumLock,
    /// Print Screen key (Kitty keyboard protocol only).
    PrintScreen,
    /// Pause/Break key (Kitty keyboard protocol only).
    Pause,
    /// Menu / context menu key.
    Menu,
    /// Keypad center key (numpad 5 without NumLock).
    KeypadBegin,
    /// Function key `F1`..`F12` (and beyond). The inner `u8` is the number.
    F(u8),
}

/// Modifier keys held during a key press.
///
/// Stored as bitflags in a `u8`. Check individual modifiers with
/// [`KeyModifiers::contains`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModifiers(pub u8);

impl KeyModifiers {
    /// No modifier keys held.
    pub const NONE: Self = Self(0);
    /// Shift key held.
    pub const SHIFT: Self = Self(1 << 0);
    /// Control key held.
    pub const CONTROL: Self = Self(1 << 1);
    /// Alt / Option key held.
    pub const ALT: Self = Self(1 << 2);
    /// Super key (Cmd on macOS, Win on Windows). Kitty keyboard protocol only.
    pub const SUPER: Self = Self(1 << 3);
    /// Hyper modifier. Kitty keyboard protocol only.
    pub const HYPER: Self = Self(1 << 4);
    /// Meta modifier. Kitty keyboard protocol only.
    pub const META: Self = Self(1 << 5);

    /// Returns `true` if all bits in `other` are set in `self`.
    #[inline]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

/// A mouse event with position and kind.
///
/// Coordinates are zero-based terminal columns (`x`) and rows (`y`).
/// When the terminal supports pixel-level reporting (e.g. Kitty, or WASM),
/// `pixel_x` and `pixel_y` contain the sub-cell position in pixels.
/// Mouse events are only produced when `mouse: true` is set in
/// [`crate::RunConfig`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseEvent {
    /// The type of mouse action that occurred.
    pub kind: MouseKind,
    /// Column (horizontal position), zero-based.
    pub x: u32,
    /// Row (vertical position), zero-based.
    pub y: u32,
    /// Modifier keys held at the time of the event.
    pub modifiers: KeyModifiers,
    /// Pixel-level x coordinate, if available.
    pub pixel_x: Option<u16>,
    /// Pixel-level y coordinate, if available.
    pub pixel_y: Option<u16>,
}

impl MouseEvent {
    /// Create a new MouseEvent with all fields.
    pub fn new(
        kind: MouseKind,
        x: u32,
        y: u32,
        modifiers: KeyModifiers,
        pixel_x: Option<u16>,
        pixel_y: Option<u16>,
    ) -> Self {
        Self {
            kind,
            x,
            y,
            modifiers,
            pixel_x,
            pixel_y,
        }
    }

    /// Returns true if this is a scroll event.
    pub fn is_scroll(&self) -> bool {
        matches!(
            self.kind,
            MouseKind::ScrollUp
                | MouseKind::ScrollDown
                | MouseKind::ScrollLeft
                | MouseKind::ScrollRight
        )
    }
}

/// The type of mouse event.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MouseKind {
    /// A mouse button was pressed.
    Down(MouseButton),
    /// A mouse button was released.
    Up(MouseButton),
    /// The mouse was moved while a button was held.
    Drag(MouseButton),
    /// The scroll wheel was rotated upward.
    ScrollUp,
    /// The scroll wheel was rotated downward.
    ScrollDown,
    /// The scroll wheel was rotated leftward (horizontal scroll).
    ScrollLeft,
    /// The scroll wheel was rotated rightward (horizontal scroll).
    ScrollRight,
    /// The mouse was moved without any button held.
    Moved,
}

/// Mouse button identifier.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Primary (left) mouse button.
    Left,
    /// Secondary (right) mouse button.
    Right,
    /// Middle mouse button (scroll wheel click).
    Middle,
}

#[cfg(feature = "crossterm")]
fn convert_modifiers(modifiers: crossterm_event::KeyModifiers) -> KeyModifiers {
    let mut out = KeyModifiers::NONE;
    if modifiers.contains(crossterm_event::KeyModifiers::SHIFT) {
        out.0 |= KeyModifiers::SHIFT.0;
    }
    if modifiers.contains(crossterm_event::KeyModifiers::CONTROL) {
        out.0 |= KeyModifiers::CONTROL.0;
    }
    if modifiers.contains(crossterm_event::KeyModifiers::ALT) {
        out.0 |= KeyModifiers::ALT.0;
    }
    if modifiers.contains(crossterm_event::KeyModifiers::SUPER) {
        out.0 |= KeyModifiers::SUPER.0;
    }
    if modifiers.contains(crossterm_event::KeyModifiers::HYPER) {
        out.0 |= KeyModifiers::HYPER.0;
    }
    if modifiers.contains(crossterm_event::KeyModifiers::META) {
        out.0 |= KeyModifiers::META.0;
    }
    out
}

#[cfg(feature = "crossterm")]
fn convert_button(button: crossterm_event::MouseButton) -> MouseButton {
    match button {
        crossterm_event::MouseButton::Left => MouseButton::Left,
        crossterm_event::MouseButton::Right => MouseButton::Right,
        crossterm_event::MouseButton::Middle => MouseButton::Middle,
    }
}

// ── crossterm conversions ────────────────────────────────────────────

/// Convert a raw crossterm event into our lightweight [`Event`].
/// Returns `None` for event kinds we don't handle.
#[cfg(feature = "crossterm")]
pub(crate) fn from_crossterm(raw: crossterm_event::Event) -> Option<Event> {
    match raw {
        crossterm_event::Event::Key(k) => {
            let code = match k.code {
                crossterm_event::KeyCode::Char(c) => KeyCode::Char(c),
                crossterm_event::KeyCode::Enter => KeyCode::Enter,
                crossterm_event::KeyCode::Backspace => KeyCode::Backspace,
                crossterm_event::KeyCode::Tab => KeyCode::Tab,
                crossterm_event::KeyCode::BackTab => KeyCode::BackTab,
                crossterm_event::KeyCode::Esc => KeyCode::Esc,
                crossterm_event::KeyCode::Up => KeyCode::Up,
                crossterm_event::KeyCode::Down => KeyCode::Down,
                crossterm_event::KeyCode::Left => KeyCode::Left,
                crossterm_event::KeyCode::Right => KeyCode::Right,
                crossterm_event::KeyCode::Home => KeyCode::Home,
                crossterm_event::KeyCode::End => KeyCode::End,
                crossterm_event::KeyCode::PageUp => KeyCode::PageUp,
                crossterm_event::KeyCode::PageDown => KeyCode::PageDown,
                crossterm_event::KeyCode::Delete => KeyCode::Delete,
                crossterm_event::KeyCode::Insert => KeyCode::Insert,
                crossterm_event::KeyCode::Null => KeyCode::Null,
                crossterm_event::KeyCode::CapsLock => KeyCode::CapsLock,
                crossterm_event::KeyCode::ScrollLock => KeyCode::ScrollLock,
                crossterm_event::KeyCode::NumLock => KeyCode::NumLock,
                crossterm_event::KeyCode::PrintScreen => KeyCode::PrintScreen,
                crossterm_event::KeyCode::Pause => KeyCode::Pause,
                crossterm_event::KeyCode::Menu => KeyCode::Menu,
                crossterm_event::KeyCode::KeypadBegin => KeyCode::KeypadBegin,
                crossterm_event::KeyCode::F(n) => KeyCode::F(n),
                _ => return None,
            };
            let modifiers = convert_modifiers(k.modifiers);
            let kind = match k.kind {
                crossterm_event::KeyEventKind::Press => KeyEventKind::Press,
                crossterm_event::KeyEventKind::Repeat => KeyEventKind::Repeat,
                crossterm_event::KeyEventKind::Release => KeyEventKind::Release,
            };
            Some(Event::Key(KeyEvent {
                code,
                modifiers,
                kind,
            }))
        }
        crossterm_event::Event::Mouse(m) => {
            let kind = match m.kind {
                crossterm_event::MouseEventKind::Down(btn) => MouseKind::Down(convert_button(btn)),
                crossterm_event::MouseEventKind::Up(btn) => MouseKind::Up(convert_button(btn)),
                crossterm_event::MouseEventKind::Drag(btn) => MouseKind::Drag(convert_button(btn)),
                crossterm_event::MouseEventKind::Moved => MouseKind::Moved,
                crossterm_event::MouseEventKind::ScrollUp => MouseKind::ScrollUp,
                crossterm_event::MouseEventKind::ScrollDown => MouseKind::ScrollDown,
                crossterm_event::MouseEventKind::ScrollLeft => MouseKind::ScrollLeft,
                crossterm_event::MouseEventKind::ScrollRight => MouseKind::ScrollRight,
            };

            Some(Event::Mouse(MouseEvent {
                kind,
                x: m.column as u32,
                y: m.row as u32,
                modifiers: convert_modifiers(m.modifiers),
                pixel_x: None,
                pixel_y: None,
            }))
        }
        crossterm_event::Event::Resize(cols, rows) => Some(Event::Resize(cols as u32, rows as u32)),
        crossterm_event::Event::Paste(s) => Some(Event::Paste(s)),
        crossterm_event::Event::FocusGained => Some(Event::FocusGained),
        crossterm_event::Event::FocusLost => Some(Event::FocusLost),
    }
}

#[cfg(test)]
mod event_constructor_tests {
    use super::*;

    #[test]
    fn test_key_char() {
        let e = Event::key_char('q');
        if let Event::Key(k) = e {
            assert!(matches!(k.code, KeyCode::Char('q')));
            assert_eq!(k.modifiers, KeyModifiers::NONE);
            assert!(matches!(k.kind, KeyEventKind::Press));
        } else {
            panic!("Expected Key event");
        }
    }

    #[test]
    fn test_key() {
        let e = Event::key(KeyCode::Enter);
        if let Event::Key(k) = e {
            assert!(matches!(k.code, KeyCode::Enter));
            assert_eq!(k.modifiers, KeyModifiers::NONE);
            assert!(matches!(k.kind, KeyEventKind::Press));
        } else {
            panic!("Expected Key event");
        }
    }

    #[test]
    fn test_key_ctrl() {
        let e = Event::key_ctrl('s');
        if let Event::Key(k) = e {
            assert!(matches!(k.code, KeyCode::Char('s')));
            assert_eq!(k.modifiers, KeyModifiers::CONTROL);
            assert!(matches!(k.kind, KeyEventKind::Press));
        } else {
            panic!("Expected Key event");
        }
    }

    #[test]
    fn test_key_mod() {
        let modifiers = KeyModifiers(KeyModifiers::SHIFT.0 | KeyModifiers::ALT.0);
        let e = Event::key_mod(KeyCode::Tab, modifiers);
        if let Event::Key(k) = e {
            assert!(matches!(k.code, KeyCode::Tab));
            assert_eq!(k.modifiers, modifiers);
            assert!(matches!(k.kind, KeyEventKind::Press));
        } else {
            panic!("Expected Key event");
        }
    }

    #[test]
    fn test_resize() {
        let e = Event::resize(80, 24);
        assert!(matches!(e, Event::Resize(80, 24)));
    }

    #[test]
    fn test_mouse_click() {
        let e = Event::mouse_click(10, 5);
        if let Event::Mouse(m) = e {
            assert!(matches!(m.kind, MouseKind::Down(MouseButton::Left)));
            assert_eq!(m.x, 10);
            assert_eq!(m.y, 5);
            assert_eq!(m.modifiers, KeyModifiers::NONE);
        } else {
            panic!("Expected Mouse event");
        }
    }

    #[test]
    fn test_mouse_move() {
        let e = Event::mouse_move(10, 5);
        if let Event::Mouse(m) = e {
            assert!(matches!(m.kind, MouseKind::Moved));
            assert_eq!(m.x, 10);
            assert_eq!(m.y, 5);
            assert_eq!(m.modifiers, KeyModifiers::NONE);
        } else {
            panic!("Expected Mouse event");
        }
    }

    #[test]
    fn test_paste() {
        let e = Event::paste("hello");
        assert!(matches!(e, Event::Paste(s) if s == "hello"));
    }
}
