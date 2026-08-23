use super::*;
use crate::EventBuilder;
use crate::test_utils::TestBackend;

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
    pending_screen_nav_len: usize,
    screen_nav_scope_depth: usize,
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
        pending_tooltips_len: ctx.pending_tooltips.len(),
        pending_screen_nav_len: ctx.pending_screen_nav.len(),
        screen_nav_scope_depth: ctx.screen_nav_depth,
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
        message.contains(std::any::type_name::<MemoSlot<u8>>()),
        "panic message should include expected MemoSlot type, got: {message}"
    );
    assert!(
        message.contains("Hooks must be called in the same order every frame."),
        "panic message should explain hook ordering requirement, got: {message}"
    );
}

#[test]
fn use_memo_handle_releases_borrow() {
    // The handle composes with an intervening `ui.*` mutation — the exact
    // pattern that failed to compile when `use_memo` returned `&T`.
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        let m = ui.use_memo(&21i32, |d| d * 2);
        // Intervening mutation: would conflict with a live `&T` borrow.
        ui.text("memo:");
        let v = m.copied(ui);
        ui.text(format!("{v}"));
    });
    tb.assert_contains("memo:");
    tb.assert_contains("42");
}

#[test]
fn use_memo_recomputes_only_on_dep_change() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let mut tb = TestBackend::new(20, 3);

    // Frame 1: dep = 2 — first compute.
    let c1 = calls.clone();
    tb.render(move |ui| {
        let dep = ui.use_state(|| 2i32);
        let d = *dep.get(ui);
        let m = ui.use_memo(&d, |x| {
            c1.fetch_add(1, Ordering::SeqCst);
            x * 10
        });
        ui.text(format!("{}", m.copied(ui)));
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1, "first frame computes once");

    // Frame 2: same dep — no recompute (cache hit).
    let c2 = calls.clone();
    tb.render(move |ui| {
        let dep = ui.use_state(|| 2i32);
        let d = *dep.get(ui);
        let m = ui.use_memo(&d, |x| {
            c2.fetch_add(1, Ordering::SeqCst);
            x * 10
        });
        ui.text(format!("{}", m.copied(ui)));
    });
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "stable deps must not recompute"
    );

    // Frame 3: changed dep — recompute.
    let c3 = calls.clone();
    tb.render(move |ui| {
        let dep = ui.use_state(|| 2i32);
        *dep.get_mut(ui) = 5; // change the dependency
        let d = *dep.get(ui);
        let m = ui.use_memo(&d, |x| {
            c3.fetch_add(1, Ordering::SeqCst);
            x * 10
        });
        ui.text(format!("{}", m.copied(ui)));
    });
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "changed deps must recompute"
    );
    tb.assert_contains("50");
}

#[test]
fn use_memo_retries_initial_and_update_panics_atomically() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 20, 5, &mut state, Theme::dark());

    ctx.error_boundary_with(
        |ui| {
            let _ = ui.use_memo(&1u8, |_| -> u16 { panic!("initial memo panic") });
        },
        |_ui, _| {},
    );
    assert!(
        ctx.hook_states.is_empty(),
        "initial panic must not create a slot"
    );

    let initial = ctx.use_memo(&1u8, |dep| u16::from(*dep) * 10);
    assert_eq!(initial.copied(&ctx), 10);

    ctx.rollback.hook_cursor = 0;
    ctx.error_boundary_with(
        |ui| {
            let _ = ui.use_memo(&2u8, |_| -> u16 { panic!("update memo panic") });
        },
        |_ui, _| {},
    );

    ctx.rollback.hook_cursor = 0;
    let retried = ctx.use_memo(&2u8, |dep| u16::from(*dep) * 10);
    assert_eq!(
        retried.copied(&ctx),
        20,
        "same deps must recompute after panic"
    );
}

#[test]
#[allow(deprecated)]
fn use_memo_ref_retries_initial_and_update_panics_atomically() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 20, 5, &mut state, Theme::dark());

    ctx.error_boundary_with(
        |ui| {
            let _ = ui.use_memo_ref(&1u8, |_| -> u16 { panic!("initial memo_ref panic") });
        },
        |_ui, _| {},
    );
    assert!(
        ctx.hook_states.is_empty(),
        "initial panic must not create a slot"
    );

    assert_eq!(*ctx.use_memo_ref(&1u8, |dep| u16::from(*dep) * 10), 10);

    ctx.rollback.hook_cursor = 0;
    ctx.error_boundary_with(
        |ui| {
            let _ = ui.use_memo_ref(&2u8, |_| -> u16 { panic!("update memo_ref panic") });
        },
        |_ui, _| {},
    );

    ctx.rollback.hook_cursor = 0;
    assert_eq!(
        *ctx.use_memo_ref(&2u8, |dep| u16::from(*dep) * 10),
        20,
        "same deps must recompute after panic"
    );
}

#[test]
fn use_memo_copied_matches_get() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        let m = ui.use_memo(&7i32, |d| d * 3);
        assert_eq!(m.copied(ui), *m.get(ui));
        ui.text(format!("{}", m.copied(ui)));
    });
    tb.assert_contains("21");
}

#[test]
fn use_memo_ref_still_compiles() {
    // The deprecated `&T`-returning alias remains a drop-in for existing
    // callers and yields the same value as the handle form.
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        #[allow(deprecated)]
        let v = *ui.use_memo_ref(&8i32, |d| d * 2);
        assert_eq!(v, 16);
        ui.text(format!("{v}"));
    });
    tb.assert_contains("16");
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
    ctx.pending_tooltips.push(PendingTooltip {
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
            ui.pending_tooltips.push(PendingTooltip {
                anchor_rect: crate::rect::Rect::new(1, 1, 1, 1),
                lines: vec!["drop".into()],
            });
            ui.pending_screen_nav.push(ScreenNav::Pop);
            ui.screen_nav_depth += 1;
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
    assert_eq!(ctx.pending_tooltips.len(), 1);

    ctx.emit_pending_tooltips();

    assert!(ctx.pending_tooltips.is_empty());
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
    let mut screens = ScreenState::new("settings");

    backend.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            ui.text("Home Screen");
        });
        ui.screen("settings", &mut screens, |ui| {
            ui.text("Settings Screen");
        });
    });

    let rendered = backend.to_string();
    assert!(rendered.contains("Settings Screen"));
    assert!(!rendered.contains("Home Screen"));
}

// === Issue #279: navigate from inside a `ui.screen` closure ===

#[test]
fn push_screen_inside_closure_navigates_without_blending_frames() {
    let mut backend = TestBackend::new(24, 3);
    let mut screens = ScreenState::new("home");

    // `ui.push_screen` records a deferred nav that is applied once the active
    // closure returns — so it does not double-borrow `screens` (issue #279).
    backend.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            ui.text("Home Screen");
            ui.push_screen("settings");
        });
        ui.screen("settings", &mut screens, |ui| {
            ui.text("Settings Screen");
        });
    });

    assert_eq!(screens.current(), "settings");
    let transition = backend.to_string();
    assert!(transition.contains("Home Screen"));
    assert!(!transition.contains("Settings Screen"));

    backend.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            ui.text("Home Screen");
        });
        ui.screen("settings", &mut screens, |ui| {
            ui.text("Settings Screen");
        });
    });
    let settled = backend.to_string();
    assert!(!settled.contains("Home Screen"));
    assert!(settled.contains("Settings Screen"));
}

#[test]
fn pop_screen_inside_closure_navigates_without_blank_frame() {
    let mut backend = TestBackend::new(24, 3);
    let mut screens = ScreenState::new("home");
    screens.push("settings");

    backend.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            ui.text("Home Screen");
        });
        ui.screen("settings", &mut screens, |ui| {
            ui.text("Settings Screen");
            ui.pop_screen();
        });
    });

    assert_eq!(screens.current(), "home");
    let transition = backend.to_string();
    assert!(!transition.contains("Home Screen"));
    assert!(transition.contains("Settings Screen"));

    backend.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            ui.text("Home Screen");
        });
        ui.screen("settings", &mut screens, |ui| {
            ui.text("Settings Screen");
        });
    });
    let settled = backend.to_string();
    assert!(settled.contains("Home Screen"));
    assert!(!settled.contains("Settings Screen"));
}

#[test]
fn nested_screen_navigation_stays_in_its_own_scope() {
    let mut backend = TestBackend::new(24, 3);
    let mut outer = ScreenState::new("outer-home");
    let mut inner = ScreenState::new("inner-home");

    backend.render(|ui| {
        ui.screen("outer-home", &mut outer, |ui| {
            ui.push_screen("outer-settings");
            ui.screen("inner-home", &mut inner, |ui| {
                ui.push_screen("inner-details");
            });
        });
    });

    assert_eq!(outer.current(), "outer-settings");
    assert_eq!(outer.depth(), 2);
    assert_eq!(inner.current(), "inner-details");
    assert_eq!(inner.depth(), 2);
}

#[test]
#[should_panic(expected = "screen navigation helpers can only be called inside")]
fn screen_navigation_outside_screen_panics_with_guidance() {
    let mut backend = TestBackend::new(24, 3);
    backend.render(|ui| ui.push_screen("settings"));
}

#[test]
fn reset_screen_inside_closure_returns_to_root() {
    let mut backend = TestBackend::new(24, 3);
    let mut screens = ScreenState::new("home");
    screens.push("settings");
    screens.push("advanced");

    backend.render(|ui| {
        ui.screen("advanced", &mut screens, |ui| {
            ui.reset_screen();
        });
    });

    assert_eq!(screens.current(), "home");
    assert_eq!(screens.depth(), 1);
}

#[test]
fn screen_nav_no_op_without_request() {
    // A screen closure that records no navigation leaves the stack untouched.
    let mut backend = TestBackend::new(24, 3);
    let mut screens = ScreenState::new("home");

    backend.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            ui.text("Home");
        });
    });

    assert_eq!(screens.current(), "home");
}

#[test]
fn keyed_state_cleanup_removes_dynamic_entries() {
    let mut backend = TestBackend::new(24, 3);

    backend.render(|ui| {
        let a = ui.use_state_keyed("row-a", || 1u32);
        let b = ui.use_state_keyed("row-b", || 2u32);
        assert_eq!(*a.get(ui), 1);
        assert_eq!(*b.get(ui), 2);
        assert_eq!(ui.keyed_state_count(), 2);

        assert!(ui.remove_state_keyed("row-a"));
        assert!(!ui.remove_state_keyed("missing"));
        assert_eq!(ui.keyed_state_count(), 1);

        let removed = ui.retain_state_keyed(|key| key == "row-b");
        assert_eq!(removed, 0);
        assert_eq!(ui.keyed_state_count(), 1);
    });
}

#[test]
fn screen_state_cleanup_removes_inactive_hook_and_focus_state() {
    let mut backend = TestBackend::new(24, 3);
    let mut screens = ScreenState::new("home");
    screens.push("detail-1");

    backend.render(|ui| {
        ui.screen("detail-1", &mut screens, |ui| {
            let value = ui.use_state(|| 7u32);
            ui.text(format!("{}", value.get(ui)));
            ui.register_focusable();
        });
    });
    assert_eq!(screens.focus_state_count(), 1);

    screens.pop();
    backend.render(|ui| {
        assert_eq!(ui.screen_state_count(), 1);
        assert!(ui.remove_screen_state(&mut screens, "detail-1"));
        assert_eq!(ui.screen_state_count(), 0);
    });
    assert_eq!(screens.focus_state_count(), 0);
}

#[test]
fn screen_state_cleanup_preserves_stacked_screens() {
    let mut backend = TestBackend::new(24, 3);
    let mut screens = ScreenState::new("home");

    backend.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            let value = ui.use_state(|| 1u32);
            ui.text(format!("{}", value.get(ui)));
        });
        assert!(!ui.remove_screen_state(&mut screens, "home"));
        assert_eq!(ui.screen_state_count(), 1);
    });
}

// === Issue #54: mouse_drag / mouse_up convenience methods ===

#[test]
fn mouse_drag_returns_drag_position() {
    let events = vec![Event::mouse_drag(5, 3)];
    let mut state = FrameState::default();
    let ctx = Context::new(events, 20, 10, &mut state, Theme::dark());

    assert_eq!(ctx.mouse_drag(), Some((5, 3)));
    assert_eq!(ctx.mouse_up(), None);
    assert_eq!(ctx.mouse_down(), None);
}

#[test]
fn mouse_up_returns_release_position() {
    let events = vec![Event::mouse_up(7, 2)];
    let mut state = FrameState::default();
    let ctx = Context::new(events, 20, 10, &mut state, Theme::dark());

    assert_eq!(ctx.mouse_up(), Some((7, 2)));
    assert_eq!(ctx.mouse_drag(), None);
}

#[test]
fn mouse_down_button_detects_right_click() {
    let events = vec![Event::Mouse(crate::event::MouseEvent {
        kind: MouseKind::Down(MouseButton::Right),
        x: 3,
        y: 4,
        modifiers: KeyModifiers::NONE,
        pixel_x: None,
        pixel_y: None,
    })];
    let mut state = FrameState::default();
    let ctx = Context::new(events, 20, 10, &mut state, Theme::dark());

    assert_eq!(ctx.mouse_down_button(MouseButton::Right), Some((3, 4)));
    assert_eq!(ctx.mouse_down_button(MouseButton::Left), None);
    assert_eq!(ctx.mouse_down(), None); // mouse_down is Left-only
}

#[test]
fn mouse_drag_respects_consumed_flag() {
    let events = vec![Event::mouse_drag(5, 3)];
    let mut state = FrameState::default();
    let mut ctx = Context::new(events, 20, 10, &mut state, Theme::dark());
    ctx.consumed[0] = true;

    assert_eq!(ctx.mouse_drag(), None);
}

// === Issue #56: events() / raw_events() ===

#[test]
fn events_filters_consumed() {
    let events = vec![
        Event::key_char('a'),
        Event::key_char('b'),
        Event::key_char('c'),
    ];
    let mut state = FrameState::default();
    let mut ctx = Context::new(events, 20, 10, &mut state, Theme::dark());
    ctx.consumed[1] = true;

    let visible: Vec<_> = ctx.events().collect();
    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0], &Event::key_char('a'));
    assert_eq!(visible[1], &Event::key_char('c'));
}

#[test]
fn events_blocked_by_modal_guard() {
    let events = vec![Event::key_char('x')];
    let mut state = FrameState::default();
    let mut ctx = Context::new(events, 20, 10, &mut state, Theme::dark());
    ctx.rollback.modal_active = true;
    ctx.rollback.overlay_depth = 0;

    assert_eq!(ctx.events().count(), 0);
    // raw_events bypasses modal guard
    assert_eq!(ctx.raw_events().count(), 1);
}

// === Issue #52: draw_interactive ===

#[test]
fn draw_interactive_emits_interaction_marker_and_raw_draw() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 40, 20, &mut state, Theme::dark());
    ctx.prev_hit_map = vec![crate::rect::Rect::new(0, 0, 40, 20)];
    ctx.click_pos = Some((5, 5));

    let resp = ctx
        .container()
        .w(40)
        .h(20)
        .draw_interactive(|_buf, _rect| {});

    assert!(resp.clicked);

    // Should have emitted an InteractionMarker before RawDraw
    let has_marker = ctx
        .commands
        .iter()
        .any(|c| matches!(c, Command::InteractionMarker(0)));
    assert!(has_marker, "draw_interactive must emit InteractionMarker");
    let has_raw = ctx
        .commands
        .iter()
        .any(|c| matches!(c, Command::RawDraw { .. }));
    assert!(has_raw, "draw_interactive must emit RawDraw command");
}

#[test]
fn draw_does_not_emit_interaction_marker() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 40, 20, &mut state, Theme::dark());

    ctx.container().w(40).h(20).draw(|_buf, _rect| {});

    let has_marker = ctx
        .commands
        .iter()
        .any(|c| matches!(c, Command::InteractionMarker(_)));
    assert!(
        !has_marker,
        "draw() should NOT emit InteractionMarker (backward compat)"
    );
}

// === Issue #55: grid_with ===

#[test]
fn grid_with_fixed_columns_emit_constraints() {
    use crate::widgets::GridColumn;

    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 80, 24, &mut state, Theme::dark());

    let _ = ctx.grid_with(
        &[GridColumn::Fixed(10), GridColumn::Grow(2), GridColumn::Auto],
        |ui| {
            ui.text("A");
            ui.text("B");
            ui.text("C");
        },
    );

    // Verify that fixed column got min_w == max_w == 10 and grow: 0
    let fixed_container = ctx.commands.iter().find(|c| {
        matches!(c, Command::BeginContainer(args)
            if args.constraints.min_width() == Some(10)
                && args.constraints.max_width() == Some(10)
                && args.grow == 0)
    });
    assert!(
        fixed_container.is_some(),
        "Fixed(10) column should produce min_w=max_w=10, grow=0"
    );

    // Verify that Grow(2) column exists
    let grow_container = ctx.commands.iter().find(|c| {
        matches!(c, Command::BeginContainer(args)
            if args.grow == 2
                && args.constraints.min_width().is_none()
                && args.constraints.max_width().is_none())
    });
    assert!(
        grow_container.is_some(),
        "Grow(2) column should produce grow=2, no width constraints"
    );
}

#[test]
fn scrollable_preserves_group_name_for_hover_registration() {
    // Regression for #141: group_name was dropped before BeginScrollable was
    // pushed, so is_group_hovered() always returned false on scrollable groups.
    use crate::test_utils::TestBackend;
    use crate::widgets::ScrollState;

    let scroll = ScrollState::new();
    TestBackend::new(40, 10).render(|ui| {
        let resp = ui
            .group("card")
            .scroll_offset(scroll.offset as u32)
            .col(|ui| {
                ui.text("hover text");
            });
        let _ = resp;
        // Confirm the BeginScrollable command carries the group_name.
        let cmd = ui.commands.iter().find(|c| {
            matches!(c, crate::layout::Command::BeginScrollable(a) if a.group_name.as_deref() == Some("card"))
        });
        assert!(
            cmd.is_some(),
            "BeginScrollable must carry group_name=\"card\"; #141 regression"
        );
    });
}

#[test]
fn scrollable_propagates_bg_color_align_justify_gap() {
    // Regression for #142: bg_color/align/justify/gap were silently dropped
    // because BeginScrollableArgs lacked those fields.
    use crate::style::{Align, Color, Justify};
    use crate::test_utils::TestBackend;
    use crate::widgets::ScrollState;

    let mut scroll = ScrollState::new();
    TestBackend::new(40, 10).render(|ui| {
        let _ = ui
            .scrollable(&mut scroll)
            .bg(Color::Indexed(236))
            .gap(2)
            .align(Align::Center)
            .justify(Justify::Center)
            .col(|ui| {
                ui.text("line");
            });
        // Confirm the BeginScrollable command carries the expected values.
        let cmd = ui.commands.iter().find(|c| {
            matches!(
                c,
                crate::layout::Command::BeginScrollable(a)
                    if a.bg_color == Some(Color::Indexed(236))
                    && a.gap == 2
                    && a.align == Align::Center
                    && a.justify == Justify::Center
            )
        });
        assert!(
            cmd.is_some(),
            "BeginScrollable must carry bg_color/gap/align/justify; #142 regression"
        );
    });
}

#[test]
fn group_uses_arc_str_and_single_allocation_path() {
    // Regression for #139 / #145: `group_stack` is `Vec<Arc<str>>` and
    // `Context::group()` no longer double-allocates the name.
    use crate::test_utils::TestBackend;

    TestBackend::new(20, 5).render(|ui| {
        let _ = ui.group("ring").col(|ui| {
            // The current group must be observable as `Arc<str>` on the
            // rollback stack while the closure runs.
            assert_eq!(
                ui.rollback.group_stack.last().map(|a| a.as_ref()),
                Some("ring")
            );
            ui.text("inside");
        });
        // Stack must unwind cleanly after the group ends.
        assert!(ui.rollback.group_stack.is_empty());
    });
}

#[test]
fn is_group_hovered_uses_o1_hashset_lookup() {
    // Regression for the cache half of #136 / #139: `is_group_hovered` must
    // consult the precomputed `hovered_groups` HashSet rather than scan
    // `prev_group_rects` linearly. We assert behavior, not the algorithm:
    // populate the HashSet manually and confirm the lookup honors it without
    // requiring matching rects.
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 20, 5, &mut state, Theme::dark());

    // Without a mouse position the lookup must short-circuit to `false`,
    // even if the HashSet is non-empty.
    ctx.hovered_groups
        .insert(std::sync::Arc::<str>::from("widget"));
    assert!(!ctx.is_group_hovered("widget"));

    // With a mouse position, lookup must consult the HashSet.
    ctx.mouse_pos = Some((1, 1));
    assert!(ctx.is_group_hovered("widget"));
    assert!(!ctx.is_group_hovered("other"));
}

#[test]
fn screen_hook_map_avoids_repeat_allocation_on_cache_hit() {
    // Regression for #134 (Option B): the second and later frames for the
    // same screen must reuse the existing `String` key. We verify the slot
    // is populated and updated in place across frames.
    use crate::test_utils::TestBackend;
    use crate::widgets::ScreenState;

    let mut screens = ScreenState::new("a");
    let mut backend = TestBackend::new(20, 5);

    backend.render(|ui| {
        ui.screen("a", &mut screens, |ui| {
            let _ = ui.use_state(|| 0i32);
        });
        // First frame inserted a key.
        assert!(
            ui.screen_hook_map
                .get(&screens.id())
                .is_some_and(|hooks| hooks.contains_key("a"))
        );
    });

    backend.render(|ui| {
        ui.screen("a", &mut screens, |ui| {
            let _ = ui.use_state(|| 0i32);
        });
        // Second frame must reuse the same slot, not double up.
        assert_eq!(ui.screen_state_count(), 1);
    });
}

#[test]
fn same_named_screen_states_keep_independent_hook_segments() {
    let mut backend = crate::TestBackend::new(40, 8);
    let mut first = crate::widgets::ScreenState::new("home");
    let mut second = crate::widgets::ScreenState::new("home");

    backend.render(|ui| {
        ui.screen("home", &mut first, |ui| {
            let value = ui.use_state(|| 1u32);
            *value.get_mut(ui) = 7;
        });
        ui.screen("home", &mut second, |ui| {
            let value = ui.use_state(|| String::from("two"));
            value.get_mut(ui).push_str("-saved");
        });
    });

    backend.render(|ui| {
        ui.screen("home", &mut first, |ui| {
            assert_eq!(*ui.use_state(|| 0u32).get(ui), 7);
        });
        ui.screen("home", &mut second, |ui| {
            assert_eq!(ui.use_state(String::new).get(ui).as_str(), "two-saved");
        });
    });
}

#[test]
fn inactive_screen_allocates_its_hook_segment_on_first_activation() {
    let mut backend = crate::TestBackend::new(40, 8);
    let mut screens = crate::widgets::ScreenState::new("home");
    screens.push("settings");

    backend.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            let _ = ui.use_state(String::new);
        });
        ui.screen("settings", &mut screens, |ui| {
            let value = ui.use_state(|| 5u32);
            *value.get_mut(ui) = 9;
        });
    });

    screens.pop();
    backend.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            let value = ui.use_state(|| String::from("home"));
            assert_eq!(value.get(ui), "home");
        });
        ui.screen("settings", &mut screens, |_| {});
    });
}

#[test]
fn render_notifications_preserves_queue_after_render() {
    // Regression for the non-empty path of #138: `render_notifications`
    // moves the queue out for rendering, then restores it so subsequent
    // frames continue to display un-expired entries.
    use crate::test_utils::TestBackend;

    let mut backend = TestBackend::new(40, 10);
    backend.render(|ui| {
        ui.notify("saved", crate::widgets::ToastLevel::Success);
        assert_eq!(ui.rollback.notification_queue.len(), 1);
        ui.render_notifications();
        // Queue must still hold the entry after rendering.
        assert_eq!(ui.rollback.notification_queue.len(), 1);
    });
}

// ── key_chord (issue #262) ───────────────────────────────────────────

#[test]
fn key_chord_matches_across_frames() {
    use crate::test_utils::TestBackend;
    use std::cell::Cell;

    let mut tb = TestBackend::new(40, 4);
    let frame1 = Cell::new(false);
    let frame2 = Cell::new(false);

    tb.sequence()
        .key(KeyCode::Char('g'), |ui| {
            frame1.set(ui.key_chord("gg"));
            ui.text("hi");
        })
        .key(KeyCode::Char('g'), |ui| {
            frame2.set(ui.key_chord("gg"));
            ui.text("hi");
        })
        .run();

    assert!(!frame1.get(), "first `g` must not complete the chord");
    assert!(
        frame2.get(),
        "second `g` on the next frame completes the chord"
    );
}

#[test]
fn key_chord_resets_on_mismatch() {
    use crate::test_utils::TestBackend;
    use std::cell::Cell;

    fn bump(fired: &Cell<u32>, ui: &mut Context) {
        if ui.key_chord("gg") {
            fired.set(fired.get() + 1);
        }
        ui.text("hi");
    }

    let mut tb = TestBackend::new(40, 4);
    let fired = Cell::new(0u32);

    tb.sequence()
        .key(KeyCode::Char('g'), |ui| bump(&fired, ui))
        .key(KeyCode::Char('x'), |ui| bump(&fired, ui)) // cancels the pending `gg`
        .key(KeyCode::Char('g'), |ui| bump(&fired, ui))
        .key(KeyCode::Char('g'), |ui| bump(&fired, ui)) // re-armed: this completes
        .run();

    assert_eq!(
        fired.get(),
        1,
        "chord fires once, only on the trailing `gg`"
    );
}

#[test]
fn key_chord_overlap_rearm() {
    // `g g g` should complete on the second `g` via longest-suffix overlap.
    use crate::test_utils::TestBackend;
    use std::cell::Cell;

    fn bump(fired: &Cell<u32>, ui: &mut Context) {
        if ui.key_chord("gg") {
            fired.set(fired.get() + 1);
        }
        ui.text("hi");
    }

    let mut tb = TestBackend::new(40, 4);
    let fired = Cell::new(0u32);

    tb.sequence()
        .key(KeyCode::Char('g'), |ui| bump(&fired, ui))
        .key(KeyCode::Char('g'), |ui| bump(&fired, ui)) // completes here
        .run();

    assert_eq!(fired.get(), 1);
}

#[test]
fn key_chord_timeout_expires() {
    // Drive `Context::new` directly so we can advance the tick clock, which
    // `TestBackend` does not bump between `render` calls.
    let mut state = FrameState::default();

    // Frame at tick 0: type `g`, arming the chord.
    state.diagnostics.tick = 0;
    let mut ctx = Context::new(vec![Event::key_char('g')], 40, 4, &mut state, Theme::dark());
    assert!(!ctx.key_chord("gg"));
    state.chord_states = std::mem::take(&mut ctx.chord);
    assert_eq!(state.chord_states.pending, "g");

    // Frame well past the default timeout, with no key: prefix must expire.
    state.diagnostics.tick = crate::DEFAULT_CHORD_TIMEOUT_TICKS + 5;
    let mut ctx = Context::new(Vec::new(), 40, 4, &mut state, Theme::dark());
    assert!(!ctx.key_chord("gg"));
    state.chord_states = std::mem::take(&mut ctx.chord);
    assert_eq!(
        state.chord_states.pending, "",
        "stale prefix must be cleared after timeout"
    );

    // A fresh `g` after expiry only arms again; it must not complete.
    let mut ctx = Context::new(vec![Event::key_char('g')], 40, 4, &mut state, Theme::dark());
    assert!(!ctx.key_chord("gg"), "post-timeout `g` must not complete");
}

#[test]
fn key_chord_consumes_final_key() {
    use crate::test_utils::TestBackend;
    use std::cell::Cell;

    let mut tb = TestBackend::new(40, 4);
    let completed = Cell::new(false);
    let leftover = Cell::new(true);

    tb.sequence()
        .key(KeyCode::Char('g'), |ui| {
            ui.key_chord("gg");
            ui.text("hi");
        })
        .key(KeyCode::Char('g'), |ui| {
            completed.set(ui.key_chord("gg"));
            // The completing `g` was consumed, so a sibling `key('g')` check
            // in the same frame must see nothing.
            leftover.set(ui.key('g'));
            ui.text("hi");
        })
        .run();

    assert!(completed.get());
    assert!(!leftover.get(), "completing key must be consumed");
}

#[test]
fn key_chord_leader_notation() {
    use crate::test_utils::TestBackend;
    use std::cell::Cell;

    // `<space>` token form.
    let mut tb = TestBackend::new(40, 4);
    let fired = Cell::new(false);
    tb.sequence()
        .key(KeyCode::Char(' '), |ui| {
            ui.key_chord("<space>ff");
            ui.text("hi");
        })
        .key(KeyCode::Char('f'), |ui| {
            ui.key_chord("<space>ff");
            ui.text("hi");
        })
        .key(KeyCode::Char('f'), |ui| {
            fired.set(ui.key_chord("<space>ff"));
            ui.text("hi");
        })
        .run();
    assert!(fired.get(), "`<space>ff` matches space then f f");

    // Literal-space form must behave identically.
    let mut tb = TestBackend::new(40, 4);
    let fired_literal = Cell::new(false);
    tb.sequence()
        .key(KeyCode::Char(' '), |ui| {
            ui.key_chord(" ff");
            ui.text("hi");
        })
        .key(KeyCode::Char('f'), |ui| {
            ui.key_chord(" ff");
            ui.text("hi");
        })
        .key(KeyCode::Char('f'), |ui| {
            fired_literal.set(ui.key_chord(" ff"));
            ui.text("hi");
        })
        .run();
    assert!(fired_literal.get(), "literal `\" ff\"` matches identically");
}

#[test]
fn key_chord_leader_alias_token() {
    use crate::test_utils::TestBackend;
    use std::cell::Cell;

    let mut tb = TestBackend::new(40, 4);
    let fired = Cell::new(false);
    tb.sequence()
        .key(KeyCode::Char(' '), |ui| {
            ui.key_chord("<leader>w");
            ui.text("hi");
        })
        .key(KeyCode::Char('w'), |ui| {
            fired.set(ui.key_chord("<leader>w"));
            ui.text("hi");
        })
        .run();
    assert!(fired.get(), "`<leader>w` maps the leader token to space");
}

#[test]
fn key_chord_empty_returns_false() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(vec![Event::key_char('g')], 40, 4, &mut state, Theme::dark());
    assert!(!ctx.key_chord(""), "empty sequence is always false");
}

#[test]
fn key_chord_modal_guard() {
    // With a modal active last frame and no overlay layered on top, a fully
    // typed chord must not fire.
    let mut state = FrameState::default();
    state.focus.prev_modal_active = true;
    let events = vec![Event::key_char('g'), Event::key_char('g')];
    let mut ctx = Context::new(events, 40, 4, &mut state, Theme::dark());
    assert!(
        !ctx.key_chord("gg"),
        "modal guard suppresses chords (overlay_depth == 0)"
    );
}

#[test]
#[allow(deprecated)] // regression-lock the deprecated `key_seq` delegation
fn key_seq_deprecated_alias_matches_across_frames() {
    use crate::test_utils::TestBackend;
    use std::cell::Cell;

    let mut tb = TestBackend::new(40, 4);
    let frame2 = Cell::new(false);
    tb.sequence()
        .key(KeyCode::Char('g'), |ui| {
            ui.key_seq("gg");
            ui.text("hi");
        })
        .key(KeyCode::Char('g'), |ui| {
            frame2.set(ui.key_seq("gg"));
            ui.text("hi");
        })
        .run();
    assert!(
        frame2.get(),
        "deprecated `key_seq` now matches across frames via `key_chord`"
    );
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
