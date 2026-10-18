//! Pointer model: per-press hit-target resolution (#439) and press /
//! release / drag gestures with pointer capture (#442).
//!
//! Every mouse press in a frame is kept (not only the last one) and resolved
//! independently to a single *winner* slot: the topmost, innermost
//! interaction slot whose previous-frame rect contains the press. Responses
//! report `clicked` / `right_clicked` / `double_clicked` only for that winner,
//! so a popup button no longer also clicks the container behind it and a
//! clickable row inside a clickable card fires once. Ancestors opt back in
//! with [`ContainerBuilder::bubble_clicks`]; raw containment stays available
//! as [`Response::clicked_any`].
//!
//! A press also *captures* its winner until the matching release: the
//! captured slot sees `pressed`, `drag_started`, `dragging`, `drag_delta` and
//! `released` even when the pointer leaves its rect. Capture ends on release,
//! on terminal focus loss, on resize, and on motion reported without a held
//! button (a release the terminal never delivered).

use super::*;
use crate::layout::{HitMeta, NO_HIT_PARENT};
use smallvec::SmallVec;
use std::cell::OnceCell;

/// Slot flag: the slot never wins hit-testing (CSS `pointer-events: none`).
/// Used for full-area wrappers SLT creates itself (overlay roots, anchoring
/// wrappers, tooltip panels) so they do not block the content underneath.
pub(crate) const HIT_PASSTHROUGH: u8 = 1 << 0;
/// Slot flag: the slot also receives clicks whose winner is a descendant.
pub(crate) const HIT_BUBBLE: u8 = 1 << 1;
/// Builder flag: a scroll region keeps wheel events at its limit (#440).
pub(crate) const SCROLL_CONTAIN: u8 = 1 << 2;

/// One mouse button press (or release) observed this frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Press {
    /// Index into `Context::events`.
    pub event_index: usize,
    /// Cell position of the press.
    pub pos: (u32, u32),
    /// Which button was pressed.
    pub button: MouseButton,
    /// Modifier keys held at the press.
    pub modifiers: KeyModifiers,
    /// Second left press on the same cell inside the double-click window.
    pub double: bool,
}

/// The slot holding the pointer between a press and its release (#442).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PointerCapture {
    /// Winner of the capturing press, if any slot was hit.
    pub target: Option<usize>,
    /// Button that started the gesture.
    pub button: MouseButton,
    /// Last pointer position seen during the gesture.
    pub last: (u32, u32),
    /// Whether the pointer moved with the button held since the press.
    pub moved: bool,
}

/// Pointer bookkeeping that persists across frames inside `FrameState`.
#[derive(Default)]
pub(crate) struct PointerState {
    /// Slot flags recorded by the previous frame, indexed by interaction id.
    pub hit_flags_prev: Vec<u8>,
    /// Recycled buffer for the current frame's slot flags.
    pub hit_flags_buf: Vec<u8>,
    /// Active pointer capture carried across frames (#442).
    pub capture: Option<PointerCapture>,
    /// `ScrollState` ids marked `overscroll_contain` last frame (#440).
    pub contain_prev: Vec<u64>,
    /// Recycled buffer for this frame's contained scroll ids.
    pub contain_buf: Vec<u64>,
    /// Recycled buffer for this frame's scroll anchoring policies (#443).
    pub scroll_policy_buf: Vec<crate::layout::ScrollPolicy>,
    /// `RunConfig::click_on_release`, refreshed by the frame kernel.
    pub click_on_release: bool,
    /// Text-selection edge auto-scroll for the next frame: `ScrollState` id
    /// and direction (#445). Written by the frame kernel.
    pub selection_autoscroll: Option<(u64, i8)>,
}

impl PointerState {
    /// Copy the pointer-related `RunConfig` switches for the next frame.
    pub(crate) fn configure(&mut self, config: &crate::RunConfig) {
        self.click_on_release = config.click_on_release;
    }

    /// Drop the active capture (focus loss, resize, layout cache reset).
    pub(crate) fn reset_capture(&mut self) {
        self.capture = None;
    }
}

/// What happened to one captured slot during this frame.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GestureRecord {
    pub target: usize,
    pub pressed: bool,
    pub released: bool,
    pub drag_started: bool,
    pub drag_delta: (i32, i32),
    pub modifiers: KeyModifiers,
}

/// Per-frame gesture summary derived from the events and the carried capture.
#[derive(Default)]
pub(crate) struct Gestures {
    /// One record per slot that was pressed, dragged or released this frame.
    pub records: SmallVec<[GestureRecord; 2]>,
    /// Slots activated by a press and release inside the same slot
    /// (`click_on_release` mode).
    pub release_activations: SmallVec<[usize; 2]>,
    /// Capture still active at the end of the frame.
    pub capture_out: Option<PointerCapture>,
}

impl Gestures {
    fn record(&mut self, target: Option<usize>) -> Option<&mut GestureRecord> {
        let target = target?;
        let index = match self.records.iter().position(|r| r.target == target) {
            Some(index) => index,
            None => {
                self.records.push(GestureRecord {
                    target,
                    ..GestureRecord::default()
                });
                self.records.len() - 1
            }
        };
        self.records.get_mut(index)
    }
}

/// Per-frame pointer data owned by `Context`.
#[derive(Default)]
pub(crate) struct PointerFrame {
    /// Every press this frame, in event order.
    pub presses: SmallVec<[Press; 2]>,
    /// Previous-frame z-order metadata, parallel to `Context::prev_hit_map`.
    pub hit_meta: Vec<HitMeta>,
    /// Previous-frame slot flags (passthrough), indexed by interaction id.
    pub hit_flags_prev: Vec<u8>,
    /// This frame's slot flags, swapped into `PointerState` at frame end.
    pub hit_flags: Vec<u8>,
    /// Nesting depth of scopes whose containers are hit-transparent.
    pub passthrough_depth: u32,
    /// Capture carried in from the previous frame (#442).
    pub capture_in: Option<PointerCapture>,
    /// `ScrollState` ids marked `overscroll_contain` last frame (#440).
    pub contain_prev: Vec<u64>,
    /// `ScrollState` ids marked `overscroll_contain` this frame.
    pub contain: Vec<u64>,
    /// Anchoring / stick-to-bottom policy of every `ScrollState` bound this
    /// frame, consumed by the layout pass (#443).
    pub scroll_policies: Vec<crate::layout::ScrollPolicy>,
    /// `RunConfig::click_on_release` for this frame.
    pub click_on_release: bool,
    /// Selection edge auto-scroll requested by the previous frame (#445).
    pub selection_autoscroll: Option<(u64, i8)>,
    /// Lazily computed gestures for this frame.
    gestures: OnceCell<Gestures>,
    /// Lazily resolved winner per entry of `presses`.
    targets: OnceCell<SmallVec<[Option<usize>; 2]>>,
    /// Lazily resolved winner under the hover position.
    hover_target: OnceCell<Option<usize>>,
}

impl PointerFrame {
    /// Move the persistent pieces out of `state` for one frame.
    pub(crate) fn begin(state: &mut FrameState) -> Self {
        let mut hit_flags = std::mem::take(&mut state.pointer.hit_flags_buf);
        hit_flags.clear();
        let mut contain = std::mem::take(&mut state.pointer.contain_buf);
        contain.clear();
        let mut scroll_policies = std::mem::take(&mut state.pointer.scroll_policy_buf);
        scroll_policies.clear();
        Self {
            presses: SmallVec::new(),
            hit_meta: std::mem::take(&mut state.layout_feedback.prev_hit_meta),
            hit_flags_prev: std::mem::take(&mut state.pointer.hit_flags_prev),
            hit_flags,
            passthrough_depth: 0,
            capture_in: state.pointer.capture,
            contain_prev: std::mem::take(&mut state.pointer.contain_prev),
            contain,
            scroll_policies,
            click_on_release: state.pointer.click_on_release,
            selection_autoscroll: state.pointer.selection_autoscroll,
            gestures: OnceCell::new(),
            targets: OnceCell::new(),
            hover_target: OnceCell::new(),
        }
    }

    /// Attach this frame's presses (collected while scanning the events).
    pub(crate) fn with_presses(mut self, presses: SmallVec<[Press; 2]>) -> Self {
        self.presses = presses;
        self
    }
}

/// Hit-test rank: `(layer, depth, smaller area first, paint order)`.
type HitRank = (u16, u16, std::cmp::Reverse<u64>, u32);

fn rect_contains(rect: &Rect, (x, y): (u32, u32)) -> bool {
    x >= rect.x && x < rect.right() && y >= rect.y && y < rect.bottom()
}

impl Context {
    /// Hand the persistent pointer pieces back to `state` at frame end. This
    /// frame's flags become the next frame's `hit_flags_prev`, and the
    /// capture left at the end of this frame's events is carried forward.
    pub(crate) fn reclaim_pointer(&mut self, state: &mut FrameState) {
        state.pointer.capture = self.gestures().capture_out;
        let pointer = &mut self.pointer;
        state.layout_feedback.prev_hit_meta = std::mem::take(&mut pointer.hit_meta);
        let mut previous = std::mem::take(&mut pointer.hit_flags_prev);
        previous.clear();
        state.pointer.hit_flags_buf = previous;
        state.pointer.hit_flags_prev = std::mem::take(&mut pointer.hit_flags);
        let mut previous = std::mem::take(&mut pointer.contain_prev);
        previous.clear();
        state.pointer.contain_buf = previous;
        state.pointer.contain_prev = std::mem::take(&mut pointer.contain);
        pointer.gestures = OnceCell::new();
        pointer.targets = OnceCell::new();
    }

    /// Replay this frame's mouse events against the carried capture.
    ///
    /// Sequential so that a press, drag and release in one batch, or a
    /// release followed by a new press, are attributed to the right slot.
    fn gestures(&self) -> &Gestures {
        self.pointer.gestures.get_or_init(|| {
            let targets = self.press_targets();
            let mut out = Gestures::default();
            let mut capture = self.pointer.capture_in;
            let mut press_index = 0usize;
            for (event_index, event) in self.events.iter().enumerate() {
                let mouse = match event {
                    Event::FocusLost | Event::Resize(_, _) => {
                        // The release will never arrive: cancel the gesture.
                        capture = None;
                        continue;
                    }
                    Event::Mouse(mouse) => mouse,
                    _ => continue,
                };
                let pos = (mouse.x, mouse.y);
                match mouse.kind {
                    MouseKind::Down(button) => {
                        let Some(position) = self.pointer.presses[press_index..]
                            .iter()
                            .position(|press| press.event_index == event_index)
                        else {
                            // Presses before a FocusLost were discarded.
                            continue;
                        };
                        press_index += position;
                        let target = targets[press_index];
                        let modifiers = self.pointer.presses[press_index].modifiers;
                        press_index += 1;
                        capture = Some(PointerCapture {
                            target,
                            button,
                            last: pos,
                            moved: false,
                        });
                        if let Some(record) = out.record(target) {
                            record.modifiers = modifiers;
                            record.pressed |= button == MouseButton::Left;
                        }
                    }
                    MouseKind::Drag(button) => {
                        let Some(active) = capture.as_mut().filter(|c| c.button == button) else {
                            continue;
                        };
                        let delta = (
                            i64::from(pos.0) - i64::from(active.last.0),
                            i64::from(pos.1) - i64::from(active.last.1),
                        );
                        active.last = pos;
                        let started = !active.moved;
                        active.moved = true;
                        let target = active.target;
                        if let Some(record) = out.record(target) {
                            record.drag_started |= started;
                            record.drag_delta.0 = record.drag_delta.0.saturating_add(
                                i32::try_from(delta.0).unwrap_or(if delta.0 < 0 {
                                    i32::MIN
                                } else {
                                    i32::MAX
                                }),
                            );
                            record.drag_delta.1 = record.drag_delta.1.saturating_add(
                                i32::try_from(delta.1).unwrap_or(if delta.1 < 0 {
                                    i32::MIN
                                } else {
                                    i32::MAX
                                }),
                            );
                        }
                    }
                    MouseKind::Moved => {
                        // Motion without a held button: the release was lost
                        // (released outside the window, pane switch, ...).
                        capture = None;
                    }
                    MouseKind::Up(button) => {
                        let Some(active) = capture.filter(|c| c.button == button) else {
                            continue;
                        };
                        capture = None;
                        if let Some(record) = out.record(active.target) {
                            record.released = true;
                        }
                        if button == MouseButton::Left
                            && let Some(target) = active.target
                            && self
                                .prev_hit_map
                                .get(target)
                                .is_some_and(|rect| rect_contains(rect, pos))
                        {
                            out.release_activations.push(target);
                        }
                    }
                    _ => {}
                }
            }
            out.capture_out = capture;
            out
        })
    }

    /// Resolve the topmost, innermost slot under `pos` against the previous
    /// frame's hit map. `None` when no non-transparent slot contains `pos`.
    ///
    /// Ranking: higher overlay layer, then deeper tree depth, then smaller
    /// area (matching click-to-focus), then later paint order.
    pub(crate) fn hit_test(&self, pos: (u32, u32)) -> Option<usize> {
        let mut best: Option<(usize, HitRank)> = None;
        for (id, rect) in self.prev_hit_map.iter().enumerate() {
            if rect.width == 0 || rect.height == 0 || !rect_contains(rect, pos) {
                continue;
            }
            if self
                .pointer
                .hit_flags_prev
                .get(id)
                .is_some_and(|flags| flags & HIT_PASSTHROUGH != 0)
            {
                continue;
            }
            let meta = self.pointer.hit_meta.get(id).copied().unwrap_or_default();
            let order = if meta.live {
                meta.order
            } else {
                u32::try_from(id).unwrap_or(u32::MAX)
            };
            let key = (
                meta.layer,
                meta.depth,
                std::cmp::Reverse(rect.area_u64()),
                order,
            );
            if best.as_ref().is_none_or(|(_, best_key)| key > *best_key) {
                best = Some((id, key));
            }
        }
        best.map(|(id, _)| id)
    }

    /// Winner slot of each press this frame, resolved once on first use.
    fn press_targets(&self) -> &SmallVec<[Option<usize>; 2]> {
        self.pointer.targets.get_or_init(|| {
            self.pointer
                .presses
                .iter()
                .map(|press| self.hit_test(press.pos))
                .collect()
        })
    }

    /// Winner slot under the hover position, resolved once on first use.
    pub(crate) fn hover_target(&self) -> Option<usize> {
        *self
            .pointer
            .hover_target
            .get_or_init(|| self.mouse_pos.and_then(|pos| self.hit_test(pos)))
    }

    /// Whether `ancestor` is a strict ancestor slot of `id` in the previous
    /// frame's tree (same paint layer).
    pub(crate) fn is_hit_ancestor(&self, ancestor: usize, id: usize) -> bool {
        let mut current = id;
        // Bounded by tree depth; the guard also protects against a corrupted
        // parent chain ever looping.
        for _ in 0..=crate::layout::MAX_LAYOUT_DEPTH {
            let parent = self
                .pointer
                .hit_meta
                .get(current)
                .map_or(NO_HIT_PARENT, |meta| meta.parent);
            if parent == NO_HIT_PARENT {
                return false;
            }
            let parent = parent as usize;
            if parent == ancestor {
                return true;
            }
            current = parent;
        }
        false
    }

    fn hit_flags_now(&self, id: usize) -> u8 {
        self.pointer.hit_flags.get(id).copied().unwrap_or(0)
    }

    /// Whether a press resolved to `target` activates slot `id`: the winner
    /// itself, or an ancestor that opted in with `bubble_clicks`.
    fn slot_receives(&self, id: usize, target: Option<usize>) -> bool {
        match target {
            Some(target) if target == id => true,
            Some(target) => {
                self.hit_flags_now(id) & HIT_BUBBLE != 0 && self.is_hit_ancestor(id, target)
            }
            None => false,
        }
    }

    /// Whether any press with `button` (and optionally only double presses)
    /// activates slot `id` this frame.
    pub(crate) fn slot_pressed(&self, id: usize, button: MouseButton, double_only: bool) -> bool {
        let targets = self.press_targets();
        self.pointer
            .presses
            .iter()
            .zip(targets.iter())
            .any(|(press, target)| {
                press.button == button
                    && (!double_only || press.double)
                    && self.slot_receives(id, *target)
            })
    }

    /// Whether slot `id` was activated by a left click this frame: on the
    /// press by default, or on a release inside the pressed slot when
    /// `RunConfig::click_on_release(true)` is set.
    pub(crate) fn slot_clicked(&self, id: usize) -> bool {
        if !self.pointer.click_on_release {
            return self.slot_pressed(id, MouseButton::Left, false);
        }
        self.gestures()
            .release_activations
            .iter()
            .any(|&target| self.slot_receives(id, Some(target)))
    }

    /// Whether any left press this frame lands inside `rect`, ignoring the
    /// winner rule (the pre-#439 `clicked` semantics).
    pub(crate) fn any_left_press_in(&self, rect: &Rect) -> bool {
        self.pointer
            .presses
            .iter()
            .any(|press| press.button == MouseButton::Left && rect_contains(rect, press.pos))
    }

    /// Position of the last left press this frame (click-to-focus and the
    /// `confirm` widget's inline hit-test).
    pub(crate) fn last_left_press_pos(&self) -> Option<(u32, u32)> {
        self.pointer
            .presses
            .iter()
            .rev()
            .find(|press| press.button == MouseButton::Left)
            .map(|press| press.pos)
    }

    fn press_target_for_event(&self, event_index: usize) -> Option<Option<usize>> {
        let position = self
            .pointer
            .presses
            .iter()
            .position(|press| press.event_index == event_index)?;
        Some(self.press_targets()[position])
    }

    /// Whether the press at `event_index` may reach the composite widget that
    /// owns `slot`: the winner is the slot itself, one of its descendants, or
    /// no slot at all. Non-press events always pass.
    pub(crate) fn press_reaches_slot(&self, event_index: usize, slot: usize) -> bool {
        match self.press_target_for_event(event_index) {
            None | Some(None) => true,
            Some(Some(target)) => target == slot || self.is_hit_ancestor(slot, target),
        }
    }

    /// Whether the press at `event_index` landed on overlay content while the
    /// caller renders in the base layer. Rect-based handlers use this so an
    /// overlay blocks the widgets drawn underneath it.
    pub(crate) fn press_blocked_by_overlay(&self, event_index: usize) -> bool {
        if self.rollback.overlay_depth > 0 {
            return false;
        }
        match self.press_target_for_event(event_index) {
            Some(Some(target)) => self
                .pointer
                .hit_meta
                .get(target)
                .is_some_and(|meta| meta.layer > 0),
            _ => false,
        }
    }

    /// Record a finished container's pointer flags: hit-test flags for its
    /// interaction slot and, for a scrollable, its overscroll containment.
    pub(crate) fn mark_pointer_flags(&mut self, id: usize, flags: u8, scroll_state_id: u64) {
        if flags & SCROLL_CONTAIN != 0 && scroll_state_id != 0 {
            self.pointer.contain.push(scroll_state_id);
        }
        self.mark_hit_flags(id, flags & (HIT_PASSTHROUGH | HIT_BUBBLE));
    }

    /// Whether the scroll region bound to `state_id` was marked
    /// `overscroll_contain` on the previous frame.
    pub(crate) fn overscroll_contained(&self, state_id: u64) -> bool {
        state_id != 0 && self.pointer.contain_prev.contains(&state_id)
    }

    /// Record pointer flags for interaction slot `id` this frame. Slots
    /// created inside a hit-transparent scope are marked transparent too.
    pub(crate) fn mark_hit_flags(&mut self, id: usize, mut flags: u8) {
        if self.pointer.passthrough_depth > 0 {
            flags |= HIT_PASSTHROUGH;
        }
        if flags == 0 {
            return;
        }
        if self.pointer.hit_flags.len() <= id {
            self.pointer.hit_flags.resize(id + 1, 0);
        }
        self.pointer.hit_flags[id] |= flags;
    }

    /// Build the pointer part of a [`Response`] for slot `id` at `rect`.
    pub(crate) fn pointer_response(&self, id: usize, rect: Rect) -> Response {
        let hovered = self.mouse_pos.is_some_and(|pos| rect_contains(&rect, pos));
        // v0.21.1: per-widget wheel delta is hover-gated — only the widget
        // under the cursor when the wheel moved sees a non-zero delta.
        let scroll_delta = self
            .scroll_pos
            .filter(|&pos| rect_contains(&rect, pos))
            .map_or(0, |_| self.scroll_delta_frame);
        let gestures = self.gestures();
        let record = gestures.records.iter().find(|record| record.target == id);
        let dragging = gestures
            .capture_out
            .is_some_and(|capture| capture.target == Some(id) && capture.moved);
        Response {
            clicked: self.slot_clicked(id),
            clicked_any: self.any_left_press_in(&rect),
            right_clicked: self.slot_pressed(id, MouseButton::Right, false),
            middle_clicked: self.slot_pressed(id, MouseButton::Middle, false),
            double_clicked: self.slot_pressed(id, MouseButton::Left, true),
            hovered,
            hovered_top: hovered && self.hover_target() == Some(id),
            pressed: record.is_some_and(|record| record.pressed),
            released: record.is_some_and(|record| record.released),
            drag_started: record.is_some_and(|record| record.drag_started),
            dragging,
            drag_delta: record.map_or((0, 0), |record| record.drag_delta),
            modifiers: record.map_or(KeyModifiers::NONE, |record| record.modifiers),
            scroll_delta,
            rect,
            ..Response::default()
        }
    }
}

impl ContainerBuilder<'_> {
    /// Also report clicks that land on a nested interactive descendant.
    ///
    /// Since v0.26 a press activates only the topmost, innermost interactive
    /// slot under the pointer: a clickable row inside a clickable card sets
    /// `clicked` on the row, not on the card. Call `bubble_clicks()` on the
    /// outer container to receive those clicks as well (like a DOM handler
    /// that does not call `stopPropagation`). `right_clicked` and
    /// `double_clicked` bubble the same way. For raw containment, read
    /// [`Response::clicked_any`] instead.
    ///
    /// # Example
    ///
    /// ```
    /// # slt::TestBackend::new(30, 5).render(|ui| {
    /// let card = ui.bordered(slt::Border::Rounded).bubble_clicks().col(|ui| {
    ///     if ui.button("Open").clicked {
    ///         // the button fires
    ///     }
    /// });
    /// if card.clicked {
    ///     // ...and so does the card, because it opted in
    /// }
    /// # });
    /// ```
    pub fn bubble_clicks(mut self) -> Self {
        self.pointer_flags |= HIT_BUBBLE;
        self
    }

    /// Keep wheel events inside this scroll region at its scroll limit.
    ///
    /// By default a nested [`scrollable`](Context::scrollable) that reaches
    /// its end hands further wheel motion to the enclosing scrollable (the
    /// browser default, `overscroll-behavior: auto`). `overscroll_contain()`
    /// stops that chain for this region, like CSS
    /// `overscroll-behavior: contain`: useful for popups, pickers and modals
    /// whose list should not scroll the page behind them. Has no effect on a
    /// container that is not a `scrollable`.
    ///
    /// # Example
    ///
    /// ```
    /// # use slt::ScrollState;
    /// # let mut page = ScrollState::new();
    /// # let mut popup = ScrollState::new();
    /// # slt::TestBackend::new(30, 10).render(|ui| {
    /// let _ = ui.scrollable(&mut page).grow(1).col(|ui| {
    ///     let _ = ui.scrollable(&mut popup).max_h(4).overscroll_contain().col(|ui| {
    ///         for i in 0..20 {
    ///             ui.text(format!("item {i}"));
    ///         }
    ///     });
    /// });
    /// # });
    /// ```
    pub fn overscroll_contain(mut self) -> Self {
        self.pointer_flags |= SCROLL_CONTAIN;
        self
    }

    /// Make this container's own slot transparent to hit-testing. Children
    /// still receive clicks. Used for SLT's internal full-area wrappers.
    pub(crate) fn hit_passthrough(mut self) -> Self {
        self.pointer_flags |= HIT_PASSTHROUGH;
        self
    }
}
