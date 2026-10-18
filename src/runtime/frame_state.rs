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
    /// Issue #453: focus-slot metadata carried between frames.
    pub identity: FocusIdentityCarry,
}

/// Issue #453: what kind of text entry a focus slot performs. Keys the
/// widget edits with are withheld from global shortcut queries while it owns
/// focus (see `Context::key`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextEntryKind {
    /// Single-line editor: printable keys plus Backspace, Delete, Left,
    /// Right, Home and End.
    SingleLine,
    /// Multi-line editor: everything `SingleLine` owns plus Enter, Up and
    /// Down.
    MultiLine,
    /// Issue #466: code editor: everything `MultiLine` owns plus Tab,
    /// Shift+Tab (indent / dedent), PageUp and PageDown.
    Code,
}

/// Per-frame metadata recorded for every focus slot, indexed by slot id.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FocusSlotMeta {
    /// Issue #454: stable identity of the widget that owns the slot (a
    /// widget-state id, a hashed `register_focusable_keyed` key, or a hashed
    /// `register_focusable_named` name). `None` for positional slots.
    pub key: Option<u64>,
    /// Issue #453: `Some` when the slot is a text-entry widget.
    pub text_entry: Option<TextEntryKind>,
    /// Issue #465: `CHORD_*` bits for the Ctrl/Super chords the text widget
    /// handles itself (select all, copy/cut while a selection exists, undo /
    /// redo, kill line). Shortcut queries do not see those chords while the
    /// widget owns focus.
    pub chords: u8,
}

/// Issue #465: the focused text widget handles `Ctrl+A`.
pub(crate) const CHORD_SELECT_ALL: u8 = 1;
/// Issue #465: the focused text widget has a selection and handles
/// `Ctrl+C` / `Ctrl+X`.
pub(crate) const CHORD_COPY_CUT: u8 = 2;
/// Issue #465: the focused text widget handles `Ctrl+Z` / `Ctrl+Y`.
pub(crate) const CHORD_HISTORY: u8 = 4;
/// Issue #465: the focused text widget handles `Ctrl+K`.
pub(crate) const CHORD_KILL_LINE: u8 = 8;

/// Issue #454: high bit reserved for widget-state focus keys so they never
/// collide with hashed user keys (which have it cleared).
pub(crate) const STATE_FOCUS_KEY_BIT: u64 = 1 << 63;

/// Issue #454: allocate a process-unique focus key for a widget state.
pub(crate) fn next_state_focus_key() -> u64 {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed) | STATE_FOCUS_KEY_BIT
}

/// Issue #454: map a user-supplied key to the hashed-key namespace.
pub(crate) fn hashed_focus_key(key: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher as _;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish() & !STATE_FOCUS_KEY_BIT
}

/// In-frame view of the focus-slot metadata, owned by `Context`.
#[derive(Debug, Default)]
pub(crate) struct FocusIdentity {
    /// Metadata recorded this frame, indexed by slot id.
    pub slots: Vec<FocusSlotMeta>,
    /// Metadata recorded by the previous frame, indexed by slot id.
    pub prev_slots: Vec<FocusSlotMeta>,
    /// Issue #453: text-entry ownership (kind and handled chords) at the
    /// start of this frame.
    pub prev_text_entry: Option<(TextEntryKind, u8)>,
    /// Issue #454: key of the slot focus starts on. While set (and focus has
    /// not been moved explicitly this frame), focus follows this key instead
    /// of the slot number.
    pub expected_key: Option<u64>,
    /// Issue #454: `focus_index` value the key rule is anchored to. Any other
    /// value means focus was moved explicitly and the positional rule applies.
    pub origin_index: usize,
    /// Issue #454: key of the slot that was focused when the previous frame
    /// ended, for `gained_focus` / `lost_focus`.
    pub prev_focused_key: Option<u64>,
}

impl FocusIdentity {
    /// Whether `key` belonged to some slot in the previous frame.
    pub(crate) fn seen_last_frame(&self, key: u64) -> bool {
        self.prev_slots.iter().any(|slot| slot.key == Some(key))
    }
}

impl FocusIdentityCarry {
    /// Hand the carried metadata to a new frame whose focus starts on
    /// `target` with `focus_index == origin_index`. `target` and
    /// `prev_focused` are slot ids in the previous frame's numbering.
    pub(crate) fn begin_frame(
        &mut self,
        target: usize,
        origin_index: usize,
        prev_focused: Option<usize>,
    ) -> FocusIdentity {
        let mut slots = std::mem::take(&mut self.slots_buf);
        slots.clear();
        let prev_slots = std::mem::take(&mut self.slots);
        let start = prev_slots.get(target).copied().unwrap_or_default();
        let prev_text_entry = self
            .text_claim
            .map(|kind| (kind, 0))
            .or(start.text_entry.map(|kind| (kind, start.chords)));
        let prev_focused_key = prev_focused
            .and_then(|id| prev_slots.get(id))
            .and_then(|slot| slot.key);
        FocusIdentity {
            slots,
            prev_slots,
            prev_text_entry,
            expected_key: start.key,
            origin_index,
            prev_focused_key,
        }
    }

    /// Take back the metadata recorded by a finished frame.
    pub(crate) fn end_frame(
        &mut self,
        identity: &mut FocusIdentity,
        count: usize,
        text_claim: Option<TextEntryKind>,
    ) {
        let mut recycled = std::mem::take(&mut identity.prev_slots);
        recycled.clear();
        identity.slots.truncate(count);
        self.slots = std::mem::take(&mut identity.slots);
        self.slots_buf = recycled;
        self.text_claim = text_claim;
    }
}

/// Focus-slot metadata handed from one frame to the next.
#[derive(Debug, Default)]
pub(crate) struct FocusIdentityCarry {
    /// Slot metadata recorded by the most recent completed frame.
    pub slots: Vec<FocusSlotMeta>,
    /// Recycled buffer the next frame records its slot metadata into.
    pub slots_buf: Vec<FocusSlotMeta>,
    /// Issue #453: text-entry ownership claimed by a widget without a focus
    /// slot (an open command palette) on the most recent completed frame.
    pub text_claim: Option<TextEntryKind>,
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
    /// Z-order metadata parallel to `prev_hit_map` (#439).
    pub prev_hit_meta: Vec<layout::HitMeta>,
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
    /// Issue #469: frames run under the accessible linear runtime. Copied
    /// into `Context::accessible`; `false` for every other run mode.
    pub accessible: bool,
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
    /// Persistent pointer bookkeeping (#439/#442): previous-frame slot flags,
    /// the active pointer capture and hover timing.
    pub pointer: context::PointerState,
    #[cfg(feature = "crossterm")]
    pub selection: terminal::SelectionState,
}
