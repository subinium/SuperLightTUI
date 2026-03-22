use super::*;
use crate::test_utils::TestBackend;
use crate::EventBuilder;

#[test]
fn use_memo_type_mismatch_includes_index_and_expected_type() {
    let mut state = FrameState::default();
    let mut ctx = Context::new(Vec::new(), 20, 5, &mut state, Theme::dark());
    ctx.hook_states.push(Box::new(42u32));
    ctx.hook_cursor = 0;

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
    let mut state = FrameState {
        focus_index: 3,
        prev_focus_count: 5,
        prev_modal_active: true,
        prev_modal_focus_start: 3,
        prev_modal_focus_count: 2,
        ..FrameState::default()
    };
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
    let mut state = FrameState {
        focus_index: 3,
        prev_focus_count: 5,
        prev_modal_active: true,
        prev_modal_focus_start: 3,
        prev_modal_focus_count: 2,
        ..FrameState::default()
    };
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
