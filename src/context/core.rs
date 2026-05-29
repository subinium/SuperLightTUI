use super::*;

/// The main rendering context passed to your closure each frame.
///
/// Provides all methods for building UI: text, containers, widgets, and event
/// handling. You receive a `&mut Context` on every frame and describe what to
/// render by calling its methods. SLT collects those calls, lays them out with
/// flexbox, diffs against the previous frame, and flushes only changed cells.
///
/// # Example
///
/// ```no_run
/// slt::run(|ui: &mut slt::Context| {
///     if ui.key('q') { ui.quit(); }
///     ui.text("Hello, world!").bold();
/// });
/// ```
pub struct Context {
    pub(crate) commands: Vec<Command>,
    pub(crate) events: Vec<Event>,
    pub(crate) consumed: Vec<bool>,
    pub(crate) should_quit: bool,
    pub(crate) area_width: u32,
    pub(crate) area_height: u32,
    pub(crate) tick: u64,
    pub(crate) focus_index: usize,
    pub(crate) hook_states: Vec<Box<dyn std::any::Any>>,
    pub(crate) named_states: std::collections::HashMap<&'static str, Box<dyn std::any::Any>>,
    /// Issue #215: persistent state keyed by a runtime `String`. Mirrors
    /// `named_states` but accepts dynamic keys (e.g. `format!("item-{i}")`).
    /// The map is moved into `Context::new` from `FrameState` and moved back
    /// at frame end, identical to the `named_states` lifetime.
    pub(crate) keyed_states: std::collections::HashMap<String, Box<dyn std::any::Any>>,
    /// Issue #262: cross-frame partial-chord buffer for [`Context::key_chord`].
    /// Moved into `Context::new` from `FrameState` and moved back at frame end,
    /// identical to the `keyed_states` lifetime.
    pub(crate) chord: crate::widgets::ChordState,
    pub(crate) context_stack: Vec<Box<dyn std::any::Any>>,
    pub(crate) prev_focus_count: usize,
    pub(crate) prev_modal_focus_start: usize,
    pub(crate) prev_modal_focus_count: usize,
    /// `(content_extent, viewport_extent, is_horizontal)` per scrollable from
    /// the previous frame (#247).
    pub(crate) prev_scroll_infos: Vec<(u32, u32, bool)>,
    pub(crate) prev_scroll_rects: Vec<Rect>,
    pub(crate) prev_hit_map: Vec<Rect>,
    pub(crate) prev_group_rects: Vec<(std::sync::Arc<str>, Rect)>,
    pub(crate) prev_focus_groups: Vec<Option<std::sync::Arc<str>>>,
    pub(crate) mouse_pos: Option<(u32, u32)>,
    pub(crate) click_pos: Option<(u32, u32)>,
    /// Issue #208: position of the most recent `MouseButton::Right` `Down`
    /// event in this frame. Mirrors `click_pos` for the right-button. Used
    /// by `response_for` to populate `Response::right_clicked`.
    pub(crate) right_click_pos: Option<(u32, u32)>,
    pub(crate) prev_modal_active: bool,
    pub(crate) clipboard_text: Option<String>,
    pub(crate) debug: bool,
    /// Issue #201: which layers the F12 debug overlay should outline. Read
    /// from `state.diagnostics.debug_layer` at frame start and written back
    /// at frame end so [`Context::set_debug_layer`] persists across frames.
    pub(crate) debug_layer: crate::DebugLayer,
    /// Issue #268: whether the devtools inspector panel (Ctrl+F12) is active.
    /// Read from `state.diagnostics.inspector_mode` at frame start and written
    /// back at frame end so [`Context::set_inspector`] persists across frames.
    pub(crate) inspector_mode: bool,
    pub(crate) theme: Theme,
    pub(crate) is_real_terminal: bool,
    /// Issue #264: read-only snapshot of negotiated terminal capabilities
    /// (DA1/DA2/XTGETTCAP), exposed via [`Context::capabilities`]. Populated
    /// from the process-global probe in `run_frame_kernel`; defaults
    /// conservatively on headless backends. Diagnostics-only — image rendering
    /// routes through the automatic blitter ladder, so app code never branches
    /// on this.
    #[cfg(feature = "crossterm")]
    pub(crate) capabilities: crate::terminal::Capabilities,
    pub(crate) deferred_draws: Vec<Option<RawDrawCallback>>,
    pub(crate) rollback: ContextRollbackState,
    pub(crate) pending_tooltips: Vec<PendingTooltip>,
    pub(crate) hovered_groups: std::collections::HashSet<std::sync::Arc<str>>,
    /// Issue #273: version keys recorded by [`Context::cached`] regions on the
    /// PREVIOUS frame, moved in from `FrameState::region_versions`. Indexed by
    /// the order `cached` regions are declared this frame; consulted by
    /// `cached` to classify a region as a hit (key unchanged) or miss.
    pub(crate) region_versions_prev: Vec<u64>,
    /// Issue #273: version keys recorded by `cached` regions on THIS frame, in
    /// declaration order. Swapped back into `FrameState::region_versions` at
    /// frame end to become next frame's `region_versions_prev`.
    pub(crate) region_versions_cur: Vec<u64>,
    /// Issue #273: number of `cached` regions this frame whose key matched the
    /// previous frame (a cache hit). Diagnostics-only — exposed via
    /// [`Context::region_cache_hits`].
    pub(crate) region_cache_hits: u32,
    /// Issue #273: number of `cached` regions this frame whose key changed or
    /// was new/first-frame (a cache miss). Exposed via
    /// [`Context::region_cache_misses`].
    pub(crate) region_cache_misses: u32,
    pub(crate) scroll_lines_per_event: u32,
    pub(crate) screen_hook_map: std::collections::HashMap<String, (usize, usize)>,
    pub(crate) widget_theme: WidgetTheme,
    /// Issue #208: which focus index was current at the END of the previous
    /// frame. `None` on the very first frame. Used to compute
    /// `Response::gained_focus` / `Response::lost_focus` per widget.
    pub(crate) prev_focus_index: Option<usize>,
    /// Issue #217: name → focus-index map built in the previous frame, used
    /// to resolve `focus_by_name(...)` requests at the start of this frame.
    /// Empty on the first frame.
    pub(crate) focus_name_map_prev: std::collections::HashMap<String, usize>,
    /// Issue #217: name → focus-index map being built this frame as widgets
    /// call `register_focusable_named(...)`. Swapped into `focus_name_map_prev`
    /// at frame end.
    pub(crate) focus_name_map: std::collections::HashMap<String, usize>,
    /// Issue #217: name requested by `focus_by_name(...)`; consumed at the
    /// start of the next frame. Outlives a single frame so the resolution
    /// happens against `focus_name_map_prev`.
    pub(crate) pending_focus_name: Option<String>,
    /// Issue #248: wall-clock instant sampled once at frame start. All
    /// frame-clock timer deadlines (`schedule`/`every`/`debounce`) compare
    /// against this single instant so every timer sampled in the same frame
    /// sees a consistent "now". Deliberately wall-clock, not the frame tick
    /// (`run_frame_kernel` never advances `diagnostics.tick`).
    pub(crate) frame_instant: std::time::Instant,
    /// Issue #248: persistent timer table. Moved in from `FrameState` at
    /// frame start and moved back at frame end (where untouched slots are
    /// GC'd), identical to the `named_states` lifetime.
    pub(crate) scheduler: SchedulerState,
    /// Issue #234: in-frame async task registry backing
    /// [`Context::spawn`](crate::Context::spawn) /
    /// [`Context::poll`](crate::Context::poll). Round-tripped through
    /// `FrameState` like `scheduler`. Gated behind `async`; the field does not
    /// exist (zero overhead) when the feature is off.
    #[cfg(feature = "async")]
    pub(crate) async_tasks: AsyncTasks,
}

type RawDrawCallback = Box<dyn FnOnce(&mut crate::buffer::Buffer, Rect)>;

#[derive(Debug, Clone)]
pub(crate) struct PendingTooltip {
    pub anchor_rect: Rect,
    pub lines: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct ContextRollbackState {
    pub(crate) last_text_idx: Option<usize>,
    pub(crate) focus_count: usize,
    /// Issue #208: id assigned by the most recent `register_focusable()` /
    /// `register_focusable_named(...)` call. `begin_widget_interaction`
    /// reads this to compute `Response::gained_focus` / `lost_focus`
    /// without changing the public `register_focusable` signature. Reset
    /// to `None` at frame start; left alone after read so widgets that
    /// don't pair `register_focusable` with `begin_widget_interaction`
    /// still get correct behavior.
    pub(crate) last_focusable_id: Option<usize>,
    /// Issue #217 follow-up: slot id reserved by the most-recent
    /// `register_focusable_named(name)` for the next `register_focusable()`
    /// to *reuse* instead of allocating a fresh slot.
    ///
    /// `register_focusable_named` allocates the slot eagerly (so the name
    /// is already bound in `focus_name_map` and `focused_name()` works
    /// even when no widget follows), and stores the slot id here. When a
    /// SLT widget like `text_input` / `button` / `tabs` calls
    /// `register_focusable()` immediately after — every such widget does
    /// — the call drains this reservation and reuses the same slot, so
    /// the name binds to the slot the widget actually occupies rather
    /// than to a dummy slot allocated by `register_focusable_named`.
    ///
    /// Cleared in three cases:
    ///   1. drained by the next `register_focusable()` and reused (common
    ///      path: named widget),
    ///   2. overwritten by a second `register_focusable_named()` that
    ///      runs without an intervening widget (last-write-wins on the
    ///      reservation; the first slot is left orphaned but harmless,
    ///      its name binding already lives in `focus_name_map`),
    ///   3. dropped by the modal/overlay suppression branch when the
    ///      named registration itself is suppressed.
    pub(crate) pending_focusable_id: Option<usize>,
    pub(crate) interaction_count: usize,
    pub(crate) scroll_count: usize,
    pub(crate) group_count: usize,
    pub(crate) group_stack: Vec<std::sync::Arc<str>>,
    pub(crate) overlay_depth: usize,
    pub(crate) modal_active: bool,
    pub(crate) modal_focus_start: usize,
    pub(crate) modal_focus_count: usize,
    pub(crate) hook_cursor: usize,
    pub(crate) dark_mode: bool,
    pub(crate) notification_queue: Vec<(String, ToastLevel, u64)>,
    pub(crate) text_color_stack: Vec<Option<Color>>,
}

pub(super) struct ContextCheckpoint {
    commands_len: usize,
    hook_states_len: usize,
    deferred_draws_len: usize,
    context_stack_len: usize,
    pending_tooltips_len: usize,
    /// Issue #273: `cached` region keys recorded so far, so a panicking
    /// `cached` region inside an `error_boundary` rolls back its key entry
    /// (and any nested ones) — keeping the recorded keys consistent with the
    /// commands that actually survived the rollback.
    region_versions_cur_len: usize,
    rollback: ContextRollbackState,
}

impl ContextCheckpoint {
    pub(super) fn capture(ctx: &Context) -> Self {
        Self {
            commands_len: ctx.commands.len(),
            hook_states_len: ctx.hook_states.len(),
            deferred_draws_len: ctx.deferred_draws.len(),
            context_stack_len: ctx.context_stack.len(),
            pending_tooltips_len: ctx.pending_tooltips.len(),
            region_versions_cur_len: ctx.region_versions_cur.len(),
            rollback: ctx.rollback.clone(),
        }
    }

    pub(super) fn restore(&self, ctx: &mut Context) {
        ctx.commands.truncate(self.commands_len);
        ctx.hook_states.truncate(self.hook_states_len);
        ctx.deferred_draws.truncate(self.deferred_draws_len);
        ctx.context_stack.truncate(self.context_stack_len);
        ctx.rollback = self.rollback.clone();
        // Drop tooltips queued by the panicking widget but keep any that were
        // already pending before the error boundary was entered.
        ctx.pending_tooltips.truncate(self.pending_tooltips_len);
        // Issue #273: drop `cached` keys recorded by the panicking subtree.
        ctx.region_versions_cur
            .truncate(self.region_versions_cur_len);
    }
}
