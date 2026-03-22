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
    // NOTE: If you add a mutable per-frame field, also add it to ContextSnapshot in error_boundary_with
    pub(crate) commands: Vec<Command>,
    pub(crate) events: Vec<Event>,
    pub(crate) consumed: Vec<bool>,
    pub(crate) should_quit: bool,
    pub(crate) area_width: u32,
    pub(crate) area_height: u32,
    pub(crate) tick: u64,
    pub(crate) focus_index: usize,
    pub(crate) focus_count: usize,
    pub(crate) hook_states: Vec<Box<dyn std::any::Any>>,
    pub(crate) hook_cursor: usize,
    prev_focus_count: usize,
    pub(crate) modal_focus_start: usize,
    pub(crate) modal_focus_count: usize,
    prev_modal_focus_start: usize,
    prev_modal_focus_count: usize,
    scroll_count: usize,
    prev_scroll_infos: Vec<(u32, u32)>,
    prev_scroll_rects: Vec<Rect>,
    interaction_count: usize,
    pub(crate) prev_hit_map: Vec<Rect>,
    pub(crate) group_stack: Vec<String>,
    pub(crate) prev_group_rects: Vec<(String, Rect)>,
    group_count: usize,
    prev_focus_groups: Vec<Option<String>>,
    _prev_focus_rects: Vec<(usize, Rect)>,
    mouse_pos: Option<(u32, u32)>,
    click_pos: Option<(u32, u32)>,
    last_text_idx: Option<usize>,
    overlay_depth: usize,
    pub(crate) modal_active: bool,
    prev_modal_active: bool,
    pub(crate) clipboard_text: Option<String>,
    debug: bool,
    theme: Theme,
    pub(crate) dark_mode: bool,
    pub(crate) is_real_terminal: bool,
    pub(crate) deferred_draws: Vec<Option<RawDrawCallback>>,
    pub(crate) notification_queue: Vec<(String, ToastLevel, u64)>,
    pub(crate) pending_tooltips: Vec<PendingTooltip>,
    pub(crate) text_color_stack: Vec<Option<Color>>,
    scroll_lines_per_event: u32,
}

type RawDrawCallback = Box<dyn FnOnce(&mut crate::buffer::Buffer, Rect)>;

pub(crate) struct PendingTooltip {
    pub anchor_rect: Rect,
    pub lines: Vec<String>,
}

struct ContextSnapshot {
    cmd_count: usize,
    last_text_idx: Option<usize>,
    focus_count: usize,
    interaction_count: usize,
    scroll_count: usize,
    group_count: usize,
    group_stack_len: usize,
    overlay_depth: usize,
    modal_active: bool,
    modal_focus_start: usize,
    modal_focus_count: usize,
    hook_cursor: usize,
    hook_states_len: usize,
    dark_mode: bool,
    deferred_draws_len: usize,
    notification_queue_len: usize,
    pending_tooltips_len: usize,
    text_color_stack_len: usize,
}

impl ContextSnapshot {
    fn capture(ctx: &Context) -> Self {
        Self {
            cmd_count: ctx.commands.len(),
            last_text_idx: ctx.last_text_idx,
            focus_count: ctx.focus_count,
            interaction_count: ctx.interaction_count,
            scroll_count: ctx.scroll_count,
            group_count: ctx.group_count,
            group_stack_len: ctx.group_stack.len(),
            overlay_depth: ctx.overlay_depth,
            modal_active: ctx.modal_active,
            modal_focus_start: ctx.modal_focus_start,
            modal_focus_count: ctx.modal_focus_count,
            hook_cursor: ctx.hook_cursor,
            hook_states_len: ctx.hook_states.len(),
            dark_mode: ctx.dark_mode,
            deferred_draws_len: ctx.deferred_draws.len(),
            notification_queue_len: ctx.notification_queue.len(),
            pending_tooltips_len: ctx.pending_tooltips.len(),
            text_color_stack_len: ctx.text_color_stack.len(),
        }
    }

    fn restore(&self, ctx: &mut Context) {
        ctx.commands.truncate(self.cmd_count);
        ctx.last_text_idx = self.last_text_idx;
        ctx.focus_count = self.focus_count;
        ctx.interaction_count = self.interaction_count;
        ctx.scroll_count = self.scroll_count;
        ctx.group_count = self.group_count;
        ctx.group_stack.truncate(self.group_stack_len);
        ctx.overlay_depth = self.overlay_depth;
        ctx.modal_active = self.modal_active;
        ctx.modal_focus_start = self.modal_focus_start;
        ctx.modal_focus_count = self.modal_focus_count;
        ctx.hook_cursor = self.hook_cursor;
        ctx.hook_states.truncate(self.hook_states_len);
        ctx.dark_mode = self.dark_mode;
        ctx.deferred_draws.truncate(self.deferred_draws_len);
        ctx.notification_queue.truncate(self.notification_queue_len);
        ctx.pending_tooltips.truncate(self.pending_tooltips_len);
        ctx.text_color_stack.truncate(self.text_color_stack_len);
    }
}

