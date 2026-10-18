//! Event and environment helpers — keyboard / mouse / paste / scroll
//! queries, quit, clipboard, theme, terminal size, tick and debug toggles.
//!
//! Not a widget family: these read the current frame's input and runtime
//! state and have no state type.

use super::*;
use crate::event::KeyEvent;
use crate::runtime::TextEntryKind;

/// Issue #453: whether a focused text-entry widget of `kind` edits with `key`.
///
/// Printable keys belong to the widget unless they carry a global shortcut
/// modifier (Ctrl / Super / Meta, see #349). Cursor and deletion keys belong
/// to it with any modifier (Ctrl+Left is word navigation). Multi-line editors
/// also own Enter, Up and Down, and code editors (#466) also own Tab,
/// Shift+Tab, PageUp and PageDown. Everything else (Esc, Tab, F-keys,
/// chords) stays visible to shortcut queries.
///
/// Issue #465: Ctrl/Super chords the widget handles itself (`chords`, see
/// `CHORD_*`) are owned too: select all, copy / cut while text is selected,
/// undo / redo and kill-line.
fn text_entry_owns(kind: TextEntryKind, chords: u8, key: &KeyEvent) -> bool {
    use crate::runtime::{CHORD_COPY_CUT, CHORD_HISTORY, CHORD_KILL_LINE, CHORD_SELECT_ALL};
    match key.code {
        KeyCode::Char(ch) if has_global_shortcut_modifier(key.modifiers) => {
            let command = key.modifiers.contains(KeyModifiers::CONTROL)
                || key.modifiers.contains(KeyModifiers::SUPER);
            let bit = match ch.to_ascii_lowercase() {
                'a' => CHORD_SELECT_ALL,
                'c' | 'x' if !key.modifiers.contains(KeyModifiers::SHIFT) => CHORD_COPY_CUT,
                'z' | 'y' => CHORD_HISTORY,
                'k' => CHORD_KILL_LINE,
                _ => 0,
            };
            command && chords & bit != 0
        }
        KeyCode::Char(_) => true,
        KeyCode::Backspace
        | KeyCode::Delete
        | KeyCode::Left
        | KeyCode::Right
        | KeyCode::Home
        | KeyCode::End => true,
        KeyCode::Enter | KeyCode::Up | KeyCode::Down => kind != TextEntryKind::SingleLine,
        KeyCode::Tab | KeyCode::BackTab | KeyCode::PageUp | KeyCode::PageDown => {
            kind == TextEntryKind::Code
        }
        _ => false,
    }
}

impl Context {
    // ── events ───────────────────────────────────────────────────────

    fn shortcuts_blocked_by_modal(&self) -> bool {
        (self.rollback.modal_active || self.prev_modal_active) && self.rollback.overlay_depth == 0
    }

    /// Unconsumed key presses a shortcut query may see: keys a focused
    /// text-entry widget edits with are withheld (issue #453), so the answer
    /// does not depend on whether the query runs before or after the widget.
    fn shortcut_key_presses(&self) -> impl Iterator<Item = (usize, &KeyEvent)> + '_ {
        let owner = self.text_entry_owner();
        self.available_key_presses().filter(move |(_, key)| {
            !owner.is_some_and(|(kind, chords)| text_entry_owns(kind, chords, key))
        })
    }

    /// Check if a character key was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    ///
    /// While a text-entry widget ([`text_input`](Self::text_input),
    /// [`textarea`](Self::textarea), a form field or an open command palette)
    /// owns focus, a printable key without Ctrl / Super / Meta belongs to that
    /// widget and `key` returns `false` for it, whether this call runs before
    /// or after the widget in the frame closure. Use [`raw_key`](Self::raw_key)
    /// to observe every press, or [`shortcut`](Self::shortcut) for an exact
    /// key + modifier match.
    ///
    /// The owner is the slot that holds focus as the frame starts (or the slot
    /// focused earlier in this frame), so on the very first frame of an app,
    /// before any widget has registered, nothing is withheld.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::{EventBuilder, TestBackend, widgets::TextInputState};
    ///
    /// let mut tb = TestBackend::new(30, 5);
    /// let mut input = TextInputState::new();
    /// tb.render(|ui| {
    ///     let _ = ui.text_input(&mut input); // the input takes focus
    /// });
    /// let mut quit = false;
    /// tb.run_with_events(EventBuilder::new().key('q').build(), |ui| {
    ///     quit = ui.key('q'); // checked before the widget runs
    ///     let _ = ui.text_input(&mut input);
    /// });
    /// assert!(!quit);
    /// assert_eq!(input.value, "q");
    /// ```
    pub fn key(&self, c: char) -> bool {
        if self.shortcuts_blocked_by_modal() {
            return false;
        }
        self.shortcut_key_presses()
            .any(|(_, k)| k.code == KeyCode::Char(c))
    }

    /// Check if a character key was pressed this frame, ignoring the modal
    /// guard and text-entry ownership.
    ///
    /// Escape hatch with the pre-0.26 semantics of [`key`](Self::key): it
    /// reports a press even while a modal is open or a text field owns focus
    /// (the field still inserts the character when it runs). Returns `true`
    /// if the key event has not been consumed by another widget.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// if ui.raw_key('`') {
    ///     // e.g. toggle a debug console even while typing
    /// }
    /// # });
    /// ```
    pub fn raw_key(&self, c: char) -> bool {
        self.available_key_presses()
            .any(|(_, k)| k.code == KeyCode::Char(c))
    }

    /// Order-independent keyboard shortcut: `code` pressed with exactly
    /// `modifiers` this frame.
    ///
    /// Safe to call anywhere in the frame closure. It follows the same rules
    /// as [`key`](Self::key): blocked outside an open modal, and blind to the
    /// keys a focused text-entry widget edits with, so a plain `q` shortcut
    /// never fires while the user types a `q` into an input. Chords with
    /// Ctrl / Super / Meta always reach it.
    ///
    /// Modifiers must match exactly, except that `SHIFT` is ignored for
    /// [`KeyCode::Char`] (the character already encodes case, and terminals
    /// disagree on whether they report Shift for it). The event is not
    /// consumed.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::{EventBuilder, KeyCode, KeyModifiers, TestBackend};
    ///
    /// let mut tb = TestBackend::new(20, 3);
    /// let mut quit = false;
    /// tb.run_with_events(EventBuilder::new().key('q').build(), |ui| {
    ///     if ui.shortcut(KeyCode::Char('q'), KeyModifiers::NONE) {
    ///         quit = true;
    ///     }
    /// });
    /// assert!(quit);
    /// ```
    pub fn shortcut(&self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        if self.shortcuts_blocked_by_modal() {
            return false;
        }
        let ignore_shift = matches!(code, KeyCode::Char(_));
        let normalize = |mods: KeyModifiers| {
            if ignore_shift {
                KeyModifiers(mods.0 & !KeyModifiers::SHIFT.0)
            } else {
                mods
            }
        };
        let wanted = normalize(modifiers);
        self.shortcut_key_presses()
            .any(|(_, k)| k.code == code && normalize(k.modifiers) == wanted)
    }

    /// Check if a specific key code was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    /// Blocked when a modal/overlay is active and the caller is outside the overlay.
    /// Use [`raw_key_code`](Self::raw_key_code) for global shortcuts that must work
    /// regardless of modal/overlay state.
    ///
    /// While a text-entry widget owns focus, the editing keys it handles
    /// (printable characters, Backspace, Delete, Left, Right, Home, End, and
    /// for multi-line editors Enter, Up and Down) are not reported. Keys the
    /// widget never edits with (`Esc`, `Tab`, function keys) still are.
    pub fn key_code(&self, code: KeyCode) -> bool {
        if self.shortcuts_blocked_by_modal() {
            return false;
        }
        self.shortcut_key_presses().any(|(_, k)| k.code == code)
    }

    /// Check if a specific key code was pressed this frame, ignoring modal/overlay state.
    ///
    /// Unlike [`key_code`](Self::key_code), this method bypasses the modal/overlay guard
    /// so it works even when a modal or overlay is active. Use this for global shortcuts
    /// (e.g. Esc to close a modal, Ctrl+Q to quit) that must always be reachable.
    /// It also ignores text-entry ownership (see [`key`](Self::key)).
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn raw_key_code(&self, code: KeyCode) -> bool {
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == code)
        })
    }

    /// Check if a character key was released this frame.
    ///
    /// Returns `true` if the key release event has not been consumed by another widget.
    pub fn key_release(&self, c: char) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Release && k.code == KeyCode::Char(c))
        })
    }

    /// Check if a specific key code was released this frame.
    ///
    /// Returns `true` if the key release event has not been consumed by another widget.
    pub fn key_code_release(&self, code: KeyCode) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Release && k.code == code)
        })
    }

    /// Check for a character key press and consume the event, preventing other
    /// handlers from seeing it.
    ///
    /// Returns `true` if the key was found unconsumed and is now consumed.
    /// Unlike [`key()`](Self::key) which peeks without consuming, this claims
    /// exclusive ownership of the event.
    ///
    /// Call **after** widgets if you want widgets to have priority over your
    /// handler, or **before** widgets to intercept first.
    ///
    /// Keys a focused text-entry widget edits with are never claimed (see
    /// [`key`](Self::key)), so calling this before a text input cannot steal
    /// the characters typed into it.
    pub fn consume_key(&mut self, c: char) -> bool {
        if self.shortcuts_blocked_by_modal() {
            return false;
        }
        let index = self.shortcut_key_presses().find_map(|(i, key)| {
            if key.code == KeyCode::Char(c) {
                Some(i)
            } else {
                None
            }
        });
        if let Some(index) = index {
            self.consume_indices([index]);
            true
        } else {
            false
        }
    }

    /// Check for a special key press and consume the event, preventing other
    /// handlers from seeing it.
    ///
    /// Returns `true` if the key was found unconsumed and is now consumed.
    /// Unlike [`key_code()`](Self::key_code) which peeks without consuming,
    /// this claims exclusive ownership of the event.
    ///
    /// Call **after** widgets if you want widgets to have priority over your
    /// handler, or **before** widgets to intercept first.
    ///
    /// Editing keys owned by a focused text-entry widget are never claimed
    /// (see [`key_code`](Self::key_code)).
    pub fn consume_key_code(&mut self, code: KeyCode) -> bool {
        if self.shortcuts_blocked_by_modal() {
            return false;
        }
        let index = self
            .shortcut_key_presses()
            .find_map(|(i, key)| if key.code == code { Some(i) } else { None });
        if let Some(index) = index {
            self.consume_indices([index]);
            true
        } else {
            false
        }
    }

    /// Check if a character key with specific modifiers was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    /// The press must carry at least `modifiers`. Presses without Ctrl /
    /// Super / Meta are withheld while a text-entry widget owns focus (see
    /// [`key`](Self::key)); chords such as `Ctrl+Q` always reach it.
    pub fn key_mod(&self, c: char, modifiers: KeyModifiers) -> bool {
        if self.shortcuts_blocked_by_modal() {
            return false;
        }
        self.shortcut_key_presses()
            .any(|(_, k)| k.code == KeyCode::Char(c) && k.modifiers.contains(modifiers))
    }

    /// Like [`key_mod`](Self::key_mod) but bypasses the modal/overlay guard
    /// and text-entry ownership.
    pub fn raw_key_mod(&self, c: char, modifiers: KeyModifiers) -> bool {
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == KeyCode::Char(c) && k.modifiers.contains(modifiers))
        })
    }

    /// Return the position of a left mouse button down event this frame, if any.
    ///
    /// Returns `None` if no unconsumed mouse-down event occurred.
    pub fn mouse_down(&self) -> Option<(u32, u32)> {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Mouse(mouse) = event
                && matches!(mouse.kind, MouseKind::Down(MouseButton::Left))
            {
                return Some((mouse.x, mouse.y));
            }
            None
        })
    }

    /// Return the position of a left mouse button drag event this frame, if any.
    ///
    /// Returns `None` if no unconsumed drag event occurred. Drag events fire
    /// while the left button is held and the cursor moves.
    pub fn mouse_drag(&self) -> Option<(u32, u32)> {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Mouse(mouse) = event
                && matches!(mouse.kind, MouseKind::Drag(MouseButton::Left))
            {
                return Some((mouse.x, mouse.y));
            }
            None
        })
    }

    /// Return the position of a left mouse button release event this frame, if any.
    ///
    /// Returns `None` if no unconsumed mouse-up event occurred.
    pub fn mouse_up(&self) -> Option<(u32, u32)> {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Mouse(mouse) = event
                && matches!(mouse.kind, MouseKind::Up(MouseButton::Left))
            {
                return Some((mouse.x, mouse.y));
            }
            None
        })
    }

    /// Return the position of a mouse button down event for the specified button.
    ///
    /// This is a generalized version of [`mouse_down`](Self::mouse_down) that
    /// accepts any [`MouseButton`].
    pub fn mouse_down_button(&self, button: MouseButton) -> Option<(u32, u32)> {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Mouse(mouse) = event
                && matches!(&mouse.kind, MouseKind::Down(b) if *b == button)
            {
                return Some((mouse.x, mouse.y));
            }
            None
        })
    }

    /// Return the position of a mouse drag event for the specified button.
    pub fn mouse_drag_button(&self, button: MouseButton) -> Option<(u32, u32)> {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Mouse(mouse) = event
                && matches!(&mouse.kind, MouseKind::Drag(b) if *b == button)
            {
                return Some((mouse.x, mouse.y));
            }
            None
        })
    }

    /// Return the position of a mouse button release event for the specified button.
    pub fn mouse_up_button(&self, button: MouseButton) -> Option<(u32, u32)> {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Mouse(mouse) = event
                && matches!(&mouse.kind, MouseKind::Up(b) if *b == button)
            {
                return Some((mouse.x, mouse.y));
            }
            None
        })
    }

    /// Return the current mouse cursor position, if known.
    ///
    /// The position is updated on every mouse move or click event. Returns
    /// `None` until the first mouse event is received.
    pub fn mouse_pos(&self) -> Option<(u32, u32)> {
        self.mouse_pos
    }

    /// Return the first unconsumed paste event text, if any.
    pub fn paste(&self) -> Option<&str> {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return None;
        }
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Paste(text) = event {
                return Some(text.as_str());
            }
            None
        })
    }

    /// Check if an unconsumed scroll-up event occurred this frame.
    pub fn scroll_up(&self) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        self.events.iter().enumerate().any(|(i, event)| {
            !self.consumed[i]
                && matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollUp))
        })
    }

    /// Check if an unconsumed scroll-down event occurred this frame.
    pub fn scroll_down(&self) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        self.events.iter().enumerate().any(|(i, event)| {
            !self.consumed[i]
                && matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollDown))
        })
    }

    /// Check if an unconsumed scroll-left event occurred this frame.
    pub fn scroll_left(&self) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        self.events.iter().enumerate().any(|(i, event)| {
            !self.consumed[i]
                && matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollLeft))
        })
    }

    /// Check if an unconsumed scroll-right event occurred this frame.
    pub fn scroll_right(&self) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        self.events.iter().enumerate().any(|(i, event)| {
            !self.consumed[i]
                && matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollRight))
        })
    }

    /// Iterate over unconsumed events this frame, respecting the modal guard.
    ///
    /// Returns an empty iterator when a modal is active and the caller is not
    /// inside an overlay. Use [`raw_events`](Self::raw_events) to bypass the
    /// modal guard (e.g., for global hotkeys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// for event in ui.events() {
    ///     if let slt::Event::Mouse(mouse) = event {
    ///         if matches!(mouse.kind, slt::MouseKind::Down(slt::MouseButton::Right)) {
    ///             // handle right-click
    ///         }
    ///     }
    /// }
    /// # });
    /// ```
    pub fn events(&self) -> impl Iterator<Item = &Event> {
        let blocked = (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0;
        self.events.iter().enumerate().filter_map(move |(i, e)| {
            if blocked || self.consumed[i] {
                None
            } else {
                Some(e)
            }
        })
    }

    /// Iterate over all unconsumed events, bypassing the modal guard.
    ///
    /// Use this for global shortcuts that must work even when a modal or
    /// overlay is active. Prefer [`events`](Self::events) for normal use.
    pub fn raw_events(&self) -> impl Iterator<Item = &Event> + '_ {
        self.events
            .iter()
            .enumerate()
            .filter_map(|(i, e)| if self.consumed[i] { None } else { Some(e) })
    }

    /// Signal the run loop to exit after this frame.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Copy text to the system clipboard via OSC 52.
    ///
    /// The text is queued and written to the terminal after the current
    /// frame renders; it travels over SSH because the terminal, not the
    /// remote host, owns the clipboard. Requires a terminal that accepts
    /// OSC 52 writes (Ghostty, kitty, WezTerm, iTerm2, Windows Terminal).
    ///
    /// Inside tmux the default `set-clipboard external` drops the sequence.
    /// Set `set -g set-clipboard on`, or enable
    /// [`RunConfig::clipboard_passthrough`](crate::RunConfig::clipboard_passthrough)
    /// together with `set -g allow-passthrough on`. Payloads larger than
    /// [`RunConfig::clipboard_max_bytes`](crate::RunConfig::clipboard_max_bytes)
    /// are not sent. The outcome is reported for one frame through
    /// [`clipboard_status`](Self::clipboard_status); OSC 52 has no
    /// acknowledgement, so [`ClipboardStatus::Sent`](crate::ClipboardStatus::Sent)
    /// means "written", not "received". Custom backends and `TestBackend`
    /// never write the clipboard.
    pub fn copy_to_clipboard(&mut self, text: impl Into<String>) {
        self.clipboard_text = Some(text.into());
    }

    /// Get the current theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Resolve a [`ThemeColor`] token against the current theme.
    pub fn color(&self, token: ThemeColor) -> Color {
        self.theme.resolve(token)
    }

    /// Get the current spacing scale from the theme.
    pub fn spacing(&self) -> Spacing {
        self.theme.spacing
    }

    /// Change the theme for subsequent rendering.
    ///
    /// All widgets rendered after this call will use the new theme's colors.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Check if dark mode is active.
    pub fn is_dark_mode(&self) -> bool {
        self.rollback.dark_mode
    }

    /// Set dark mode. When true, dark_* style variants are applied.
    pub fn set_dark_mode(&mut self, dark: bool) {
        self.rollback.dark_mode = dark;
    }

    // ── info ─────────────────────────────────────────────────────────

    /// Get the terminal width in cells.
    pub fn width(&self) -> u32 {
        self.area_width
    }

    /// Get the current terminal width breakpoint.
    ///
    /// Returns a [`Breakpoint`] based on the terminal width:
    /// - `Xs`: < 40 columns
    /// - `Sm`: 40-79 columns
    /// - `Md`: 80-119 columns
    /// - `Lg`: 120-159 columns
    /// - `Xl`: >= 160 columns
    ///
    /// Use this for responsive layouts that adapt to terminal size:
    /// ```no_run
    /// # use slt::{Breakpoint, Context};
    /// # slt::run(|ui: &mut Context| {
    /// match ui.breakpoint() {
    ///     Breakpoint::Xs | Breakpoint::Sm => {
    ///         ui.col(|ui| { ui.text("Stacked layout"); });
    ///     }
    ///     _ => {
    ///         ui.row(|ui| { ui.text("Side-by-side layout"); });
    ///     }
    /// }
    /// # });
    /// ```
    pub fn breakpoint(&self) -> Breakpoint {
        let w = self.area_width;
        if w < 40 {
            Breakpoint::Xs
        } else if w < 80 {
            Breakpoint::Sm
        } else if w < 120 {
            Breakpoint::Md
        } else if w < 160 {
            Breakpoint::Lg
        } else {
            Breakpoint::Xl
        }
    }

    /// Get the terminal height in cells.
    pub fn height(&self) -> u32 {
        self.area_height
    }

    /// Get the current tick count (increments each frame).
    ///
    /// Useful for animations and time-based logic. The tick starts at 0 and
    /// increases by 1 on every rendered frame.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Return whether the layout debugger is enabled.
    ///
    /// The debugger is toggled with F12 at runtime.
    pub fn debug_enabled(&self) -> bool {
        self.debug
    }

    /// Return which layers the F12 debug overlay outlines (issue #201).
    ///
    /// Default is [`crate::DebugLayer::All`], which outlines the base tree
    /// plus any active overlays/modals. See
    /// [`set_debug_layer`](Self::set_debug_layer) to narrow the outline to
    /// a specific layer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{Context, DebugLayer};
    ///
    /// slt::run(|ui: &mut Context| {
    ///     // Read the current layer to drive a UI badge or debug toolbar.
    ///     match ui.debug_layer() {
    ///         DebugLayer::All => ui.text("layer: all"),
    ///         DebugLayer::TopMost => ui.text("layer: topmost"),
    ///         DebugLayer::BaseOnly => ui.text("layer: base"),
    ///     };
    /// }).unwrap();
    /// ```
    pub fn debug_layer(&self) -> crate::DebugLayer {
        self.debug_layer
    }

    /// Choose which layers the F12 debug overlay outlines (issue #201).
    ///
    /// Persists across frames. The default ([`crate::DebugLayer::All`])
    /// matches the reporter's expectation that F12 reflects everything the
    /// renderer is drawing. Use [`crate::DebugLayer::TopMost`] to focus on
    /// the active modal / overlay only, or [`crate::DebugLayer::BaseOnly`]
    /// to keep the legacy behavior of skipping overlays.
    ///
    /// # Runtime keybinding
    ///
    /// At runtime, **Shift+F12** cycles through `All` → `TopMost` →
    /// `BaseOnly` → `All`. Plain F12 still toggles the overlay on/off.
    /// The two keys are independent: enabling the overlay does not change
    /// the active layer, and cycling layers does not enable the overlay.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{Context, DebugLayer};
    ///
    /// slt::run(|ui: &mut Context| {
    ///     // Toggle between viewing only the base tree and viewing all
    ///     // layers, e.g. from a custom debug menu.
    ///     let next = match ui.debug_layer() {
    ///         DebugLayer::All => DebugLayer::BaseOnly,
    ///         DebugLayer::BaseOnly => DebugLayer::TopMost,
    ///         DebugLayer::TopMost => DebugLayer::All,
    ///     };
    ///     ui.set_debug_layer(next);
    /// }).unwrap();
    /// ```
    pub fn set_debug_layer(&mut self, layer: crate::DebugLayer) {
        self.debug_layer = layer;
    }

    /// Return whether the devtools inspector panel is active (issue #268).
    ///
    /// The inspector is toggled with **Ctrl+F12** at runtime, or
    /// programmatically via [`set_inspector`](Self::set_inspector). It is
    /// independent of the F12 outline overlay and the Shift+F12 layer cycle.
    /// When on, the inspector draws a resolved-style panel for the focused
    /// widget and a focus-chain panel listing every focusable in order.
    ///
    /// Since 0.21.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::Context;
    ///
    /// slt::run(|ui: &mut Context| {
    ///     if ui.inspector() {
    ///         ui.text("inspector on (Ctrl+F12 to hide)");
    ///     }
    /// })
    /// .unwrap();
    /// ```
    pub fn inspector(&self) -> bool {
        self.inspector_mode
    }

    /// Show or hide the devtools inspector panel (issue #268).
    ///
    /// Symmetric with [`set_debug_layer`](Self::set_debug_layer): the change
    /// persists across frames. Enables (or disables) the focused-widget
    /// resolved-style panel and the focus-chain inspector. Equivalent to the
    /// runtime **Ctrl+F12** toggle; does not affect the F12 outline overlay.
    ///
    /// Since 0.21.0.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::Context;
    ///
    /// slt::run(|ui: &mut Context| {
    ///     // Open the inspector from a custom debug menu.
    ///     ui.set_inspector(true);
    /// })
    /// .unwrap();
    /// ```
    pub fn set_inspector(&mut self, on: bool) {
        self.inspector_mode = on;
    }
}
