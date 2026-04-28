// Split-pane container widgets — `split_pane` (horizontal) and
// `vsplit_pane` (vertical). Each renders two panes separated by a 1-cell
// drag handle. Mouse drag and arrow-key adjustment update the stored ratio.
//
// Introduced in v0.20.0 (#223).

use super::*;

/// Keyboard step applied to `state.ratio` per arrow-key press when the handle is focused.
const KEY_STEP: f32 = 0.05;

/// Scale factor applied to the `[0.0, 1.0]` ratio to produce a `u16` flexbox
/// `grow` weight. 1000 gives ~0.1% precision in pane sizes — finer than any
/// terminal cell can render at typical widths, while staying well below
/// `u16::MAX` so the two-pane sum can never overflow.
const RATIO_GROW_SCALE: f32 = 1000.0;

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
    /// 1-row horizontal divider (`─`) between them.
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
        // Reserve the focusable slot for the handle BEFORE the panes so
        // tab order stays stable across frames regardless of pane content.
        let handle_focused = self.register_focusable();

        // Process keyboard input (arrow keys) when the handle is focused.
        if handle_focused {
            self.consume_split_pane_keys(state, orientation);
        }

        // Process mouse drag against the previous-frame handle rect.
        // The handle interaction id is the next slot to be allocated when
        // we render the splitter cell below; record it now so we can match
        // mouse events with the prior frame's rect.
        let handle_interaction_id = self.rollback.interaction_count;
        self.consume_split_pane_drag(state, handle_interaction_id, orientation);

        let theme = self.theme;
        let ratio = state.ratio.clamp(state.min_ratio, 1.0 - state.min_ratio);
        let left_grow = ((ratio * RATIO_GROW_SCALE).round() as u16).max(1);
        let right_grow = (((1.0 - ratio) * RATIO_GROW_SCALE).round() as u16).max(1);

        let drag_active = state.dragging;

        let response = match orientation {
            SplitOrientation::Horizontal => self.row(|ui| {
                let _ = ui.container().grow(left_grow).col(first);
                let handle_color = if handle_focused || drag_active {
                    theme.accent
                } else {
                    theme.border
                };
                let _ = ui.container().w(1).grow(0).col(|ui| {
                    ui.styled("│", Style::new().fg(handle_color));
                });
                let _ = ui.container().grow(right_grow).col(second);
            }),
            SplitOrientation::Vertical => self.col(|ui| {
                let _ = ui.container().grow(left_grow).col(first);
                let handle_color = if handle_focused || drag_active {
                    theme.accent
                } else {
                    theme.border
                };
                let _ = ui.container().h(1).grow(0).col(|ui| {
                    ui.styled("─", Style::new().fg(handle_color));
                });
                let _ = ui.container().grow(right_grow).col(second);
            }),
        };

        SplitPaneResponse {
            response,
            ratio,
            drag_active,
        }
    }

    fn consume_split_pane_keys(
        &mut self,
        state: &mut SplitPaneState,
        orientation: SplitOrientation,
    ) {
        let mut consumed: Vec<usize> = Vec::new();
        let mut delta = 0.0_f32;
        for (i, key) in self.available_key_presses() {
            let is_left = matches!(key.code, KeyCode::Left);
            let is_right = matches!(key.code, KeyCode::Right);
            let is_up = matches!(key.code, KeyCode::Up);
            let is_down = matches!(key.code, KeyCode::Down);
            match orientation {
                SplitOrientation::Horizontal => {
                    if is_left {
                        delta -= KEY_STEP;
                        consumed.push(i);
                    } else if is_right {
                        delta += KEY_STEP;
                        consumed.push(i);
                    }
                }
                SplitOrientation::Vertical => {
                    if is_up {
                        delta -= KEY_STEP;
                        consumed.push(i);
                    } else if is_down {
                        delta += KEY_STEP;
                        consumed.push(i);
                    }
                }
            }
        }
        if delta != 0.0 {
            state.set_ratio(state.ratio + delta);
        }
        self.consume_indices(consumed);
    }

    fn consume_split_pane_drag(
        &mut self,
        state: &mut SplitPaneState,
        handle_interaction_id: usize,
        orientation: SplitOrientation,
    ) {
        // The container that owns the panes has its own interaction id
        // allocated by `row()` / `col()` later in this method. To compute the
        // ratio we need the bounds of THAT container, but at this point in
        // execution we haven't pushed it yet. Instead, we track drag activity
        // against the handle's own rect from the previous frame and use the
        // larger axis bound from the previous handle position to anchor the
        // drag math. Concretely:
        //
        //   - On Mouse::Down inside the handle rect → enter drag mode.
        //   - While dragging, update ratio based on the cursor's position
        //     within the previous outer container rect (interaction_id - 1
        //     for the row/col that hosts the handle).
        //   - On Mouse::Up → exit drag mode.
        //
        // The container's interaction id is allocated AFTER the handle, but
        // the previous-frame `prev_hit_map` already has both. We resolve the
        // outer container by `handle_interaction_id - 1` — the splitter ran
        // last frame too and the slots are stable.
        let outer_id = handle_interaction_id.saturating_sub(1);
        let outer_rect = self.prev_hit_map.get(outer_id).copied();
        let handle_rect = self.prev_hit_map.get(handle_interaction_id).copied();

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
                    if let Some(rect) = handle_rect {
                        if rect.width > 0
                            && mouse.x >= rect.x
                            && mouse.x < rect.right()
                            && mouse.y >= rect.y
                            && mouse.y < rect.bottom()
                        {
                            state.dragging = true;
                            consumed.push(i);
                        }
                    }
                }
                MouseKind::Drag(MouseButton::Left) if state.dragging => {
                    if let Some(outer) = outer_rect {
                        let new_ratio = match orientation {
                            SplitOrientation::Horizontal => {
                                if outer.width <= 1 {
                                    state.ratio
                                } else {
                                    let rel = mouse
                                        .x
                                        .saturating_sub(outer.x)
                                        .min(outer.width.saturating_sub(1));
                                    rel as f32 / outer.width as f32
                                }
                            }
                            SplitOrientation::Vertical => {
                                if outer.height <= 1 {
                                    state.ratio
                                } else {
                                    let rel = mouse
                                        .y
                                        .saturating_sub(outer.y)
                                        .min(outer.height.saturating_sub(1));
                                    rel as f32 / outer.height as f32
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
