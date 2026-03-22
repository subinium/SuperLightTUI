impl Context {
    pub(crate) fn new(
        events: Vec<Event>,
        width: u32,
        height: u32,
        state: &mut FrameState,
        theme: Theme,
    ) -> Self {
        let consumed = vec![false; events.len()];

        let mut mouse_pos = state.last_mouse_pos;
        let mut click_pos = None;
        for event in &events {
            if let Event::Mouse(mouse) = event {
                mouse_pos = Some((mouse.x, mouse.y));
                if matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                    click_pos = Some((mouse.x, mouse.y));
                }
            }
        }

        let mut focus_index = state.focus_index;
        if let Some((mx, my)) = click_pos {
            let mut best: Option<(usize, u64)> = None;
            for &(fid, rect) in &state.prev_focus_rects {
                if mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom() {
                    let area = rect.width as u64 * rect.height as u64;
                    if best.map_or(true, |(_, ba)| area < ba) {
                        best = Some((fid, area));
                    }
                }
            }
            if let Some((fid, _)) = best {
                focus_index = fid;
            }
        }

        Self {
            commands: Vec::new(),
            events,
            consumed,
            should_quit: false,
            area_width: width,
            area_height: height,
            tick: state.tick,
            focus_index,
            focus_count: 0,
            hook_states: std::mem::take(&mut state.hook_states),
            hook_cursor: 0,
            prev_focus_count: state.prev_focus_count,
            modal_focus_start: 0,
            modal_focus_count: 0,
            prev_modal_focus_start: state.prev_modal_focus_start,
            prev_modal_focus_count: state.prev_modal_focus_count,
            scroll_count: 0,
            prev_scroll_infos: std::mem::take(&mut state.prev_scroll_infos),
            prev_scroll_rects: std::mem::take(&mut state.prev_scroll_rects),
            interaction_count: 0,
            prev_hit_map: std::mem::take(&mut state.prev_hit_map),
            group_stack: Vec::new(),
            prev_group_rects: std::mem::take(&mut state.prev_group_rects),
            group_count: 0,
            prev_focus_groups: std::mem::take(&mut state.prev_focus_groups),
            _prev_focus_rects: std::mem::take(&mut state.prev_focus_rects),
            mouse_pos,
            click_pos,
            last_text_idx: None,
            overlay_depth: 0,
            modal_active: false,
            prev_modal_active: state.prev_modal_active,
            clipboard_text: None,
            debug: state.debug_mode,
            theme,
            dark_mode: theme.is_dark,
            is_real_terminal: false,
            deferred_draws: Vec::new(),
            notification_queue: std::mem::take(&mut state.notification_queue),
            pending_tooltips: Vec::new(),
            text_color_stack: Vec::new(),
            scroll_lines_per_event: 1,
        }
    }

    /// Set how many lines each scroll event moves. Default is 1.
    pub fn set_scroll_speed(&mut self, lines: u32) {
        self.scroll_lines_per_event = lines.max(1);
    }

    /// Get the current scroll speed (lines per scroll event).
    pub fn scroll_speed(&self) -> u32 {
        self.scroll_lines_per_event
    }

    /// Get the current focus index.
    ///
    /// Widget indices are assigned in the order [`register_focusable()`](Self::register_focusable) is called.
    /// Indices are 0-based and wrap at [`focus_count()`](Self::focus_count).
    pub fn focus_index(&self) -> usize {
        self.focus_index
    }

    /// Set the focus index to a specific focusable widget.
    ///
    /// Widget indices are assigned in the order [`register_focusable()`](Self::register_focusable) is called
    /// (0-based). If `index` exceeds the number of focusable widgets it will
    /// be clamped by the modulo in [`register_focusable`](Self::register_focusable).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// // Focus the second focusable widget (index 1)
    /// ui.set_focus_index(1);
    /// # });
    /// ```
    pub fn set_focus_index(&mut self, index: usize) {
        self.focus_index = index;
    }

    /// Get the number of focusable widgets registered in the previous frame.
    ///
    /// Returns 0 on the very first frame. Useful together with
    /// [`set_focus_index()`](Self::set_focus_index) for programmatic focus control.
    ///
    /// Note: this intentionally reads `prev_focus_count` (the settled count
    /// from the last completed frame) rather than `focus_count` (the
    /// still-incrementing counter for the current frame).
    #[allow(clippy::misnamed_getters)]
    pub fn focus_count(&self) -> usize {
        self.prev_focus_count
    }

    pub(crate) fn process_focus_keys(&mut self) {
        for (i, event) in self.events.iter().enumerate() {
            if self.consumed[i] {
                continue;
            }
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.code == KeyCode::Tab && !key.modifiers.contains(KeyModifiers::SHIFT) {
                    if self.prev_modal_active && self.prev_modal_focus_count > 0 {
                        let mut modal_local =
                            self.focus_index.saturating_sub(self.prev_modal_focus_start);
                        modal_local %= self.prev_modal_focus_count;
                        let next = (modal_local + 1) % self.prev_modal_focus_count;
                        self.focus_index = self.prev_modal_focus_start + next;
                    } else if self.prev_focus_count > 0 {
                        self.focus_index = (self.focus_index + 1) % self.prev_focus_count;
                    }
                    self.consumed[i] = true;
                } else if (key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT))
                    || key.code == KeyCode::BackTab
                {
                    if self.prev_modal_active && self.prev_modal_focus_count > 0 {
                        let mut modal_local =
                            self.focus_index.saturating_sub(self.prev_modal_focus_start);
                        modal_local %= self.prev_modal_focus_count;
                        let prev = if modal_local == 0 {
                            self.prev_modal_focus_count - 1
                        } else {
                            modal_local - 1
                        };
                        self.focus_index = self.prev_modal_focus_start + prev;
                    } else if self.prev_focus_count > 0 {
                        self.focus_index = if self.focus_index == 0 {
                            self.prev_focus_count - 1
                        } else {
                            self.focus_index - 1
                        };
                    }
                    self.consumed[i] = true;
                }
            }
        }
    }

    /// Render a custom [`Widget`].
    ///
    /// Calls [`Widget::ui`] with this context and returns the widget's response.
    pub fn widget<W: Widget>(&mut self, w: &mut W) -> W::Response {
        w.ui(self)
    }

    /// Wrap child widgets in a panic boundary.
    ///
    /// If the closure panics, the panic is caught and an error message is
    /// rendered in place of the children. The app continues running.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.error_boundary(|ui| {
    ///     ui.text("risky widget");
    /// });
    /// # });
    /// ```
    pub fn error_boundary(&mut self, f: impl FnOnce(&mut Context)) {
        self.error_boundary_with(f, |ui, msg| {
            ui.styled(
                format!("⚠ Error: {msg}"),
                Style::new().fg(ui.theme.error).bold(),
            );
        });
    }

    /// Like [`error_boundary`](Self::error_boundary), but renders a custom
    /// fallback instead of the default error message.
    ///
    /// The fallback closure receives the panic message as a [`String`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.error_boundary_with(
    ///     |ui| {
    ///         ui.text("risky widget");
    ///     },
    ///     |ui, msg| {
    ///         ui.text(format!("Recovered from panic: {msg}"));
    ///     },
    /// );
    /// # });
    /// ```
    pub fn error_boundary_with(
        &mut self,
        f: impl FnOnce(&mut Context),
        fallback: impl FnOnce(&mut Context, String),
    ) {
        let snapshot = ContextSnapshot::capture(self);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            f(self);
        }));

        match result {
            Ok(()) => {}
            Err(panic_info) => {
                if self.is_real_terminal {
                    #[cfg(feature = "crossterm")]
                    {
                        let _ = crossterm::terminal::enable_raw_mode();
                        let _ = crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::EnterAlternateScreen
                        );
                    }

                    #[cfg(not(feature = "crossterm"))]
                    {}
                }

                snapshot.restore(self);

                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "widget panicked".to_string()
                };

                fallback(self, msg);
            }
        }
    }

    /// Reserve the next interaction ID and emit a marker command.
    pub(crate) fn next_interaction_id(&mut self) -> usize {
        let id = self.interaction_count;
        self.interaction_count += 1;
        self.commands.push(Command::InteractionMarker(id));
        id
    }

    /// Allocate a click/hover interaction slot and return the [`Response`].
    ///
    /// Use this in custom widgets to detect mouse clicks and hovers without
    /// wrapping content in a container. Each call reserves one slot in the
    /// hit-test map, so the call order must be stable across frames.
    pub fn interaction(&mut self) -> Response {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return Response::none();
        }
        let id = self.interaction_count;
        self.interaction_count += 1;
        self.response_for(id)
    }

    /// Register a widget as focusable and return whether it currently has focus.
    ///
    /// Call this in custom widgets that need keyboard focus. Each call increments
    /// the internal focus counter, so the call order must be stable across frames.
    pub fn register_focusable(&mut self) -> bool {
        if (self.modal_active || self.prev_modal_active) && self.overlay_depth == 0 {
            return false;
        }
        let id = self.focus_count;
        self.focus_count += 1;
        self.commands.push(Command::FocusMarker(id));
        if self.prev_modal_active
            && self.prev_modal_focus_count > 0
            && self.modal_active
            && self.overlay_depth > 0
        {
            let mut modal_local_id = id.saturating_sub(self.modal_focus_start);
            modal_local_id %= self.prev_modal_focus_count;
            let mut modal_focus_idx = self.focus_index.saturating_sub(self.prev_modal_focus_start);
            modal_focus_idx %= self.prev_modal_focus_count;
            return modal_local_id == modal_focus_idx;
        }
        if self.prev_focus_count == 0 {
            return true;
        }
        self.focus_index % self.prev_focus_count == id
    }

    /// Create persistent state that survives across frames.
    ///
    /// Returns a `State<T>` handle. Access with `state.get(ui)` / `state.get_mut(ui)`.
    ///
    /// # Rules
    /// - Must be called in the same order every frame (like React hooks)
    /// - Do NOT call inside if/else that changes between frames
    ///
    /// # Example
    /// ```ignore
    /// let count = ui.use_state(|| 0i32);
    /// let val = count.get(ui);
    /// ui.text(format!("Count: {val}"));
    /// if ui.button("+1").clicked {
    ///     *count.get_mut(ui) += 1;
    /// }
    /// ```
    pub fn use_state<T: 'static>(&mut self, init: impl FnOnce() -> T) -> State<T> {
        let idx = self.hook_cursor;
        self.hook_cursor += 1;

        if idx >= self.hook_states.len() {
            self.hook_states.push(Box::new(init()));
        }

        State {
            idx,
            _marker: std::marker::PhantomData,
        }
    }

    /// Memoize a computed value. Recomputes only when `deps` changes.
    ///
    /// # Example
    /// ```ignore
    /// let doubled = ui.use_memo(&count, |c| c * 2);
    /// ui.text(format!("Doubled: {doubled}"));
    /// ```
    pub fn use_memo<T: 'static, D: PartialEq + Clone + 'static>(
        &mut self,
        deps: &D,
        compute: impl FnOnce(&D) -> T,
    ) -> &T {
        let idx = self.hook_cursor;
        self.hook_cursor += 1;

        let should_recompute = if idx >= self.hook_states.len() {
            true
        } else {
            let (stored_deps, _) = self.hook_states[idx]
                .downcast_ref::<(D, T)>()
                .unwrap_or_else(|| {
                    panic!(
                        "Hook type mismatch at index {}: expected {}. Hooks must be called in the same order every frame.",
                        idx,
                        std::any::type_name::<(D, T)>()
                    )
                });
            stored_deps != deps
        };

        if should_recompute {
            let value = compute(deps);
            let slot = Box::new((deps.clone(), value));
            if idx < self.hook_states.len() {
                self.hook_states[idx] = slot;
            } else {
                self.hook_states.push(slot);
            }
        }

        let (_, value) = self.hook_states[idx]
            .downcast_ref::<(D, T)>()
            .unwrap_or_else(|| {
                panic!(
                    "Hook type mismatch at index {}: expected {}. Hooks must be called in the same order every frame.",
                    idx,
                    std::any::type_name::<(D, T)>()
                )
            });
        value
    }

    /// Returns `light` color if current theme is light mode, `dark` color if dark mode.
    pub fn light_dark(&self, light: Color, dark: Color) -> Color {
        if self.theme.is_dark {
            dark
        } else {
            light
        }
    }

    /// Show a toast notification without managing ToastState.
    ///
    /// # Examples
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(80, 24).render(|ui| {
    /// ui.notify("File saved!", ToastLevel::Success);
    /// # });
    /// ```
    pub fn notify(&mut self, message: &str, level: ToastLevel) {
        let tick = self.tick;
        self.notification_queue
            .push((message.to_string(), level, tick));
    }

    pub(crate) fn render_notifications(&mut self) {
        self.notification_queue
            .retain(|(_, _, created)| self.tick.saturating_sub(*created) < 180);
        if self.notification_queue.is_empty() {
            return;
        }

        let items: Vec<(String, Color)> = self
            .notification_queue
            .iter()
            .rev()
            .map(|(message, level, _)| {
                let color = match level {
                    ToastLevel::Info => self.theme.primary,
                    ToastLevel::Success => self.theme.success,
                    ToastLevel::Warning => self.theme.warning,
                    ToastLevel::Error => self.theme.error,
                };
                (message.clone(), color)
            })
            .collect();

        let _ = self.overlay(|ui| {
            let _ = ui.row(|ui| {
                ui.spacer();
                let _ = ui.col(|ui| {
                    for (message, color) in &items {
                        let mut line = String::with_capacity(2 + message.len());
                        line.push_str("● ");
                        line.push_str(message);
                        ui.styled(line, Style::new().fg(*color));
                    }
                });
            });
        });
    }
}

