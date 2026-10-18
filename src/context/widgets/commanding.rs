//! Commanding family — screen stacks, the command palette, and key chords /
//! key sequences.
//!
//! State types ([`crate::widgets::ScreenState`],
//! [`crate::widgets::CommandPaletteState`], [`crate::widgets::ChordState`],
//! [`crate::widgets::ModeState`]) live in `src/widgets/commanding.rs`.

use super::*;
use crate::DEFAULT_CHORD_TIMEOUT_TICKS;

impl Context {
    /// Conditionally render content when the named screen is active.
    ///
    /// Each screen gets an isolated hook segment — `use_state` / `use_memo`
    /// calls inside one screen do not interfere with another screen's hooks,
    /// even when you switch between screens across frames.
    ///
    /// Focus state is saved and restored per screen automatically.
    /// Navigation requested inside a screen mutates `ScreenState` when that
    /// closure returns, while the frame finishes rendering the screen that was
    /// active at its start. The destination renders on the next frame, so
    /// source and destination content are never composed together.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let mut screens = slt::ScreenState::new("main");
    /// # slt::run(|ui| {
    /// ui.screen("main", &mut screens, |ui| {
    ///     ui.text("Main screen");
    /// });
    /// # });
    /// ```
    pub fn screen(&mut self, name: &str, screens: &mut ScreenState, f: impl FnOnce(&mut Context)) {
        let screen_key = screens as *mut ScreenState as usize;
        let screen_state_id = screens.id();
        let is_active = self
            .screen_nav_render_origins
            .get(&screen_key)
            .map_or_else(|| screens.current() == name, |origin| origin == name);

        if is_active {
            // Allocate a segment only when this exact ScreenState/name pair is
            // active for the first time. Inactive declarations do not reserve
            // the same zero-length segment as a later active screen.
            let existing = self
                .screen_hook_map
                .get(&screen_state_id)
                .and_then(|hooks| hooks.get(name))
                .copied();
            let (seg_start, _seg_count) = existing.unwrap_or_else(|| {
                let value = (self.hook_states.len(), 0);
                self.screen_hook_map
                    .entry(screen_state_id)
                    .or_default()
                    .insert(name.to_string(), value);
                value
            });
            screens.sync_focus();
            let previous_range = self.screen_focus_ranges.get(&screen_state_id).cloned();
            let focus_count_before = self.rollback.focus_count;
            let previous_focus_count = self.prev_focus_count;
            let requested = self.rollback.focus_origin.unwrap_or_else(|| {
                if self.prev_focus_count > 0 {
                    self.focus_index % self.prev_focus_count
                } else {
                    self.focus_index
                }
            });
            let (saved_focus_idx, saved_focus_count) = screens.restore_focus(name);
            let returning = previous_range
                .as_ref()
                .is_none_or(|range| range.name.as_ref() != name);
            let owned_previous_focus = self.rollback.focus_origin.is_some()
                && previous_range.as_ref().is_some_and(|range| {
                    requested >= range.start && requested - range.start < range.count
                });
            let restored = returning
                && self.rollback.focus_generation == 0
                && (owned_previous_focus
                    || (previous_range.is_none()
                        && saved_focus_count > 0
                        && requested >= focus_count_before
                        && requested - focus_count_before < saved_focus_count));
            if restored {
                self.focus_index = focus_count_before.saturating_add(if saved_focus_count > 0 {
                    saved_focus_idx % saved_focus_count
                } else {
                    0
                });
                // A restored slot belongs to this screen, not the previous
                // screen's total. Do not modulo it by that unrelated count.
                self.prev_focus_count = 0;
                self.prev_focus_index = None;
                self.rollback.focus_origin = None;
                self.rollback.focus_history_origin = None;
            } else if let Some(range) = &previous_range
                && range.name.as_ref() == name
                && owned_previous_focus
            {
                self.focus_index = focus_count_before.saturating_add(requested - range.start);
                self.prev_focus_count = self
                    .rollback
                    .focus_count_origin
                    .saturating_sub(range.start)
                    .saturating_add(focus_count_before);
                if let Some(previous) = self.rollback.focus_history_origin
                    && previous >= range.start
                    && previous - range.start < range.count
                {
                    self.prev_focus_index =
                        Some(focus_count_before.saturating_add(previous - range.start));
                }
            }

            let previous_guard = self.rollback.screen_focus_guard;
            let previous_new_slot = self.rollback.new_focus_slot;
            let inherited_block = previous_guard
                .and_then(|guard| guard.blocked_generation)
                .filter(|generation| *generation == self.rollback.focus_generation);
            let blocked_generation = inherited_block.or_else(|| {
                (self.rollback.focus_origin.is_some()
                    && previous_range.is_some()
                    && !owned_previous_focus)
                    .then_some(self.rollback.focus_generation)
            });
            self.rollback.screen_focus_guard = Some(ScreenFocusGuard {
                start: focus_count_before,
                previous_count: previous_range
                    .as_ref()
                    .filter(|range| range.name.as_ref() == name)
                    .map_or(0, |range| range.count),
                blocked_generation,
            });
            self.rollback.new_focus_slot = false;

            // Set hook cursor to this screen's segment start
            self.rollback.hook_cursor = seg_start;

            // Scope deferred navigation to this exact screen closure. Nested
            // screens get their own range and cannot drain an outer request.
            let nav_scope_start = self.pending_screen_nav.len();
            self.screen_nav_depth += 1;
            let request_generation = self.rollback.focus_generation;

            // Execute the screen's closure
            f(self);

            self.screen_nav_depth = self
                .screen_nav_depth
                .checked_sub(1)
                .expect("active screen must own a navigation scope");

            // Record the hook count for this screen.
            //
            // The first-frame path above already inserted an owned `String`
            // key for this screen; subsequent frames reuse it. Locate that
            // existing slot via `&str` and overwrite the value in place,
            // avoiding a second `to_string()` allocation per active frame.
            let hooks_used = self.rollback.hook_cursor - seg_start;
            if let Some(slot) = self
                .screen_hook_map
                .get_mut(&screen_state_id)
                .and_then(|hooks| hooks.get_mut(name))
            {
                *slot = (seg_start, hooks_used);
            } else {
                self.screen_hook_map
                    .entry(screen_state_id)
                    .or_default()
                    .insert(name.to_string(), (seg_start, hooks_used));
            }

            // Store local slots, then let the frame's post-widget Tab pass
            // deliver its final request through an owned update handle.
            let screen_focus_count = self.rollback.focus_count - focus_count_before;
            let end = focus_count_before.saturating_add(screen_focus_count);
            if request_generation == self.rollback.focus_generation {
                if (restored || owned_previous_focus) && self.focus_index >= end {
                    // A removed/restored slot must not spill its activation into
                    // a footer that happens to reuse the old numeric index.
                    self.consume_activation_keys(true);
                    self.focus_index =
                        focus_count_before.saturating_add(screen_focus_count.saturating_sub(1));
                } else if let Some(origin) = self.rollback.focus_origin
                    && let Some(range) = &previous_range
                    && range.name.as_ref() == name
                    && origin >= range.start.saturating_add(range.count)
                {
                    self.focus_index =
                        end.saturating_add(origin - range.start.saturating_add(range.count));
                }
            }
            if let Some(origin) = self.rollback.focus_history_origin
                && let Some(range) = &previous_range
                && range.name.as_ref() == name
                && origin >= range.start.saturating_add(range.count)
            {
                self.prev_focus_index =
                    Some(end.saturating_add(origin - range.start.saturating_add(range.count)));
            }
            let current = if self.prev_focus_count > 0 {
                self.focus_index % self.prev_focus_count
            } else {
                self.focus_index
            };
            let local = if current >= focus_count_before
                && current - focus_count_before < screen_focus_count
            {
                current - focus_count_before
            } else {
                saved_focus_idx.min(screen_focus_count.saturating_sub(1))
            };
            screens.save_focus(name, local, screen_focus_count);
            self.prev_focus_count = if self.rollback.focus_count_origin == 0 {
                0
            } else if let Some(range) = &previous_range {
                self.rollback
                    .focus_count_origin
                    .saturating_sub(range.start.saturating_add(range.count))
                    .saturating_add(end)
            } else {
                previous_focus_count.saturating_add(screen_focus_count)
            };
            self.rollback.screen_focus_guard = previous_guard;
            self.rollback.new_focus_slot = previous_new_slot;
            let screen_name = previous_range
                .as_ref()
                .filter(|range| range.name.as_ref() == name)
                .map_or_else(
                    || std::sync::Arc::<str>::from(name),
                    |range| std::sync::Arc::clone(&range.name),
                );
            let (updates, generation) = screens.focus_updates(&screen_name);
            self.screen_focus_scopes.push(ScreenFocusScope {
                id: screen_state_id,
                range: ScreenFocusRange {
                    name: screen_name,
                    start: focus_count_before,
                    count: screen_focus_count,
                },
                updates,
                generation,
            });

            // Issue #279: apply navigation requested from inside the closure
            // now that the closure's `&mut Context` borrow has ended. We still
            // hold `&mut screens` here, so there is no double mutable borrow —
            // app code can call `ui.push_screen(...)` / `ui.pop_screen()` from
            // within the closure without the borrow conflict from the issue.
            if self.pending_screen_nav.len() > nav_scope_start {
                // Preserve the screen that rendered at the start of this
                // transition. ScreenState changes immediately, but later
                // declarations wait until the next frame to render the new
                // destination, avoiding a one-frame source/destination blend.
                self.screen_nav_render_origins
                    .entry(screen_key)
                    .or_insert_with(|| screens.current().to_owned());
                for nav in self.pending_screen_nav.drain(nav_scope_start..) {
                    screens.apply_nav(nav);
                }
            }
        } else {
            // Skip an already-initialized segment without allocating one for a
            // screen that has never been active.
            if let Some(&(seg_start, seg_count)) = self
                .screen_hook_map
                .get(&screen_state_id)
                .and_then(|hooks| hooks.get(name))
                && seg_count > 0
                && seg_start >= self.rollback.hook_cursor
            {
                self.rollback.hook_cursor = seg_start.saturating_add(seg_count);
            }
        }
    }

    /// Request pushing a new screen onto the active [`ScreenState`] stack.
    ///
    /// Call this from inside a [`Context::screen`] closure to navigate forward.
    /// The push is deferred and applied to your `ScreenState` the moment the
    /// closure returns, so it does not conflict with the `&mut ScreenState`
    /// already borrowed by `screen(...)` (issue #279). Rendering stays on the
    /// source screen for that transition frame; the destination appears on the
    /// next frame.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let mut screens = slt::ScreenState::new("home");
    /// # slt::run(|ui| {
    /// ui.screen("home", &mut screens, |ui| {
    ///     if ui.button("Settings").clicked {
    ///         ui.push_screen("settings");
    ///     }
    /// });
    /// # });
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when called outside an active [`Context::screen`] closure. Use
    /// [`ScreenState::push`] directly when navigating outside a screen.
    #[track_caller]
    pub fn push_screen(&mut self, name: impl Into<String>) {
        self.assert_screen_nav_scope();
        self.pending_screen_nav.push(ScreenNav::Push(name.into()));
    }

    /// Request popping the current screen off the active [`ScreenState`] stack
    /// (the root screen is preserved).
    ///
    /// Like [`Self::push_screen`], the pop is deferred and applied when the
    /// enclosing [`Context::screen`] closure returns (issue #279).
    ///
    /// # Panics
    ///
    /// Panics when called outside an active [`Context::screen`] closure. Use
    /// [`ScreenState::pop`] directly when navigating outside a screen.
    #[track_caller]
    pub fn pop_screen(&mut self) {
        self.assert_screen_nav_scope();
        self.pending_screen_nav.push(ScreenNav::Pop);
    }

    /// Request resetting the active [`ScreenState`] stack to just its root
    /// screen.
    ///
    /// Deferred and applied when the enclosing [`Context::screen`] closure
    /// returns (issue #279).
    ///
    /// # Panics
    ///
    /// Panics when called outside an active [`Context::screen`] closure. Use
    /// [`ScreenState::reset`] directly when navigating outside a screen.
    #[track_caller]
    pub fn reset_screen(&mut self) {
        self.assert_screen_nav_scope();
        self.pending_screen_nav.push(ScreenNav::Reset);
    }

    #[track_caller]
    fn assert_screen_nav_scope(&self) {
        assert!(
            self.screen_nav_depth > 0,
            "screen navigation helpers can only be called inside an active Context::screen closure; mutate ScreenState directly outside a screen"
        );
    }

    /// Remove retained hook/focus state for an inactive screen.
    ///
    /// Returns `false` when `name` is still present in `screens`' stack. Call
    /// this after popping runtime-generated detail screens to release their
    /// isolated hook segment and saved focus entry.
    pub fn remove_screen_state(&mut self, screens: &mut ScreenState, name: &str) -> bool {
        if screens.contains(name) {
            return false;
        }
        let focus_removed = screens.remove_inactive(name);
        let hooks_removed = self.remove_screen_hooks(screens.id(), name);
        focus_removed || hooks_removed
    }

    /// Retain inactive screen states accepted by `keep`.
    ///
    /// Screens currently present in `screens`' stack are always kept. Returns
    /// the number of screen names removed from either the hook map or the
    /// saved-focus map.
    pub fn retain_screen_state(
        &mut self,
        screens: &mut ScreenState,
        mut keep: impl FnMut(&str) -> bool,
    ) -> usize {
        let screen_id = screens.id();
        let remove_names: Vec<String> = self
            .screen_hook_map
            .get(&screen_id)
            .into_iter()
            .flat_map(|hooks| hooks.keys())
            .filter(|name| !screens.contains(name) && !keep(name))
            .cloned()
            .collect();

        let mut removed = 0;
        for name in remove_names {
            if self.remove_screen_state(screens, &name) {
                removed += 1;
            }
        }
        removed + screens.retain_inactive(keep)
    }

    /// Number of retained screen hook segments.
    ///
    /// Diagnostic helper for apps that generate screen names from runtime ids.
    pub fn screen_state_count(&self) -> usize {
        self.screen_hook_map.values().map(|hooks| hooks.len()).sum()
    }

    fn remove_screen_hooks(&mut self, screen_id: u64, name: &str) -> bool {
        let Some((seg_start, seg_count)) = self
            .screen_hook_map
            .get_mut(&screen_id)
            .and_then(|hooks| hooks.remove(name))
        else {
            return false;
        };
        if self
            .screen_hook_map
            .get(&screen_id)
            .is_some_and(|hooks| hooks.is_empty())
        {
            self.screen_hook_map.remove(&screen_id);
        }

        if seg_count == 0 {
            return true;
        }

        let end = seg_start
            .saturating_add(seg_count)
            .min(self.hook_states.len());
        if seg_start >= end {
            return true;
        }

        let removed = end - seg_start;
        self.hook_states.drain(seg_start..end);
        for hooks in self.screen_hook_map.values_mut() {
            for (other_start, _) in hooks.values_mut() {
                if *other_start > seg_start {
                    *other_start = other_start.saturating_sub(removed);
                }
            }
        }
        if self.rollback.hook_cursor > seg_start {
            self.rollback.hook_cursor = self.rollback.hook_cursor.saturating_sub(removed);
        }
        true
    }

    // ── command palette ──────────────────────────────────────────────

    /// Render a command palette overlay.
    pub fn command_palette(&mut self, state: &mut CommandPaletteState) -> Response {
        if !state.open {
            return Response::none();
        }

        state.last_selected = None;
        let interaction_id = self.next_interaction_id();
        // Issue #453: an open palette owns typed keys, so `key()` shortcuts
        // stay quiet while the user searches.
        self.rollback.text_entry_claim = Some(crate::runtime::TextEntryKind::MultiLine);

        let filtered: Vec<usize> = state.filtered_indices_cached().to_vec();
        let sel = state.selected().min(filtered.len().saturating_sub(1));
        state.set_selected(sel);

        let mut consumed_indices = Vec::new();

        for (i, key) in self.available_key_presses() {
            match key.code {
                KeyCode::Esc => {
                    state.open = false;
                    consumed_indices.push(i);
                }
                KeyCode::Up => {
                    let s = state.selected();
                    state.set_selected(s.saturating_sub(1));
                    consumed_indices.push(i);
                }
                KeyCode::Down => {
                    let filtered_len = state.filtered_indices_cached().len();
                    let s = state.selected();
                    state.set_selected((s + 1).min(filtered_len.saturating_sub(1)));
                    consumed_indices.push(i);
                }
                KeyCode::Enter => {
                    let filtered = state.filtered_indices_cached().to_vec();
                    if let Some(&cmd_idx) = filtered.get(state.selected()) {
                        state.last_selected = Some(cmd_idx);
                        state.open = false;
                    }
                    consumed_indices.push(i);
                }
                KeyCode::Backspace => {
                    if state.cursor > 0 {
                        let byte_idx = byte_index_for_grapheme(&state.input, state.cursor - 1);
                        let end_idx = byte_index_for_grapheme(&state.input, state.cursor);
                        state.input.replace_range(byte_idx..end_idx, "");
                        state.cursor -= 1;
                        state.set_selected(0);
                    }
                    consumed_indices.push(i);
                }
                KeyCode::Char(ch) if !has_global_shortcut_modifier(key.modifiers) => {
                    let byte_idx = byte_index_for_grapheme(&state.input, state.cursor);
                    state.input.insert(byte_idx, ch);
                    state.cursor = grapheme_count(&state.input[..byte_idx + ch.len_utf8()]);
                    state.set_selected(0);
                    consumed_indices.push(i);
                }
                _ => {}
            }
        }
        for (i, text) in self.available_pastes() {
            let inserted = text
                .graphemes(true)
                .filter(|cluster| {
                    cluster
                        .chars()
                        .all(|ch| (ch as u32) >= 0x20 && ch != '\u{7f}')
                })
                .collect::<String>();
            if !inserted.is_empty() {
                let byte_idx = byte_index_for_grapheme(&state.input, state.cursor);
                let inserted_end = byte_idx + inserted.len();
                state.input.insert_str(byte_idx, &inserted);
                state.cursor = grapheme_count(&state.input[..inserted_end]);
                state.set_selected(0);
            }
            consumed_indices.push(i);
        }
        self.consume_indices(consumed_indices);

        let filtered: Vec<usize> = state.filtered_indices_cached().to_vec();

        let _ = self.modal(|ui| {
            let primary = ui.theme.primary;
            let palette_pad = ui.theme.spacing.xs();
            let palette_input_padx = ui.theme.spacing.xs();
            let _ = ui
                .container()
                .border(Border::Rounded)
                .border_style(Style::new().fg(primary))
                .p(palette_pad)
                .max_w(60)
                .col(|ui| {
                    let border_color = ui.theme.primary;
                    let _ = ui
                        .bordered(Border::Rounded)
                        .border_style(Style::new().fg(border_color))
                        .px(palette_input_padx)
                        .col(|ui| {
                            let display = if state.input.is_empty() {
                                "Type to search...".to_string()
                            } else {
                                state.input.clone()
                            };
                            let style = if state.input.is_empty() {
                                Style::new().dim().fg(ui.theme.text_dim)
                            } else {
                                Style::new().fg(ui.theme.text)
                            };
                            ui.styled(display, style);
                        });

                    for (list_idx, &cmd_idx) in filtered.iter().enumerate() {
                        let cmd = &state.commands()[cmd_idx];
                        let is_selected = list_idx == state.selected();
                        let style = if is_selected {
                            Style::new().bold().fg(ui.theme.primary)
                        } else {
                            Style::new().fg(ui.theme.text)
                        };
                        let prefix = if is_selected { "▸ " } else { "  " };
                        let shortcut_text = cmd
                            .shortcut
                            .as_deref()
                            .map(|s| {
                                let mut text = String::with_capacity(s.len() + 4);
                                text.push_str("  (");
                                text.push_str(s);
                                text.push(')');
                                text
                            })
                            .unwrap_or_default();
                        let mut line = String::with_capacity(
                            prefix.len() + cmd.label.len() + shortcut_text.len(),
                        );
                        line.push_str(prefix);
                        line.push_str(&cmd.label);
                        line.push_str(&shortcut_text);
                        ui.styled(line, style);
                        if is_selected && !cmd.description.is_empty() {
                            let mut desc = String::with_capacity(4 + cmd.description.len());
                            desc.push_str("    ");
                            desc.push_str(&cmd.description);
                            ui.styled(desc, Style::new().dim().fg(ui.theme.text_dim));
                        }
                    }

                    if filtered.is_empty() {
                        ui.styled(
                            "  No matching commands",
                            Style::new().dim().fg(ui.theme.text_dim),
                        );
                    }
                });
        });

        let mut response = self.response_for(interaction_id);
        response.changed = state.last_selected.is_some();
        response
    }

    // ── key chord (cross-frame multi-key sequence) ───────────────────

    /// Match a multi-key sequence whose keystrokes may span multiple frames
    /// (vi `gg`, leader keys).
    ///
    /// Unlike a single-frame matcher, `key_chord` buffers partial input in
    /// crate-internal `FrameState` across frames: typing `g` on one frame
    /// and `g` on the next returns `true` on the second frame. The partial
    /// prefix is cleared on a non-matching key press (vi semantics: `g` then
    /// `x` cancels a pending `gg`) or after
    /// [`DEFAULT_CHORD_TIMEOUT_TICKS`](crate::DEFAULT_CHORD_TIMEOUT_TICKS) of
    /// inactivity (measured on the same tick clock as notifications/animation).
    ///
    /// Returns `true` exactly once, on the frame that completes the sequence;
    /// the completing key event is consumed so downstream widgets in the same
    /// frame do not also handle it. It does not re-fire on later frames without
    /// new input.
    ///
    /// # Leader notation
    ///
    /// A leading `<space>` or `<leader>` token (or a literal space) matches the
    /// space key, e.g. `key_chord("<space>ff")`, `key_chord("<leader>ff")`, and
    /// `key_chord(" ff")` are equivalent. Only `<space>` / `<leader>` are
    /// recognized as special tokens; every other character is matched
    /// literally. Modifier-aware chords (`C-x C-s`) are out of scope.
    ///
    /// An empty sequence always returns `false`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// slt::run(|ui: &mut slt::Context| {
    ///     if ui.key_chord("gg") {
    ///         // vi-style: jump to the top
    ///     }
    ///     if ui.key_chord("<space>ff") {
    ///         // leader key: open a file finder
    ///     }
    /// });
    /// ```
    pub fn key_chord(&mut self, seq: &str) -> bool {
        self.key_chord_timeout(seq, DEFAULT_CHORD_TIMEOUT_TICKS)
    }

    /// [`key_chord`](Self::key_chord) with an explicit per-call timeout in ticks.
    ///
    /// A partial sequence is abandoned if `timeout_ticks` elapse on the tick
    /// clock without a matching next key. Use this when a chord should be more
    /// forgiving (large value) or stricter (small value) than the
    /// [`DEFAULT_CHORD_TIMEOUT_TICKS`](crate::DEFAULT_CHORD_TIMEOUT_TICKS)
    /// default. All other behavior matches [`key_chord`](Self::key_chord).
    ///
    /// # Example
    ///
    /// ```no_run
    /// slt::run(|ui: &mut slt::Context| {
    ///     // Require the second `g` within ~0.25s at 60Hz.
    ///     if ui.key_chord_timeout("gg", 15) {
    ///         // jump to top
    ///     }
    /// });
    /// ```
    pub fn key_chord_timeout(&mut self, seq: &str, timeout_ticks: u64) -> bool {
        let target = parse_chord(seq);
        if target.is_empty() {
            return false;
        }
        // Modal guard parity with the (deprecated) `key_seq`: suppress chords
        // while a modal owns input and no overlay is layered on top.
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }

        // Expire a stale prefix before processing this frame's keys.
        if self.tick.saturating_sub(self.chord.last_tick) > timeout_ticks {
            self.chord.pending.clear();
        }

        // Snapshot this frame's unconsumed char presses up front so the
        // immutable borrow from `available_key_presses` is released before we
        // mutate `self.chord` / call `consume_indices`.
        let char_presses: Vec<(usize, char)> = self
            .available_key_presses()
            .filter_map(|(i, key)| match key.code {
                KeyCode::Char(c) => Some((i, c)),
                _ => None,
            })
            .collect();

        let tick = self.tick;
        let mut completed_index: Option<usize> = None;
        let mut buf: Vec<char> = self.chord.pending.chars().collect();

        for (i, c) in char_presses {
            buf.push(c);
            // Keep only the longest suffix of `buf` that is a prefix of
            // `target`, giving vi-style overlap semantics (typing `gxg` still
            // arms `gg` from the trailing `g`).
            retain_longest_prefix(&mut buf, &target);
            self.chord.last_tick = tick;
            if buf.len() == target.len() {
                completed_index = Some(i);
                buf.clear();
                break;
            }
        }

        self.chord.pending = buf.into_iter().collect();
        if let Some(i) = completed_index {
            self.consume_indices([i]);
            true
        } else {
            false
        }
    }
}

/// Expand `<space>` / `<leader>` tokens in a chord spec into the characters
/// the matcher compares against. Everything else is taken literally. The only
/// special tokens are `<space>` and `<leader>` (both map to a literal space);
/// a literal space in the input is preserved as-is.
fn parse_chord(seq: &str) -> Vec<char> {
    let mut out = Vec::new();
    let mut rest = seq;
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix("<space>") {
            out.push(' ');
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("<leader>") {
            out.push(' ');
            rest = tail;
        } else {
            let c = rest.chars().next().expect("rest is non-empty");
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

/// Shrink `buf` to the longest suffix that is still a prefix of `target`.
///
/// This gives vi-style overlap semantics: after a mismatch the matcher does
/// not reset to empty but keeps any trailing characters that could begin a
/// fresh match. For example, with `target = ['g', 'g']`, the input `g x g`
/// leaves `buf = ['g']` (the trailing `g` re-arms the chord) rather than
/// discarding it.
fn retain_longest_prefix(buf: &mut Vec<char>, target: &[char]) {
    // Try progressively shorter suffixes of `buf`; the first that is a prefix
    // of `target` wins. An empty suffix is always a prefix, so this terminates.
    let mut start = 0;
    while start < buf.len() {
        if buf[start..].iter().zip(target).all(|(b, t)| b == t) {
            break;
        }
        start += 1;
    }
    if start > 0 {
        buf.drain(0..start);
    }
}
