use super::*;

impl Context {
    pub(crate) fn new(
        events: Vec<Event>,
        width: u32,
        height: u32,
        state: &mut FrameState,
        theme: Theme,
    ) -> Self {
        let hook_states = &mut state.hook_states;
        let named_states = std::mem::take(&mut state.named_states);
        let screen_hook_map = std::mem::take(&mut state.screen_hook_map);
        let focus = &mut state.focus;
        let layout_feedback = &mut state.layout_feedback;
        let diagnostics = &mut state.diagnostics;
        let consumed = vec![false; events.len()];

        let mut mouse_pos = layout_feedback.last_mouse_pos;
        let mut click_pos = None;
        for event in &events {
            if let Event::Mouse(mouse) = event {
                mouse_pos = Some((mouse.x, mouse.y));
                if matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                    click_pos = Some((mouse.x, mouse.y));
                }
            }
        }

        let mut focus_index = focus.focus_index;
        if let Some((mx, my)) = click_pos {
            let mut best: Option<(usize, u64)> = None;
            for &(fid, rect) in &layout_feedback.prev_focus_rects {
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

        let mut ctx = Self {
            commands: Vec::new(),
            events,
            consumed,
            should_quit: false,
            area_width: width,
            area_height: height,
            tick: diagnostics.tick,
            focus_index,
            hook_states: std::mem::take(hook_states),
            named_states,
            context_stack: Vec::new(),
            prev_focus_count: focus.prev_focus_count,
            prev_modal_focus_start: focus.prev_modal_focus_start,
            prev_modal_focus_count: focus.prev_modal_focus_count,
            prev_scroll_infos: std::mem::take(&mut layout_feedback.prev_scroll_infos),
            prev_scroll_rects: std::mem::take(&mut layout_feedback.prev_scroll_rects),
            prev_hit_map: std::mem::take(&mut layout_feedback.prev_hit_map),
            prev_group_rects: std::mem::take(&mut layout_feedback.prev_group_rects),
            prev_focus_groups: std::mem::take(&mut layout_feedback.prev_focus_groups),
            _prev_focus_rects: std::mem::take(&mut layout_feedback.prev_focus_rects),
            mouse_pos,
            click_pos,
            prev_modal_active: focus.prev_modal_active,
            clipboard_text: None,
            debug: diagnostics.debug_mode,
            theme,
            is_real_terminal: false,
            deferred_draws: Vec::new(),
            rollback: ContextRollbackState {
                last_text_idx: None,
                focus_count: 0,
                interaction_count: 0,
                scroll_count: 0,
                group_count: 0,
                group_stack: Vec::new(),
                overlay_depth: 0,
                modal_active: false,
                modal_focus_start: 0,
                modal_focus_count: 0,
                hook_cursor: 0,
                dark_mode: theme.is_dark,
                notification_queue: std::mem::take(&mut diagnostics.notification_queue),
                text_color_stack: Vec::new(),
            },
            pending_tooltips: Vec::new(),
            hovered_groups: std::collections::HashSet::new(),
            scroll_lines_per_event: 1,
            screen_hook_map,
            widget_theme: WidgetTheme::new(),
        };
        ctx.build_hovered_groups();
        ctx
    }

    fn build_hovered_groups(&mut self) {
        self.hovered_groups.clear();
        if let Some(pos) = self.mouse_pos {
            for (name, rect) in &self.prev_group_rects {
                if pos.0 >= rect.x
                    && pos.0 < rect.x + rect.width
                    && pos.1 >= rect.y
                    && pos.1 < rect.y + rect.height
                {
                    self.hovered_groups.insert(std::sync::Arc::clone(name));
                }
            }
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
        let snapshot = ContextCheckpoint::capture(self);

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

    /// Reserve the next interaction slot without emitting a marker command.
    pub(crate) fn reserve_interaction_slot(&mut self) -> usize {
        let id = self.rollback.interaction_count;
        self.rollback.interaction_count += 1;
        id
    }

    /// Advance the interaction counter for structural commands that still
    /// participate in hit-map indexing.
    pub(crate) fn skip_interaction_slot(&mut self) {
        self.reserve_interaction_slot();
    }

    /// Reserve the next interaction ID and emit a marker command.
    pub(crate) fn next_interaction_id(&mut self) -> usize {
        let id = self.reserve_interaction_slot();
        self.commands.push(Command::InteractionMarker(id));
        id
    }

    /// Allocate a click/hover interaction slot and return the [`Response`].
    ///
    /// Use this in custom widgets to detect mouse clicks and hovers without
    /// wrapping content in a container. Call it immediately before the text,
    /// rich text, link, or container that should own the interaction rect.
    /// Each call reserves one slot in the hit-test map, so the call order
    /// must be stable across frames.
    pub fn interaction(&mut self) -> Response {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return Response::none();
        }
        let id = self.next_interaction_id();
        self.response_for(id)
    }

    pub(crate) fn begin_widget_interaction(&mut self, focused: bool) -> (usize, Response) {
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        (interaction_id, response)
    }

    pub(crate) fn consume_indices<I>(&mut self, indices: I)
    where
        I: IntoIterator<Item = usize>,
    {
        for index in indices {
            self.consumed[index] = true;
        }
    }

    pub(crate) fn available_key_presses(
        &self,
    ) -> impl Iterator<Item = (usize, &crate::event::KeyEvent)> + '_ {
        self.events.iter().enumerate().filter_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => Some((i, key)),
                _ => None,
            }
        })
    }

    pub(crate) fn available_pastes(&self) -> impl Iterator<Item = (usize, &str)> + '_ {
        self.events.iter().enumerate().filter_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            match event {
                Event::Paste(text) => Some((i, text.as_str())),
                _ => None,
            }
        })
    }

    pub(crate) fn left_clicks_in_rect(
        &self,
        rect: Rect,
    ) -> impl Iterator<Item = (usize, &crate::event::MouseEvent)> + '_ {
        self.mouse_events_in_rect(rect).filter_map(|(i, mouse)| {
            if matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                Some((i, mouse))
            } else {
                None
            }
        })
    }

    pub(crate) fn mouse_events_in_rect(
        &self,
        rect: Rect,
    ) -> impl Iterator<Item = (usize, &crate::event::MouseEvent)> + '_ {
        self.events
            .iter()
            .enumerate()
            .filter_map(move |(i, event)| {
                if self.consumed[i] {
                    return None;
                }

                let Event::Mouse(mouse) = event else {
                    return None;
                };

                if mouse.x < rect.x
                    || mouse.x >= rect.right()
                    || mouse.y < rect.y
                    || mouse.y >= rect.bottom()
                {
                    return None;
                }

                Some((i, mouse))
            })
    }

    pub(crate) fn left_clicks_for_interaction(
        &self,
        interaction_id: usize,
    ) -> Option<(Rect, Vec<(usize, &crate::event::MouseEvent)>)> {
        let rect = self.prev_hit_map.get(interaction_id).copied()?;
        let clicks = self.left_clicks_in_rect(rect).collect();
        Some((rect, clicks))
    }

    pub(crate) fn consume_activation_keys(&mut self, focused: bool) -> bool {
        if !focused {
            return false;
        }

        // Activation keys (Enter / Space) are typically 0–1 per frame and
        // bounded above by the simultaneous-keypress count from the input
        // pipeline (well under 8 in practice). A small inline buffer
        // eliminates the per-focusable `Vec<usize>` heap allocation that
        // showed up on every focused widget × every frame. Closes #135.
        const INLINE_CAP: usize = 8;
        let mut buf = [0usize; INLINE_CAP];
        let mut count = 0usize;
        let mut overflow: Vec<usize> = Vec::new();

        for (i, key) in self.available_key_presses() {
            if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                if count < INLINE_CAP {
                    buf[count] = i;
                    count += 1;
                } else {
                    overflow.push(i);
                }
            }
        }

        let activated = count > 0 || !overflow.is_empty();
        if activated {
            // `consume_indices` takes `IntoIterator<Item = usize>` — pass an
            // iterator directly, no allocation needed for the inline path.
            self.consume_indices(buf[..count].iter().copied().chain(overflow));
        }
        activated
    }

    /// Register a widget as focusable and return whether it currently has focus.
    ///
    /// Call this in custom widgets that need keyboard focus. Each call increments
    /// the internal focus counter, so the call order must be stable across frames.
    pub fn register_focusable(&mut self) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        let id = self.rollback.focus_count;
        self.rollback.focus_count += 1;
        self.commands.push(Command::FocusMarker(id));
        if self.prev_modal_active
            && self.prev_modal_focus_count > 0
            && self.rollback.modal_active
            && self.rollback.overlay_depth > 0
        {
            let mut modal_local_id = id.saturating_sub(self.rollback.modal_focus_start);
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
        let idx = self.rollback.hook_cursor;
        self.rollback.hook_cursor += 1;

        if idx >= self.hook_states.len() {
            self.hook_states.push(Box::new(init()));
        }

        State::from_idx(idx)
    }

    /// Component-local persistent state keyed by a stable id.
    ///
    /// Unlike [`use_state`](Self::use_state), this is **not order-dependent** —
    /// the value is looked up by `id` instead of call position. Safe to call
    /// inside conditional branches or reusable component functions.
    ///
    /// Returns a `State<T>` handle. Access with `state.get(ui)` /
    /// `state.get_mut(ui)`. Persists across frames.
    ///
    /// # Scoping
    ///
    /// Keys are `&'static str` and live in a single global namespace per
    /// `Context` (no automatic per-component scoping). Two calls with the same
    /// `id` in the same frame share the same value, regardless of where they
    /// occur in the tree. Pick unique ids — for example, prefix with a
    /// component name (`"counter::value"`).
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn counter(ui: &mut slt::Context) {
    ///     let count = ui.use_state_named_with("counter::value", || 0i32);
    ///     ui.text(format!("Count: {}", count.get(ui)));
    ///     if ui.button("+1").clicked {
    ///         *count.get_mut(ui) += 1;
    ///     }
    /// }
    /// ```
    pub fn use_state_named_with<T: 'static>(
        &mut self,
        id: &'static str,
        init: impl FnOnce() -> T,
    ) -> State<T> {
        self.named_states
            .entry(id)
            .or_insert_with(|| Box::new(init()));
        State::from_named(id)
    }

    /// Like [`use_state_named_with`](Self::use_state_named_with), but uses
    /// [`Default::default()`] to initialize the value on first call.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let value = ui.use_state_named::<i32>("counter::value");
    /// ```
    pub fn use_state_named<T: 'static + Default>(&mut self, id: &'static str) -> State<T> {
        self.use_state_named_with(id, T::default)
    }

    /// Push a value onto the context stack for the duration of `body`.
    ///
    /// Inside `body`, child widgets can call
    /// [`use_context::<T>()`](Self::use_context) or
    /// [`try_use_context::<T>()`](Self::try_use_context) to look up the
    /// nearest provided value of type `T`. Provides cascade in LIFO order:
    /// nested calls with the same `T` shadow outer ones.
    ///
    /// The value is automatically popped when `body` returns — including on
    /// panic, so the context stack is always restored.
    ///
    /// # Example
    ///
    /// ```ignore
    /// struct Theme { accent: slt::Color }
    /// ui.provide(Theme { accent: slt::Color::Red }, |ui| {
    ///     // Any widget here can `let theme = ui.use_context::<Theme>();`
    ///     render_button(ui);
    /// });
    /// ```
    pub fn provide<T: 'static, R>(&mut self, value: T, body: impl FnOnce(&mut Context) -> R) -> R {
        self.context_stack
            .push(Box::new(value) as Box<dyn std::any::Any>);

        // catch_unwind ensures the entry is popped even if `body` panics, so
        // the context stack is never left with leaked frames. We re-panic
        // afterwards so the panic propagates normally to outer scopes.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(self)));

        // Pop in both success and panic paths.
        self.context_stack.pop();

        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    /// Look up the nearest provided value of type `T` on the context stack.
    ///
    /// Searches from the top of the stack (most-recent
    /// [`provide`](Self::provide)) downward. Returns the first match.
    ///
    /// # Panics
    ///
    /// Panics if no value of type `T` is currently provided. Use
    /// [`try_use_context`](Self::try_use_context) for a non-panicking variant.
    pub fn use_context<T: 'static>(&self) -> &T {
        self.try_use_context::<T>().unwrap_or_else(|| {
            panic!(
                "no context of type {} was provided; use ui.provide(value, |ui| ...) in a parent scope",
                std::any::type_name::<T>()
            )
        })
    }

    /// Like [`use_context`](Self::use_context), but returns `None` instead of
    /// panicking when no value of type `T` is on the stack.
    pub fn try_use_context<T: 'static>(&self) -> Option<&T> {
        self.context_stack
            .iter()
            .rev()
            .find_map(|entry| entry.downcast_ref::<T>())
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
        let idx = self.rollback.hook_cursor;
        self.rollback.hook_cursor += 1;

        // First call at this slot: allocate fresh state.
        if idx >= self.hook_states.len() {
            let value = compute(deps);
            self.hook_states.push(Box::new((deps.clone(), value)));
            return self.hook_states[idx]
                .downcast_ref::<(D, T)>()
                .map(|(_, v)| v)
                .expect("freshly inserted slot must downcast to its own type");
        }

        // Slot already exists: it must be the same `(D, T)` shape we used last
        // frame, or the caller broke the rules-of-hooks contract.
        match self.hook_states[idx].downcast_ref::<(D, T)>() {
            Some((stored, _)) => {
                if stored != deps {
                    let value = compute(deps);
                    self.hook_states[idx] = Box::new((deps.clone(), value));
                }
            }
            None => panic!(
                "Hook type mismatch at index {}: expected {}. Hooks must be called in the same order every frame.",
                idx,
                std::any::type_name::<(D, T)>()
            ),
        }

        self.hook_states[idx]
            .downcast_ref::<(D, T)>()
            .map(|(_, v)| v)
            .expect("slot was just verified or replaced with the correct type")
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
        self.rollback
            .notification_queue
            .push((message.to_string(), level, tick));
    }

    pub(crate) fn render_notifications(&mut self) {
        let tick = self.tick;
        self.rollback
            .notification_queue
            .retain(|(_, _, created)| tick.saturating_sub(*created) < 180);
        if self.rollback.notification_queue.is_empty() {
            return;
        }

        // The `overlay` closure captures `self` mutably, so we cannot keep an
        // immutable borrow of `self.rollback.notification_queue` alive across
        // the call. Move the queue out for the render, then move it back —
        // no `String::clone` per notification, no intermediate `Vec` alloc.
        // Closes the non-empty path of #138.
        let queue = std::mem::take(&mut self.rollback.notification_queue);
        let theme = self.theme;

        let _ = self.overlay(|ui| {
            let _ = ui.row(|ui| {
                ui.spacer();
                let _ = ui.col(|ui| {
                    for (message, level, _) in queue.iter().rev() {
                        let color = match level {
                            ToastLevel::Info => theme.primary,
                            ToastLevel::Success => theme.success,
                            ToastLevel::Warning => theme.warning,
                            ToastLevel::Error => theme.error,
                        };
                        let mut line = String::with_capacity(2 + message.len());
                        line.push_str("● ");
                        line.push_str(message);
                        ui.styled(line, Style::new().fg(color));
                    }
                });
            });
        });

        // Restore the queue so subsequent frames can re-render until each
        // entry's TTL expires above.
        self.rollback.notification_queue = queue;
    }
}
