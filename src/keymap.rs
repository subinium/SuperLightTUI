use crate::{KeyCode, KeyModifiers, ModifierKey};

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

    /// Bind a special key with modifier keys (Ctrl+Enter, Alt+Up, Shift+F5, etc.).
    ///
    /// Unlike [`KeyMap::bind_mod`], which is restricted to character keys, this
    /// accepts any [`KeyCode`] together with optional modifier keys.
    ///
    /// # Example
    /// ```
    /// use slt::{KeyMap, KeyCode, KeyModifiers};
    ///
    /// let km = KeyMap::new()
    ///     .bind_code_mod(KeyCode::Enter, KeyModifiers::CONTROL, "Submit")
    ///     .bind_code_mod(KeyCode::Up, KeyModifiers::ALT, "Jump to top");
    /// ```
    pub fn bind_code_mod(mut self, key: KeyCode, mods: KeyModifiers, description: &str) -> Self {
        let display = display_for_code_mod(&key, mods);
        self.bindings.push(Binding {
            key,
            modifiers: Some(mods),
            display,
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
        KeyCode::Insert => "Ins".to_string(),
        KeyCode::Null => "Null".to_string(),
        KeyCode::CapsLock => "CapsLock".to_string(),
        KeyCode::ScrollLock => "ScrollLock".to_string(),
        KeyCode::NumLock => "NumLock".to_string(),
        KeyCode::PrintScreen => "PrtSc".to_string(),
        KeyCode::Pause => "Pause".to_string(),
        KeyCode::Menu => "Menu".to_string(),
        KeyCode::KeypadBegin => "KP5".to_string(),
        KeyCode::F(n) => format!("F{n}"),
        KeyCode::Modifier(m) => display_for_modifier_key(*m).to_string(),
    }
}

fn display_for_modifier_key(m: ModifierKey) -> &'static str {
    match m {
        ModifierKey::LeftShift => "LShift",
        ModifierKey::LeftCtrl => "LCtrl",
        ModifierKey::LeftAlt => "LAlt",
        ModifierKey::LeftSuper => "LSuper",
        ModifierKey::RightShift => "RShift",
        ModifierKey::RightCtrl => "RCtrl",
        ModifierKey::RightAlt => "RAlt",
        ModifierKey::RightSuper => "RSuper",
        ModifierKey::LeftHyper => "LHyper",
        ModifierKey::LeftMeta => "LMeta",
        ModifierKey::RightHyper => "RHyper",
        ModifierKey::RightMeta => "RMeta",
        ModifierKey::IsoLevel3Shift => "ISO3",
        ModifierKey::IsoLevel5Shift => "ISO5",
    }
}

fn display_for_code_mod(key: &KeyCode, mods: KeyModifiers) -> String {
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
    if mods.contains(KeyModifiers::SUPER) {
        parts.push("Super");
    }
    if mods.contains(KeyModifiers::HYPER) {
        parts.push("Hyper");
    }
    if mods.contains(KeyModifiers::META) {
        parts.push("Meta");
    }

    let key_label = display_for_key_code(key);
    if parts.is_empty() {
        key_label
    } else {
        format!("{}+{}", parts.join("+"), key_label)
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
    if mods.contains(KeyModifiers::SUPER) {
        parts.push("Super");
    }
    if mods.contains(KeyModifiers::HYPER) {
        parts.push("Hyper");
    }
    if mods.contains(KeyModifiers::META) {
        parts.push("Meta");
    }

    if parts.is_empty() {
        key.to_string()
    } else {
        format!("{}+{}", parts.join("+"), key.to_ascii_uppercase())
    }
}

/// Opt-in trait for users to publish a widget's keymap to the framework so
/// `Context::keymap_help_overlay` can list it on `?` press (issue #236).
///
/// # Scope
///
/// SLT does **not** implement this on built-in widgets — it is a user-facing
/// extension point. Built-in widgets register their bindings directly inside
/// their `impl Context::*` methods. To publish the keymap of your *own*
/// custom widget, implement this trait, then call
/// [`Context::publish_keymap`](crate::Context::publish_keymap) in your render
/// method:
///
/// ```ignore
/// ctx.publish_keymap("my_widget", MyState::key_help(&state));
/// ```
///
/// In other words, this trait is the same shape as `std::fmt::Display`: zero
/// blanket / built-in impls; you opt in by implementing it on your own type.
/// If you prefer a free-function call, [`Context::publish_keymap`] takes the
/// same `(name, &'static [...])` signature without the trait.
///
/// # Format
///
/// The returned slice must be `'static` — hardcode a `const` array per
/// widget so the registration is allocation-free. Each tuple is
/// `(key_combo, description)` using the same display style as
/// [`Binding::display`] (e.g. `"↑/k"`, `"Ctrl+S"`, `"PgDn"`).
///
/// # Example
///
/// ```
/// use slt::keymap::WidgetKeyHelp;
///
/// struct Counter;
///
/// impl WidgetKeyHelp for Counter {
///     fn key_help(&self) -> &'static [(&'static str, &'static str)] {
///         const HELP: &[(&str, &str)] = &[
///             ("↑/k", "increment"),
///             ("↓/j", "decrement"),
///             ("r", "reset"),
///         ];
///         HELP
///     }
/// }
///
/// let counter = Counter;
/// assert_eq!(counter.key_help().len(), 3);
/// ```
pub trait WidgetKeyHelp {
    /// Return the keyboard shortcuts for this widget.
    ///
    /// Format: `(key_combo, description)`.
    fn key_help(&self) -> &'static [(&'static str, &'static str)];
}

/// A single published keymap entry collected via
/// [`Context::publish_keymap`](crate::Context::publish_keymap) (issue #236).
///
/// Once registered for the current frame, every entry is queryable through
/// [`Context::published_keymaps`](crate::Context::published_keymaps) and is
/// rendered automatically by the command-palette help overlay.
#[derive(Debug, Clone)]
pub struct PublishedKeymap {
    /// Optional widget / scope name (e.g. `"rich_log"`, `"table"`).
    pub name: &'static str,
    /// `(key_combo, description)` pairs. Always `'static` — no per-frame
    /// allocation.
    pub bindings: &'static [(&'static str, &'static str)],
}

impl PublishedKeymap {
    /// Construct a [`PublishedKeymap`] from a static slice of bindings.
    pub const fn new(
        name: &'static str,
        bindings: &'static [(&'static str, &'static str)],
    ) -> Self {
        Self { name, bindings }
    }
}
