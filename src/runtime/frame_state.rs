//! Per-session frame state carried between frames: focus, layout feedback,
//! diagnostics and the debug overlay layer.

use std::time::Duration;

use crate::clock::Instant;
#[cfg(feature = "crossterm")]
use crate::terminal;
use crate::{Event, ToastLevel, context, layout, rect, widgets};

#[derive(Default)]
pub(crate) struct FocusState {
    pub scroll: layout::FocusScrollState,
    pub screen_ranges: std::collections::HashMap<u64, context::ScreenFocusRange>,
    pub screen_scopes_buf: Vec<context::ScreenFocusScope>,
    pub focus_index: usize,
    pub prev_focus_count: usize,
    pub prev_modal_active: bool,
    pub prev_modal_focus_start: usize,
    pub prev_modal_focus_count: usize,
    pub prev_modal_id: Option<usize>,
    pub modal_restore_index: Option<usize>,
    /// Issue #208: focus index at the end of the previous frame. `None` on
    /// the first frame so widgets do not falsely report `gained_focus`.
    pub prev_focus_index: Option<usize>,
    /// Issue #217: persisted `name → focus_index` map from the most recent
    /// completed frame. Used at frame start to resolve a pending
    /// `focus_by_name(...)` against the previous render's registrations.
    pub focus_name_map_prev: std::collections::HashMap<String, usize>,
    pub focus_name_map_buf: std::collections::HashMap<String, usize>,
    /// Issue #217: a name passed to `focus_by_name(...)` that has not yet
    /// been resolved. Consumed once the matching registration is found in
    /// `focus_name_map_prev`.
    pub pending_focus_name: Option<String>,
}

/// v0.21.1: maximum gap between two same-cell left clicks for them to count as
/// a double-click. Tuned to the common desktop default (~400ms).
pub(crate) const DOUBLE_CLICK_WINDOW: std::time::Duration = std::time::Duration::from_millis(400);

#[derive(Default)]
pub(crate) struct LayoutFeedbackState {
    pub geometry: layout::GeometryFeedback,
    pub size: Option<(u32, u32)>,
    #[cfg(feature = "crossterm")]
    pub origin_row: Option<u32>,
    /// `(content_extent, viewport_extent, is_horizontal)` per scrollable last
    /// frame (#247). `is_horizontal` selects which `ScrollState` axis the
    /// `scrollable` binding updates.
    pub prev_scroll_infos: Vec<(u32, u32, bool)>,
    pub prev_scroll_rects: Vec<rect::Rect>,
    pub prev_hit_map: Vec<rect::Rect>,
    pub prev_allocated_areas: Vec<rect::Rect>,
    pub prev_group_rects: Vec<(std::sync::Arc<str>, rect::Rect)>,
    pub prev_content_map: Vec<(rect::Rect, rect::Rect)>,
    pub prev_focus_rects: Vec<(usize, rect::Rect)>,
    pub prev_focus_groups: Vec<Option<std::sync::Arc<str>>>,
    pub last_mouse_pos: Option<(u32, u32)>,
    /// v0.21.1: wall-clock time of the previous left-click `Down`, used to
    /// detect a double-click (a second click on the same cell within
    /// `DOUBLE_CLICK_WINDOW`, ~400ms). `None` after a double-click fires (so a
    /// triple click is not double-counted) or when no click has occurred.
    pub last_click_at: Option<Instant>,
    /// v0.21.1: cell position of the previous left-click `Down`, paired with
    /// `last_click_at` for same-cell double-click detection.
    pub last_click_pos: Option<(u32, u32)>,
}

#[derive(Default)]
pub(crate) struct DiagnosticsState {
    pub tick: u64,
    pub notification_queue: Vec<(String, ToastLevel, u64)>,
    pub debug_mode: bool,
    pub debug_layer: DebugLayer,
    /// Issue #268: whether the devtools inspector panel (Ctrl+F12) is active.
    /// Independent of `debug_mode`/`debug_layer`. Round-trips through
    /// `Context::inspector_mode` like `debug_layer` so `set_inspector` persists.
    pub inspector_mode: bool,
    pub fps_ema: f32,
    pub frame_started: Option<Instant>,
    pub clock_override: Option<Instant>,
    pub frame_interval: Duration,
    pub render_duration: Duration,
    pub flush_duration: Duration,
}

/// Which layers the F12 debug overlay should outline (issue #201).
///
/// `All` (the default) outlines both the base layer and any active
/// overlays/modals — matching the user's expectation for "show everything
/// the renderer is producing this frame." `TopMost` only outlines the
/// topmost overlay (or the base if no overlay is active), and `BaseOnly`
/// keeps the legacy pre-fix behavior of skipping overlays entirely.
///
/// At runtime, **Shift+F12** cycles `All → TopMost → BaseOnly → All` so a
/// developer debugging a stacked modal can shrink the visible outlines to
/// just the layer they care about without leaving the keyboard. Plain
/// **F12** independently toggles the overlay on/off.
///
/// # Example
///
/// ```no_run
/// use slt::{Context, DebugLayer};
///
/// slt::run(|ui: &mut Context| {
///     // Match on the current layer to drive bespoke debug UI.
///     let label = match ui.debug_layer() {
///         DebugLayer::All => "showing base + overlays",
///         DebugLayer::TopMost => "showing topmost overlay only",
///         DebugLayer::BaseOnly => "showing base layer only",
///     };
///     ui.text(label);
/// })
/// .unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DebugLayer {
    /// Outline both the base tree and every active overlay/modal.
    ///
    /// Default. Matches the reporter expectation that F12 reflects
    /// everything the renderer is producing this frame. Each layer family
    /// gets its own hue so a glance distinguishes base, overlay, and modal
    /// containers.
    #[default]
    All,
    /// Outline only the topmost overlay (or the base if no overlay is
    /// active).
    ///
    /// Useful when modals or popovers stack and you only care about the
    /// active dialog — base-tree outlines become noise underneath an open
    /// modal.
    TopMost,
    /// Outline only the base layer (legacy v0.19.x behavior).
    ///
    /// Skips overlays and modals entirely. Use when an overlay is
    /// confirmed correct and you want to inspect the base layout
    /// underneath it.
    BaseOnly,
}

/// Type alias matching `context::core::RawDrawCallback` (private over there);
/// used inside `FrameState` for the recycled-Vec field for issue #204. Kept
/// in lib.rs to avoid leaking a public type alias.
pub(crate) type FrameDeferredDrawSlot =
    Option<Box<dyn FnOnce(&mut crate::buffer::Buffer, crate::rect::Rect)>>;

#[derive(Default)]
pub(crate) struct FrameState {
    pub consumed_buf: Vec<bool>,
    pub events_buf: Vec<Event>,
    pub pending_input: std::collections::VecDeque<Event>,
    pub geometry_stack_buf: Vec<(usize, Option<usize>)>,
    pub hook_states: Vec<Box<dyn std::any::Any>>,
    pub named_states: std::collections::HashMap<&'static str, Box<dyn std::any::Any>>,
    /// Issue #215: runtime-string-keyed parallel of `named_states`. Persisted
    /// across frames; survives panics inside `error_boundary` (matching the
    /// `named_states` policy).
    pub keyed_states: std::collections::HashMap<String, Box<dyn std::any::Any>>,
    /// Issue #262: cross-frame partial-chord buffer for [`Context::key_chord`].
    /// Round-trips across frames using the same `std::mem::take` out/in policy
    /// as `keyed_states` (moved out in `Context::new`, restored at frame end in
    /// `run_frame_kernel`).
    pub chord_states: widgets::ChordState,
    /// Issue #248: persistent frame-clock timer table. Round-tripped through
    /// `Context` exactly like `named_states` — moved out at frame start, moved
    /// back at frame end where untouched slots are garbage-collected.
    pub scheduler: widgets::SchedulerState,
    /// Issue #234: persistent async task registry backing `Context::spawn` /
    /// `Context::poll`. Round-tripped through `Context` exactly like
    /// `scheduler` — moved out at frame start, moved back at frame end. Gated
    /// behind `async`; absent (zero overhead) when the feature is off.
    #[cfg(feature = "async")]
    pub async_tasks: context::AsyncTasks,
    pub screen_hook_map:
        std::collections::HashMap<u64, std::collections::HashMap<String, (usize, usize)>>,
    pub focus: FocusState,
    pub layout_feedback: LayoutFeedbackState,
    pub diagnostics: DiagnosticsState,
    /// Recycled command Vec (issue #150). `Context::new` swaps this into the
    /// new context (capacity preserved, len reset to 0). After `build_tree`
    /// drains the commands, the now-empty Vec is reclaimed back here.
    pub commands_buf: Vec<crate::layout::Command>,
    /// Recycled per-frame layout collection scratch (issue #155). Same
    /// pattern as `commands_buf`: clear before use, restore after.
    pub frame_data: crate::layout::FrameData,
    /// Recycled `Context::context_stack` Vec (issue #204). Empty/cleared at
    /// frame end (same pattern as `commands_buf`).
    pub context_stack_buf: Vec<Box<dyn std::any::Any>>,
    /// Recycled `Context::deferred_draws` Vec (issue #204). Slots are emptied
    /// (set to `None`) when callbacks fire; we clear before reuse.
    pub deferred_draws_buf: Vec<FrameDeferredDrawSlot>,
    /// Recycled `rollback.group_stack` Vec (issue #204). Asserted empty at
    /// frame end before reclamation.
    pub group_stack_buf: Vec<std::sync::Arc<str>>,
    /// Recycled `rollback.text_color_stack` Vec (issue #204). Asserted empty
    /// at frame end before reclamation.
    pub text_color_stack_buf: Vec<Option<crate::style::Color>>,
    /// Recycled `Context::pending_tooltips` Vec (issue #204). Asserted empty
    /// at frame end before reclamation.
    pub pending_tooltips_buf: Vec<context::PendingTooltip>,
    /// Recycled `Context::hovered_groups` set (issue #204). Cleared at the
    /// start of each frame by `build_hovered_groups`.
    pub hovered_groups_buf: std::collections::HashSet<std::sync::Arc<str>>,
    /// Issue #273: per-call-site version keys recorded by
    /// [`ContainerBuilder::cached`](crate::ContainerBuilder::cached) on the
    /// previous frame, indexed by the order `cached` regions were declared.
    /// Compared against this frame's keys to classify each cached region as a
    /// hit (key unchanged) or miss (key changed / new slot / first frame).
    /// Cleared on resize by [`clear_frame_layout_cache`] so every cached
    /// region misses after a geometry change. Round-trips through `Context`
    /// exactly like `commands_buf` (moved out at frame start, moved back at
    /// frame end). Empty (zero overhead) for apps that never call `cached`.
    pub region_versions: Vec<u64>,
    /// Issue #273: recycled scratch Vec for the CURRENT frame's `cached`
    /// region keys (same alloc-reuse discipline as `commands_buf`). Cleared
    /// before reuse; swapped into `region_versions` at frame end so the keys
    /// recorded this frame become next frame's comparison baseline.
    pub region_versions_buf: Vec<u64>,
    #[cfg(feature = "crossterm")]
    pub selection: terminal::SelectionState,
}
