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
    pub(crate) prev_focus_count: usize,
    pub(crate) prev_modal_focus_start: usize,
    pub(crate) prev_modal_focus_count: usize,
    pub(crate) prev_scroll_infos: Vec<(u32, u32)>,
    pub(crate) prev_scroll_rects: Vec<Rect>,
    pub(crate) prev_hit_map: Vec<Rect>,
    pub(crate) prev_group_rects: Vec<(String, Rect)>,
    pub(crate) prev_focus_groups: Vec<Option<String>>,
    pub(crate) _prev_focus_rects: Vec<(usize, Rect)>,
    pub(crate) mouse_pos: Option<(u32, u32)>,
    pub(crate) click_pos: Option<(u32, u32)>,
    pub(crate) prev_modal_active: bool,
    pub(crate) clipboard_text: Option<String>,
    pub(crate) debug: bool,
    pub(crate) theme: Theme,
    pub(crate) is_real_terminal: bool,
    pub(crate) deferred_draws: Vec<Option<RawDrawCallback>>,
    pub(crate) rollback: ContextRollbackState,
    pub(crate) scroll_lines_per_event: u32,
    pub(crate) screen_hook_map: std::collections::HashMap<String, (usize, usize)>,
    pub(crate) widget_theme: WidgetTheme,
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
    pub(crate) interaction_count: usize,
    pub(crate) scroll_count: usize,
    pub(crate) group_count: usize,
    pub(crate) group_stack: Vec<String>,
    pub(crate) overlay_depth: usize,
    pub(crate) modal_active: bool,
    pub(crate) modal_focus_start: usize,
    pub(crate) modal_focus_count: usize,
    pub(crate) hook_cursor: usize,
    pub(crate) dark_mode: bool,
    pub(crate) notification_queue: Vec<(String, ToastLevel, u64)>,
    pub(crate) pending_tooltips: Vec<PendingTooltip>,
    pub(crate) text_color_stack: Vec<Option<Color>>,
}

pub(super) struct ContextCheckpoint {
    commands_len: usize,
    hook_states_len: usize,
    deferred_draws_len: usize,
    rollback: ContextRollbackState,
}

impl ContextCheckpoint {
    pub(super) fn capture(ctx: &Context) -> Self {
        Self {
            commands_len: ctx.commands.len(),
            hook_states_len: ctx.hook_states.len(),
            deferred_draws_len: ctx.deferred_draws.len(),
            rollback: ctx.rollback.clone(),
        }
    }

    pub(super) fn restore(&self, ctx: &mut Context) {
        ctx.commands.truncate(self.commands_len);
        ctx.hook_states.truncate(self.hook_states_len);
        ctx.deferred_draws.truncate(self.deferred_draws_len);
        ctx.rollback = self.rollback.clone();
    }
}
