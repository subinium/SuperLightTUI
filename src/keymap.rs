use crate::event::{KeyEvent, KeyEventKind};
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

impl Binding {
    /// Returns `true` if `key` is a press of this binding's registered chord.
    ///
    /// This is the dispatch primitive that mirrors bubbletea's
    /// `key.Matches`: a [`KeyEvent`] matches when it is a **press** (releases
    /// and repeats never match), its [`KeyCode`] equals [`Binding::key`], and
    /// its modifiers satisfy [`Binding::modifiers`]:
    ///
    /// - `modifiers: None` requires that **no** modifiers are held (so a plain
    ///   `q` binding does not fire on `Ctrl+q`).
    /// - `modifiers: Some(mods)` requires that *at least* every modifier in
    ///   `mods` is held, matching the lenient semantics of
    ///   [`Context::key_mod`](crate::Context::key_mod).
    ///
    /// # Examples
    /// ```
    /// use slt::{Event, KeyCode, KeyMap, KeyModifiers};
    ///
    /// let km = KeyMap::new().bind('q', "Quit");
    /// let binding = &km.bindings[0];
    ///
    /// let Event::Key(quit) = Event::key_char('q') else { unreachable!() };
    /// assert!(binding.matches(&quit));
    ///
    /// // A plain binding ignores modified presses of the same key.
    /// let Event::Key(ctrl_q) = Event::key_ctrl('q') else { unreachable!() };
    /// assert!(!binding.matches(&ctrl_q));
    ///
    /// // A different key never matches.
    /// let Event::Key(other) = Event::key_char('x') else { unreachable!() };
    /// assert!(!binding.matches(&other));
    ///
    /// // Modifier bindings use `contains` semantics.
    /// let save = KeyMap::new().bind_mod('s', KeyModifiers::CONTROL, "Save");
    /// let Event::Key(ctrl_s) =
    ///     Event::key_mod(KeyCode::Char('s'), KeyModifiers::CONTROL)
    /// else {
    ///     unreachable!()
    /// };
    /// assert!(save.bindings[0].matches(&ctrl_s));
    /// ```
    pub fn matches(&self, key: &KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if key.code != self.key {
            return false;
        }
        match self.modifiers {
            None | Some(KeyModifiers::NONE) => key.modifiers == KeyModifiers::NONE,
            Some(mods) => key.modifiers.contains(mods),
        }
    }
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

    /// Return the first registered binding whose chord matches `key`.
    ///
    /// Bindings are checked in registration order, so if two bindings could
    /// match the same key the earlier `.bind*()` call wins. Returns `None`
    /// when no binding matches (including for release / repeat events, which
    /// [`Binding::matches`] never accepts).
    ///
    /// # Examples
    /// ```
    /// use slt::{Event, KeyCode, KeyMap};
    ///
    /// let km = KeyMap::new()
    ///     .bind('q', "Quit")
    ///     .bind_code(KeyCode::Up, "Move up");
    ///
    /// let Event::Key(up) = Event::key(KeyCode::Up) else { unreachable!() };
    /// assert_eq!(km.matched(&up).unwrap().description, "Move up");
    ///
    /// let Event::Key(unbound) = Event::key_char('z') else { unreachable!() };
    /// assert!(km.matched(&unbound).is_none());
    /// ```
    pub fn matched(&self, key: &KeyEvent) -> Option<&Binding> {
        self.bindings.iter().find(|binding| binding.matches(key))
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
/// If you prefer a free-function call, [`publish_keymap`](crate::Context::publish_keymap) takes the
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

impl crate::Context {
    /// Match the current frame's unconsumed key presses against `map` and
    /// return the first [`Binding`] that fires.
    ///
    /// This is the [`KeyMap`] counterpart to the per-key peek helpers like
    /// [`key`](crate::Context::key) and [`key_code`](crate::Context::key_code):
    /// it scans every unconsumed key-**press** event for this frame, in arrival
    /// order, and returns the first binding in `map` whose chord matches (see
    /// [`Binding::matches`]). The event is **not** consumed — callers can react
    /// to the returned binding and, if desired, still let other handlers see
    /// the key. Returns `None` when no press matches any binding.
    ///
    /// Like the other peek helpers, this respects the modal/overlay guard: when
    /// a modal is active and the caller is outside an overlay, no presses are
    /// visible and the method returns `None`.
    ///
    /// # Examples
    /// ```
    /// use slt::{KeyCode, KeyMap, TestBackend};
    ///
    /// let km = KeyMap::new()
    ///     .bind('q', "Quit")
    ///     .bind_code(KeyCode::Up, "Move up");
    ///
    /// let mut tb = TestBackend::new(20, 3);
    /// tb.run_with_events(vec![slt::Event::key_char('q')], |ui| {
    ///     let hit = ui.keymap_match(&km);
    ///     ui.text(hit.map(|b| b.description.as_str()).unwrap_or("none"));
    /// });
    /// tb.assert_contains("Quit");
    /// ```
    pub fn keymap_match<'m>(&self, map: &'m KeyMap) -> Option<&'m Binding> {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return None;
        }
        self.available_key_presses()
            .find_map(|(_, key)| map.matched(key))
    }
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::TestBackend;
    use crate::event::Event;

    fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        match Event::key_mod(code, modifiers) {
            Event::Key(k) => k,
            _ => unreachable!("key_mod always builds a Key event"),
        }
    }

    fn release_event(c: char) -> KeyEvent {
        match Event::key_release(c) {
            Event::Key(k) => k,
            _ => unreachable!("key_release always builds a Key event"),
        }
    }

    #[test]
    fn binding_matches_plain_char() {
        let km = KeyMap::new().bind('q', "Quit");
        let binding = &km.bindings[0];
        assert!(binding.matches(&key_event(KeyCode::Char('q'), KeyModifiers::NONE)));
        // Different char does not match.
        assert!(!binding.matches(&key_event(KeyCode::Char('x'), KeyModifiers::NONE)));
        // Plain binding rejects a modified press of the same key.
        assert!(!binding.matches(&key_event(KeyCode::Char('q'), KeyModifiers::CONTROL)));
    }

    #[test]
    fn binding_matches_modifier_chord_contains() {
        let km = KeyMap::new().bind_mod('s', KeyModifiers::CONTROL, "Save");
        let binding = &km.bindings[0];
        // Exact modifier matches.
        assert!(binding.matches(&key_event(KeyCode::Char('s'), KeyModifiers::CONTROL)));
        // Extra modifiers still satisfy `contains` semantics.
        let ctrl_shift = KeyModifiers(KeyModifiers::CONTROL.0 | KeyModifiers::SHIFT.0);
        assert!(binding.matches(&key_event(KeyCode::Char('s'), ctrl_shift)));
        // Missing the required modifier does not match.
        assert!(!binding.matches(&key_event(KeyCode::Char('s'), KeyModifiers::NONE)));
    }

    #[test]
    fn explicit_none_modifier_matches_only_unmodified_presses() {
        let km = KeyMap::new().bind_code_mod(KeyCode::Enter, KeyModifiers::NONE, "Submit");
        let binding = &km.bindings[0];

        assert!(binding.matches(&key_event(KeyCode::Enter, KeyModifiers::NONE)));
        for modifier in [
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::SHIFT,
            KeyModifiers::SUPER,
            KeyModifiers::HYPER,
            KeyModifiers::META,
        ] {
            assert!(
                !binding.matches(&key_event(KeyCode::Enter, modifier)),
                "explicit NONE unexpectedly matched {modifier:?}"
            );
        }
    }

    #[test]
    fn binding_rejects_release_events() {
        let km = KeyMap::new().bind('q', "Quit");
        // Edge case: a release of the bound key must never match.
        assert!(!km.bindings[0].matches(&release_event('q')));
    }

    #[test]
    fn matched_returns_first_registered_binding() {
        let km = KeyMap::new()
            .bind('q', "Quit")
            .bind_code(KeyCode::Up, "Move up")
            .bind_mod('s', KeyModifiers::CONTROL, "Save");

        let up = km
            .matched(&key_event(KeyCode::Up, KeyModifiers::NONE))
            .expect("Up should match");
        assert_eq!(up.description, "Move up");

        let save = km
            .matched(&key_event(KeyCode::Char('s'), KeyModifiers::CONTROL))
            .expect("Ctrl+S should match");
        assert_eq!(save.description, "Save");

        // Non-matching key returns None.
        assert!(
            km.matched(&key_event(KeyCode::Char('z'), KeyModifiers::NONE))
                .is_none()
        );
    }

    #[test]
    fn matched_first_registration_wins_on_overlap() {
        // Two bindings claim the same chord; the earlier registration wins.
        let km = KeyMap::new().bind('a', "First").bind('a', "Second");
        let hit = km
            .matched(&key_event(KeyCode::Char('a'), KeyModifiers::NONE))
            .expect("'a' should match");
        assert_eq!(hit.description, "First");
    }

    #[test]
    fn context_keymap_match_returns_binding_for_frame_press() {
        let km = KeyMap::new()
            .bind('q', "Quit")
            .bind_code(KeyCode::Up, "Move up");

        let mut tb = TestBackend::new(20, 3);
        tb.run_with_events(vec![Event::key(KeyCode::Up)], |ui| {
            let hit = ui.keymap_match(&km);
            ui.text(hit.map(|b| b.description.as_str()).unwrap_or("none"));
        });
        tb.assert_contains("Move up");
    }

    #[test]
    fn context_keymap_match_none_when_no_press_matches() {
        let km = KeyMap::new().bind('q', "Quit");

        let mut tb = TestBackend::new(20, 3);
        // A press the map does not bind yields no match.
        tb.run_with_events(vec![Event::key_char('z')], |ui| {
            let hit = ui.keymap_match(&km);
            ui.text(hit.map(|b| b.description.as_str()).unwrap_or("none"));
        });
        tb.assert_contains("none");
    }
}
