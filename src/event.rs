/// A terminal input event.
///
/// Produced each frame by the run loop and passed to your UI closure via
/// [`crate::Context`]. Use the helper methods on `Context` (e.g., `key()`,
/// `key_code()`) rather than matching on this type directly.
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
}

/// A keyboard event with key code and modifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// The key that was pressed.
    pub code: KeyCode,
    /// Modifier keys held at the time of the press.
    pub modifiers: KeyModifiers,
}

/// Key identifier.
///
/// Covers printable characters, control keys, arrow keys, function keys,
/// and navigation keys. Unrecognized keys are silently dropped by the
/// crossterm conversion layer.
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

    /// Returns `true` if all bits in `other` are set in `self`.
    #[inline]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

/// A mouse event with position and kind.
///
/// Coordinates are zero-based terminal columns (`x`) and rows (`y`).
/// Mouse events are only produced when `mouse: true` is set in
/// [`crate::RunConfig`].
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
}

/// The type of mouse event.
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
    /// The mouse was moved without any button held.
    Moved,
}

/// Mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Primary (left) mouse button.
    Left,
    /// Secondary (right) mouse button.
    Right,
    /// Middle mouse button (scroll wheel click).
    Middle,
}

fn convert_modifiers(modifiers: crossterm::event::KeyModifiers) -> KeyModifiers {
    let mut out = KeyModifiers::NONE;
    if modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
        out.0 |= KeyModifiers::SHIFT.0;
    }
    if modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
        out.0 |= KeyModifiers::CONTROL.0;
    }
    if modifiers.contains(crossterm::event::KeyModifiers::ALT) {
        out.0 |= KeyModifiers::ALT.0;
    }
    out
}

fn convert_button(button: crossterm::event::MouseButton) -> MouseButton {
    match button {
        crossterm::event::MouseButton::Left => MouseButton::Left,
        crossterm::event::MouseButton::Right => MouseButton::Right,
        crossterm::event::MouseButton::Middle => MouseButton::Middle,
    }
}

// ── crossterm conversions ────────────────────────────────────────────

/// Convert a raw crossterm event into our lightweight [`Event`].
/// Returns `None` for event kinds we don't handle.
pub(crate) fn from_crossterm(raw: crossterm::event::Event) -> Option<Event> {
    match raw {
        crossterm::event::Event::Key(k) => {
            // Only handle key-press (not repeat/release) to avoid double-fire.
            if k.kind != crossterm::event::KeyEventKind::Press {
                return None;
            }
            let code = match k.code {
                crossterm::event::KeyCode::Char(c) => KeyCode::Char(c),
                crossterm::event::KeyCode::Enter => KeyCode::Enter,
                crossterm::event::KeyCode::Backspace => KeyCode::Backspace,
                crossterm::event::KeyCode::Tab => KeyCode::Tab,
                crossterm::event::KeyCode::BackTab => KeyCode::BackTab,
                crossterm::event::KeyCode::Esc => KeyCode::Esc,
                crossterm::event::KeyCode::Up => KeyCode::Up,
                crossterm::event::KeyCode::Down => KeyCode::Down,
                crossterm::event::KeyCode::Left => KeyCode::Left,
                crossterm::event::KeyCode::Right => KeyCode::Right,
                crossterm::event::KeyCode::Home => KeyCode::Home,
                crossterm::event::KeyCode::End => KeyCode::End,
                crossterm::event::KeyCode::PageUp => KeyCode::PageUp,
                crossterm::event::KeyCode::PageDown => KeyCode::PageDown,
                crossterm::event::KeyCode::Delete => KeyCode::Delete,
                crossterm::event::KeyCode::F(n) => KeyCode::F(n),
                _ => return None,
            };
            let modifiers = convert_modifiers(k.modifiers);
            Some(Event::Key(KeyEvent { code, modifiers }))
        }
        crossterm::event::Event::Mouse(m) => {
            let kind = match m.kind {
                crossterm::event::MouseEventKind::Down(btn) => MouseKind::Down(convert_button(btn)),
                crossterm::event::MouseEventKind::Up(btn) => MouseKind::Up(convert_button(btn)),
                crossterm::event::MouseEventKind::Drag(btn) => MouseKind::Drag(convert_button(btn)),
                crossterm::event::MouseEventKind::Moved => MouseKind::Moved,
                crossterm::event::MouseEventKind::ScrollUp => MouseKind::ScrollUp,
                crossterm::event::MouseEventKind::ScrollDown => MouseKind::ScrollDown,
                _ => return None,
            };

            Some(Event::Mouse(MouseEvent {
                kind,
                x: m.column as u32,
                y: m.row as u32,
                modifiers: convert_modifiers(m.modifiers),
            }))
        }
        crossterm::event::Event::Resize(cols, rows) => {
            Some(Event::Resize(cols as u32, rows as u32))
        }
        crossterm::event::Event::Paste(s) => Some(Event::Paste(s)),
        _ => None,
    }
}
