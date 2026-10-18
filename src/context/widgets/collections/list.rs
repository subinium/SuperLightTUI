use super::*;

impl Context {
    /// Render a selectable list. Handles Up/Down (and `k`/`j`) navigation when focused.
    ///
    /// The selected item is highlighted with the theme's primary color. If the
    /// list is empty, nothing is rendered.
    pub fn list(&mut self, state: &mut ListState) -> Response {
        let colors = self.widget_theme.list;
        self.list_colored(state, &colors)
    }

    /// Render a navigable list with custom widget colors.
    pub fn list_colored(&mut self, state: &mut ListState, colors: &WidgetColors) -> Response {
        let visible_len = state.visible_indices().len();
        if visible_len == 0 && state.is_empty() {
            state.selected = 0;
            return Response::none();
        }

        if visible_len > 0 {
            state.selected = state.selected.min(visible_len.saturating_sub(1));
        }

        let old_selected = state.selected;
        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                        let _ = handle_vertical_nav(
                            &mut state.selected,
                            visible_len.saturating_sub(1),
                            key.code.clone(),
                        );
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            for (i, mouse) in clicks {
                let clicked_idx = (mouse.y - rect.y) as usize;
                if clicked_idx < visible_len {
                    state.selected = clicked_idx;
                    consumed.push(i);
                }
            }
            self.consume_indices(consumed);
        }

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(colors.border.unwrap_or(self.theme.border)),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

        for (view_idx, &item_idx) in state.visible_indices().iter().enumerate() {
            let item = &state.items()[item_idx];
            if view_idx == state.selected {
                let mut selected_style = Style::new()
                    .bg(colors.accent.unwrap_or(self.theme.selected_bg))
                    .fg(colors.fg.unwrap_or(self.theme.selected_fg));
                if focused {
                    selected_style = selected_style.bold();
                }
                let mut row = String::with_capacity(2 + item.len());
                row.push_str("▸ ");
                row.push_str(item);
                self.styled(row, selected_style);
            } else {
                let mut row = String::with_capacity(2 + item.len());
                row.push_str("  ");
                row.push_str(item);
                self.styled(row, Style::new().fg(colors.fg.unwrap_or(self.theme.text)));
            }
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.changed = state.selected != old_selected;
        response
    }

    /// Render a selectable list that supports keyboard reordering of items.
    ///
    /// Behaves exactly like [`list`](Context::list) for navigation (Up/Down and
    /// `k`/`j` move the selection) and click selection, but additionally lets the
    /// focused user reorder the selected item with `Shift+Up`/`Shift+Down` or
    /// `Alt+Up`/`Alt+Down`. Reordering operates on the underlying item order via
    /// [`ListState::move_item`], keeping the selection on the moved item.
    ///
    /// Returns a [`ListResponse`](crate::ListResponse) which derefs to the
    /// standard [`Response`] and exposes
    /// [`reordered`](crate::ListResponse::reordered) — `Some((from, to))` with
    /// the data indices when an item moved this frame, otherwise `None`.
    ///
    /// The plain [`list`](Context::list) entry point is unchanged; opt into
    /// reordering by calling this method instead.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ListState;
    /// # let mut list = ListState::new(vec!["First", "Second", "Third"]);
    /// # slt::run(move |ui: &mut slt::Context| {
    /// let r = ui.list_reorderable(&mut list);
    /// if let Some((from, to)) = r.reordered {
    ///     let _ = (from, to); // persist new order
    /// }
    /// # });
    /// ```
    ///
    /// Available since `0.21.1`.
    pub fn list_reorderable(&mut self, state: &mut ListState) -> crate::widgets::ListResponse {
        let colors = self.widget_theme.list;
        self.list_reorderable_colored(state, &colors)
    }

    /// Render a reorderable list with custom widget colors.
    ///
    /// See [`list_reorderable`](Context::list_reorderable) for the reorder
    /// keybindings and return semantics.
    ///
    /// Available since `0.21.1`.
    pub fn list_reorderable_colored(
        &mut self,
        state: &mut ListState,
        colors: &WidgetColors,
    ) -> crate::widgets::ListResponse {
        let visible = state.visible_indices().to_vec();
        if visible.is_empty() && state.is_empty() {
            state.selected = 0;
            return crate::widgets::ListResponse::default();
        }

        if !visible.is_empty() {
            state.selected = state.selected.min(visible.len().saturating_sub(1));
        }

        let old_selected = state.selected;
        let focused = self.register_focusable();
        let (interaction_id, mut response) = self.begin_widget_interaction(focused);

        let mut reordered: Option<(usize, usize)> = None;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                // Reorder takes precedence over navigation when a Shift/Alt
                // modifier is held with a vertical-movement key.
                let modded = key.modifiers.contains(KeyModifiers::SHIFT)
                    || key.modifiers.contains(KeyModifiers::ALT);
                // Direction of the move: -1 (up) or +1 (down) for the selected
                // view row, `None` for non-movement keys.
                let dir: Option<isize> = match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => Some(-1),
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => Some(1),
                    _ => None,
                };

                if modded {
                    if let Some(delta) = dir {
                        let cur_view = state.selected;
                        let target_view = if delta < 0 {
                            cur_view.checked_sub(1)
                        } else {
                            let next = cur_view + 1;
                            (next < visible.len()).then_some(next)
                        };
                        // Map both endpoints from view positions to data indices
                        // so reordering survives an active filter.
                        if let Some(target_view) = target_view
                            && let (Some(&from), Some(&to)) =
                                (visible.get(cur_view), visible.get(target_view))
                            && state.move_item(from, to)
                        {
                            reordered = Some((from, to));
                        }
                        // Consume regardless so a held modifier never also
                        // triggers a plain navigation step on the same key.
                        consumed_indices.push(i);
                    }
                    continue;
                }

                if dir.is_some() {
                    let _ = handle_vertical_nav(
                        &mut state.selected,
                        visible.len().saturating_sub(1),
                        key.code.clone(),
                    );
                    consumed_indices.push(i);
                }
            }
            self.consume_indices(consumed_indices);
        }

        if let Some((rect, clicks)) = self.left_clicks_for_interaction(interaction_id) {
            let mut consumed = Vec::new();
            // `visible` may be stale after a reorder rebuilt the view; re-read
            // the current visible count for bounds.
            let visible_len = state.visible_indices().len();
            for (i, mouse) in clicks {
                let clicked_idx = (mouse.y - rect.y) as usize;
                if clicked_idx < visible_len {
                    state.selected = clicked_idx;
                    consumed.push(i);
                }
            }
            self.consume_indices(consumed);
        }

        // Re-read the (possibly reordered) view for rendering.
        let visible = state.visible_indices().to_vec();

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(colors.border.unwrap_or(self.theme.border)),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));

        for (view_idx, &item_idx) in visible.iter().enumerate() {
            let item = &state.items()[item_idx];
            if view_idx == state.selected {
                let mut selected_style = Style::new()
                    .bg(colors.accent.unwrap_or(self.theme.selected_bg))
                    .fg(colors.fg.unwrap_or(self.theme.selected_fg));
                if focused {
                    selected_style = selected_style.bold();
                }
                let mut row = String::with_capacity(2 + item.len());
                row.push_str("▸ ");
                row.push_str(item);
                self.styled(row, selected_style);
            } else {
                let mut row = String::with_capacity(2 + item.len());
                row.push_str("  ");
                row.push_str(item);
                self.styled(row, Style::new().fg(colors.fg.unwrap_or(self.theme.text)));
            }
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        response.changed = state.selected != old_selected || reordered.is_some();
        crate::widgets::ListResponse {
            response,
            reordered,
        }
    }

    // ── virtual list ─────────────────────────────────────────────────

    /// Render a virtual list that only renders visible items.
    ///
    /// `visible_height` limits how many rows are rendered. The closure `f`
    /// receives underlying item indices from [`ListState::visible_indices`],
    /// while selection and viewport offsets address the filtered view.
    ///
    /// This is the uniform fixed-height fast path: every item is treated as
    /// exactly one row. For chat/feed bubbles of differing heights see
    /// [`virtual_list_variable`](Context::virtual_list_variable).
    pub fn virtual_list(
        &mut self,
        state: &mut ListState,
        visible_height: u32,
        f: impl Fn(&mut Context, usize),
    ) -> Response {
        self.virtual_list_impl(state, visible_height, false, f)
    }

    /// Variable-height variant of [`virtual_list`](Context::virtual_list).
    ///
    /// Each item's height (in rows) comes from
    /// [`ListState::set_item_heights`](crate::widgets::ListState::set_item_heights);
    /// the visible range is computed so the rendered items fill at most
    /// `visible_height` rows starting from the current viewport. This is the
    /// chat/feed use case where bubbles vary in height (a one-line reply next
    /// to a 30-line code block). When no per-item heights are set it falls back
    /// to the uniform fast path and produces output identical to
    /// [`virtual_list`](Context::virtual_list). Rendering remains `O(visible)`:
    /// only items in the computed range invoke `f`, prefix-sum lookups are
    /// `O(log n)` and the prefix-sum rebuild is `O(n)` gated behind a dirty
    /// flag.
    ///
    /// An item taller than `visible_height` renders from its top and is never
    /// skipped. `PageUp`/`PageDown` move the selection by the number of *items*
    /// that fill `visible_height` *rows* from the current position. The
    /// "↑ N more / ↓ N more" affordances continue to count *items*.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::widgets::ListState;
    ///
    /// let mut state = ListState::new(vec!["short reply", "long\ncode\nblock", "ok"])
    ///     .with_item_heights(vec![1, 3, 1]);
    ///
    /// slt::run(|ui| {
    ///     ui.virtual_list_variable(&mut state, 10, |ui, idx| {
    ///         ui.text(format!("bubble {idx}"));
    ///     });
    /// })?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    ///
    /// Available since `0.21.0`.
    pub fn virtual_list_variable(
        &mut self,
        state: &mut ListState,
        visible_height: u32,
        f: impl Fn(&mut Context, usize),
    ) -> Response {
        self.virtual_list_impl(state, visible_height, true, f)
    }

    fn virtual_list_impl(
        &mut self,
        state: &mut ListState,
        visible_height: u32,
        variable: bool,
        f: impl Fn(&mut Context, usize),
    ) -> Response {
        let total = state.visible_indices().len();
        if total == 0 {
            return Response::none();
        }
        state.selected = state.selected.min(total.saturating_sub(1));
        let use_heights = variable && state.has_item_heights();
        let focused = self.register_focusable();
        let (_interaction_id, mut response) = self.begin_widget_interaction(focused);
        let old_selected = state.selected;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, key) in self.available_key_presses() {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Down | KeyCode::Char('j') => {
                        let max_index = total.saturating_sub(1);
                        let _ =
                            handle_vertical_nav(&mut state.selected, max_index, key.code.clone());
                        consumed_indices.push(i);
                    }
                    KeyCode::PageUp => {
                        state.selected = if use_heights {
                            page_up_target(state, state.selected, visible_height)
                        } else {
                            state.selected.saturating_sub(visible_height as usize)
                        };
                        consumed_indices.push(i);
                    }
                    KeyCode::PageDown => {
                        state.selected = if use_heights {
                            page_down_target(state, state.selected, visible_height)
                        } else {
                            (state.selected + visible_height as usize).min(total.saturating_sub(1))
                        };
                        consumed_indices.push(i);
                    }
                    KeyCode::Home => {
                        state.selected = 0;
                        consumed_indices.push(i);
                    }
                    KeyCode::End => {
                        state.selected = total.saturating_sub(1);
                        consumed_indices.push(i);
                    }
                    _ => {}
                }
            }
            self.consume_indices(consumed_indices);
        }

        let vh = visible_height as usize;
        let (start, end) = if use_heights {
            row_visible_range(state, vh)
        } else {
            // Uniform fixed-height path — byte-identical to the original
            // `virtual_list`: one item == one row.
            //
            // Clamp viewport_offset so `selected` stays inside [offset, offset + vh)
            // without forcing the cursor onto the bottom row when scrolling down.
            if state.selected < state.viewport_offset {
                state.viewport_offset = state.selected;
            }
            if vh > 0 && state.selected >= state.viewport_offset + vh {
                state.viewport_offset = state.selected - vh + 1;
            }
            let start = state.viewport_offset;
            state.viewport_row_offset = start;
            (start, start.saturating_add(vh).min(total))
        };

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction: Direction::Column,
                gap: 0,
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

        if start > 0 {
            let hidden = start.to_string();
            let mut line = String::with_capacity(hidden.len() + 10);
            line.push_str("  ↑ ");
            line.push_str(&hidden);
            line.push_str(" more");
            self.styled(line, Style::new().fg(self.theme.text_dim).dim());
        }

        for idx in start..end {
            let raw = state.visible_indices()[idx];
            if use_heights {
                let height = state.item_height(raw).min(visible_height);
                let _ = self.container().h(height).col(|ui| f(ui, raw));
            } else {
                f(self, raw);
            }
        }

        let remaining = total.saturating_sub(end);
        if remaining > 0 {
            let hidden = remaining.to_string();
            let mut line = String::with_capacity(hidden.len() + 10);
            line.push_str("  ↓ ");
            line.push_str(&hidden);
            line.push_str(" more");
            self.styled(line, Style::new().fg(self.theme.text_dim).dim());
        }

        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
        response.changed = state.selected != old_selected;
        response
    }
}

// ── variable-height virtual_list helpers ─────────────────────────────────
//
// These operate on the per-item `row_prefix` cached in `ListState` so the
// visible range and page jumps are computed in *rows*, not items, while the
// public `viewport_offset` keeps its "top item index" meaning. All lookups are
// O(log n) (binary search) or O(visible) (bounded linear accumulation).

/// Largest item index `i` such that `row_prefix[i] <= target_row` — i.e. the
/// item containing (or starting at) `target_row`. Result is in `0..n`.
fn item_at_row(row_prefix: &[u32], target_row: u32, n: usize) -> usize {
    // `row_prefix` has `n + 1` entries; entry `i` is the first row of item `i`.
    // partition_point returns the count of entries `<= target_row`; subtract one
    // to get the index of the item that owns that row, clamped to the last item.
    if n == 0 {
        return 0;
    }
    let count = row_prefix.partition_point(|&r| r <= target_row);
    count.saturating_sub(1).min(n - 1)
}

/// Compute the `[start, end)` item range for the variable-height path.
///
/// Clamps `state.viewport_offset` (top item index) so `selected` is fully
/// visible by *rows*, then accumulates item heights from the top until the
/// viewport is filled. `viewport_row_offset` is kept in sync with the top
/// item's starting row. `end` always covers at least one item, so an item
/// taller than the viewport renders from its top instead of being skipped
/// (no zero-progress loop).
fn row_visible_range(state: &mut ListState, vh: usize) -> (usize, usize) {
    state.ensure_row_prefix();
    let n = state.visible_indices().len();
    if n == 0 || vh == 0 {
        state.viewport_offset = state.viewport_offset.min(n.saturating_sub(1));
        state.viewport_row_offset = 0;
        return (state.viewport_offset, state.viewport_offset);
    }

    let vh_rows = vh as u32;
    let row_prefix = state.row_prefix();
    // `row_prefix[i]` is the top row of item `i`.
    let sel = state.selected.min(n - 1);
    let sel_top = row_prefix[sel];
    let sel_bottom = row_prefix[sel + 1]; // exclusive bottom row of `selected`

    let mut top = state.viewport_offset.min(n - 1);

    // Scroll up: if the selection's top row is above the viewport top row,
    // pull the viewport up to the selected item.
    if sel_top < row_prefix[top] {
        top = sel;
    }

    // Jump directly to the first item whose top allows the selection to fit.
    // A selected item taller than the viewport starts at the viewport top.
    if top < sel && sel_bottom.saturating_sub(row_prefix[top]) > vh_rows {
        let earliest = sel_bottom.saturating_sub(vh_rows);
        top = row_prefix.partition_point(|&row| row < earliest).min(sel);
    }

    // Accumulate items from `top` until adding the next item would overflow
    // `vh` rows; the rendered items then sum to at most `vh` rows (a partially
    // clipped item is excluded). Always include at least the top item so a
    // tall item is never skipped (it renders from its top).
    let top_row = row_prefix[top];
    let target_bottom = top_row.saturating_add(vh_rows);
    // Largest exclusive `end` such that `row_prefix[end] <= target_bottom`,
    // i.e. items `top..end` fully fit within `vh` rows (their cumulative bottom
    // row does not exceed the viewport bottom). `partition_point` returns the
    // count of entries `<= target_bottom` (one past the largest matching
    // index), so subtract one to get the inclusive prefix index = exclusive
    // item end. Clamp to `[top + 1, n]` so at least one item always renders
    // (a tall item that overflows `vh` shows alone, from its top).
    let end = row_prefix
        .partition_point(|&r| r <= target_bottom)
        .saturating_sub(1)
        .clamp(top + 1, n);

    state.viewport_offset = top;
    state.viewport_row_offset = top_row as usize;
    (top, end)
}

/// Item index reached by paging *down* one viewport (`vh` rows) from `from`.
/// Advances by the count of items whose cumulative height fills `vh` rows,
/// guaranteeing forward progress of at least one item.
fn page_down_target(state: &mut ListState, from: usize, visible_height: u32) -> usize {
    state.ensure_row_prefix();
    let n = state.visible_indices().len();
    if n == 0 {
        return 0;
    }
    let from = from.min(n - 1);
    let row_prefix = state.row_prefix();
    let from_top = row_prefix[from];
    let target = from_top.saturating_add(visible_height.max(1));
    let next = item_at_row(row_prefix, target, n);
    next.max(from + 1).min(n - 1)
}

/// Item index reached by paging *up* one viewport (`vh` rows) from `from`.
/// Retreats by the count of items whose cumulative height fills `vh` rows,
/// guaranteeing backward progress of at least one item (until index 0).
fn page_up_target(state: &mut ListState, from: usize, visible_height: u32) -> usize {
    state.ensure_row_prefix();
    let n = state.visible_indices().len();
    if n == 0 {
        return 0;
    }
    let from = from.min(n - 1);
    let row_prefix = state.row_prefix();
    let from_bottom = row_prefix[from + 1];
    let target = from_bottom.saturating_sub(visible_height.max(1));
    let prev = item_at_row(row_prefix, target, n);
    prev.min(from.saturating_sub(1))
}

#[cfg(test)]
mod list_reorder_render_tests {
    use crate::widgets::ListState;
    use crate::{EventBuilder, KeyCode, KeyModifiers, TestBackend};

    #[test]
    fn shift_down_reorders_selected_item() {
        let mut backend = TestBackend::new(20, 6);
        let mut state = ListState::new(vec!["alpha", "beta", "gamma"]);
        state.selected = 0; // "alpha"

        let events = EventBuilder::new()
            .key_with(KeyCode::Down, KeyModifiers::SHIFT)
            .build();

        let mut reordered = None;
        backend.run_with_events(events, |ui| {
            let r = ui.list_reorderable(&mut state);
            reordered = r.reordered;
        });

        // "alpha" (data 0) swapped down with "beta" (data 1).
        assert_eq!(reordered, Some((0, 1)));
        assert_eq!(state.items(), ["beta", "alpha", "gamma"]);
        // Selection follows the moved item to its new position.
        assert_eq!(state.selected, 1);
        assert_eq!(state.selected_item(), Some("alpha"));
    }

    #[test]
    fn alt_up_reorders_selected_item() {
        let mut backend = TestBackend::new(20, 6);
        let mut state = ListState::new(vec!["one", "two", "three"]);
        state.selected = 2; // "three"

        let events = EventBuilder::new()
            .key_with(KeyCode::Up, KeyModifiers::ALT)
            .build();

        let mut reordered = None;
        backend.run_with_events(events, |ui| {
            reordered = ui.list_reorderable(&mut state).reordered;
        });

        assert_eq!(reordered, Some((2, 1)));
        assert_eq!(state.items(), ["one", "three", "two"]);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn shift_up_at_top_is_a_noop() {
        let mut backend = TestBackend::new(20, 6);
        let mut state = ListState::new(vec!["a", "b", "c"]);
        state.selected = 0;

        let events = EventBuilder::new()
            .key_with(KeyCode::Up, KeyModifiers::SHIFT)
            .build();

        let mut reordered = Some((9, 9));
        backend.run_with_events(events, |ui| {
            reordered = ui.list_reorderable(&mut state).reordered;
        });

        // No room to move up from the top: nothing reordered.
        assert_eq!(reordered, None);
        assert_eq!(state.items(), ["a", "b", "c"]);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn plain_down_navigates_without_reordering() {
        let mut backend = TestBackend::new(20, 6);
        let mut state = ListState::new(vec!["a", "b", "c"]);
        state.selected = 0;

        let events = EventBuilder::new().key_code(KeyCode::Down).build();

        let mut reordered = Some((9, 9));
        backend.run_with_events(events, |ui| {
            reordered = ui.list_reorderable(&mut state).reordered;
        });

        // Without a modifier, Down moves the selection but never reorders.
        assert_eq!(reordered, None);
        assert_eq!(state.items(), ["a", "b", "c"]);
        assert_eq!(state.selected, 1);
    }
}
