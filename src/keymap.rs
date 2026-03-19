use crate::{KeyCode, KeyModifiers};

/// A single key binding with display text and description.
#[derive(Debug, Clone)]
pub struct Binding {
    /// The key code for matching.
    pub key: KeyCode,
    /// Optional modifier (Ctrl, Alt, Shift).
    pub modifiers: Option<KeyModifiers>,
    /// Display text shown in help bar (e.g., "q", "Ctrl+S", "↑").
    pub display: String,
    /// Description of what this binding does.
    pub description: String,
    /// Whether to show in help bar.
    pub visible: bool,
}

/// Declarative key binding map.
///
/// # Examples
/// ```
/// use slt::KeyMap;
///
/// let km = KeyMap::new()
///     .bind('q', "Quit")
///     .bind_code(slt::KeyCode::Up, "Move up")
///     .bind_mod('s', slt::KeyModifiers::CONTROL, "Save")
///     .bind_hidden('?', "Toggle help");
/// ```
#[derive(Debug, Clone, Default)]
pub struct KeyMap {
    /// Registered key bindings.
    pub bindings: Vec<Binding>,
}

impl KeyMap {
    /// Create an empty key map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a character key.
    pub fn bind(mut self, key: char, description: &str) -> Self {
        self.bindings.push(Binding {
            key: KeyCode::Char(key),
            modifiers: None,
            display: key.to_string(),
            description: description.to_string(),
            visible: true,
        });
        self
    }

    /// Bind a special key (Enter, Esc, Up, Down, etc.).
    pub fn bind_code(mut self, key: KeyCode, description: &str) -> Self {
        self.bindings.push(Binding {
            display: display_for_key_code(&key),
            key,
            modifiers: None,
            description: description.to_string(),
            visible: true,
        });
        self
    }

    /// Bind a key with modifier (Ctrl+S, etc.).
    pub fn bind_mod(mut self, key: char, mods: KeyModifiers, description: &str) -> Self {
        self.bindings.push(Binding {
            key: KeyCode::Char(key),
            modifiers: Some(mods),
            display: display_for_mod_char(mods, key),
            description: description.to_string(),
            visible: true,
        });
        self
    }

    /// Bind but hide from help bar display.
    pub fn bind_hidden(mut self, key: char, description: &str) -> Self {
        self.bindings.push(Binding {
            key: KeyCode::Char(key),
            modifiers: None,
            display: key.to_string(),
            description: description.to_string(),
            visible: false,
        });
        self
    }

    /// Get visible bindings for help bar rendering.
    pub fn visible_bindings(&self) -> impl Iterator<Item = &Binding> {
        self.bindings.iter().filter(|binding| binding.visible)
    }
}

fn display_for_key_code(key: &KeyCode) -> String {
    match key {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "Shift+Tab".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Up => "↑".to_string(),
        KeyCode::Down => "↓".to_string(),
        KeyCode::Left => "←".to_string(),
        KeyCode::Right => "→".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::Delete => "Del".to_string(),
        KeyCode::F(n) => format!("F{n}"),
    }
}

fn display_for_mod_char(mods: KeyModifiers, key: char) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if mods.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if mods.contains(KeyModifiers::ALT) {
        parts.push("Alt");
    }
    if mods.contains(KeyModifiers::SHIFT) {
        parts.push("Shift");
    }

    if parts.is_empty() {
        key.to_string()
    } else {
        format!("{}+{}", parts.join("+"), key.to_ascii_uppercase())
    }
}
