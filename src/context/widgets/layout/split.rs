// Split-pane container widgets — `split_pane` (horizontal) and
// `vsplit_pane` (vertical). Each renders two panes separated by a 1-cell
// drag handle. Mouse drag and arrow-key adjustment update the stored ratio.
//
// Introduced in v0.20.0 (#223).

use super::*;

/// Keyboard step applied to `state.ratio` per arrow-key press when the handle is focused.
const KEY_STEP: f64 = 0.05;

/// Scale factor applied to the `[0.0, 1.0]` ratio to produce a `u16` flexbox
/// `grow` weight. 1000 gives ~0.1% precision in pane sizes — finer than any
/// terminal cell can render at typical widths, while staying well below
/// `u16::MAX` so the two-pane sum can never overflow.
const RATIO_GROW_SCALE: f64 = 1000.0;

/// Direction of the split. Internal helper — public API is the `split_pane`
/// (horizontal) / `vsplit_pane` (vertical) entry points.
#[derive(Debug, Clone, Copy)]
enum SplitOrientation {
    /// Horizontal split: left | handle | right.
    Horizontal,
    /// Vertical split: top / handle / bottom.
    Vertical,
}

impl Context {
    /// Horizontal split container with a draggable handle.
    ///
    /// Renders `left | │ | right`, where `│` is a 1-cell wide drag handle.
    /// The handle is focusable; arrow keys (`Left`/`Right`) adjust the
    /// ratio by 5% per press, and dragging the handle with the mouse
    /// updates the ratio proportionally to the cursor's x position.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::SplitPaneState;
    /// # let mut split = SplitPaneState::new(0.5);
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.split_pane(
    ///     &mut split,
    ///     |ui| { ui.text("left pane"); },
    ///     |ui| { ui.text("right pane"); },
    /// );
    /// # });
    /// ```
    pub fn split_pane<L, R>(
        &mut self,
        state: &mut SplitPaneState,
        left: L,
        right: R,
    ) -> SplitPaneResponse
    where
        L: FnOnce(&mut Context),
        R: FnOnce(&mut Context),
    {
        self.split_pane_impl(SplitOrientation::Horizontal, state, left, right)
    }

    /// Vertical split container with a draggable handle.
    ///
    /// Mirrors [`Self::split_pane`] but stacks the panes vertically with a
    /// 1-row horizontal divider (`─`) between them. The handle is focusable;
    /// arrow keys (`Up`/`Down`) adjust the ratio by 5% per press.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::SplitPaneState;
    /// # let mut split = SplitPaneState::new(0.5);
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.vsplit_pane(
    ///     &mut split,
    ///     |ui| { ui.text("top pane"); },
    ///     |ui| { ui.text("bottom pane"); },
    /// );
    /// # });
    /// ```
    pub fn vsplit_pane<T, B>(
        &mut self,
        state: &mut SplitPaneState,
        top: T,
        bottom: B,
    ) -> SplitPaneResponse
    where
        T: FnOnce(&mut Context),
        B: FnOnce(&mut Context),
    {
        self.split_pane_impl(SplitOrientation::Vertical, state, top, bottom)
    }

    fn split_pane_impl<A, B>(
        &mut self,
        orientation: SplitOrientation,
        state: &mut SplitPaneState,
        first: A,
        second: B,
    ) -> SplitPaneResponse
    where
        A: FnOnce(&mut Context),
        B: FnOnce(&mut Context),
    {
        let old_ratio = state.ratio;
        // Reserve the focusable slot for the handle BEFORE the panes so
        // tab order stays stable across frames regardless of pane content.
        let handle_focused = self.register_focusable();
        let focus_marker = if matches!(self.commands.last(), Some(Command::FocusMarker(_))) {
            self.commands.pop()
        } else {
            None
        };
        let (gained_focus, lost_focus) = self.focus_transitions(handle_focused);

        // Process keyboard input (arrow keys) when the handle is focused.
        if handle_focused {
            self.consume_split_pane_keys(state, orientation);
        }

        // Reserve the divider independently of the number of widgets in either pane.
        let handle_interaction_id = self.reserve_interaction_slot();
        let outer_interaction_id = self.rollback.interaction_count;
        self.consume_split_pane_drag(
            state,
            handle_interaction_id,
            outer_interaction_id,
            orientation,
        );

        let theme = self.theme;
        let ratio = state.ratio.clamp(state.min_ratio, 1.0 - state.min_ratio);
        let left_grow = ((ratio * RATIO_GROW_SCALE).round() as u16).max(1);
        let right_grow = (((1.0 - ratio) * RATIO_GROW_SCALE).round() as u16).max(1);

        let drag_active = state.dragging;

        let mut response = match orientation {
            SplitOrientation::Horizontal => self.container().grow(1).row(|ui| {
                let _ = ui.container().basis(0).grow(left_grow).col(first);
                let handle_color = if handle_focused || drag_active {
                    theme.accent
                } else {
                    theme.border
                };
                ui.split_handle(
                    handle_interaction_id,
                    focus_marker,
                    orientation,
                    handle_color,
                );
                let _ = ui.container().basis(0).grow(right_grow).col(second);
            }),
            SplitOrientation::Vertical => self.container().grow(1).col(|ui| {
                let _ = ui.container().basis(0).grow(left_grow).col(first);
                let handle_color = if handle_focused || drag_active {
                    theme.accent
                } else {
                    theme.border
                };
                ui.split_handle(
                    handle_interaction_id,
                    focus_marker,
                    orientation,
                    handle_color,
                );
                let _ = ui.container().basis(0).grow(right_grow).col(second);
            }),
        };
        response.focused = handle_focused;
        response.gained_focus = gained_focus;
        response.lost_focus = lost_focus;
        response.changed = state.ratio != old_ratio;

        SplitPaneResponse {
            response,
            ratio,
            drag_active,
        }
    }

    fn split_handle(
        &mut self,
        interaction_id: usize,
        focus_marker: Option<Command>,
        orientation: SplitOrientation,
        color: Color,
    ) {
        if let Some(marker) = focus_marker {
            self.commands.push(marker);
        }
        self.commands
            .push(Command::InteractionMarker(interaction_id));
        let (constraints, glyph) = match orientation {
            SplitOrientation::Horizontal => (Constraints::default().w(1).h_pct(100), '│'),
            SplitOrientation::Vertical => (Constraints::default().h(1).w_pct(100), '─'),
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
                border_style: Style::new(),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints,
                title: None,
                grow: 0,
                group_name: None,
            })));
        self.container().grow(1).draw(move |buffer, rect| {
            for y in rect.y..rect.bottom() {
                for x in rect.x..rect.right() {
                    buffer.set_char(x, y, glyph, Style::new().fg(color));
                }
            }
        });
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;
    }

    fn consume_split_pane_keys(
        &mut self,
        state: &mut SplitPaneState,
        orientation: SplitOrientation,
    ) {
        // Hoist the orientation-dependent key codes outside the per-event
        // loop so the match runs once per call, not once per pending key.
        let (neg, pos) = match orientation {
            SplitOrientation::Horizontal => (KeyCode::Left, KeyCode::Right),
            SplitOrientation::Vertical => (KeyCode::Up, KeyCode::Down),
        };
        let mut consumed: Vec<usize> = Vec::new();
        let mut delta = 0.0_f64;
        for (i, key) in self.available_key_presses() {
            if key.code == neg {
                delta -= KEY_STEP;
                consumed.push(i);
            } else if key.code == pos {
                delta += KEY_STEP;
                consumed.push(i);
            }
        }
        // Use abs/EPSILON instead of `!= 0.0` for clarity; behavior is
        // unchanged for the realistic input range (delta is a sum of exact
        // 0.05 increments, so any non-zero result is well above EPSILON).
        if delta.abs() > f64::EPSILON {
            state.set_ratio(state.ratio + delta);
        }
        self.consume_indices(consumed);
    }

    fn consume_split_pane_drag(
        &mut self,
        state: &mut SplitPaneState,
        handle_interaction_id: usize,
        outer_interaction_id: usize,
        orientation: SplitOrientation,
    ) {
        if !self.interaction_allowed() {
            state.dragging = false;
            return;
        }
        if self.events.is_empty() {
            return;
        }
        let outer_rect = self.prev_allocated_areas.get(outer_interaction_id).copied();
        let handle_rect = self.prev_hit_map.get(handle_interaction_id).copied();
        let logical_handle = self
            .prev_allocated_areas
            .get(handle_interaction_id)
            .copied();
        let mut offsets = (0_i64, 0_i64);
        let mut parents = Vec::new();
        for command in &self.commands {
            match command {
                Command::BeginContainer(_) => parents.push(offsets),
                Command::BeginScrollable(args) => {
                    parents.push(offsets);
                    match args.direction {
                        Direction::Row => offsets.0 += i64::from(args.scroll_offset_x),
                        Direction::Column => offsets.1 += i64::from(args.scroll_offset),
                    }
                }
                Command::BeginOverlay { .. } => {
                    parents.push(offsets);
                    offsets = (0, 0);
                }
                Command::EndContainer | Command::EndOverlay => {
                    offsets = parents.pop().unwrap_or_default();
                }
                _ => {}
            }
        }
        // A visible one-cell divider gives an exact previous-frame translation,
        // even when an ancestor clips the outer pane. The active scroll stack
        // is the fallback while a captured divider is outside the viewport.
        if let (Some(hit), Some(logical)) = (handle_rect, logical_handle)
            && !hit.is_empty()
        {
            match orientation {
                SplitOrientation::Horizontal => offsets.0 = i64::from(logical.x) - i64::from(hit.x),
                SplitOrientation::Vertical => offsets.1 = i64::from(logical.y) - i64::from(hit.y),
            }
        }

        let mut consumed: Vec<usize> = Vec::new();
        let events: Vec<(usize, crate::event::MouseEvent)> = self
            .events
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match e {
                Event::Mouse(m) if !self.consumed[i] => Some((i, m.clone())),
                _ => None,
            })
            .collect();

        for (i, mouse) in events {
            match mouse.kind {
                MouseKind::Down(MouseButton::Left) => {
                    if let Some(rect) = handle_rect
                        && rect.width > 0
                        && mouse.x >= rect.x
                        && mouse.x < rect.right()
                        && mouse.y >= rect.y
                        && mouse.y < rect.bottom()
                    {
                        state.dragging = true;
                        consumed.push(i);
                    }
                }
                MouseKind::Drag(MouseButton::Left) if state.dragging => {
                    if let Some(outer) = outer_rect {
                        let new_ratio = match orientation {
                            SplitOrientation::Horizontal => {
                                if outer.width <= 1 {
                                    state.ratio
                                } else {
                                    let rel = (i64::from(mouse.x) + offsets.0 - i64::from(outer.x))
                                        .clamp(0, i64::from(outer.width - 1));
                                    rel as f64 / f64::from(outer.width - 1)
                                }
                            }
                            SplitOrientation::Vertical => {
                                if outer.height <= 1 {
                                    state.ratio
                                } else {
                                    let rel = (i64::from(mouse.y) + offsets.1 - i64::from(outer.y))
                                        .clamp(0, i64::from(outer.height - 1));
                                    rel as f64 / f64::from(outer.height - 1)
                                }
                            }
                        };
                        state.set_ratio(new_ratio);
                    }
                    consumed.push(i);
                }
                MouseKind::Up(MouseButton::Left) if state.dragging => {
                    state.dragging = false;
                    consumed.push(i);
                }
                _ => {}
            }
        }
        self.consume_indices(consumed);
    }
}
