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
        // Issue #215: hand off the keyed-state map for this frame. Same
        // lifetime as `named_states`: moved out at frame start, moved back
        // at frame end (see `run_frame_kernel`).
        let keyed_states = std::mem::take(&mut state.keyed_states);
        let screen_hook_map = std::mem::take(&mut state.screen_hook_map);
        let focus = &mut state.focus;
        // Issue #217: name→index map from the previous frame, used to resolve
        // `focus_by_name(name)` at frame start. We move it out so the
        // `register_focusable_named` calls in this frame can rebuild a fresh
        // `focus_name_map`. The fresh map is swapped back into
        // `focus_name_map_prev` at frame end.
        let focus_name_map_prev = std::mem::take(&mut focus.focus_name_map_prev);
        let pending_focus_name = focus.pending_focus_name.take();
        let prev_focus_index = focus.prev_focus_index;
        let layout_feedback = &mut state.layout_feedback;
        let diagnostics = &mut state.diagnostics;
        let consumed = vec![false; events.len()];

        let mut mouse_pos = layout_feedback.last_mouse_pos;
        let mut click_pos = None;
        let mut right_click_pos = None;
        for event in &events {
            if let Event::Mouse(mouse) = event {
                mouse_pos = Some((mouse.x, mouse.y));
                match mouse.kind {
                    MouseKind::Down(MouseButton::Left) => {
                        click_pos = Some((mouse.x, mouse.y));
                    }
                    MouseKind::Down(MouseButton::Right) => {
                        // Issue #208: capture last right-click position so
                        // `response_for` can hit-test against per-widget rects.
                        right_click_pos = Some((mouse.x, mouse.y));
                    }
                    _ => {}
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

        // Issue #217: resolve a pending `focus_by_name(...)` request against
        // the previous frame's `name → index` map. If the name wasn't
        // registered last frame, we keep the request pending for the next
        // frame so a widget that registers later can still receive focus.
        // If the request resolves, we consume it.
        let mut still_pending: Option<String> = None;
        if let Some(name) = pending_focus_name {
            if let Some(&resolved) = focus_name_map_prev.get(&name) {
                focus_index = resolved;
            } else {
                still_pending = Some(name);
            }
        }

        // Reuse `commands_buf` capacity from the previous frame (issue #150).
        // `mem::take` swaps an empty Vec into `state.commands_buf`; we then
        // clear (no-op since len==0) and reuse the reclaimed allocation. After
        // `build_tree` consumes commands, the empty Vec is returned to
        // `state.commands_buf` for the next frame in `run_frame_kernel`.
        let mut commands = std::mem::take(&mut state.commands_buf);
        commands.clear();

        // Issue #204: reuse the six per-frame `Vec`/`HashSet` allocations
        // (`context_stack`, `deferred_draws`, `rollback.group_stack`,
        // `rollback.text_color_stack`, `pending_tooltips`, `hovered_groups`).
        // Same `mem::take` pattern as `commands_buf` (#150). Each buffer is
        // empty at frame end (asserted at `run_frame_kernel`) — `mem::take`
        // hands a `Default::default()` empty back to the state, the Vec/HashSet
        // we move into `Context` keeps its capacity from the prior frame, and
        // `clear()` here is a no-op except as a defensive guard against future
        // refactors that might leak items past the assertions.
        let mut context_stack = std::mem::take(&mut state.context_stack_buf);
        context_stack.clear();
        let mut deferred_draws = std::mem::take(&mut state.deferred_draws_buf);
        deferred_draws.clear();
        let mut group_stack = std::mem::take(&mut state.group_stack_buf);
        group_stack.clear();
        let mut text_color_stack = std::mem::take(&mut state.text_color_stack_buf);
        text_color_stack.clear();
        let mut pending_tooltips = std::mem::take(&mut state.pending_tooltips_buf);
        pending_tooltips.clear();
        let hovered_groups = std::mem::take(&mut state.hovered_groups_buf);
        // `hovered_groups` is `clear()`-ed inside `build_hovered_groups`
        // immediately below, so we do not pre-clear here — capacity is
        // preserved across frames.

        let mut ctx = Self {
            commands,
            events,
            consumed,
            should_quit: false,
            area_width: width,
            area_height: height,
            tick: diagnostics.tick,
            focus_index,
            hook_states: std::mem::take(hook_states),
            named_states,
            keyed_states,
            context_stack,
            prev_focus_count: focus.prev_focus_count,
            prev_modal_focus_start: focus.prev_modal_focus_start,
            prev_modal_focus_count: focus.prev_modal_focus_count,
            prev_scroll_infos: std::mem::take(&mut layout_feedback.prev_scroll_infos),
            prev_scroll_rects: std::mem::take(&mut layout_feedback.prev_scroll_rects),
            prev_hit_map: std::mem::take(&mut layout_feedback.prev_hit_map),
            prev_group_rects: std::mem::take(&mut layout_feedback.prev_group_rects),
            prev_focus_groups: std::mem::take(&mut layout_feedback.prev_focus_groups),
            mouse_pos,
            click_pos,
            right_click_pos,
            prev_modal_active: focus.prev_modal_active,
            clipboard_text: None,
            debug: diagnostics.debug_mode,
            debug_layer: diagnostics.debug_layer,
            theme,
            is_real_terminal: false,
            deferred_draws,
            rollback: ContextRollbackState {
                last_text_idx: None,
                focus_count: 0,
                last_focusable_id: None,
                interaction_count: 0,
                scroll_count: 0,
                group_count: 0,
                group_stack,
                overlay_depth: 0,
                modal_active: false,
                modal_focus_start: 0,
                modal_focus_count: 0,
                hook_cursor: 0,
                dark_mode: theme.is_dark,
                notification_queue: std::mem::take(&mut diagnostics.notification_queue),
                text_color_stack,
            },
            pending_tooltips,
            hovered_groups,
            scroll_lines_per_event: 1,
            screen_hook_map,
            widget_theme: WidgetTheme::new(),
            prev_focus_index,
            focus_name_map_prev,
            focus_name_map: std::collections::HashMap::new(),
            pending_focus_name: still_pending,
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
        // Issue #208: compute focus transitions from the most recent
        // `register_focusable` call. If that focusable lined up with the
        // previously-focused widget index from the prior frame, focus
        // changes since map directly to gained/lost.
        if let Some(this_id) = self.rollback.last_focusable_id {
            let was_focused = self
                .prev_focus_index
                .map(|prev| prev == this_id)
                .unwrap_or(false);
            response.gained_focus = focused && !was_focused;
            response.lost_focus = !focused && was_focused;
            // Consume the marker so a single `register_focusable` powers
            // exactly one `begin_widget_interaction` call.
            self.rollback.last_focusable_id = None;
        }
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
        // pipeline (well under 8 in practice). A `SmallVec` with an 8-slot
        // inline capacity eliminates the per-focusable `Vec<usize>` heap
        // allocation that showed up on every focused widget × every frame.
        // Spillover beyond 8 falls back to the heap automatically. Closes #135.
        let consumed: smallvec::SmallVec<[usize; 8]> = self
            .available_key_presses()
            .filter_map(|(i, key)| {
                if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        let activated = !consumed.is_empty();
        if activated {
            // `consume_indices` takes `IntoIterator<Item = usize>` — `SmallVec`
            // satisfies that bound directly, no signature change needed.
            self.consume_indices(consumed);
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
            self.rollback.last_focusable_id = None;
            return false;
        }
        let id = self.rollback.focus_count;
        self.rollback.focus_count += 1;
        // Issue #208: remember this widget's focus id so the immediately
        // following `begin_widget_interaction` call can compare against
        // `prev_focus_index` and emit gained/lost focus signals.
        self.rollback.last_focusable_id = Some(id);
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

    /// Smoothly animate between `0.0` and `1.0` driven by a boolean.
    ///
    /// Returns the current interpolated value (0.0..=1.0). When `value` is
    /// `true` the result tweens toward `1.0`; when `false` it tweens back
    /// toward `0.0`. The transition duration defaults to
    /// [`DEFAULT_ANIMATE_TICKS`](crate::anim::DEFAULT_ANIMATE_TICKS) (12 ticks
    /// ≈ 200 ms at 60 Hz). Use [`Context::animate_value`] for custom duration
    /// or non-binary targets.
    ///
    /// State is stored in [`Context::named_states`](Self::named_states) under
    /// `id`. The id is `&'static str` (single global namespace per context),
    /// matching [`Context::use_state_named`]. Pick a unique key per call site
    /// — two `animate_bool` calls with the same id share state.
    ///
    /// On the first call, the value snaps to the target with no visible
    /// transition (so widgets that mount in their final state don't pop).
    ///
    /// # Example
    /// ```ignore
    /// let opacity = ui.animate_bool("sidebar::visible", is_open);
    /// // 0.0 ≤ opacity ≤ 1.0; use as alpha or visibility threshold.
    /// ```
    pub fn animate_bool(&mut self, id: &'static str, value: bool) -> f64 {
        let target = if value { 1.0 } else { 0.0 };
        self.animate_value(id, target, crate::anim::DEFAULT_ANIMATE_TICKS)
    }

    /// Smoothly animate a `f64` value toward `target` over `duration_ticks`.
    ///
    /// Uses a linear-easing [`Tween`] stored implicitly in
    /// [`Context::named_states`](Self::named_states) under `id`. Returns the
    /// current interpolated value. On the first call the value snaps to
    /// `target` with no visible transition; on subsequent calls when
    /// `target` changes the tween is rebuilt starting from the current
    /// interpolated value, so retargeting mid-flight does not produce a
    /// jump.
    ///
    /// `duration_ticks == 0` snaps immediately to the new target.
    ///
    /// # Example
    /// ```ignore
    /// let bar_height = ui.animate_value("loading::bar", target_height, 30);
    /// ui.bar(bar_height);
    /// ```
    ///
    /// # Comparison with `Tween`
    /// Use this shorthand when you want zero boilerplate and linear easing
    /// is acceptable. For custom easing, a non-static key, or
    /// non-tick-based control, construct a [`Tween`] explicitly via
    /// [`Context::use_state_named_with`](Self::use_state_named_with).
    pub fn animate_value(&mut self, id: &'static str, target: f64, duration_ticks: u64) -> f64 {
        let tick = self.tick;
        let entry = self
            .named_states
            .entry(id)
            .or_insert_with(|| Box::new(crate::anim::AnimState::new(target, tick)));
        let state = entry
            .downcast_mut::<crate::anim::AnimState>()
            .unwrap_or_else(|| {
                panic!(
                    "animate_value: id {:?} is already used for a different state type",
                    id
                )
            });
        state.sample(target, duration_ticks, tick)
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
        //
        // Single downcast on the cache-hit path (closes #133): use
        // `downcast_mut` to update deps/value in place when they change, and
        // return `&stored.1` directly — eliminating the redundant second
        // `downcast_ref` that ran on every call regardless of cache state.
        match self.hook_states[idx].downcast_mut::<(D, T)>() {
            Some(stored) => {
                if stored.0 != *deps {
                    stored.0 = deps.clone();
                    stored.1 = compute(deps);
                }
                &stored.1
            }
            None => panic!(
                "Hook type mismatch at index {}: expected {}. Hooks must be called in the same order every frame.",
                idx,
                std::any::type_name::<(D, T)>()
            ),
        }
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

    // ----------------------------------------------------------------
    // v0.20.0 hooks: keyed state, effects, named focus, key gating
    // ----------------------------------------------------------------

    /// Component-local persistent state keyed by a runtime string.
    ///
    /// Unlike [`use_state_named`](Self::use_state_named), `id` can be a
    /// runtime value such as `format!("row-{i}")`. The key is converted to
    /// `String` once per call, incurring **one allocation per unique key
    /// per frame**.
    ///
    /// # When to use
    /// - Per-item state in a dynamic list where positional [`use_state`]
    ///   would break if items are reordered or filtered.
    /// - Reusable component functions called with a runtime discriminator.
    ///
    /// # Namespace
    /// Keys live in a single global namespace per `Context`. Prefix them
    /// to avoid collisions: `format!("my_component::item-{i}")`.
    ///
    /// # Stale entries
    /// Removed items leak their state until the `Context` is dropped (or
    /// the program exits). For long-running sessions with churn, manage
    /// state externally via a single `Vec<T>` in [`use_state`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// for (i, item) in items.iter().enumerate() {
    ///     let row_state = ui.use_state_keyed(format!("row-{i}"), || ItemState::default());
    ///     // ...
    /// }
    /// ```
    ///
    /// [`use_state`]: Self::use_state
    pub fn use_state_keyed<T: 'static>(
        &mut self,
        id: impl Into<String>,
        init: impl FnOnce() -> T,
    ) -> State<T> {
        let key: String = id.into();
        // `entry().or_insert_with` avoids the double-lookup on the hot
        // (already-populated) path and only invokes `init` on first entry.
        self.keyed_states
            .entry(key.clone())
            .or_insert_with(|| Box::new(init()));
        State::from_keyed(key)
    }

    /// Like [`use_state_keyed`](Self::use_state_keyed), but uses
    /// [`Default::default()`] to initialize the value on first call.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let counter = ui.use_state_keyed_default::<i32>(format!("c-{i}"));
    /// ```
    pub fn use_state_keyed_default<T: Default + 'static>(
        &mut self,
        id: impl Into<String>,
    ) -> State<T> {
        self.use_state_keyed(id, T::default)
    }

    /// Run a side-effecting closure when `deps` changes.
    ///
    /// On the **first frame** the hook slot is encountered, `f` is called
    /// unconditionally. On **subsequent frames**, `f` is only called when
    /// `*deps != stored_deps`. The hook is **positional** (same ordering
    /// rules as [`use_state`](Self::use_state)).
    ///
    /// # Fire-and-forget semantics
    ///
    /// There is no cleanup callback. If setup resources need teardown,
    /// store a handle in [`use_state`](Self::use_state) and drop it on
    /// a later frame.
    ///
    /// # Caveat: `error_boundary` re-fire
    ///
    /// Effects placed inside an [`error_boundary`](Self::error_boundary)
    /// scope can re-fire when the boundary catches a panic and rolls back
    /// the hook slots. For non-idempotent side effects (network requests,
    /// payments) put the effect outside the boundary or guard with an
    /// idempotency key.
    ///
    /// # Common patterns
    ///
    /// ```ignore
    /// // Run once on first frame:
    /// ui.use_effect(|_| initialize_logger(), &());
    ///
    /// // Run when `selected_tab` changes:
    /// ui.use_effect(|tab| load_tab_data(*tab), &selected_tab);
    /// ```
    pub fn use_effect<D: PartialEq + Clone + 'static>(&mut self, f: impl FnOnce(&D), deps: &D) {
        let idx = self.rollback.hook_cursor;
        self.rollback.hook_cursor += 1;

        if idx >= self.hook_states.len() {
            // First encounter: run the effect, then store the deps so we
            // can detect future changes.
            f(deps);
            self.hook_states.push(Box::new(deps.clone()));
            return;
        }

        match self.hook_states[idx].downcast_mut::<D>() {
            Some(stored) => {
                if *stored != *deps {
                    f(deps);
                    *stored = deps.clone();
                }
            }
            None => panic!(
                "Hook type mismatch at index {idx}: expected {}. \
                 Hooks must be called in the same order every frame.",
                std::any::type_name::<D>()
            ),
        }
    }

    /// Register a widget as focusable with a stable string name.
    ///
    /// Returns `true` if this widget currently has focus. Identical to
    /// [`register_focusable`](Self::register_focusable) except it also
    /// records the `name → current_index` mapping so other code can later
    /// call [`focus_by_name`](Self::focus_by_name).
    ///
    /// The name must be unique per frame. Duplicate names in the same
    /// frame are silently ignored (first registration wins).
    ///
    /// Names must be re-registered each frame (the map is rebuilt every
    /// frame), the same model as
    /// [`use_state_named`](Self::use_state_named).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let focused = ui.register_focusable_named("search");
    /// // ... draw search box ...
    /// ```
    pub fn register_focusable_named(&mut self, name: &str) -> bool {
        let focused = self.register_focusable();
        // Use the focus id captured by `register_focusable` (or recompute
        // the most-recently-assigned slot when the modal suppression path
        // skipped recording). This keeps the map aligned with the same
        // index space `Tab` cycling uses.
        if let Some(this_id) = self.rollback.last_focusable_id {
            // First-write-wins so duplicate names in the same frame are
            // silent no-ops (rather than aliasing into the second slot).
            self.focus_name_map
                .entry(name.to_string())
                .or_insert(this_id);
        }
        focused
    }

    /// Request focus on the named widget.
    ///
    /// If the named widget was registered last frame the focus change
    /// takes effect at the **start of the next frame** (one-frame delay
    /// is the deferred-command pattern used throughout SLT). If the name
    /// has never been registered, the request stays pending: the next
    /// frame to register that name receives focus.
    ///
    /// Returns `true` if the name resolved against the most recent frame's
    /// registrations; `false` if the request was queued for later.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if ui.button("Find").clicked {
    ///     ui.focus_by_name("search");
    /// }
    /// ```
    pub fn focus_by_name(&mut self, name: &str) -> bool {
        let resolved = self.focus_name_map_prev.contains_key(name);
        // Always store the request — even if it resolved this frame, the
        // next-frame plumbing (`Context::new`) is what actually applies
        // the index. We use take/replace so the caller cannot stack two
        // pending names; the most recent wins.
        self.pending_focus_name = Some(name.to_string());
        resolved
    }

    /// Return the name of the currently focused widget, if it was
    /// registered with
    /// [`register_focusable_named`](Self::register_focusable_named) this
    /// frame.
    ///
    /// Returns `None` if the focused widget used the unnamed
    /// [`register_focusable`](Self::register_focusable) API or if no widget
    /// has focus.
    pub fn focused_name(&self) -> Option<&str> {
        // Search this frame's map for the entry whose index equals
        // `focus_index`. The map is small (one entry per named focusable),
        // so a linear scan is fine — typical apps register <50 names.
        self.focus_name_map
            .iter()
            .find_map(|(name, &idx)| (idx == self.focus_index).then_some(name.as_str()))
    }

    /// Iterate unconsumed key-press events, gated on `active`.
    ///
    /// When `active` is `false`, returns an empty iterator. When `active`
    /// is `true`, behaves identically to the internal
    /// `available_key_presses`. The returned indices are valid for
    /// [`consume_event`](Self::consume_event).
    ///
    /// This is the **preferred pattern** for focus-gated keyboard handling
    /// in custom widgets. Because the iterator borrows `self.events`
    /// immutably, collect the indices first and consume them after the
    /// loop:
    ///
    /// ```ignore
    /// let focused = ui.register_focusable();
    /// let mut hits: Vec<usize> = Vec::new();
    /// for (i, key) in ui.key_presses_when(focused) {
    ///     if key.code == slt::KeyCode::Enter {
    ///         hits.push(i);
    ///         // ... handle Enter ...
    ///     }
    /// }
    /// for i in hits { ui.consume_event(i); }
    /// ```
    pub fn key_presses_when(
        &self,
        active: bool,
    ) -> impl Iterator<Item = (usize, &crate::event::KeyEvent)> + '_ {
        // The `!active` short-circuit at the head of the predicate yields
        // an empty iterator at zero allocation cost when the widget isn't
        // focused. Indices are still drawn from `self.events` so callers
        // can pass them straight to `consume_event`.
        self.events
            .iter()
            .enumerate()
            .filter_map(move |(i, event)| {
                if !active {
                    return None;
                }
                if self.consumed.get(i).copied().unwrap_or(true) {
                    return None;
                }
                match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => Some((i, key)),
                    _ => None,
                }
            })
    }

    /// Mark the event at `index` as consumed.
    ///
    /// Public counterpart to the crate-internal `consume_indices`. Use
    /// this in custom widgets after handling an event yielded by
    /// [`key_presses_when`](Self::key_presses_when) so subsequent widgets
    /// don't react to the same key. Out-of-range indices are silently
    /// ignored (matching the iterator-pair semantics).
    pub fn consume_event(&mut self, index: usize) {
        if let Some(slot) = self.consumed.get_mut(index) {
            *slot = true;
        }
    }

    // ── Issue #233: in-frame static-log append ───────────────────────────
    //
    // The runtime holds the buffer inside `named_states` under a reserved
    // sentinel key. `Context::new` (owned by another agent) does not need to
    // initialise this field — `or_insert_with` handles first-call creation,
    // and `lib::run_frame_kernel` drains the buffer back into `FrameState`
    // for the run-loop to consume.

    /// Append a line that will be flushed to terminal scrollback **before**
    /// the dynamic frame content (issue #233).
    ///
    /// Lines accumulated this frame are written via the active runtime — for
    /// [`crate::run_static`] / [`crate::run_static_with`], they are printed
    /// above the inline dynamic area as committed scrollback. For full-screen
    /// runtimes ([`crate::run`], [`crate::run_async`]) and inline mode
    /// ([`crate::run_inline`]), the buffer is silently dropped after a debug
    /// warning is emitted on the first call per frame, since those modes have
    /// no scrollback area to write to.
    ///
    /// The headless [`crate::TestBackend`] accumulates the lines into the
    /// frame state where they can be inspected by tests via
    /// [`crate::TestBackend::static_log_lines`] (or by reading the buffer
    /// returned by [`Context::take_static_log_pending`] when constructing a
    /// custom backend).
    ///
    /// # Order
    ///
    /// `static_log` may be called any number of times per frame. Lines are
    /// flushed in call order, all before the dynamic frame for the same
    /// tick.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(40, 4).render(|ui| {
    /// ui.static_log("event 1");
    /// ui.static_log(format!("event {}", 2));
    /// ui.text("dynamic content");
    /// # });
    /// ```
    pub fn static_log(&mut self, line: impl Into<String>) {
        let entry = self
            .named_states
            .entry(STATIC_LOG_KEY)
            .or_insert_with(|| Box::new(Vec::<String>::new()) as Box<dyn std::any::Any>);
        if let Some(buf) = entry.downcast_mut::<Vec<String>>() {
            buf.push(line.into());
        }
    }

    /// Drain and return the queued static-log lines for the current frame
    /// (issue #233). Used by tests / external backends to inspect what
    /// `ui.static_log(...)` emitted during a [`crate::TestBackend::render`]
    /// call.
    pub fn take_static_log(&mut self) -> Vec<String> {
        if let Some(boxed) = self.named_states.get_mut(STATIC_LOG_KEY) {
            if let Some(buf) = boxed.downcast_mut::<Vec<String>>() {
                return std::mem::take(buf);
            }
        }
        Vec::new()
    }

    // ── Issue #236: widget keymap publishing ─────────────────────────────

    /// Publish a widget's keymap so the framework can show it in the help
    /// overlay (issue #236).
    ///
    /// Each call registers `(name, bindings)` for the current frame. Widgets
    /// implementing [`crate::keymap::WidgetKeyHelp`] typically forward their
    /// `key_help()` slice here:
    ///
    /// ```
    /// # use slt::*;
    /// # use slt::keymap::WidgetKeyHelp;
    /// struct Counter;
    /// impl WidgetKeyHelp for Counter {
    ///     fn key_help(&self) -> &'static [(&'static str, &'static str)] {
    ///         const HELP: &[(&str, &str)] = &[("↑", "increment"), ("↓", "decrement")];
    ///         HELP
    ///     }
    /// }
    /// # TestBackend::new(40, 4).render(|ui| {
    /// let counter = Counter;
    /// ui.publish_keymap("counter", counter.key_help());
    /// # });
    /// ```
    ///
    /// The registry is reset at the start of every frame (the first call on a
    /// new tick clears stale entries). Both calls in the same frame
    /// accumulate; calls across frames do not leak.
    pub fn publish_keymap(
        &mut self,
        name: &'static str,
        bindings: &'static [(&'static str, &'static str)],
    ) {
        // The registry is cleared at frame start by `run_frame_kernel`
        // (issue #236) — see `clear_keymap_registry` in `lib.rs`. We just
        // need to insert/append here.
        let entry = self
            .named_states
            .entry(KEYMAP_REGISTRY_KEY)
            .or_insert_with(|| {
                Box::new(Vec::<crate::keymap::PublishedKeymap>::new()) as Box<dyn std::any::Any>
            });
        if let Some(vec) = entry.downcast_mut::<Vec<crate::keymap::PublishedKeymap>>() {
            vec.push(crate::keymap::PublishedKeymap::new(name, bindings));
        }
    }

    /// Return all keymaps published this frame (issue #236).
    ///
    /// Empty if no widget called [`Context::publish_keymap`] yet on the
    /// current frame. The registry is reset at the start of every frame.
    pub fn published_keymaps(&self) -> &[crate::keymap::PublishedKeymap] {
        if let Some(boxed) = self.named_states.get(KEYMAP_REGISTRY_KEY) {
            if let Some(vec) = boxed.downcast_ref::<Vec<crate::keymap::PublishedKeymap>>() {
                return vec;
            }
        }
        &[]
    }

    /// Render an automatic keymap-help overlay listing every widget keymap
    /// published this frame (issue #236).
    ///
    /// Pass `open = true` to render the overlay (typically gated on a
    /// `?` / `F1` keypress). When `open` is `false`, this method is a
    /// no-op. The overlay groups bindings by widget name and dismisses
    /// when the next frame is rendered with `open = false`.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::*;
    /// # TestBackend::new(40, 12).render(|ui| {
    /// const RICHLOG: &[(&str, &str)] = &[("↑/k", "scroll up"), ("↓/j", "scroll down")];
    /// ui.publish_keymap("rich_log", RICHLOG);
    /// // Show the help overlay when '?' is pressed
    /// let show = ui.key('?');
    /// ui.keymap_help_overlay(show);
    /// # });
    /// ```
    pub fn keymap_help_overlay(&mut self, open: bool) {
        if !open {
            return;
        }

        let entries: Vec<crate::keymap::PublishedKeymap> = self.published_keymaps().to_vec();
        if entries.is_empty() {
            return;
        }

        let theme = self.theme;
        let _ = self.modal(|ui| {
            ui.styled("Keyboard shortcuts", Style::new().bold().fg(theme.primary));
            ui.text("");
            for entry in &entries {
                ui.styled(entry.name, Style::new().bold().fg(theme.text));
                for (key, desc) in entry.bindings {
                    let line = format!("  {key:<14}  {desc}");
                    ui.styled(line, Style::new().fg(theme.text_dim));
                }
                ui.text("");
            }
            ui.styled(
                "Press Esc / ? to close",
                Style::new().fg(theme.text_dim).italic(),
            );
        });
    }
}

// Sentinel keys reused from `lib.rs` so the two reads/writes can never drift.
use crate::{
    KEYMAP_REGISTRY_NAMED_STATE_KEY as KEYMAP_REGISTRY_KEY,
    STATIC_LOG_NAMED_STATE_KEY as STATIC_LOG_KEY,
};
