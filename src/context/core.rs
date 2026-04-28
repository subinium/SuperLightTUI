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
    pub(crate) context_stack: Vec<Box<dyn std::any::Any>>,
    pub(crate) prev_focus_count: usize,
    pub(crate) prev_modal_focus_start: usize,
    pub(crate) prev_modal_focus_count: usize,
    pub(crate) prev_scroll_infos: Vec<(u32, u32)>,
    pub(crate) prev_scroll_rects: Vec<Rect>,
    pub(crate) prev_hit_map: Vec<Rect>,
    pub(crate) prev_group_rects: Vec<(std::sync::Arc<str>, Rect)>,
    pub(crate) prev_focus_groups: Vec<Option<std::sync::Arc<str>>>,
    pub(crate) _prev_focus_rects: Vec<(usize, Rect)>,
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
    pub(crate) theme: Theme,
    pub(crate) is_real_terminal: bool,
    pub(crate) deferred_draws: Vec<Option<RawDrawCallback>>,
    pub(crate) rollback: ContextRollbackState,
    pub(crate) pending_tooltips: Vec<PendingTooltip>,
    pub(crate) hovered_groups: std::collections::HashSet<std::sync::Arc<str>>,
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
    }
}
