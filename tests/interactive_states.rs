//! Issue #458: element-level `:hover` / `:focus` / `:focus-within` /
//! `:active` container styles, the `disabled` scope, keyed animations, and
//! the group-hover fixes.

use slt::widgets::TextInputState;
use slt::{Border, Color, Context, EventBuilder, KeyCode, TestBackend};

fn bg(tb: &TestBackend, x: u32, y: u32) -> Option<Color> {
    tb.buffer().get(x, y).style.bg
}

fn fg(tb: &TestBackend, x: u32, y: u32) -> Option<Color> {
    tb.buffer().get(x, y).style.fg
}

#[test]
fn hover_bg_highlights_only_the_hovered_sibling() {
    let rows = |ui: &mut Context| {
        for label in ["row-one", "row-two"] {
            let _ = ui.container().hover_bg(Color::Blue).row(|ui| {
                ui.text(label);
            });
        }
    };
    let mut tb = TestBackend::new(20, 2);
    tb.render(rows);
    tb.render_with_events(EventBuilder::new().mouse_move(2, 0).build(), 0, 0, rows);
    assert_eq!(bg(&tb, 0, 0), Some(Color::Blue));
    assert_eq!(bg(&tb, 0, 1), None);
    tb.render_with_events(EventBuilder::new().mouse_move(2, 1).build(), 0, 0, rows);
    assert_eq!(bg(&tb, 0, 0), None);
    assert_eq!(bg(&tb, 0, 1), Some(Color::Blue));
}

#[test]
fn focus_bg_applies_without_a_mouse_and_hover_bg_does_not() {
    let mut tb = TestBackend::new(20, 2);
    tb.render(|ui| {
        let _ = ui.register_focusable();
        let _ = ui
            .container()
            .focus_bg(Color::Blue)
            .hover_bg(Color::Red)
            .row(|ui| {
                ui.text("focused");
            });
        let _ = ui.register_focusable();
        let _ = ui
            .container()
            .focus_bg(Color::Blue)
            .hover_bg(Color::Red)
            .row(|ui| {
                ui.text("other");
            });
    });
    assert_eq!(bg(&tb, 0, 0), Some(Color::Blue), "focused slot");
    assert_eq!(bg(&tb, 0, 1), None, "unfocused, unhovered");

    // A hover-only container whose focusable holds focus stays unstyled.
    let mut tb = TestBackend::new(20, 2);
    tb.render(|ui| {
        let _ = ui.register_focusable();
        let _ = ui.container().hover_bg(Color::Red).row(|ui| {
            ui.text("focused");
        });
    });
    assert_eq!(bg(&tb, 0, 0), None, "hover_bg never fires from focus");
}

#[test]
fn focus_within_border_tracks_a_nested_text_input() {
    let mut input = TextInputState::new();
    let mut scene = |ui: &mut Context| {
        let _ = ui
            .bordered(Border::Rounded)
            .focus_within_border_fg(Color::Cyan)
            .col(|ui| {
                let _ = ui.text_input(&mut input);
            });
        let _ = ui.button("Next");
    };
    let mut tb = TestBackend::new(30, 6);
    tb.render(&mut scene);
    assert_eq!(fg(&tb, 0, 0), Some(Color::Cyan), "input focused");
    tb.sequence()
        .events(
            EventBuilder::new().key_code(KeyCode::Tab).build(),
            &mut scene,
        )
        .run();
    tb.render(&mut scene);
    assert_ne!(fg(&tb, 0, 0), Some(Color::Cyan), "focus left the container");
}

#[test]
fn active_bg_applies_between_press_and_release() {
    let scene = |ui: &mut Context| {
        let _ = ui.container().active_bg(Color::Green).row(|ui| {
            ui.text("press me");
        });
    };
    let mut tb = TestBackend::new(20, 2);
    tb.render(scene);
    assert_eq!(bg(&tb, 0, 0), None);
    tb.sequence()
        .events(EventBuilder::new().click(2, 0).build(), scene)
        .run();
    assert_eq!(bg(&tb, 0, 0), Some(Color::Green), "pressed");
    tb.sequence().tick(scene).run();
    assert_eq!(bg(&tb, 0, 0), Some(Color::Green), "still held");
    tb.sequence()
        .events(EventBuilder::new().mouse_up(2, 0).build(), scene)
        .run();
    assert_eq!(bg(&tb, 0, 0), None, "released");
}

#[test]
fn container_style_carries_state_styles() {
    const ROW: slt::ContainerStyle = slt::ContainerStyle::new().hover_bg(Color::Blue);
    let rows = |ui: &mut Context| {
        let _ = ui.container().apply(&ROW).row(|ui| {
            ui.text("styled");
        });
    };
    let mut tb = TestBackend::new(20, 1);
    tb.render(rows);
    tb.render_with_events(EventBuilder::new().mouse_move(1, 0).build(), 0, 0, rows);
    assert_eq!(bg(&tb, 0, 0), Some(Color::Blue));
}

#[derive(Default, Debug)]
struct Frame {
    focused: Vec<&'static str>,
    clicked: Vec<&'static str>,
    focus_index: usize,
}

fn buttons(ui: &mut Context, disable_b: bool, frame: &mut Frame) {
    let record = |name: &'static str, response: slt::Response, frame: &mut Frame| {
        if response.focused {
            frame.focused.push(name);
        }
        if response.clicked {
            frame.clicked.push(name);
        }
    };
    let a = ui.button("A").show();
    record("A", a, frame);
    let b = ui.disabled(disable_b, |ui| ui.button("B").show());
    assert_eq!(b.disabled, disable_b);
    record("B", b, frame);
    let c = ui.button("C").show();
    record("C", c, frame);
    frame.focus_index = ui.focus_index();
}

#[test]
fn disabled_button_ignores_click_enter_and_space() {
    let mut tb = TestBackend::new(30, 3);
    let mut frame = Frame::default();
    tb.render(|ui| buttons(ui, true, &mut frame));
    let b = tb.find_text("B").expect("button B");

    let mut frame = Frame::default();
    tb.run_with_events(EventBuilder::new().click(b.0, b.1).build(), |ui| {
        buttons(ui, true, &mut frame)
    });
    assert!(frame.clicked.is_empty(), "click: {frame:?}");

    for code in [KeyCode::Enter, KeyCode::Char(' ')] {
        let mut frame = Frame::default();
        tb.render_with_events(
            EventBuilder::new().key_code(code.clone()).build(),
            1,
            3,
            |ui| buttons(ui, true, &mut frame),
        );
        assert!(frame.clicked.is_empty(), "{code:?}: {frame:?}");
        assert!(
            frame.focused.is_empty(),
            "a disabled slot never reports focus"
        );
    }
}

#[test]
fn tab_skips_a_disabled_button() {
    let mut tb = TestBackend::new(30, 3);
    let mut frame = Frame::default();
    tb.render(|ui| buttons(ui, true, &mut frame));
    assert_eq!(frame.focused, ["A"]);
    tb.sequence()
        .events(EventBuilder::new().key_code(KeyCode::Tab).build(), |ui| {
            buttons(ui, true, &mut Frame::default())
        })
        .run();
    let mut frame = Frame::default();
    tb.sequence().tick(|ui| buttons(ui, true, &mut frame)).run();
    assert_eq!(frame.focused, ["C"], "Tab went from A straight to C");

    // Shift+Tab from C skips B back to A.
    tb.sequence()
        .events(
            EventBuilder::new().key_code(KeyCode::BackTab).build(),
            |ui| buttons(ui, true, &mut Frame::default()),
        )
        .run();
    let mut frame = Frame::default();
    tb.sequence().tick(|ui| buttons(ui, true, &mut frame)).run();
    assert_eq!(frame.focused, ["A"]);
}

#[test]
fn toggling_disabled_keeps_later_focus_indices() {
    for disabled in [false, true] {
        let mut tb = TestBackend::new(30, 3);
        let mut frame = Frame::default();
        tb.render(|ui| buttons(ui, disabled, &mut frame));
        let mut frame = Frame::default();
        tb.render_with_events(Vec::new(), 2, 3, |ui| buttons(ui, disabled, &mut frame));
        assert_eq!(frame.focused, ["C"], "disabled={disabled}");
        assert_eq!(frame.focus_index, 2, "disabled={disabled}");
    }
}

#[test]
fn clicking_a_disabled_widget_does_not_move_focus_to_it() {
    let mut tb = TestBackend::new(30, 3);
    let mut frame = Frame::default();
    tb.render(|ui| buttons(ui, true, &mut frame));
    tb.render(|ui| buttons(ui, true, &mut Frame::default()));
    let b = tb.find_text("B").expect("button B");
    tb.sequence()
        .events(EventBuilder::new().click(b.0, b.1).build(), |ui| {
            buttons(ui, true, &mut Frame::default())
        })
        .run();
    let mut frame = Frame::default();
    tb.sequence().tick(|ui| buttons(ui, true, &mut frame)).run();
    assert_eq!(frame.focused, ["A"], "focus stays where it was");
}

#[test]
fn disabled_scope_blocks_checkbox_toggle_and_typing() {
    let mut checked = false;
    let mut on = false;
    let mut input = TextInputState::new();
    let mut tb = TestBackend::new(40, 6);
    let mut scene = |ui: &mut Context, events_target: &mut (bool, bool)| {
        ui.disabled(true, |ui| {
            let check = ui.checkbox("Agree", &mut checked);
            let toggle = ui.toggle("Dark", &mut on);
            let typed = ui.text_input(&mut input);
            events_target.0 |= check.changed || toggle.changed || typed.changed;
            events_target.1 |= check.focused || toggle.focused || typed.focused;
        });
    };
    let mut seen = (false, false);
    tb.render(|ui| scene(ui, &mut seen));
    let agree = tb.find_text("Agree").expect("checkbox");
    for index in 0..3 {
        tb.render_with_events(
            EventBuilder::new()
                .click(agree.0, agree.1)
                .key_code(KeyCode::Enter)
                .key('x')
                .build(),
            index,
            3,
            |ui| scene(ui, &mut seen),
        );
    }
    assert_eq!(seen, (false, false));
    assert!(!checked && !on);
    assert_eq!(input.value, "");
}

#[test]
fn disabled_scope_renders_dim_text() {
    let mut tb = TestBackend::new(30, 3);
    let dim = slt::Theme::dark().text_dim;
    tb.render(|ui| {
        ui.disabled(true, |ui| {
            let _ = ui.button("Submit");
        });
    });
    let (x, y) = tb.find_text("Submit").expect("label");
    assert_eq!(fg(&tb, x, y), Some(dim));
}

#[test]
fn nested_enable_cannot_escape_a_disabled_scope() {
    let mut tb = TestBackend::new(30, 3);
    let mut inner = slt::Response::none();
    tb.render(|ui| {
        ui.disabled(true, |ui| {
            inner = ui.disabled(false, |ui| ui.button("Inner").show());
            assert!(ui.is_disabled());
        });
        assert!(!ui.is_disabled());
    });
    assert!(inner.disabled);
}

#[test]
fn keyed_animations_progress_independently() {
    let mut tb = TestBackend::new(10, 1);
    let mut values = (0.0, 0.0);
    tb.render(|ui| {
        values = (
            ui.animate_bool_keyed("row-a", false),
            ui.animate_bool_keyed(String::from("row-b"), false),
        );
    });
    assert_eq!(values, (0.0, 0.0));
    for _ in 0..4 {
        tb.render(|ui| {
            values = (
                ui.animate_bool_keyed("row-a", true),
                ui.animate_bool_keyed(String::from("row-b"), false),
            );
        });
    }
    assert!(values.0 > 0.0, "row-a animates toward 1: {values:?}");
    assert_eq!(values.1, 0.0, "row-b stays at 0");
    let mut keyed = 0;
    tb.render(|ui| {
        assert!(ui.remove_state_keyed("row-a"));
        keyed = ui.keyed_state_count();
    });
    assert_eq!(keyed, 1);
}

#[test]
fn shared_group_names_highlight_only_the_hovered_row() {
    let rows = |ui: &mut Context| {
        for label in ["row-one", "row-two"] {
            let _ = ui.group("row").group_hover_bg(Color::Blue).row(|ui| {
                ui.text(label);
            });
        }
    };
    let mut tb = TestBackend::new(20, 2);
    tb.render(rows);
    tb.render_with_events(EventBuilder::new().mouse_move(2, 0).build(), 0, 0, rows);
    assert_eq!(bg(&tb, 0, 0), Some(Color::Blue));
    assert_eq!(bg(&tb, 0, 1), None, "the second row shares the name only");
}

#[test]
fn group_hover_does_not_fire_from_focus() {
    let mut input = TextInputState::new();
    let mut tb = TestBackend::new(30, 3);
    // Two frames: the pre-0.26 bug read the previous frame's focus groups.
    for _ in 0..2 {
        tb.render(|ui| {
            let _ = ui.group("field").group_hover_bg(Color::Blue).col(|ui| {
                let _ = ui.text_input(&mut input);
            });
        });
    }
    assert_eq!(bg(&tb, 0, 0), None, "a focused child is not a hover");
}

#[test]
fn a_disabled_text_input_does_not_keep_the_keyboard() {
    let mut input = TextInputState::new();
    let mut tb = TestBackend::new(30, 3);
    // Focused and editing.
    tb.render(|ui| {
        let _ = ui.text_input(&mut input);
    });
    // Disabled while it held focus: shortcuts work again.
    let mut quit = false;
    tb.render(|ui| {
        ui.disabled(true, |ui| {
            let _ = ui.text_input(&mut input);
        });
    });
    tb.sequence()
        .events(EventBuilder::new().key('q').build(), |ui| {
            ui.disabled(true, |ui| {
                let _ = ui.text_input(&mut input);
            });
            quit = ui.key('q');
        })
        .run();
    assert!(quit, "the disabled field no longer swallows shortcut keys");
    assert_eq!(input.value, "");
}
