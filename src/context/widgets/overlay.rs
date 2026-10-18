//! Overlay family — modals, overlays, anchored placement, popovers, menus
//! and tooltips.
//!
//! These widgets draw above the normal layout flow. The menu state types
//! ([`crate::widgets::MenuState`], [`crate::widgets::MenuBarState`]) live in
//! `src/widgets/overlay.rs`.

use super::*;

mod menu;
mod popover;
mod tooltip_wrap;

pub use popover::Placement;
pub(crate) use popover::place_popover;

pub(in crate::context) use tooltip_wrap::wrap_tooltip_text;

/// Options for [`Context::modal_with`].
///
/// Controls focus trapping and placement of a modal overlay.
/// [`Context::modal`] is exactly `modal_with(ModalOpts::default(), f)`.
/// Build options with the chainable setters; the struct is
/// `#[non_exhaustive]`, so fields can be added without breaking callers.
///
/// | Field | Default | Meaning |
/// |---|---|---|
/// | [`tab_trap`](Self::tab_trap) | `true` | Tab / Shift+Tab and stale programmatic focus stay inside the modal |
/// | [`anchor`](Self::anchor) | `None` | `None` centers the content; `Some(anchor)` pins it like [`Context::overlay_at`] |
/// | [`offset`](Self::offset) | `(0, 0)` | Cell inset from the anchored edge, see [`Context::overlay_at_offset`] |
///
/// # Example
///
/// ```no_run
/// use slt::{Anchor, ModalOpts};
/// # let mut show = true;
/// # slt::run(|ui: &mut slt::Context| {
/// if show {
///     // Pinned two cells in from the top-right corner, focus trapped.
///     let opts = ModalOpts::new().anchor(Anchor::TopRight).offset(2, 1);
///     let _ = ui.modal_with(opts, |ui| {
///         ui.text("Are you sure?");
///         if ui.button("OK").clicked { show = false; }
///     });
/// }
/// # });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModalOpts {
    /// When `true`, Tab/Shift+Tab navigation cannot leave the modal's focus
    /// range, even if [`Context::set_focus_index`] or a mouse click moved
    /// focus outside.
    ///
    /// Default: `true`, aligned with WCAG 2.1 SC 2.4.3 (Focus Order), which
    /// recommends trapping focus inside modal dialogs. Set to `false` for the
    /// pre-0.26 `modal()` behavior where stale programmatic focus could point
    /// outside the modal.
    pub tab_trap: bool,
    /// Where the modal content sits. `None` (default) centers the content in
    /// the terminal area; `Some(anchor)` pins it to one of the nine compass
    /// positions.
    pub anchor: Option<Anchor>,
    /// Cell inset `(dx, dy)` from the anchored edge. Positive values move
    /// toward the center. Only used when [`anchor`](Self::anchor) is set.
    pub offset: (i32, i32),
}

impl Default for ModalOpts {
    fn default() -> Self {
        Self {
            tab_trap: true,
            anchor: None,
            offset: (0, 0),
        }
    }
}

impl ModalOpts {
    /// Default options: focus trapped, content centered.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable focus trapping (default `true`).
    pub fn tab_trap(mut self, on: bool) -> Self {
        self.tab_trap = on;
        self
    }

    /// Pin the modal to `anchor` instead of centering it.
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    /// Inset the anchored modal by `(dx, dy)` cells toward the center.
    pub fn offset(mut self, dx: i32, dy: i32) -> Self {
        self.offset = (dx, dy);
        self
    }
}

/// Former name of [`ModalOpts`].
#[deprecated(
    since = "0.26.0",
    note = "renamed to `ModalOpts`; build it with `ModalOpts::new()`"
)]
pub type ModalOptions = ModalOpts;

/// Compass-rose anchor for [`Context::overlay_at`] / [`Context::modal_at`].
///
/// Each variant maps to a (cross-axis [`Align`], main-axis [`Justify`]) pair
/// that pins overlay content to the requested screen position. The `_at`
/// helpers expand to a full-screen wrapper (so flexbox has slack to push
/// against), then place the user's content per the selected anchor.
///
/// ```no_run
/// # use slt::Anchor;
/// # slt::run(|ui: &mut slt::Context| {
/// ui.overlay_at(Anchor::BottomRight, |ui| {
///     ui.text("v0.19.3").dim();
/// });
/// # });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Top-left corner.
    TopLeft,
    /// Top edge, horizontally centered.
    TopCenter,
    /// Top-right corner.
    TopRight,
    /// Left edge, vertically centered.
    CenterLeft,
    /// Screen center.
    Center,
    /// Right edge, vertically centered.
    CenterRight,
    /// Bottom-left corner.
    BottomLeft,
    /// Bottom edge, horizontally centered.
    BottomCenter,
    /// Bottom-right corner.
    BottomRight,
}

/// Map [`Anchor`] to the wrapper column's (cross-axis align, main-axis justify).
///
/// The inner column is `Direction::Column`, so:
///   - `Justify` controls the vertical (main-axis) position.
///   - `Align`   controls the horizontal (cross-axis) position.
fn anchor_to_align_justify(anchor: Anchor) -> (Align, Justify) {
    match anchor {
        Anchor::TopLeft => (Align::Start, Justify::Start),
        Anchor::TopCenter => (Align::Center, Justify::Start),
        Anchor::TopRight => (Align::End, Justify::Start),
        Anchor::CenterLeft => (Align::Start, Justify::Center),
        Anchor::Center => (Align::Center, Justify::Center),
        Anchor::CenterRight => (Align::End, Justify::Center),
        Anchor::BottomLeft => (Align::Start, Justify::End),
        Anchor::BottomCenter => (Align::Center, Justify::End),
        Anchor::BottomRight => (Align::End, Justify::End),
    }
}

/// Resolve `(dx, dy)` to a [`Margin`] for the outer grow-1 anchor column,
/// given an [`Anchor`].
///
/// Sign convention: **positive `dx` / `dy` inset toward the viewport center**
/// (mirrors the CSS `inset` shorthand intuition). The margin shrinks the
/// column's slack on the side adjacent to the anchored edge, so subsequent
/// flexbox `align`/`justify` push the user's content inward by `(dx, dy)`:
///   - `BottomRight` + `(dx=2, dy=1)` → `mr=2, mb=1` (push 2 left, 1 up)
///   - `TopLeft`     + `(dx=2, dy=1)` → `ml=2, mt=1` (push 2 right, 1 down)
///   - `Center`      + `(dx=2, dy=1)` → `ml=2, mt=1` (shift 2 right, 1 down)
///   - `Center`      + `(dx=-2, dy=-1)` → `mr=2, mb=1` (shift 2 left, 1 up)
///
/// Negative values for corner / edge anchors would push the content
/// off-screen (no opposite-side slack to consume), so they are clamped to 0;
/// see [`Context::overlay_at_offset`] for the documented contract.
fn anchor_offset_to_margin(anchor: Anchor, dx: i32, dy: i32) -> Margin {
    let mut margin = Margin::default();

    // Horizontal axis: positive dx insets toward center.
    let h_anchor = match anchor {
        Anchor::TopLeft | Anchor::CenterLeft | Anchor::BottomLeft => HSide::Left,
        Anchor::TopRight | Anchor::CenterRight | Anchor::BottomRight => HSide::Right,
        Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => HSide::Center,
    };
    match h_anchor {
        HSide::Left => {
            // Anchored to left edge: positive dx pushes right via ml.
            // Negative dx would push left (offscreen) — no slack on the
            // opposite side, and `u32` margin can't represent negatives,
            // so we clamp to 0. See `Context::overlay_at_offset` doc.
            if dx > 0 {
                margin.left = dx as u32;
            }
        }
        HSide::Right => {
            // Anchored to right edge: positive dx pushes left via mr.
            if dx > 0 {
                margin.right = dx as u32;
            }
        }
        HSide::Center => {
            // Centered: positive dx shifts right (ml), negative shifts left (mr).
            if dx > 0 {
                margin.left = dx as u32;
            } else if dx < 0 {
                margin.right = dx.unsigned_abs();
            }
        }
    }

    // Vertical axis: positive dy insets toward center.
    let v_anchor = match anchor {
        Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => VSide::Top,
        Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => VSide::Bottom,
        Anchor::CenterLeft | Anchor::Center | Anchor::CenterRight => VSide::Center,
    };
    match v_anchor {
        VSide::Top => {
            if dy > 0 {
                margin.top = dy as u32;
            }
        }
        VSide::Bottom => {
            if dy > 0 {
                margin.bottom = dy as u32;
            }
        }
        VSide::Center => {
            if dy > 0 {
                margin.top = dy as u32;
            } else if dy < 0 {
                margin.bottom = dy.unsigned_abs();
            }
        }
    }

    margin
}

enum HSide {
    Left,
    Right,
    Center,
}

enum VSide {
    Top,
    Bottom,
    Center,
}

impl Context {
    /// Render content in a modal overlay with dimmed background.
    ///
    /// Exactly [`modal_with(ModalOpts::default(), f)`](Self::modal_with):
    /// focus is trapped inside the modal (WCAG 2.1 SC 2.4.3) and the content
    /// is centered. Widgets outside the modal ignore the mouse and keyboard
    /// while it is open, and focus returns to the widget that held it before
    /// the modal opened once the modal stops rendering.
    ///
    /// Since 0.26.0 the focus trap is on by default; use
    /// `modal_with(ModalOpts::new().tab_trap(false), f)` for the old behavior.
    ///
    /// ```no_run
    /// # let mut show = true;
    /// # slt::run(|ui: &mut slt::Context| {
    /// if show {
    ///     ui.modal(|ui| {
    ///         ui.text("Are you sure?");
    ///         if ui.button("OK").clicked { show = false; }
    ///     });
    /// }
    /// # });
    /// ```
    pub fn modal(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        self.modal_with(ModalOpts::default(), f)
    }

    /// Render content in a modal overlay with configurable options.
    ///
    /// See [`ModalOpts`] for the options: focus trapping (`tab_trap`, on by
    /// default) and placement (`anchor` / `offset`, centered by default).
    ///
    /// When `opts.tab_trap` is `true`, focus cannot escape the modal's
    /// focusable range — Tab/Shift+Tab keep cycling within the modal even
    /// if [`Context::set_focus_index`] or a mouse click moved focus to a
    /// background widget. WCAG 2.1 SC 2.4.3 (Focus Order) recommends
    /// trapping focus inside modal dialogs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{Anchor, Border, ModalOpts};
    /// # let mut show = true;
    /// # slt::run(|ui: &mut slt::Context| {
    /// if show {
    ///     let opts = ModalOpts::new().anchor(Anchor::TopRight).offset(2, 1);
    ///     ui.modal_with(opts, |ui| {
    ///         ui.bordered(Border::Rounded).p(1).col(|ui| {
    ///             ui.text("Saved!");
    ///             if ui.button("OK").clicked { show = false; }
    ///         });
    ///     });
    /// }
    /// # });
    /// ```
    pub fn modal_with(&mut self, opts: ModalOpts, f: impl FnOnce(&mut Context)) -> Response {
        match opts.anchor {
            None => self.modal_layer(opts.tab_trap, true, f),
            Some(anchor) => {
                let (dx, dy) = opts.offset;
                self.modal_layer(opts.tab_trap, true, |ui| {
                    let (align, justify) = anchor_to_align_justify(anchor);
                    let margin = anchor_offset_to_margin(anchor, dx, dy);
                    // See `overlay_at_offset` for why margin lives on the
                    // outer grow-1 column rather than a wrapper around `f`.
                    let _ = ui
                        .container()
                        .grow(1)
                        .align(align)
                        .justify(justify)
                        .margin(margin)
                        .hit_passthrough()
                        .col(f);
                })
            }
        }
    }

    /// Shared modal machinery: focus range bookkeeping, the focus trap, and
    /// background interaction blocking. `dim` selects whether the layer
    /// dims the content behind it (dialogs) or not (menus, #459).
    pub(crate) fn modal_layer(
        &mut self,
        tab_trap: bool,
        dim: bool,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        let interaction_id = self.next_interaction_id();
        // #439: the full-screen modal root never wins hit-testing itself.
        self.mark_hit_flags(interaction_id, HIT_PASSTHROUGH);
        // Modal ownership must not shift when unrelated widgets reserve hit IDs.
        let modal_id = self.rollback.modal_count;
        self.rollback.modal_count += 1;
        self.commands.push(Command::BeginOverlay { modal: dim });
        self.rollback.overlay_depth += 1;
        if !self.prev_modal_active && !self.rollback.modal_active {
            self.rollback.modal_restore_index = Some(self.focus_index);
        }
        self.rollback.modal_active = true;
        let previous_owner = self.rollback.current_modal_id.replace(modal_id);
        self.rollback.modal_id = Some(modal_id);
        let modal_focus_start = self.rollback.focus_count;
        self.rollback.modal_focus_start = modal_focus_start;
        let previous_start = self.prev_modal_focus_start;
        let previous_count = self.prev_focus_count;
        let previous_focus_index = self.prev_focus_index;
        let requested_focus = self.focus_index;
        if self.prev_modal_active && self.prev_modal_focus_count > 0 {
            let local = *self.rollback.modal_request_local.get_or_insert_with(|| {
                self.focus_index.saturating_sub(previous_start) % self.prev_modal_focus_count
            });
            self.focus_index = modal_focus_start.saturating_add(local);
            self.prev_modal_focus_start = modal_focus_start;
            if let Some(previous) = self.prev_focus_index
                && previous >= previous_start
                && previous - previous_start < self.prev_modal_focus_count
            {
                self.prev_focus_index =
                    Some(modal_focus_start.saturating_add(previous - previous_start));
            }
        } else {
            self.focus_index = modal_focus_start;
            self.prev_focus_count = 0;
        }

        let rendered_request = self.focus_index;
        f(self);
        let requested_in_body = self.focus_index != rendered_request;
        let modal_focus_count = self.rollback.focus_count.saturating_sub(modal_focus_start);
        if self.rollback.modal_id == Some(modal_id) {
            self.rollback.modal_focus_count = modal_focus_count;
        }

        // Tab trap: when enabled, ensure `focus_index` lies in this frame's
        // modal range `[start, start + count)`. If `set_focus_index` from a
        // previous frame (or a stale state) left focus pointing at a
        // background widget, clamp it to the first modal focusable so the
        // next [`process_focus_keys`] tick cycles cleanly within the modal.
        //
        // WCAG 2.1 SC 2.4.3 (Focus Order) requirement: the user must not be
        // able to navigate to content outside an active modal dialog.
        if tab_trap && modal_focus_count > 0 && self.rollback.modal_id == Some(modal_id) {
            let lo = modal_focus_start;
            let hi = lo.saturating_add(modal_focus_count);
            if self.focus_index < lo || self.focus_index >= hi {
                self.focus_index = lo;
            }
            // #458: never park the trap on a disabled slot (for example a
            // disabled first button when the modal opens). Move forward,
            // wrapping inside the modal, to the first enabled slot. When
            // every slot is disabled focus stays put: no modal widget reports
            // `focused`, and Tab / Shift+Tab remain no-ops inside the trap.
            let disabled = |index: usize| {
                self.focus_identity
                    .slots
                    .get(index)
                    .is_some_and(|slot| slot.disabled)
            };
            if disabled(self.focus_index)
                && let Some(enabled) = (self.focus_index..hi)
                    .chain(lo..self.focus_index)
                    .find(|&index| !disabled(index))
            {
                self.focus_index = enabled;
            }
        }

        if self.rollback.modal_id == Some(modal_id) {
            let resolved = if modal_focus_count > 0 {
                modal_focus_start
                    + self.focus_index.saturating_sub(modal_focus_start) % modal_focus_count
            } else {
                self.focus_index
            };
            if !tab_trap && !requested_in_body {
                // Keep the legacy callback-visible request untouched. The
                // frame boundary translates it into the new modal slot range.
                self.focus_index = requested_focus;
            }
            self.rollback.modal_focus_rebase = Some((self.focus_index, resolved));
        }

        self.prev_modal_focus_start = previous_start;
        self.prev_focus_count = previous_count;
        self.prev_focus_index = previous_focus_index;
        self.rollback.current_modal_id = previous_owner;
        self.rollback.overlay_depth = self.rollback.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.rollback.last_text_idx = None;
        self.response_for(interaction_id)
    }

    /// Render floating content without dimming the background.
    pub fn overlay(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.next_interaction_id();
        // #439: the overlay root spans the whole area. It is transparent to
        // hit-testing so only the overlay's actual content blocks the widgets
        // underneath (CSS `pointer-events: none` on the positioning layer).
        self.mark_hit_flags(interaction_id, HIT_PASSTHROUGH);
        self.commands.push(Command::BeginOverlay { modal: false });
        self.rollback.overlay_depth += 1;
        f(self);
        self.rollback.overlay_depth = self.rollback.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.rollback.last_text_idx = None;
        self.response_for(interaction_id)
    }

    /// Render floating content anchored to one of the 9 compass positions.
    ///
    /// Wraps [`overlay`](Self::overlay) with a full-area column that pins the
    /// content to the requested anchor via flexbox `align`/`justify`. The
    /// inner column gets `grow(1)` so the wrapper consumes the screen, giving
    /// `align`/`justify` room to push the content to the corner.
    ///
    /// ```no_run
    /// # use slt::Anchor;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.overlay_at(Anchor::TopRight, |ui| {
    ///     ui.text("0:42").bold();
    /// });
    /// # });
    /// ```
    pub fn overlay_at(&mut self, anchor: Anchor, f: impl FnOnce(&mut Context)) -> Response {
        self.overlay(|ui| {
            let (align, justify) = anchor_to_align_justify(anchor);
            let _ = ui
                .container()
                .grow(1)
                .align(align)
                .justify(justify)
                .hit_passthrough()
                .col(f);
        })
    }

    /// Render a modal overlay anchored to one of the 9 compass positions.
    ///
    /// Thin wrapper over
    /// [`modal_with(ModalOpts::new().anchor(anchor), f)`](Self::modal_with),
    /// so it traps focus like [`modal`](Self::modal).
    #[deprecated(
        since = "0.26.0",
        note = "use `modal_with(ModalOpts::new().anchor(anchor), f)`"
    )]
    pub fn modal_at(&mut self, anchor: Anchor, f: impl FnOnce(&mut Context)) -> Response {
        self.modal_with(ModalOpts::new().anchor(anchor), f)
    }

    /// Render `f` at `anchor` with cell offset `(dx, dy)` from the anchored edge.
    ///
    /// This is the SLT analog of CSS `position: absolute; top/right/bottom/left`,
    /// or Flutter's `Positioned(top:, right:, ...)`. The 9-cell [`Anchor`]
    /// chooses which edge to anchor to; `(dx, dy)` insets toward the center.
    ///
    /// # Sign convention
    /// Positive `dx` / `dy` always inset toward the viewport center. So
    /// `overlay_at_offset(Anchor::BottomRight, 2, 1, ...)` places the widget
    /// 2 cells left and 1 cell up from the bottom-right corner.
    ///
    /// For [`Anchor::Center`] (and other centered axes) negative values shift
    /// in the opposite direction — `(dx=-2, dy=-1)` shifts 2 cells left and 1
    /// cell up. For corner / edge anchors, negative values would push the
    /// content off-screen, so they are clamped to 0; use a different anchor
    /// instead of negative offsets to escape an edge.
    ///
    /// # CSS analogy
    /// ```text
    /// CSS:    place-self: end end; bottom: 1px; right: 2px;
    /// SLT:    overlay_at_offset(Anchor::BottomRight, 2, 1, |ui| { ... })
    /// ```
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::Anchor;
    /// # slt::run(|ui: &mut slt::Context| {
    /// // Inset corner badge — 2 cells from the right, 1 row from the bottom.
    /// ui.overlay_at_offset(Anchor::BottomRight, 2, 1, |ui| {
    ///     ui.text("v0.19.3").dim();
    /// });
    /// # });
    /// ```
    pub fn overlay_at_offset(
        &mut self,
        anchor: Anchor,
        dx: i32,
        dy: i32,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        self.overlay(|ui| {
            let (align, justify) = anchor_to_align_justify(anchor);
            let margin = anchor_offset_to_margin(anchor, dx, dy);
            // Apply margin on the outer (grow=1) column so flexbox's parent
            // (the synthetic overlay root) shrinks the column's area before
            // align/justify pick a position. This avoids a wrapper container
            // around `f`, which would expose a flexbox limitation where
            // `Align::End` shifts the immediate child's `pos` but does not
            // propagate the shift down to grandchildren.
            let _ = ui
                .container()
                .grow(1)
                .align(align)
                .justify(justify)
                .margin(margin)
                .hit_passthrough()
                .col(f);
        })
    }

    /// Modal variant of [`overlay_at_offset`](Self::overlay_at_offset).
    ///
    /// Thin wrapper over
    /// [`modal_with(ModalOpts::new().anchor(anchor).offset(dx, dy), f)`](Self::modal_with),
    /// so it traps focus like [`modal`](Self::modal). Positive offsets inset
    /// toward the center — see [`overlay_at_offset`](Self::overlay_at_offset)
    /// for the full sign convention.
    #[deprecated(
        since = "0.26.0",
        note = "use `modal_with(ModalOpts::new().anchor(anchor).offset(dx, dy), f)`"
    )]
    pub fn modal_at_offset(
        &mut self,
        anchor: Anchor,
        dx: i32,
        dy: i32,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        self.modal_with(ModalOpts::new().anchor(anchor).offset(dx, dy), f)
    }

    /// Render a hover tooltip for the previously rendered interactive widget.
    ///
    /// The tooltip appears once the pointer has rested on the same target for
    /// [`RunConfig::tooltip_delay`](crate::RunConfig::tooltip_delay) (500 ms by
    /// default), so sweeping across a toolbar does not flash one tooltip per
    /// button. Hover needs [`MouseMode::Hover`](crate::MouseMode::Hover).
    ///
    /// Call this right after a widget or container response:
    /// ```ignore
    /// if ui.button("Save").clicked { save(); }
    /// ui.tooltip("Save the current document to disk");
    /// ```
    pub fn tooltip(&mut self, text: impl Into<String>) {
        let tooltip_text = text.into();
        if tooltip_text.is_empty() {
            return;
        }
        let last_interaction_id = self.rollback.interaction_count.saturating_sub(1);
        let last_response = self.response_for(last_interaction_id);
        if !last_response.hovered
            || last_response.rect.width == 0
            || last_response.rect.height == 0
            || !self.tooltip_ready()
        {
            return;
        }
        let lines = wrap_tooltip_text(&tooltip_text, 38);
        self.pending_tooltips.push(PendingTooltip {
            anchor_rect: last_response.rect,
            lines,
        });
    }

    pub(crate) fn emit_pending_tooltips(&mut self) {
        let tooltips = std::mem::take(&mut self.pending_tooltips);
        if tooltips.is_empty() {
            return;
        }
        let area_w = self.area_width;
        let area_h = self.area_height;
        let surface = self.theme.surface;
        let border_color = self.theme.border;
        let text_color = self.theme.surface_text;

        // #439: tooltips are informational; they never take clicks from the
        // widget they describe or from anything else underneath.
        self.pointer.passthrough_depth += 1;
        for tooltip in tooltips {
            let content_w = tooltip
                .lines
                .iter()
                .map(|l| crate::width::unicode_width(l.as_str()) as u32)
                .max()
                .unwrap_or(0);
            let box_w = content_w.saturating_add(4).min(area_w);
            let box_h = (tooltip.lines.len() as u32).saturating_add(4).min(area_h);

            let tooltip_x = tooltip.anchor_rect.x.min(area_w.saturating_sub(box_w));
            let below_y = tooltip.anchor_rect.bottom();
            let tooltip_y = if below_y.saturating_add(box_h) <= area_h {
                below_y
            } else {
                tooltip.anchor_rect.y.saturating_sub(box_h)
            };

            let lines = tooltip.lines;
            let pad = self.theme.spacing.xs();
            let _ = self.overlay(|ui| {
                let _ = ui.container().w(area_w).h(area_h).col(|ui| {
                    let _ = ui
                        .container()
                        .ml(tooltip_x)
                        .mt(tooltip_y)
                        .max_w(box_w)
                        .border(Border::Rounded)
                        .border_fg(border_color)
                        .bg(surface)
                        .p(pad)
                        .col(|ui| {
                            for line in &lines {
                                ui.text(line.as_str()).fg(text_color);
                            }
                        });
                });
            });
        }
        self.pointer.passthrough_depth = self.pointer.passthrough_depth.saturating_sub(1);
    }
}
