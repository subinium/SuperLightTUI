use super::*;
use crate::test_utils::TestBackend;
use crate::EventBuilder;

#[derive(Debug, PartialEq, Eq)]
struct SnapshotShape {
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

fn snapshot_shape(ctx: &Context) -> SnapshotShape {
    SnapshotShape {
        cmd_count: ctx.commands.len(),
        last_text_idx: ctx.rollback.last_text_idx,
        focus_count: ctx.rollback.focus_count,
        interaction_count: ctx.rollback.interaction_count,
        scroll_count: ctx.rollback.scroll_count,
        group_count: ctx.rollback.group_count,
        group_stack_len: ctx.rollback.group_stack.len(),
        overlay_depth: ctx.rollback.overlay_depth,
        modal_active: ctx.rollback.modal_active,
        modal_focus_start: ctx.rollback.modal_focus_start,
        modal_focus_count: ctx.rollback.modal_focus_count,
        hook_cursor: ctx.rollback.hook_cursor,
        hook_states_len: ctx.hook_states.len(),
        dark_mode: ctx.rollback.dark_mode,
        deferred_draws_len: ctx.deferred_draws.len(),
        notification_queue_len: ctx.rollback.notification_queue.len(),
        pending_tooltips_len: ctx.rollback.pending_tooltips.len(),
        text_color_stack_len: ctx.rollback.text_color_stack.len(),
    }
}

#[test]
fn use_memo_type_mismatch_includes_index_and_expected_type() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 20, 5, &mut state, Theme::dark());
    ctx.hook_states.push(Box::new(42u32));
    ctx.rollback.hook_cursor = 0;

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let deps = 1u8;
        let _ = ctx.use_memo(&deps, |_| 7u8);
    }))
    .expect_err("use_memo should panic on type mismatch");

    let message = panic_message(panic);
    assert!(
        message.contains("Hook type mismatch at index 0"),
        "panic message should include hook index, got: {message}"
    );
    assert!(
        message.contains(std::any::type_name::<(u8, u8)>()),
        "panic message should include expected type, got: {message}"
    );
    assert!(
        message.contains("Hooks must be called in the same order every frame."),
        "panic message should explain hook ordering requirement, got: {message}"
    );
}

#[test]
fn interaction_allocator_keeps_dense_slots_and_explicit_markers() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 20, 5, &mut state, Theme::dark());
    ctx.prev_hit_map = vec![
        crate::rect::Rect::new(0, 0, 1, 1),
        crate::rect::Rect::new(2, 0, 1, 1),
        crate::rect::Rect::new(4, 0, 1, 1),
    ];
    ctx.mouse_pos = Some((4, 0));
    ctx.click_pos = Some((4, 0));

    let marked = ctx.next_interaction_id();
    let skipped = ctx.reserve_interaction_slot();
    let response = ctx.interaction();

    assert_eq!(marked, 0);
    assert_eq!(skipped, 1);
    assert_eq!(ctx.rollback.interaction_count, 3);
    assert!(response.clicked);
    assert!(response.hovered);
    assert_eq!(response.rect, crate::rect::Rect::new(4, 0, 1, 1));
    assert!(matches!(
        ctx.commands.as_slice(),
        [Command::InteractionMarker(0), Command::InteractionMarker(2)]
    ));
}

#[test]
fn consume_activation_keys_claims_enter_and_space_only_when_focused() {
    let events = vec![Event::key(KeyCode::Enter), Event::key_char(' ')];
    let mut state = FrameState::default();
    let mut ctx = Context::new(events, 20, 5, &mut state, Theme::dark());

    assert!(ctx.consume_activation_keys(true));
    assert_eq!(ctx.consumed, vec![true, true]);
    assert!(!ctx.consume_activation_keys(false));
}

#[test]
fn left_clicks_for_interaction_filters_bounds_and_consumed_events() {
    let events = vec![
        Event::mouse_click(1, 1),
        Event::mouse_click(8, 8),
        Event::mouse_click(2, 1),
    ];
    let mut state = FrameState::default();
    let mut ctx = Context::new(events, 20, 5, &mut state, Theme::dark());
    ctx.prev_hit_map = vec![crate::rect::Rect::new(0, 0, 4, 2)];
    ctx.consumed[2] = true;

    let (rect, clicks) = ctx
        .left_clicks_for_interaction(0)
        .expect("interaction rect should exist");

    assert_eq!(rect, crate::rect::Rect::new(0, 0, 4, 2));
    assert_eq!(clicks.len(), 1);
    assert_eq!(clicks[0].0, 0);
    assert_eq!(clicks[0].1.x, 1);
    assert_eq!(clicks[0].1.y, 1);
}

#[test]
fn error_boundary_restores_snapshot_state_after_panic() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 20, 5, &mut state, Theme::dark());

    ctx.rollback.group_stack.push("baseline".into());
    ctx.rollback
        .notification_queue
        .push(("keep".into(), ToastLevel::Info, 1));
    ctx.rollback.pending_tooltips.push(PendingTooltip {
        anchor_rect: crate::rect::Rect::new(0, 0, 1, 1),
        lines: vec!["keep".into()],
    });
    ctx.rollback.text_color_stack.push(Some(Color::Blue));
    ctx.deferred_draws.push(None);
    ctx.hook_states.push(Box::new(1usize));
    ctx.rollback.hook_cursor = 1;
    ctx.rollback.focus_count = 2;
    ctx.rollback.interaction_count = 3;
    ctx.rollback.scroll_count = 4;
    ctx.rollback.group_count = 1;
    ctx.rollback.overlay_depth = 1;
    ctx.rollback.modal_active = true;
    ctx.rollback.modal_focus_start = 1;
    ctx.rollback.modal_focus_count = 2;

    let before = snapshot_shape(&ctx);

    ctx.error_boundary_with(
        |ui| {
            ui.text("temp");
            ui.rollback.last_text_idx = Some(99);
            ui.rollback.focus_count = 10;
            ui.rollback.interaction_count = 11;
            ui.rollback.scroll_count = 12;
            ui.rollback.group_count = 13;
            ui.rollback.group_stack.push("transient".into());
            ui.rollback.overlay_depth = 14;
            ui.rollback.modal_active = false;
            ui.rollback.modal_focus_start = 15;
            ui.rollback.modal_focus_count = 16;
            ui.rollback.hook_cursor = 17;
            ui.hook_states.push(Box::new(2usize));
            ui.rollback.dark_mode = false;
            ui.deferred_draws.push(None);
            ui.rollback
                .notification_queue
                .push(("drop".into(), ToastLevel::Error, 2));
            ui.rollback.pending_tooltips.push(PendingTooltip {
                anchor_rect: crate::rect::Rect::new(1, 1, 1, 1),
                lines: vec!["drop".into()],
            });
            ui.rollback.text_color_stack.push(Some(Color::Red));
            panic!("boom");
        },
        |_ui, _msg| {},
    );

    assert_eq!(snapshot_shape(&ctx), before);
}

#[test]
fn scoped_context_state_unwinds_after_group_and_modal() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 40, 10, &mut state, Theme::dark());

    let _ = ctx.group("card").border(Border::Rounded).col(|ui| {
        ui.text("inside");
    });
    assert_eq!(ctx.rollback.group_count, 0);
    assert!(ctx.rollback.group_stack.is_empty());
    assert!(ctx.rollback.text_color_stack.is_empty());

    let _ = ctx.modal(|ui| {
        ui.text("modal");
    });
    assert_eq!(ctx.rollback.overlay_depth, 0);
    assert!(ctx.rollback.text_color_stack.is_empty());
}

#[test]
fn emit_pending_tooltips_drains_queue_and_settles_overlay_depth() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 40, 10, &mut state, Theme::dark());
    ctx.prev_hit_map = vec![crate::rect::Rect::new(2, 1, 6, 1)];
    ctx.mouse_pos = Some((3, 1));

    let _ = ctx.interaction();
    ctx.tooltip("Helpful tip");
    assert_eq!(ctx.rollback.pending_tooltips.len(), 1);

    ctx.emit_pending_tooltips();

    assert!(ctx.rollback.pending_tooltips.is_empty());
    assert_eq!(ctx.rollback.overlay_depth, 0);
}

#[test]
fn light_dark_uses_current_theme_mode() {
    let mut dark_backend = TestBackend::new(10, 2);
    dark_backend.render(|ui| {
        let color = ui.light_dark(Color::Red, Color::Blue);
        ui.text("X").fg(color);
    });
    assert_eq!(dark_backend.buffer().get(0, 0).style.fg, Some(Color::Blue));

    let mut light_backend = TestBackend::new(10, 2);
    light_backend.render(|ui| {
        ui.set_theme(Theme::light());
        let color = ui.light_dark(Color::Red, Color::Blue);
        ui.text("X").fg(color);
    });
    assert_eq!(light_backend.buffer().get(0, 0).style.fg, Some(Color::Red));
}

#[test]
fn modal_focus_trap_tabs_only_within_modal_scope() {
    let events = EventBuilder::new().key_code(KeyCode::Tab).build();
    let mut state = FrameState::default();
    state.focus.focus_index = 3;
    state.focus.prev_focus_count = 5;
    state.focus.prev_modal_active = true;
    state.focus.prev_modal_focus_start = 3;
    state.focus.prev_modal_focus_count = 2;
    let mut ctx = Context::new(events, 40, 10, &mut state, Theme::dark());

    ctx.process_focus_keys();
    assert_eq!(ctx.focus_index, 4);

    let outside = ctx.register_focusable();
    let mut first_modal = false;
    let mut second_modal = false;
    let _ = ctx.modal(|ui| {
        first_modal = ui.register_focusable();
        second_modal = ui.register_focusable();
    });

    assert!(!outside, "focus should not be granted outside modal");
    assert!(
        !first_modal,
        "first modal focusable should be unfocused at index 4"
    );
    assert!(
        second_modal,
        "second modal focusable should be focused at index 4"
    );
}

#[test]
fn modal_focus_trap_shift_tab_wraps_within_modal_scope() {
    let events = EventBuilder::new().key_code(KeyCode::BackTab).build();
    let mut state = FrameState::default();
    state.focus.focus_index = 3;
    state.focus.prev_focus_count = 5;
    state.focus.prev_modal_active = true;
    state.focus.prev_modal_focus_start = 3;
    state.focus.prev_modal_focus_count = 2;
    let mut ctx = Context::new(events, 40, 10, &mut state, Theme::dark());

    ctx.process_focus_keys();
    assert_eq!(ctx.focus_index, 4);

    let mut first_modal = false;
    let mut second_modal = false;
    let _ = ctx.modal(|ui| {
        first_modal = ui.register_focusable();
        second_modal = ui.register_focusable();
    });

    assert!(!first_modal);
    assert!(second_modal);
}

#[test]
fn screen_helper_renders_only_current_screen() {
    let mut backend = TestBackend::new(24, 3);
    let screens = ScreenState::new("settings");

    backend.render(|ui| {
        ui.screen("home", &screens, |ui| {
            ui.text("Home Screen");
        });
        ui.screen("settings", &screens, |ui| {
            ui.text("Settings Screen");
        });
    });

    let rendered = backend.to_string();
    assert!(rendered.contains("Settings Screen"));
    assert!(!rendered.contains("Home Screen"));
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "<non-string panic payload>".to_string()
    }
}
