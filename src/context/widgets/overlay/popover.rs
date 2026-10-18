//! Element-anchored floating panels (#459): placement math with edge
//! flipping and clamping, the floating-panel primitive shared by popovers,
//! menus and the select dropdown, and [`Context::popover`].

use super::*;
use crate::layout::build_tree;
use crate::widgets::PopoverResponse;

/// Side of an anchor rect a [`Context::popover`] (or menu) opens on.
///
/// The panel flips to the opposite side when it does not fit on the
/// requested one but fits (or has more room) there, and is clamped inside the
/// terminal area on the cross axis, like CSS anchor positioning with
/// `position-try-fallbacks: flip-block, flip-inline`.
///
/// | Placement | Panel position |
/// |---|---|
/// | `Below` | under the anchor, left edges aligned |
/// | `BelowEnd` | under the anchor, right edges aligned |
/// | `Above` | over the anchor, left edges aligned |
/// | `AboveEnd` | over the anchor, right edges aligned |
/// | `Right` | right of the anchor, top edges aligned |
/// | `Left` | left of the anchor, top edges aligned |
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Placement {
    /// Below the anchor, left edges aligned (default).
    #[default]
    Below,
    /// Below the anchor, right edges aligned.
    BelowEnd,
    /// Above the anchor, left edges aligned.
    Above,
    /// Above the anchor, right edges aligned.
    AboveEnd,
    /// Right of the anchor, top edges aligned.
    Right,
    /// Left of the anchor, top edges aligned.
    Left,
}

/// Where a panel of `size` goes for `anchor` and `placement` inside `area`.
///
/// Returns the panel rect (always inside `area`, shrunk to it when larger)
/// and the placement actually used after flipping.
pub(crate) fn place_popover(
    anchor: Rect,
    size: (u32, u32),
    placement: Placement,
    area: Rect,
) -> (Rect, Placement) {
    let width = size.0.min(area.width);
    let height = size.1.min(area.height);
    let clamp = |value: u32, lo: u32, extent: u32, len: u32| -> u32 {
        let hi = lo.saturating_add(extent.saturating_sub(len));
        value.clamp(lo, hi)
    };
    // Prefer the requested side when it fits; otherwise the other side when
    // it fits; otherwise whichever side has more room (ties keep the request).
    let choose = |preferred_room: u32, other_room: u32, len: u32| -> bool {
        len <= preferred_room || (len > other_room && preferred_room >= other_room)
    };
    match placement {
        Placement::Below | Placement::BelowEnd | Placement::Above | Placement::AboveEnd => {
            let room_below = area.bottom().saturating_sub(anchor.bottom());
            let room_above = anchor.y.saturating_sub(area.y);
            let wants_below = matches!(placement, Placement::Below | Placement::BelowEnd);
            let below = if wants_below {
                choose(room_below, room_above, height)
            } else {
                !choose(room_above, room_below, height)
            };
            let end = matches!(placement, Placement::BelowEnd | Placement::AboveEnd);
            let y = if below {
                anchor.bottom()
            } else {
                anchor.y.saturating_sub(height)
            };
            let x = if end {
                anchor.right().saturating_sub(width)
            } else {
                anchor.x
            };
            let resolved = match (below, end) {
                (true, false) => Placement::Below,
                (true, true) => Placement::BelowEnd,
                (false, false) => Placement::Above,
                (false, true) => Placement::AboveEnd,
            };
            (
                Rect::new(
                    clamp(x, area.x, area.width, width),
                    clamp(y, area.y, area.height, height),
                    width,
                    height,
                ),
                resolved,
            )
        }
        Placement::Right | Placement::Left => {
            let room_right = area.right().saturating_sub(anchor.right());
            let room_left = anchor.x.saturating_sub(area.x);
            let right = if placement == Placement::Right {
                choose(room_right, room_left, width)
            } else {
                !choose(room_left, room_right, width)
            };
            let x = if right {
                anchor.right()
            } else {
                anchor.x.saturating_sub(width)
            };
            (
                Rect::new(
                    clamp(x, area.x, area.width, width),
                    clamp(anchor.y, area.y, area.height, height),
                    width,
                    height,
                ),
                if right {
                    Placement::Right
                } else {
                    Placement::Left
                },
            )
        }
    }
}

impl Context {
    /// Full terminal area in screen cells.
    pub(crate) fn screen_area(&self) -> Rect {
        Rect::new(0, 0, self.area_width, self.area_height)
    }

    /// Render `f` in a bordered floating panel at exactly `rect`.
    ///
    /// The panel lives in its own overlay layer, so it paints over the base
    /// layout without reflowing it and blocks clicks on what it covers. With
    /// `backdrop`, the rest of the screen also swallows presses for this
    /// frame's hit map (light dismiss that does not click through).
    pub(crate) fn floating_panel(
        &mut self,
        rect: Rect,
        backdrop: bool,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        let area = self.screen_area();
        let border = self.theme.border;
        let surface = self.theme.surface;
        let mut panel = Response::none();
        let _ = self.overlay(|ui| {
            let wrapper = ui.container().w(area.width).h(area.height);
            let wrapper = if backdrop {
                wrapper
            } else {
                wrapper.hit_passthrough()
            };
            let _ = wrapper.col(|ui| {
                panel = ui
                    .container()
                    .ml(rect.x)
                    .mt(rect.y)
                    .w(rect.width)
                    .h(rect.height)
                    .border(Border::Rounded)
                    .border_fg(border)
                    .bg(surface)
                    .col(f);
            });
        });
        panel
    }

    /// Measure the natural size of the container recorded at
    /// `commands[start..]` (cells, clamped to the terminal area).
    fn measure_recorded(&self, start: usize) -> (u32, u32) {
        let mut commands: Vec<Command> = self.commands[start..].to_vec();
        let mut root = build_tree(&mut commands);
        let width = root.min_width().min(self.area_width);
        let height = root.min_height_for_width(width).min(self.area_height);
        (width, height)
    }

    /// Move the first container recorded at or after `start` to `rect`.
    fn place_recorded(&mut self, start: usize, rect: Rect) {
        let Some(index) = (start..self.commands.len())
            .find(|&index| matches!(self.commands[index], Command::BeginContainer(_)))
        else {
            return;
        };
        if let Command::BeginContainer(args) = &mut self.commands[index] {
            args.margin.left = rect.x;
            args.margin.top = rect.y;
            args.constraints = args.constraints.w(rect.width).h(rect.height);
        }
        self.invalidate_geometry_cache_from(index);
    }

    /// Render floating content next to an element, like a CSS
    /// anchor-positioned popover.
    ///
    /// `anchor` is usually the [`Response::rect`] of the widget the popover
    /// belongs to. The content is drawn in a bordered panel (one cell of
    /// horizontal padding) on the `placement` side of the anchor. Note that a
    /// widget in a column stretches to the column's width, so its rect may be
    /// wider than its text. When the panel does not fit, it flips to
    /// the opposite side, and it is clamped inside the terminal area, so a
    /// popover for a widget near the bottom edge opens above it. The panel
    /// floats above the layout (opening it never moves other widgets) and
    /// blocks clicks on what it covers.
    ///
    /// A popover is shown while you call this method. Close it when the
    /// returned [`PopoverResponse::dismissed`] is set: `Esc` was pressed, or a
    /// mouse press landed outside both the panel and `anchor` (that press
    /// still reaches the widget under the pointer, like HTML
    /// `popover="auto"`). For menus that swallow the outside click and own
    /// the keyboard, use [`context_menu`](Self::context_menu) or
    /// [`menu_bar`](Self::menu_bar).
    ///
    /// `Response::rect` is the previous frame's layout, so on the very first
    /// frame a widget is drawn its rect is empty and the popover opens at the
    /// top-left; it settles next frame.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::{Placement, TestBackend};
    ///
    /// let mut open = true;
    /// let mut tb = TestBackend::new(40, 10);
    /// for _ in 0..2 {
    ///     tb.render(|ui| {
    ///         let info = ui.button("Info").show();
    ///         if info.clicked {
    ///             open = !open;
    ///         }
    ///         if open {
    ///             let pop = ui.popover(info.rect, Placement::Below, |ui| {
    ///                 ui.text("Saved 3 files");
    ///             });
    ///             if pop.dismissed {
    ///                 open = false;
    ///             }
    ///         }
    ///     });
    /// }
    /// tb.assert_contains("Saved 3 files");
    /// ```
    pub fn popover(
        &mut self,
        anchor: Rect,
        placement: Placement,
        f: impl FnOnce(&mut Context),
    ) -> PopoverResponse {
        let area = self.screen_area();
        let border = self.theme.border;
        let surface = self.theme.surface;
        // A stable identity for the panel slot (anchor + side), so the
        // previous frame tells whether the popover was already showing.
        let key = crate::runtime::hashed_focus_key((
            "slt-popover",
            anchor.x,
            anchor.y,
            anchor.width,
            anchor.height,
            placement,
        ));
        let previous_panel = self
            .pointer
            .slot_keys_prev
            .iter()
            .find(|(_, owner)| *owner == key)
            .and_then(|(slot, _)| self.prev_hit_map.get(*slot).copied());

        let mut out = PopoverResponse {
            placement,
            ..PopoverResponse::default()
        };
        let _ = self.overlay(|ui| {
            let _ = ui
                .container()
                .w(area.width)
                .h(area.height)
                .hit_passthrough()
                .col(|ui| {
                    let start = ui.commands.len();
                    ui.pointer.pending_key = Some(key);
                    out.response = ui
                        .container()
                        .border(Border::Rounded)
                        .border_fg(border)
                        .bg(surface)
                        .px(1)
                        .col(f);
                    let size = ui.measure_recorded(start);
                    let (rect, resolved) = place_popover(anchor, size, placement, area);
                    ui.place_recorded(start, rect);
                    out.placement = resolved;
                });
        });

        if self.interaction_allowed() {
            let mut dismiss_keys: smallvec::SmallVec<[usize; 2]> = smallvec::SmallVec::new();
            for (index, key) in self.available_key_presses() {
                if key.code == KeyCode::Esc {
                    dismiss_keys.push(index);
                }
            }
            if !dismiss_keys.is_empty() {
                out.dismissed = true;
                self.consume_indices(dismiss_keys);
            }
            if let Some(panel) = previous_panel {
                let outside = self.pointer.presses.iter().any(|press| {
                    !self
                        .consumed
                        .get(press.event_index)
                        .copied()
                        .unwrap_or(true)
                        && !panel.contains(press.pos.0, press.pos.1)
                        && !anchor.contains(press.pos.0, press.pos.1)
                });
                out.dismissed |= outside;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect::new(0, 0, 40, 20);

    #[test]
    fn below_fits_stays_below() {
        let (rect, side) = place_popover(Rect::new(5, 3, 10, 1), (12, 5), Placement::Below, AREA);
        assert_eq!(side, Placement::Below);
        assert_eq!(rect, Rect::new(5, 4, 12, 5));
    }

    #[test]
    fn below_near_bottom_flips_above() {
        let (rect, side) = place_popover(Rect::new(5, 17, 10, 1), (12, 5), Placement::Below, AREA);
        assert_eq!(side, Placement::Above);
        assert_eq!(rect, Rect::new(5, 12, 12, 5));
    }

    #[test]
    fn above_near_top_flips_below() {
        let (rect, side) = place_popover(Rect::new(5, 1, 10, 1), (12, 5), Placement::Above, AREA);
        assert_eq!(side, Placement::Below);
        assert_eq!(rect, Rect::new(5, 2, 12, 5));
    }

    #[test]
    fn right_edge_clamps_inside() {
        let (rect, side) = place_popover(Rect::new(35, 3, 4, 1), (12, 5), Placement::Below, AREA);
        assert_eq!(side, Placement::Below);
        assert_eq!(rect, Rect::new(28, 4, 12, 5));
        assert!(rect.right() <= AREA.right());
    }

    #[test]
    fn below_end_aligns_right_edges_and_clamps_left() {
        let (rect, _) = place_popover(Rect::new(20, 3, 10, 1), (6, 2), Placement::BelowEnd, AREA);
        assert_eq!(rect, Rect::new(24, 4, 6, 2));
        let (rect, _) = place_popover(Rect::new(0, 3, 3, 1), (6, 2), Placement::BelowEnd, AREA);
        assert_eq!(rect.x, 0, "clamped at the left edge");
    }

    #[test]
    fn right_flips_left_at_the_right_edge() {
        let (rect, side) = place_popover(Rect::new(30, 4, 6, 1), (8, 3), Placement::Right, AREA);
        assert_eq!(side, Placement::Left);
        assert_eq!(rect, Rect::new(22, 4, 8, 3));
    }

    #[test]
    fn left_flips_right_at_the_left_edge() {
        let (rect, side) = place_popover(Rect::new(2, 4, 6, 1), (8, 3), Placement::Left, AREA);
        assert_eq!(side, Placement::Right);
        assert_eq!(rect, Rect::new(8, 4, 8, 3));
    }

    #[test]
    fn side_placement_clamps_vertically_at_the_bottom() {
        let (rect, _) = place_popover(Rect::new(2, 18, 6, 1), (8, 5), Placement::Right, AREA);
        assert_eq!(rect.bottom(), AREA.bottom());
    }

    #[test]
    fn neither_side_fits_picks_more_room_and_clamps() {
        // 12 rows above, 7 below, 15 tall: neither fits, above has more room.
        let (rect, side) = place_popover(Rect::new(0, 12, 4, 1), (10, 15), Placement::Below, AREA);
        assert_eq!(side, Placement::Above);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.height, 15);
    }

    #[test]
    fn oversized_panel_shrinks_to_the_area() {
        let (rect, _) = place_popover(Rect::new(0, 0, 1, 1), (80, 50), Placement::Below, AREA);
        assert_eq!(rect, AREA);
    }
}
