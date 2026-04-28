use super::*;

impl Context {
    /// Render a help bar showing keybinding hints.
    ///
    /// `bindings` is a slice of `(key, action)` pairs. Keys are rendered in the
    /// theme's primary color; actions in the dim text color. Pairs are separated
    /// by a `·` character.
    pub fn help(&mut self, bindings: &[(&str, &str)]) -> Response {
        if bindings.is_empty() {
            return Response::none();
        }

        self.skip_interaction_slot();
        let help_gap = self.theme.spacing.sm();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: help_gap,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(self.theme.border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));
        for (idx, (key, action)) in bindings.iter().enumerate() {
            if idx > 0 {
                self.styled("·", Style::new().fg(self.theme.text_dim));
            }
            self.styled(*key, Style::new().bold().fg(self.theme.primary));
            self.styled(*action, Style::new().fg(self.theme.text_dim));
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        Response::none()
    }

    /// Render a help bar with custom key/description colors.
    pub fn help_colored(
        &mut self,
        bindings: &[(&str, &str)],
        key_color: Color,
        text_color: Color,
    ) -> Response {
        if bindings.is_empty() {
            return Response::none();
        }

        self.skip_interaction_slot();
        let help_gap = self.theme.spacing.sm();
        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Row,
                gap: help_gap,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(self.theme.border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));
        for (idx, (key, action)) in bindings.iter().enumerate() {
            if idx > 0 {
                self.styled("·", Style::new().fg(text_color));
            }
            self.styled(*key, Style::new().bold().fg(key_color));
            self.styled(*action, Style::new().fg(text_color));
        }
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        Response::none()
    }

    // ── events ───────────────────────────────────────────────────────

    /// Check if a character key was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn key(&self, c: char) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == KeyCode::Char(c))
        })
    }

    /// Check if a specific key code was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    /// Blocked when a modal/overlay is active and the caller is outside the overlay.
    /// Use [`raw_key_code`](Self::raw_key_code) for global shortcuts that must work
    /// regardless of modal/overlay state.
    pub fn key_code(&self, code: KeyCode) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == code)
        })
    }

    /// Check if a specific key code was pressed this frame, ignoring modal/overlay state.
    ///
    /// Unlike [`key_code`](Self::key_code), this method bypasses the modal/overlay guard
    /// so it works even when a modal or overlay is active. Use this for global shortcuts
    /// (e.g. Esc to close a modal, Ctrl+Q to quit) that must always be reachable.
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
    pub fn consume_key(&mut self, c: char) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        let index = self.available_key_presses().find_map(|(i, key)| {
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
    pub fn consume_key_code(&mut self, code: KeyCode) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        let index =
            self.available_key_presses().find_map(
                |(i, key)| {
                    if key.code == code {
                        Some(i)
                    } else {
                        None
                    }
                },
            );
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
    pub fn key_mod(&self, c: char, modifiers: KeyModifiers) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.kind == KeyEventKind::Press && k.code == KeyCode::Char(c) && k.modifiers.contains(modifiers))
        })
    }

    /// Like [`key_mod`](Self::key_mod) but bypasses the modal/overlay guard.
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
            if let Event::Mouse(mouse) = event {
                if matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                    return Some((mouse.x, mouse.y));
                }
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
            if let Event::Mouse(mouse) = event {
                if matches!(mouse.kind, MouseKind::Drag(MouseButton::Left)) {
                    return Some((mouse.x, mouse.y));
                }
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
            if let Event::Mouse(mouse) = event {
                if matches!(mouse.kind, MouseKind::Up(MouseButton::Left)) {
                    return Some((mouse.x, mouse.y));
                }
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
            if let Event::Mouse(mouse) = event {
                if matches!(&mouse.kind, MouseKind::Down(b) if *b == button) {
                    return Some((mouse.x, mouse.y));
                }
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
            if let Event::Mouse(mouse) = event {
                if matches!(&mouse.kind, MouseKind::Drag(b) if *b == button) {
                    return Some((mouse.x, mouse.y));
                }
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
            if let Event::Mouse(mouse) = event {
                if matches!(&mouse.kind, MouseKind::Up(b) if *b == button) {
                    return Some((mouse.x, mouse.y));
                }
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
            if let Event::Paste(ref text) = event {
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
    /// Works transparently over SSH connections. The text is queued and
    /// written to the terminal after the current frame renders.
    ///
    /// Requires a terminal that supports OSC 52 (most modern terminals:
    /// Ghostty, kitty, WezTerm, iTerm2, Windows Terminal).
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
    /// [`set_debug_layer`](Self::set_debug_layer) to narrow the outline to a
    /// specific layer.
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
    pub fn set_debug_layer(&mut self, layer: crate::DebugLayer) {
        self.debug_layer = layer;
    }
}
