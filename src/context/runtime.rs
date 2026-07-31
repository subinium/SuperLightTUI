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
        // Issue #262: hand off the partial-chord buffer for this frame. Same
        // lifetime as `keyed_states`: moved out at frame start, moved back at
        // frame end (see `run_frame_kernel`).
        let chord = std::mem::take(&mut state.chord_states);
        // Issue #248: hand off the scheduler timer table for this frame. Same
        // lifetime as `named_states`: moved out at frame start, moved back at
        // frame end (where untouched slots are GC'd; see `run_frame_kernel`).
        let scheduler = std::mem::take(&mut state.scheduler);
        // Issue #234: hand off the async task registry for this frame. Same
        // lifetime as `scheduler`: moved out at frame start, moved back at
        // frame end (see `run_frame_kernel`).
        #[cfg(feature = "async")]
        let async_tasks = std::mem::take(&mut state.async_tasks);
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

        // Single wall-clock sample for this frame, reused for double-click
        // timing below and for `frame_instant` (the timer/scheduler clock).
        let frame_now = std::time::Instant::now();
        let mut mouse_pos = layout_feedback.last_mouse_pos;
        let mut click_pos = None;
        let mut right_click_pos = None;
        let mut double_click_pos = None;
        let mut scroll_pos = None;
        let mut scroll_delta_frame: i32 = 0;
        for event in &events {
            if let Event::Mouse(mouse) = event {
                mouse_pos = Some((mouse.x, mouse.y));
                match mouse.kind {
                    MouseKind::Down(MouseButton::Left) => {
                        click_pos = Some((mouse.x, mouse.y));
                        // v0.21.1: a left click on the same cell as the previous
                        // click, within `DOUBLE_CLICK_WINDOW`, is a double-click.
                        // Clear the tracker after firing so a third click starts
                        // a fresh pair (no triple-counting).
                        let pos = (mouse.x, mouse.y);
                        let is_double = layout_feedback.last_click_pos == Some(pos)
                            && layout_feedback.last_click_at.is_some_and(|t| {
                                frame_now.duration_since(t) <= crate::DOUBLE_CLICK_WINDOW
                            });
                        if is_double {
                            double_click_pos = Some(pos);
                            layout_feedback.last_click_at = None;
                            layout_feedback.last_click_pos = None;
                        } else {
                            layout_feedback.last_click_at = Some(frame_now);
                            layout_feedback.last_click_pos = Some(pos);
                        }
                    }
                    MouseKind::Down(MouseButton::Right) => {
                        // Issue #208: capture last right-click position so
                        // `response_for` can hit-test against per-widget rects.
                        right_click_pos = Some((mouse.x, mouse.y));
                    }
                    // v0.21.1: accumulate net vertical wheel delta + the cursor
                    // position, hover-gated per-widget by `response_for`.
                    MouseKind::ScrollUp => {
                        scroll_pos = Some((mouse.x, mouse.y));
                        scroll_delta_frame = scroll_delta_frame.saturating_add(1);
                    }
                    MouseKind::ScrollDown => {
                        scroll_pos = Some((mouse.x, mouse.y));
                        scroll_delta_frame = scroll_delta_frame.saturating_sub(1);
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
                    if best.is_none_or(|(_, ba)| area < ba) {
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
        // clear (no-op when reclaimed from a `build_tree` drain, defensive
        // when reclaimed from the quit path that ran without `build_tree`)
        // and reuse the allocation. After `build_tree(&mut ctx.commands)`
        // drains the Vec in place, the empty (but capacity-bearing) Vec is
        // moved back into `state.commands_buf` at frame end inside
        // `run_frame_kernel`.
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

        // Issue #273: hand off the previous frame's `cached` region keys and a
        // recycled (cleared) buffer to record this frame's keys into. Both
        // round-trip back into `FrameState` at frame end. Empty (zero
        // overhead) for apps that never call `cached`.
        let region_versions_prev = std::mem::take(&mut state.region_versions);
        let mut region_versions_cur = std::mem::take(&mut state.region_versions_buf);
        region_versions_cur.clear();

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
            chord,
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
            double_click_pos,
            scroll_pos,
            scroll_delta_frame,
            prev_modal_active: focus.prev_modal_active,
            clipboard_text: None,
            debug: diagnostics.debug_mode,
            debug_layer: diagnostics.debug_layer,
            inspector_mode: diagnostics.inspector_mode,
            theme,
            is_real_terminal: false,
            // Issue #264: conservative default; overwritten by the probed
            // snapshot in `run_frame_kernel` on a real terminal.
            #[cfg(feature = "crossterm")]
            capabilities: crate::terminal::Capabilities::default(),
            deferred_draws,
            rollback: ContextRollbackState {
                last_text_idx: None,
                focus_count: 0,
                last_focusable_id: None,
                pending_focusable_id: None,
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
            pending_screen_nav: Vec::new(),
            screen_nav_depth: 0,
            screen_nav_render_origins: std::collections::HashMap::new(),
            hovered_groups,
            region_versions_prev,
            region_versions_cur,
            region_cache_hits: 0,
            region_cache_misses: 0,
            scroll_lines_per_event: 1,
            screen_hook_map,
            widget_theme: WidgetTheme::new(),
            prev_focus_index,
            focus_name_map_prev,
            focus_name_map: std::collections::HashMap::new(),
            pending_focus_name: still_pending,
            // Issue #248: sample a single wall-clock "now" for every timer
            // method called this frame. v0.21.1: reuse the `frame_now` sampled
            // above (also used for double-click timing) so the frame has one
            // coherent clock reading.
            frame_instant: frame_now,
            scheduler,
            // Issue #234: async task registry round-tripped like `scheduler`.
            #[cfg(feature = "async")]
            async_tasks,
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

    /// Advance keyboard focus one step, honoring an active modal's focus trap.
    /// `forward` selects next vs previous; both wrap. Shared by
    /// [`focus_next`](Self::focus_next) / [`focus_prev`](Self::focus_prev) and
    /// the `Tab`/`Shift+Tab` handler in `process_focus_keys` (v0.21.1).
    pub(crate) fn advance_focus(&mut self, forward: bool) {
        if self.prev_modal_active && self.prev_modal_focus_count > 0 {
            let mut modal_local = self.focus_index.saturating_sub(self.prev_modal_focus_start);
            modal_local %= self.prev_modal_focus_count;
            let next = if forward {
                (modal_local + 1) % self.prev_modal_focus_count
            } else if modal_local == 0 {
                self.prev_modal_focus_count - 1
            } else {
                modal_local - 1
            };
            self.focus_index = self.prev_modal_focus_start + next;
        } else if self.prev_focus_count > 0 {
            self.focus_index = if forward {
                (self.focus_index + 1) % self.prev_focus_count
            } else if self.focus_index == 0 {
                self.prev_focus_count - 1
            } else {
                self.focus_index - 1
            };
        }
    }

    /// Move keyboard focus to the next focusable widget (wrapping), exactly as
    /// pressing `Tab` would. Honors an active modal's focus trap. Pairs with
    /// [`set_focus_index`](Self::set_focus_index) / [`focus_count`](Self::focus_count)
    /// for programmatic focus control (e.g. an app-level shortcut). Available
    /// since v0.21.1.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// // Advance focus on a custom shortcut (e.g. a vim-style 'j').
    /// if ui.key('j') {
    ///     ui.focus_next();
    /// }
    /// # });
    /// ```
    pub fn focus_next(&mut self) {
        self.advance_focus(true);
    }

    /// Move keyboard focus to the previous focusable widget (wrapping), exactly
    /// as `Shift+Tab` would. Honors an active modal's focus trap. Available
    /// since v0.21.1.
    pub fn focus_prev(&mut self) {
        self.advance_focus(false);
    }

    /// Move focus to the next focusable widget belonging to the named focus
    /// group, wrapping within the group. If focus is currently outside the
    /// group it jumps to the group's first member. No-op if the group had no
    /// focusable widgets on the previous frame.
    ///
    /// Focus groups are declared with [`group`](Self::group); this is the
    /// scoped counterpart to [`focus_next`](Self::focus_next) for building a
    /// focus trap around a panel or sub-form without a modal. Available since
    /// v0.21.1.
    pub fn focus_next_in_group(&mut self, group: &str) {
        self.advance_focus_in_group(group, true);
    }

    /// Move focus to the previous focusable widget in the named group
    /// (wrapping). See [`focus_next_in_group`](Self::focus_next_in_group).
    /// Available since v0.21.1.
    pub fn focus_prev_in_group(&mut self, group: &str) {
        self.advance_focus_in_group(group, false);
    }

    fn advance_focus_in_group(&mut self, group: &str, forward: bool) {
        // Membership comes from the previous frame's `index -> group` table,
        // the same source `is_group_focused` consults. Indices are valid
        // focus indices (0..prev_focus_count).
        let members: Vec<usize> = self
            .prev_focus_groups
            .iter()
            .enumerate()
            .filter_map(|(idx, g)| match g.as_deref() {
                Some(name) if name == group => Some(idx),
                _ => None,
            })
            .collect();
        if members.is_empty() {
            return;
        }
        let new_pos = match members.iter().position(|&m| m == self.focus_index) {
            Some(p) => {
                if forward {
                    (p + 1) % members.len()
                } else if p == 0 {
                    members.len() - 1
                } else {
                    p - 1
                }
            }
            // Focus is outside the group: jump to its first member.
            None => 0,
        };
        self.focus_index = members[new_pos];
    }

    /// Read-only snapshot of the terminal's negotiated capabilities
    /// (issue #264).
    ///
    /// Populated once at session enter via a DA1/DA2/XTGETTCAP probe. This is
    /// **diagnostics-only**: image rendering already routes through the
    /// automatic blitter ladder (Kitty > Sixel > sextant > half-block), so app
    /// code is never required to branch on the returned value. On a headless
    /// backend (e.g. [`TestBackend`](crate::TestBackend)) or piped stdout, the
    /// probe is skipped and every field is a conservative default.
    ///
    /// Available since `0.21.0`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let caps = ui.capabilities();
    /// // e.g. surface a "truecolor: on" line in a diagnostics panel.
    /// let _ = caps.truecolor;
    /// # });
    /// ```
    #[cfg(feature = "crossterm")]
    #[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
    pub fn capabilities(&self) -> &crate::terminal::Capabilities {
        &self.capabilities
    }

    pub(crate) fn process_focus_keys(&mut self) {
        // Scan for Tab / Shift+Tab / BackTab, recording the direction of each
        // and consuming the event. The mutation (`advance_focus`) is applied
        // after the scan: it borrows `&mut self` wholesale, which cannot run
        // while `self.events` is iterated by reference. Collecting first
        // preserves the original "each Tab advances once" semantics.
        let mut actions: Vec<bool> = Vec::new();
        for (i, event) in self.events.iter().enumerate() {
            if self.consumed[i] {
                continue;
            }
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.code == KeyCode::Tab && !key.modifiers.contains(KeyModifiers::SHIFT) {
                    actions.push(true);
                    self.consumed[i] = true;
                } else if (key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT))
                    || key.code == KeyCode::BackTab
                {
                    actions.push(false);
                    self.consumed[i] = true;
                }
            }
        }
        for forward in actions {
            self.advance_focus(forward);
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

    /// Issue #273: record a [`ContainerBuilder::cached`] region's version key
    /// at its (declaration-ordered) call site and classify it as a hit or
    /// miss versus the previous frame.
    ///
    /// Returns `true` if `version_key` matches the value this call site
    /// recorded last frame (a hit), `false` on a key change, a brand-new slot,
    /// the first frame, or after a resize (all misses).
    ///
    /// This is purely an *author-declared stability signal*: the caller still
    /// re-runs its closure every frame, so output stays byte-identical and the
    /// immediate-mode invariant is preserved exactly. The hit/miss result is
    /// recorded for diagnostics ([`Context::region_cache_hits`] /
    /// [`Context::region_cache_misses`]) and to give a future cell-level cache
    /// a sound, principle-preserving gate. See the type-level docs on
    /// [`ContainerBuilder::cached`] for the full design rationale.
    pub(crate) fn record_cached_region(&mut self, version_key: u64) -> bool {
        let idx = self.region_versions_cur.len();
        let hit = self
            .region_versions_prev
            .get(idx)
            .is_some_and(|&prev| prev == version_key);
        self.region_versions_cur.push(version_key);
        if hit {
            self.region_cache_hits = self.region_cache_hits.saturating_add(1);
        } else {
            self.region_cache_misses = self.region_cache_misses.saturating_add(1);
        }
        hit
    }

    /// Number of [`ContainerBuilder::cached`] regions this frame whose version
    /// key was unchanged from the previous frame (cache hits).
    ///
    /// Diagnostics for the opt-in streaming cache (issue #273). A region is a
    /// hit when its author-supplied `version_key` matches the value the same
    /// call site recorded last frame; it misses on a key change, a new call
    /// site, the first frame, or after a terminal resize.
    ///
    /// Since 0.21.0.
    ///
    /// # Example
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.container().cached(42, |ui| {
    ///     ui.text("stable chrome");
    /// });
    /// let _hits = ui.region_cache_hits();
    /// # });
    /// ```
    pub fn region_cache_hits(&self) -> u32 {
        self.region_cache_hits
    }

    /// Number of [`ContainerBuilder::cached`] regions this frame whose version
    /// key changed (or was new / first-frame / post-resize) — cache misses.
    ///
    /// The counterpart to [`Context::region_cache_hits`]. See issue #273.
    ///
    /// Since 0.21.0.
    ///
    /// # Example
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.container().cached(7, |ui| {
    ///     ui.text("chrome");
    /// });
    /// let _misses = ui.region_cache_misses();
    /// # });
    /// ```
    pub fn region_cache_misses(&self) -> u32 {
        self.region_cache_misses
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

    /// Compute and consume the `(gained_focus, lost_focus)` edge flags for the
    /// widget most recently registered via [`register_focusable`].
    ///
    /// If that focusable lined up with the previously-focused widget index from
    /// the prior frame, the focus change since maps directly to gained/lost.
    /// Takes (consumes) the `last_focusable_id` marker so a single
    /// `register_focusable` powers exactly one transition computation.
    ///
    /// Shared by [`begin_widget_interaction`](Self::begin_widget_interaction)
    /// and the widgets that assemble their `Response` by hand rather than
    /// through it (`text_input`, `slider`, `number_input`) — issue #208 left
    /// those three reporting `gained_focus`/`lost_focus` as always-false; this
    /// closes that gap (v0.21.1).
    pub(crate) fn focus_transitions(&mut self, focused: bool) -> (bool, bool) {
        if let Some(this_id) = self.rollback.last_focusable_id.take() {
            let was_focused = self
                .prev_focus_index
                .map(|prev| prev == this_id)
                .unwrap_or(false);
            (focused && !was_focused, !focused && was_focused)
        } else {
            (false, false)
        }
    }

    pub(crate) fn begin_widget_interaction(&mut self, focused: bool) -> (usize, Response) {
        let interaction_id = self.next_interaction_id();
        let mut response = self.response_for(interaction_id);
        response.focused = focused;
        let (gained, lost) = self.focus_transitions(focused);
        response.gained_focus = gained;
        response.lost_focus = lost;
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
    ///
    /// # Slot reservation by `register_focusable_named`
    ///
    /// If [`register_focusable_named`](Self::register_focusable_named) was
    /// called immediately before this call, it has already allocated a
    /// slot and bound a name to it; this call **reuses** that slot
    /// instead of allocating a fresh one. That keeps the name binding
    /// pointed at the widget the user sees rather than at a dummy slot.
    pub fn register_focusable(&mut self) -> bool {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            self.rollback.last_focusable_id = None;
            // Drop any pending reservation: the suppressed widget never
            // attached, so reusing the reserved id from a later widget in
            // the same frame would silently rebind the name to the wrong
            // slot.
            self.rollback.pending_focusable_id = None;
            return false;
        }
        // Issue #217 follow-up: if `register_focusable_named` reserved a
        // slot for us, reuse it (and skip the FocusMarker push — it was
        // already emitted when the reservation was made). Otherwise,
        // allocate a fresh slot the normal way.
        let (id, freshly_allocated) =
            if let Some(reserved) = self.rollback.pending_focusable_id.take() {
                (reserved, false)
            } else {
                let id = self.rollback.focus_count;
                self.rollback.focus_count += 1;
                (id, true)
            };
        // Issue #208: remember this widget's focus id so the immediately
        // following `begin_widget_interaction` call can compare against
        // `prev_focus_index` and emit gained/lost focus signals.
        self.rollback.last_focusable_id = Some(id);
        if freshly_allocated {
            self.commands.push(Command::FocusMarker(id));
        }
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
    /// # Naming
    ///
    /// The no-suffix form takes an `init` closure, matching
    /// [`use_state`](Self::use_state)`(init)` and
    /// [`use_state_keyed`](Self::use_state_keyed)`(id, init)`. Use
    /// [`use_state_named_default`](Self::use_state_named_default) for the
    /// `T: Default` shorthand.
    ///
    /// # Example
    ///
    /// ```no_run
    /// fn counter(ui: &mut slt::Context) {
    ///     let count = ui.use_state_named("counter::value", || 0i32);
    ///     ui.text(format!("Count: {}", count.get(ui)));
    ///     if ui.button("+1").clicked {
    ///         *count.get_mut(ui) += 1;
    ///     }
    /// }
    /// ```
    pub fn use_state_named<T: 'static>(
        &mut self,
        id: &'static str,
        init: impl FnOnce() -> T,
    ) -> State<T> {
        self.named_states
            .entry(id)
            .or_insert_with(|| Box::new(init()));
        State::from_named(id)
    }

    /// Like [`use_state_named`](Self::use_state_named), but uses
    /// [`Default::default()`] to initialize the value on first call.
    ///
    /// Mirrors [`use_state_keyed_default`](Self::use_state_keyed_default): the
    /// `_default` suffix means "no init closure, `T: Default` required".
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let value = ui.use_state_named_default::<i32>("counter::value");
    /// ui.text(format!("{}", value.get(ui)));
    /// # });
    /// ```
    pub fn use_state_named_default<T: 'static + Default>(&mut self, id: &'static str) -> State<T> {
        self.use_state_named(id, T::default)
    }

    /// Deprecated alias for [`use_state_named`](Self::use_state_named).
    ///
    /// **Deprecated since 0.21.0**: the `_named` family now follows the
    /// "no-suffix = init closure" convention so it matches
    /// [`use_state`](Self::use_state) and
    /// [`use_state_keyed`](Self::use_state_keyed). The init-closure form is now
    /// spelled `use_state_named(id, init)`; the `T: Default` shorthand is
    /// [`use_state_named_default`](Self::use_state_named_default).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// // Old: ui.use_state_named_with("counter::value", || 0i32)
    /// let count = ui.use_state_named("counter::value", || 0i32);
    /// ui.text(format!("{}", count.get(ui)));
    /// # });
    /// ```
    #[deprecated(
        since = "0.21.0",
        note = "Renamed to `use_state_named` — the no-suffix form now takes the init closure, matching `use_state` / `use_state_keyed`."
    )]
    pub fn use_state_named_with<T: 'static>(
        &mut self,
        id: &'static str,
        init: impl FnOnce() -> T,
    ) -> State<T> {
        self.use_state_named(id, init)
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
    /// State is stored in the per-context named-state map under `id`. The
    /// id is `&'static str` (single global namespace per context), matching
    /// [`Context::use_state_named`]. Pick a unique key per call site — two
    /// `animate_bool` calls with the same id share state.
    ///
    /// On the first call, the value snaps to the target with no visible
    /// transition (so widgets that mount in their final state don't pop).
    ///
    /// # Example
    /// ```ignore
    /// let opacity = ui.animate_bool("sidebar::visible", is_open);
    /// // 0.0 ≤ opacity ≤ 1.0; use as alpha or visibility threshold.
    /// ```
    ///
    /// # See also
    ///
    /// - [`animate_value`](Self::animate_value) — the underlying primitive this
    ///   delegates to; use it for a custom duration or a non-binary target.
    /// - [`Tween`](crate::Tween) — full control over easing and lifecycle.
    pub fn animate_bool(&mut self, id: &'static str, value: bool) -> f64 {
        let target = if value { 1.0 } else { 0.0 };
        self.animate_value(id, target, crate::anim::DEFAULT_ANIMATE_TICKS)
    }

    /// Smoothly animate a `f64` value toward `target` over `duration_ticks`.
    ///
    /// Uses a linear-easing [`crate::Tween`] stored implicitly in the
    /// per-context named-state map under `id`. Returns the current
    /// interpolated value. On the first call the value snaps to `target`
    /// with no visible transition; on subsequent calls when `target`
    /// changes the tween is rebuilt starting from the current interpolated
    /// value, so retargeting mid-flight does not produce a jump.
    ///
    /// `duration_ticks == 0` snaps immediately to the new target.
    ///
    /// # Panics
    ///
    /// Panics if `id` is already bound in the named-state map to a value of a
    /// different type (e.g. a [`use_state_named`](Self::use_state_named) call
    /// reused the same id), since the stored entry then fails to downcast to
    /// the internal animation state:
    ///
    /// ```text
    /// animate_value: id {id} is already used for a different state type
    /// ```
    ///
    /// Pick a unique id per call site to avoid the collision.
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
    /// non-tick-based control, construct a [`crate::Tween`] explicitly via
    /// [`Context::use_state_named`](Self::use_state_named).
    ///
    /// # See also
    ///
    /// - [`animate_bool`](Self::animate_bool) — boolean-driven shorthand that
    ///   tweens between `0.0` and `1.0`.
    /// - [`Tween`](crate::Tween) — explicit easing and lifecycle control.
    pub fn animate_value(&mut self, id: &'static str, target: f64, duration_ticks: u64) -> f64 {
        let tick = self.tick;
        let entry = self
            .named_states
            .entry(id)
            .or_insert_with(|| Box::new(crate::anim::AnimState::new(target, tick)));
        let state = entry
            .downcast_mut::<crate::anim::AnimState>()
            .unwrap_or_else(|| {
                panic!("animate_value: id {id:?} is already used for a different state type")
            });
        state.sample(target, duration_ticks, tick)
    }

    /// One-shot frame-clock timer (issue #248).
    ///
    /// Returns `true` exactly once — on the first frame at or after `dur` has
    /// elapsed since the first `schedule` call for `id` — and `false` on every
    /// other frame, both before and after. Re-arm by calling
    /// [`cancel`](Self::cancel) and then `schedule` again.
    ///
    /// Wall-clock based ([`std::time::Instant`] sampled once at frame start),
    /// so it works with the default feature set and without the `async`
    /// feature. Precision is bounded by the run loop's `tick_rate` (the
    /// deadline is observed on the next frame after it elapses), so durations
    /// well below the frame cadence are not meaningful.
    ///
    /// The id lives in the same per-context namespace as
    /// [`use_state_named`](Self::use_state_named): pick a unique key per call
    /// site.
    ///
    /// # Example
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// slt::run(|ui: &mut slt::Context| {
    ///     if ui.schedule("splash::dismiss", Duration::from_millis(800)) {
    ///         // Runs once, ~800ms after the first frame that called this.
    ///         ui.text("Splash dismissed.");
    ///     }
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn schedule(&mut self, id: &'static str, dur: std::time::Duration) -> bool {
        let now = self.frame_instant;
        let slot = self
            .scheduler
            .named
            .entry(id)
            .or_insert_with(|| SchedulerSlot {
                started: now,
                kind: SchedKind::Once { dur, fired: false },
                touched_this_frame: false,
            });
        slot.touched_this_frame = true;
        let elapsed = now.saturating_duration_since(slot.started);
        match &mut slot.kind {
            SchedKind::Once { dur, fired } if !*fired && elapsed >= *dur => {
                *fired = true;
                true
            }
            // Not yet due, already fired, or a re-used id bound to a different
            // timer kind: do not fire (a typo can't crash the app).
            _ => false,
        }
    }

    /// Recurring frame-clock timer (issue #248).
    ///
    /// Returns the number of whole `dur` intervals that elapsed since the
    /// previous frame this `id` was sampled: `0` on most frames, `1` typically,
    /// and `> 1` if the frame loop stalled past several intervals — so no ticks
    /// are silently dropped. The internal clock advances by exactly the
    /// returned number of intervals each frame, so counts never drift.
    ///
    /// Wall-clock based and `async`-free, like [`schedule`](Self::schedule).
    ///
    /// # Example
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// slt::run(|ui: &mut slt::Context| {
    ///     let ticks = ui.every("clock::second", Duration::from_secs(1));
    ///     if ticks > 0 {
    ///         // Advance a once-per-second animation by `ticks` steps.
    ///     }
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn every(&mut self, id: &'static str, dur: std::time::Duration) -> u32 {
        let now = self.frame_instant;
        let interval = dur.max(std::time::Duration::from_nanos(1));
        let slot = self
            .scheduler
            .named
            .entry(id)
            .or_insert_with(|| SchedulerSlot {
                started: now,
                kind: SchedKind::Every {
                    interval,
                    last: now,
                },
                touched_this_frame: false,
            });
        slot.touched_this_frame = true;
        match &mut slot.kind {
            SchedKind::Every { interval, last } => {
                let elapsed = now.saturating_duration_since(*last);
                let fired = crate::widgets::intervals_elapsed(elapsed, *interval);
                if fired > 0 {
                    // Advance by exactly the intervals reported so counts never
                    // drift, even across stalled frames.
                    let advance = interval.saturating_mul(fired);
                    *last = last.checked_add(advance).unwrap_or(now);
                }
                fired
            }
            _ => 0,
        }
    }

    /// Debounce timer — the typeahead / search-as-you-type primitive (#248).
    ///
    /// Each frame where `dirty == true` resets the quiet window to `dur`.
    /// Returns `true` exactly once on the first frame after `dur` of quiet (no
    /// `dirty`), then stays `false` until the next dirty frame re-arms it. This
    /// mirrors Textual's `@work(exclusive=True)` debounce: collapse a burst of
    /// keystrokes so only the final, settled query runs.
    ///
    /// Wall-clock based and `async`-free, like [`schedule`](Self::schedule).
    ///
    /// # Example
    /// ```no_run
    /// use std::time::Duration;
    /// use slt::TextInputState;
    ///
    /// let mut query = TextInputState::with_placeholder("Search...");
    /// slt::run(move |ui: &mut slt::Context| {
    ///     // `resp.changed` is true on the keystroke frame -> the dirty signal.
    ///     let resp = ui.text_input(&mut query);
    ///     // Fire the search only after 250ms of no typing.
    ///     if ui.debounce("search::run", Duration::from_millis(250), resp.changed) {
    ///         // run_search(&query.value());
    ///     }
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn debounce(&mut self, id: &'static str, dur: std::time::Duration, dirty: bool) -> bool {
        let now = self.frame_instant;
        let slot = self
            .scheduler
            .named
            .entry(id)
            .or_insert_with(|| SchedulerSlot {
                started: now,
                kind: SchedKind::Debounce {
                    dur,
                    quiet_started: now,
                    fired: false,
                },
                touched_this_frame: false,
            });
        slot.touched_this_frame = true;
        match &mut slot.kind {
            SchedKind::Debounce {
                dur: slot_dur,
                quiet_started,
                fired,
            } => {
                *slot_dur = dur;
                if dirty {
                    // Re-arm the quiet window from this frame.
                    *quiet_started = now;
                    *fired = false;
                    false
                } else if !*fired && now.saturating_duration_since(*quiet_started) >= *slot_dur {
                    *fired = true;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Exclusive-group claim — cancel stale work on supersede (issue #248).
    ///
    /// Within a `group`, only the most-recently-claimed `id` returns `true`;
    /// once a newer `id` claims the group, every prior `id` returns `false`
    /// from then on. Use it to cancel an in-flight typeahead query when a newer
    /// query supersedes it: pair with [`debounce`](Self::debounce) to fire the
    /// settled query, then guard the work with `exclusive` so only the latest
    /// claim proceeds.
    ///
    /// # Example
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// slt::run(|ui: &mut slt::Context| {
    ///     let query_id = "q-42"; // e.g. a per-keystroke sequence id
    ///     if ui.exclusive("search", query_id) {
    ///         // Only the latest claimed query runs; older ones are cancelled.
    ///     }
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn exclusive(&mut self, group: &'static str, id: &str) -> bool {
        let entry = self
            .scheduler
            .exclusive
            .entry(group.to_string())
            .or_default();
        if entry.winner == id {
            // The reigning claim re-polls itself: still the winner.
            return true;
        }
        if entry.retired.contains(id) {
            // A previously-superseded id can never win again: stale work stays
            // cancelled even if re-polled.
            return false;
        }
        // A new id supersedes the group: retire the old winner (if any) and
        // become the active claim.
        if !entry.winner.is_empty() {
            let old = std::mem::take(&mut entry.winner);
            entry.retired.insert(old);
        }
        entry.winner = id.to_string();
        true
    }

    /// Drop the scheduler slot for `id`, re-arming it on the next
    /// [`schedule`](Self::schedule) / [`every`](Self::every) /
    /// [`debounce`](Self::debounce) call (issue #248).
    ///
    /// Accepts both `&'static str` and runtime-`String` ids: clears the slot
    /// from the named map and the dynamic-id map.
    ///
    /// # Example
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// slt::run(|ui: &mut slt::Context| {
    ///     if ui.schedule("retry", Duration::from_secs(5)) {
    ///         // ...
    ///     }
    ///     if ui.key('r') {
    ///         ui.cancel("retry"); // next `schedule("retry", ..)` starts fresh
    ///     }
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn cancel(&mut self, id: &str) {
        self.scheduler.named.remove(id);
        self.scheduler.keyed.remove(id);
    }

    /// Wall-clock time elapsed since `id` was first scheduled, or `None` if no
    /// live timer slot exists for `id` (issue #248).
    ///
    /// Useful for progress UIs ("retrying in 3s…") that want the raw elapsed
    /// duration rather than a fire/no-fire signal. Measured against the same
    /// frame instant the timer methods use.
    ///
    /// # Example
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// slt::run(|ui: &mut slt::Context| {
    ///     ui.schedule("upload", Duration::from_secs(30));
    ///     if let Some(elapsed) = ui.elapsed("upload") {
    ///         ui.text(format!("Uploading for {}s", elapsed.as_secs()));
    ///     }
    /// })?;
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn elapsed(&self, id: &str) -> Option<std::time::Duration> {
        let started = self
            .scheduler
            .named
            .get(id)
            .or_else(|| self.scheduler.keyed.get(id))
            .map(|slot| slot.started)?;
        Some(self.frame_instant.saturating_duration_since(started))
    }

    /// Remove dynamic keyed state created by
    /// [`use_state_keyed`](Self::use_state_keyed).
    ///
    /// Returns `true` when a slot existed. Any old [`State`] handle for the
    /// removed id becomes invalid and will panic if used before the state is
    /// recreated by `use_state_keyed`.
    pub fn remove_state_keyed(&mut self, id: &str) -> bool {
        self.keyed_states.remove(id).is_some()
    }

    /// Retain only dynamic keyed-state entries accepted by `keep`.
    ///
    /// Returns the number of removed entries. This is intended for long-lived
    /// dynamic lists where ids come from data and removed items should release
    /// their per-row state.
    pub fn retain_state_keyed(&mut self, mut keep: impl FnMut(&str) -> bool) -> usize {
        let before = self.keyed_states.len();
        self.keyed_states.retain(|key, _| keep(key));
        before - self.keyed_states.len()
    }

    /// Number of live dynamic keyed-state entries.
    ///
    /// Diagnostic helper for spotting churn when using runtime ids.
    pub fn keyed_state_count(&self) -> usize {
        self.keyed_states.len()
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

    /// Spawn a fire-and-forget async task from inside the frame closure.
    ///
    /// Returns a [`TaskHandle<T>`](crate::TaskHandle) you store and pass to
    /// [`poll`](Self::poll) on later frames to retrieve the result. This closes
    /// the ergonomics gap of the channel pattern (`run_async` + an external
    /// `Sender`) for the common case: "click a button, kick off one async call,
    /// show its result next frame" — without wiring a channel yourself.
    ///
    /// **Dropping the returned handle cancels the in-flight task.** Keep it
    /// alive (e.g. in `use_state`) for as long as you care about the result.
    /// Each handle carries a unique id, so two `TaskHandle<String>` live at the
    /// same time never cross their results.
    ///
    /// Requires the `async` feature and an active Tokio runtime — call it
    /// inside [`run_async`](crate::run_async) /
    /// [`run_async_with`](crate::run_async_with), which inject the runtime
    /// handle.
    ///
    /// # Panics
    ///
    /// Panics if no Tokio runtime was injected (e.g. when called from the sync
    /// [`run`](crate::run) loop or `TestBackend` without a runtime).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # async fn run() -> std::io::Result<()> {
    /// use slt::{Context, RunConfig, TaskHandle};
    ///
    /// async fn fetch() -> String {
    ///     // e.g. an HTTP request
    ///     "result".to_string()
    /// }
    ///
    /// slt::run_async_with(RunConfig::default(), |ui: &mut Context, _: &mut Vec<()>| {
    ///     // One handle, stored across frames via `use_state`.
    ///     let handle = ui.use_state(|| None::<TaskHandle<String>>);
    ///
    ///     if ui.button("Fetch").clicked && handle.get(ui).is_none() {
    ///         *handle.get_mut(ui) = Some(ui.spawn(async { fetch().await }));
    ///     }
    ///
    ///     // Take the handle out of state to poll it: `ui.poll` needs `&mut ui`,
    ///     // which cannot coexist with a `&TaskHandle` borrowed from `ui`'s own
    ///     // state. Put it back if the task is still pending.
    ///     if let Some(h) = handle.get_mut(ui).take() {
    ///         match ui.poll(&h) {
    ///             Some(result) => {
    ///                 ui.text(format!("Got: {result}"));
    ///             }
    ///             None => {
    ///                 *handle.get_mut(ui) = Some(h);
    ///                 ui.text("Loading...");
    ///             }
    ///         }
    ///     }
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn spawn<T: Send + 'static>(
        &mut self,
        fut: impl std::future::Future<Output = T> + Send + 'static,
    ) -> TaskHandle<T> {
        self.async_tasks.spawn(fut)
    }

    /// Poll a [`TaskHandle`](crate::TaskHandle) for its result.
    ///
    /// Returns `Some(result)` exactly once — on the first frame after the task
    /// completes — then `None` on every subsequent call. Returns `None` while
    /// the task is still in flight.
    ///
    /// Pairs with [`spawn`](Self::spawn). Requires the `async` feature.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "async")]
    /// # fn ex(ui: &mut slt::Context, handle: &slt::TaskHandle<u32>) {
    /// if let Some(value) = ui.poll(handle) {
    ///     ui.text(format!("done: {value}"));
    /// }
    /// # }
    /// ```
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn poll<T: 'static>(&mut self, handle: &TaskHandle<T>) -> Option<T> {
        self.async_tasks.poll::<T>(handle.id())
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
    /// Returns a [`Memo<T>`] *index handle*, mirroring [`use_state`]'s
    /// [`State<T>`]. The handle holds **no** borrow of `ui`, so it composes with
    /// later `ui.*` calls — read the value on demand with `.get(ui)` /
    /// `.copied(ui)`.
    ///
    /// Before v0.21.0 this returned `&T`, a live borrow of `&mut Context` that
    /// could not be held across subsequent `ui.*` mutations. That form is now
    /// [`use_memo_ref`](Self::use_memo_ref) (deprecated). Migrate
    /// `let x = *ui.use_memo(&d, f);` to `let x = ui.use_memo(&d, f).copied(ui);`.
    ///
    /// [`use_state`]: Self::use_state
    ///
    /// # Panics
    ///
    /// Panics if the hook slot at this call position was previously used for a
    /// different hook (a rules-of-hooks / call-order violation), since the
    /// type-erased slot then fails to downcast to `MemoSlot<T>`:
    ///
    /// ```text
    /// Hook type mismatch at index {idx}: expected {type}. Hooks must be called in the same order every frame.
    /// ```
    ///
    /// Keep hook calls in the same order every frame — do not call this inside
    /// an `if`/`else` whose branch changes between frames.
    ///
    /// # Example
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let count = ui.use_state(|| 0i32);
    /// let count_val = *count.get(ui);
    /// let doubled = ui.use_memo(&count_val, |c| c * 2);
    /// // The handle survives an intervening `ui.*` call (this is the whole point).
    /// ui.text("doubled:");
    /// ui.text(format!("{}", doubled.copied(ui)));
    /// # });
    /// ```
    pub fn use_memo<T: 'static, D: PartialEq + Clone + 'static>(
        &mut self,
        deps: &D,
        compute: impl FnOnce(&D) -> T,
    ) -> Memo<T> {
        let idx = self.rollback.hook_cursor;
        self.rollback.hook_cursor += 1;

        // First call at this slot: allocate fresh state. Deps are stored
        // type-erased so the read path (`Memo::get`) can downcast `MemoSlot<T>`
        // without restating `D`.
        if idx >= self.hook_states.len() {
            self.hook_states.push(Box::new(MemoSlot {
                deps: Box::new(deps.clone()),
                value: compute(deps),
            }));
            return Memo::from_idx(idx);
        }

        // Slot already exists: it must be the same `MemoSlot<T>` shape we used
        // last frame, or the caller broke the rules-of-hooks contract.
        match self.hook_states[idx].downcast_mut::<MemoSlot<T>>() {
            Some(slot) => {
                // Compare against the previous (type-erased) deps. A failed
                // downcast of the stored deps to `&D` is treated as stale so the
                // value is recomputed rather than silently kept.
                let stale = slot
                    .deps
                    .downcast_ref::<D>()
                    .map(|prev| *prev != *deps)
                    .unwrap_or(true);
                if stale {
                    slot.deps = Box::new(deps.clone());
                    slot.value = compute(deps);
                }
            }
            None => panic!(
                "Hook type mismatch at index {}: expected {}. Hooks must be called in the same order every frame.",
                idx,
                std::any::type_name::<MemoSlot<T>>()
            ),
        }
        Memo::from_idx(idx)
    }

    /// Deprecated `&T`-returning form of [`use_memo`](Self::use_memo).
    ///
    /// **Deprecated since 0.21.0**: [`use_memo`](Self::use_memo) now returns a
    /// [`Memo<T>`] handle that does not borrow `ui`, so it composes with later
    /// `ui.*` calls. This alias preserves the original behaviour (returning a
    /// `&T` borrow of `ui`) for callers that cannot migrate immediately; the
    /// borrow keeps `ui` immutably borrowed until the reference is dropped.
    ///
    /// Migrate `let x = *ui.use_memo_ref(&d, f);` to
    /// `let x = ui.use_memo(&d, f).copied(ui);` (or `.get(ui)` for a reference).
    ///
    /// # Panics
    ///
    /// Panics if the hook slot at this call position was previously used for a
    /// different hook (a rules-of-hooks / call-order violation), since the
    /// type-erased slot then fails to downcast to `(D, T)`:
    ///
    /// ```text
    /// Hook type mismatch at index {idx}: expected {type}. Hooks must be called in the same order every frame.
    /// ```
    ///
    /// # Example
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// # #[allow(deprecated)]
    /// let doubled = *ui.use_memo_ref(&21i32, |c| c * 2);
    /// ui.text(format!("{doubled}"));
    /// # });
    /// ```
    #[deprecated(
        since = "0.21.0",
        note = "use_memo now returns a Memo<T> handle; call `.get(ui)` / `.copied(ui)`"
    )]
    pub fn use_memo_ref<T: 'static, D: PartialEq + Clone + 'static>(
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
        if self.theme.is_dark { dark } else { light }
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
    /// `String` once per call. The hot path (key already present) performs
    /// **zero string allocations beyond the [`Into<String>`] conversion at
    /// the call site** — first looking up by `&str`, only allocating a
    /// fresh map key on first insert. Together: at most **one allocation
    /// per call, regardless of cache state**.
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
        // Lookup by `&str` first to avoid cloning on the hot
        // (already-populated) path. Only on first insert do we clone the
        // key into the map; otherwise the original `key` String is the
        // sole allocation and is moved into `State::from_keyed`.
        if !self.keyed_states.contains_key(key.as_str()) {
            self.keyed_states.insert(key.clone(), Box::new(init()));
        }
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
    /// # Panics
    ///
    /// Panics if the hook slot at this call position was previously used for a
    /// different hook (a rules-of-hooks / call-order violation), since the
    /// type-erased slot then fails to downcast to the deps type `D`:
    ///
    /// ```text
    /// Hook type mismatch at index {idx}: expected {type}. Hooks must be called in the same order every frame.
    /// ```
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

    /// Register a focusable slot bound to a stable string name.
    ///
    /// Returns `true` if the registered slot currently has focus, exactly
    /// like [`register_focusable`](Self::register_focusable) — but also
    /// records the `name → slot` mapping so other code can later call
    /// [`focus_by_name`](Self::focus_by_name) and
    /// [`focused_name`](Self::focused_name).
    ///
    /// # How the slot is shared with the widget that follows
    ///
    /// Every SLT widget that takes focus (`button`, `text_input`,
    /// `tabs`, …) internally calls `register_focusable()` to claim its
    /// own slot. To keep the name pointed at the **widget the user
    /// sees**, this call:
    ///
    /// 1. allocates a slot eagerly (so the name binding works even when
    ///    no widget follows — useful for tests and for custom focusable
    ///    regions),
    /// 2. records the `name → slot` mapping into the frame's
    ///    `focus_name_map` (first-write-wins on duplicate names within
    ///    a frame),
    /// 3. **reserves** the slot id so the next `register_focusable()`
    ///    on the same frame *reuses* it instead of allocating a fresh
    ///    slot — that's how `text_input(&mut state)` placed right after
    ///    inherits the name.
    ///
    /// Names are re-registered each frame; the previous frame's map is
    /// kept under `focus_name_map_prev` so [`focus_by_name`](Context::focus_by_name) can resolve
    /// a name that has already been registered.
    ///
    /// # Two valid usage shapes
    ///
    /// **Shape A — name a widget that follows immediately** (the common
    /// pattern; the widget reuses the reserved slot):
    ///
    /// ```ignore
    /// let _ = ui.register_focusable_named("search");
    /// let _ = ui.text_input(&mut search_state);
    /// // later: ui.focus_by_name("search") jumps to the text_input
    /// ```
    ///
    /// **Shape B — register a named focusable region with no inner
    /// widget** (e.g. a custom render area that handles its own keys
    /// when focused):
    ///
    /// ```ignore
    /// let focused = ui.register_focusable_named("canvas");
    /// if focused { /* react to keys via key_presses_when */ }
    /// ```
    pub fn register_focusable_named(&mut self, name: &str) -> bool {
        // Modal/overlay suppression: when a modal is active and we're not
        // inside it, focusables outside the modal must be invisible to
        // tab/click cycling. Drop the registration entirely (no slot
        // allocation, no name binding, no reservation leak).
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            self.rollback.pending_focusable_id = None;
            return false;
        }
        // Eagerly allocate the slot — symmetric with `register_focusable`,
        // so the slot exists even when no widget follows.
        let id = self.rollback.focus_count;
        self.rollback.focus_count += 1;
        self.rollback.last_focusable_id = Some(id);
        self.commands.push(Command::FocusMarker(id));
        // First-write-wins on duplicate names within a single frame —
        // a second `register_focusable_named("dup")` keeps the first
        // slot bound to the name and orphans its own slot's name binding.
        self.focus_name_map.entry(name.to_string()).or_insert(id);
        // Reserve `id` for the very next `register_focusable()` call to
        // reuse, so widgets like `text_input` placed immediately after
        // share the named slot rather than allocating a fresh one.
        // Last-write-wins on the reservation: stacking two
        // `register_focusable_named` calls without an intervening widget
        // leaves the second slot reserved (the first slot stays bound to
        // its name in `focus_name_map`, just without a widget attached).
        self.rollback.pending_focusable_id = Some(id);
        // Same focus-index prediction as `register_focusable`.
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

    /// Request focus on the named widget.
    ///
    /// If the named widget was registered last frame the focus change
    /// takes effect at the **start of the next frame** (one-frame delay
    /// is the deferred-command pattern used throughout SLT). If the name
    /// has never been registered, the request stays pending: the next
    /// frame to register that name receives focus.
    ///
    /// Returns `true` if the call **will** resolve — i.e. the name was
    /// either registered earlier in this frame (via
    /// [`register_focusable_named`](Self::register_focusable_named)) or in
    /// the previous frame. Returns `false` only when the name has not been
    /// seen by either frame, in which case the request stays pending until
    /// some future frame registers the name.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if ui.button("Find").clicked {
    ///     ui.focus_by_name("search");
    /// }
    /// ```
    pub fn focus_by_name(&mut self, name: &str) -> bool {
        // Resolve against either the previous frame's settled map or the
        // in-progress map being built right now. The latter handles the
        // common "register, then focus_by_name in the same frame" pattern
        // that callers naturally expect to return `true`.
        //
        // The actual focus change still lands at the start of the next
        // frame via `focus_name_map_prev` lookup in `Context::new`. The
        // return value is purely about resolvability: "true" means the name
        // is known and the focus shift will land next frame; "false" means
        // the request is pending a future registration.
        let resolved =
            self.focus_name_map_prev.contains_key(name) || self.focus_name_map.contains_key(name);
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
    /// frame state where they can be drained by tests via
    /// [`Context::take_static_log`] (or by inspecting the buffer when
    /// constructing a custom backend).
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
        if let Some(boxed) = self.named_states.get_mut(STATIC_LOG_KEY)
            && let Some(buf) = boxed.downcast_mut::<Vec<String>>()
        {
            return std::mem::take(buf);
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
        if let Some(boxed) = self.named_states.get(KEYMAP_REGISTRY_KEY)
            && let Some(vec) = boxed.downcast_ref::<Vec<crate::keymap::PublishedKeymap>>()
        {
            return vec;
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
