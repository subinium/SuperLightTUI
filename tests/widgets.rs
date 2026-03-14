use slt::widgets::*;
use slt::TestBackend;

#[test]
fn text_renders() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.text("hello world");
    });
    tb.assert_contains("hello world");
}

#[test]
fn canvas_colored_shapes() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.canvas(20, 5, |cv| {
            cv.set_color(slt::Color::Cyan);
            cv.circle(20, 10, 8);
            cv.set_color(slt::Color::Yellow);
            cv.filled_rect(0, 0, 10, 10);
            cv.layer();
            cv.set_color(slt::Color::White);
            cv.print(5, 5, "Hi");
        });
    });
    tb.assert_contains("Hi");
}

#[test]
fn button_renders_label() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.button("Click me");
    });
    tb.assert_contains("Click me");
}

#[test]
fn checkbox_renders_unchecked() {
    let mut tb = TestBackend::new(40, 5);
    let mut checked = false;
    tb.render(|ui| {
        ui.checkbox("Option", &mut checked);
    });
    tb.assert_contains("[ ]");
    tb.assert_contains("Option");
}

#[test]
fn checkbox_renders_checked() {
    let mut tb = TestBackend::new(40, 5);
    let mut checked = true;
    tb.render(|ui| {
        ui.checkbox("Option", &mut checked);
    });
    tb.assert_contains("[x]");
}

#[test]
fn toggle_renders_off() {
    let mut tb = TestBackend::new(40, 5);
    let mut on = false;
    tb.render(|ui| {
        ui.toggle("Feature", &mut on);
    });
    tb.assert_contains("Feature");
    tb.assert_contains("OFF");
}

#[test]
fn toggle_renders_on() {
    let mut tb = TestBackend::new(40, 5);
    let mut on = true;
    tb.render(|ui| {
        ui.toggle("Feature", &mut on);
    });
    tb.assert_contains("Feature");
    tb.assert_contains("ON");
}

#[test]
fn text_input_renders_placeholder() {
    let mut tb = TestBackend::new(40, 5);
    let mut input = TextInputState::with_placeholder("Search...");
    tb.render(|ui| {
        ui.text_input(&mut input);
    });
    tb.assert_contains("Search...");
}

#[test]
fn text_input_renders_value() {
    let mut tb = TestBackend::new(40, 5);
    let mut input = TextInputState::new();
    input.value = "hello".into();
    input.cursor = 5;
    tb.render(|ui| {
        ui.text_input(&mut input);
    });
    tb.assert_contains("hello");
}

#[test]
fn text_input_validation_error_renders() {
    let mut tb = TestBackend::new(40, 5);
    let mut input = TextInputState::new();
    input.validation_error = Some("too short".into());
    tb.render(|ui| {
        ui.text_input(&mut input);
    });
    tb.assert_contains("⚠ too short");
}

#[test]
fn text_input_validate_method() {
    let mut input = TextInputState::new();
    input.value = "abc".into();

    input.validate(|value| {
        if value.len() >= 5 {
            Ok(())
        } else {
            Err("too short".into())
        }
    });
    assert_eq!(input.validation_error.as_deref(), Some("too short"));

    input.value = "abcdef".into();
    input.validate(|value| {
        if value.len() >= 5 {
            Ok(())
        } else {
            Err("too short".into())
        }
    });
    assert_eq!(input.validation_error, None);
}

#[test]
fn form_renders_fields() {
    let mut tb = TestBackend::new(40, 10);
    let mut form = FormState::new()
        .field(FormField::new("Email").placeholder("you@example.com"))
        .field(FormField::new("Password").placeholder("********"));

    tb.render(|ui| {
        ui.form(&mut form, |ui, form| {
            for field in form.fields.iter_mut() {
                ui.form_field(field);
            }
        });
    });

    tb.assert_contains("Email");
    tb.assert_contains("Password");
}

#[test]
fn form_validation() {
    let mut form = FormState::new()
        .field(FormField::new("Email"))
        .field(FormField::new("Password"));

    form.fields[0].input.value = "invalid-email".into();
    form.fields[1].input.value = "short".into();

    let valid = form.validate(&[
        |v| {
            if v.contains('@') {
                Ok(())
            } else {
                Err("invalid email".into())
            }
        },
        |v| {
            if v.len() >= 8 {
                Ok(())
            } else {
                Err("too short".into())
            }
        },
    ]);

    assert!(!valid);
    assert_eq!(form.fields[0].error.as_deref(), Some("invalid email"));
    assert_eq!(form.fields[1].error.as_deref(), Some("too short"));

    form.fields[0].input.value = "user@example.com".into();
    form.fields[1].input.value = "long-enough".into();

    let valid = form.validate(&[
        |v| {
            if v.contains('@') {
                Ok(())
            } else {
                Err("invalid email".into())
            }
        },
        |v| {
            if v.len() >= 8 {
                Ok(())
            } else {
                Err("too short".into())
            }
        },
    ]);

    assert!(valid);
    assert_eq!(form.fields[0].error, None);
    assert_eq!(form.fields[1].error, None);
}

#[test]
fn list_renders_items() {
    let mut tb = TestBackend::new(40, 10);
    let mut list = ListState::new(vec!["Apple", "Banana", "Cherry"]);
    tb.render(|ui| {
        ui.list(&mut list);
    });
    tb.assert_contains("Apple");
    tb.assert_contains("Banana");
    tb.assert_contains("Cherry");
}

#[test]
fn list_empty_no_panic() {
    let mut tb = TestBackend::new(40, 5);
    let mut list = ListState::new(Vec::<String>::new());
    tb.render(|ui| {
        ui.list(&mut list);
    });
}

#[test]
fn table_renders_headers() {
    let mut tb = TestBackend::new(60, 10);
    let mut table = TableState::new(
        vec!["Name", "Age"],
        vec![vec!["Alice", "30"], vec!["Bob", "25"]],
    );
    tb.render(|ui| {
        ui.table(&mut table);
    });
    tb.assert_contains("Name");
    tb.assert_contains("Age");
    tb.assert_contains("Alice");
}

#[test]
fn table_empty_rows_no_panic() {
    let mut tb = TestBackend::new(60, 10);
    let mut table = TableState::new(vec!["Name", "Age"], Vec::<Vec<String>>::new());
    tb.render(|ui| {
        ui.table(&mut table);
    });
    tb.assert_contains("Name");
}

#[test]
fn tabs_renders_labels() {
    let mut tb = TestBackend::new(40, 5);
    let mut tabs = TabsState::new(vec!["Tab1", "Tab2", "Tab3"]);
    tb.render(|ui| {
        ui.tabs(&mut tabs);
    });
    tb.assert_contains("Tab1");
    tb.assert_contains("Tab2");
}

#[test]
fn tabs_empty_no_panic() {
    let mut tb = TestBackend::new(40, 5);
    let mut tabs = TabsState::new(Vec::<String>::new());
    tb.render(|ui| {
        ui.tabs(&mut tabs);
    });
}

#[test]
fn progress_renders() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.progress(0.5);
    });
    tb.assert_contains("█");
    tb.assert_contains("░");
}

#[test]
fn spinner_renders() {
    let mut tb = TestBackend::new(40, 5);
    let spinner = SpinnerState::dots();
    tb.render(|ui| {
        ui.spinner(&spinner);
    });
    tb.assert_contains("⠋");
}

#[test]
fn separator_renders() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.text("above");
        ui.separator();
        ui.text("below");
    });
    tb.assert_contains("above");
    tb.assert_contains("below");
    tb.assert_contains("─");
}

#[test]
fn help_renders_keys() {
    let mut tb = TestBackend::new(60, 5);
    tb.render(|ui| {
        ui.help(&[("q", "quit"), ("Tab", "focus")]);
    });
    tb.assert_contains("q");
    tb.assert_contains("quit");
}

#[test]
fn textarea_renders() {
    let mut tb = TestBackend::new(40, 10);
    let mut ta = TextareaState::new();
    ta.set_value("line1\nline2");
    tb.render(|ui| {
        ui.textarea(&mut ta, 5);
    });
    tb.assert_contains("line1");
    tb.assert_contains("line2");
}

#[test]
fn scrollable_renders_content() {
    let mut tb = TestBackend::new(40, 10);
    let mut scroll = ScrollState::new();
    tb.render(|ui| {
        ui.scrollable(&mut scroll).col(|ui| {
            for i in 0..20 {
                ui.text(format!("Item {i}"));
            }
        });
    });
    tb.assert_contains("Item 0");
}

#[test]
fn col_stacks_vertically() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.col(|ui| {
            ui.text("first");
            ui.text("second");
        });
    });
    tb.assert_line_contains(0, "first");
    tb.assert_line_contains(1, "second");
}

#[test]
fn row_stacks_horizontally() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.row(|ui| {
            ui.text("left");
            ui.text("right");
        });
    });
    let line = tb.line(0);
    assert!(line.contains("left") && line.contains("right"));
}

#[test]
fn spacer_pushes_content() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.row(|ui| {
            ui.text("L");
            ui.spacer();
            ui.text("R");
        });
    });
    let line = tb.line(0);
    let l_pos = line.find('L').expect("L should render");
    let r_pos = line.rfind('R').expect("R should render");
    assert!(r_pos > l_pos + 5, "Spacer should push R far from L");
}

#[test]
fn nested_containers() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.col(|ui| {
            ui.row(|ui| {
                ui.text("A");
                ui.text("B");
            });
            ui.row(|ui| {
                ui.text("C");
                ui.text("D");
            });
        });
    });
    tb.assert_contains("A");
    tb.assert_contains("B");
    tb.assert_contains("C");
    tb.assert_contains("D");
}

#[test]
fn custom_widget_renders() {
    struct Label(String);

    impl slt::Widget for Label {
        type Response = ();

        fn ui(&mut self, ui: &mut slt::Context) {
            ui.text(&self.0);
        }
    }

    let mut tb = TestBackend::new(40, 5);
    let mut label = Label("custom".into());
    tb.render(|ui| {
        ui.widget(&mut label);
    });
    tb.assert_contains("custom");
}

#[test]
fn error_boundary_catches_panic() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.error_boundary(|_| {
            panic!("test panic");
        });
    });
    tb.assert_contains("Error");
    tb.assert_contains("test panic");
}

#[test]
fn error_boundary_passes_through_normal() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.error_boundary(|ui| {
            ui.text("safe content");
        });
    });
    tb.assert_contains("safe content");
}

#[test]
fn error_boundary_with_custom_fallback() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.error_boundary_with(
            |_| {
                panic!("oops");
            },
            |ui, msg| {
                ui.text(format!("Caught: {msg}"));
            },
        );
    });
    tb.assert_contains("Caught: oops");
}

#[test]
fn toast_renders_message() {
    let mut tb = TestBackend::new(40, 5);
    let mut toasts = ToastState::new();
    toasts.info("Hello toast", 0);
    tb.render(|ui| {
        ui.toast(&mut toasts);
    });
    tb.assert_contains("Hello toast");
}

#[test]
fn toast_empty_no_render() {
    let mut tb = TestBackend::new(40, 5);
    let mut toasts = ToastState::new();
    tb.render(|ui| {
        ui.toast(&mut toasts);
    });
}

#[test]
fn chart_renders_with_axes() {
    let mut tb = TestBackend::new(60, 20);
    tb.render(|ui| {
        ui.chart(
            |c| {
                c.title("Test");
                c.xlabel("X");
                c.ylabel("Y");
                c.line(&[(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)])
                    .label("S1")
                    .color(slt::Color::Cyan);
            },
            50,
            15,
        );
    });
    tb.assert_contains("Test");
    tb.assert_contains("┌");
    tb.assert_contains("S1");
}

#[test]
fn chart_multi_series() {
    let mut tb = TestBackend::new(60, 20);
    tb.render(|ui| {
        ui.chart(
            |c| {
                c.line(&[(0.0, 1.0), (1.0, 4.0)])
                    .label("A")
                    .color(slt::Color::Cyan);
                c.scatter(&[(0.5, 2.0), (1.5, 3.0)])
                    .label("B")
                    .color(slt::Color::Yellow);
                c.legend(slt::LegendPosition::TopRight);
            },
            50,
            15,
        );
    });
    tb.assert_contains("A");
    tb.assert_contains("B");
}

#[test]
fn chart_empty_data_no_panic() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.chart(|_c| {}, 30, 8);
    });
}

#[test]
fn histogram_renders() {
    let mut tb = TestBackend::new(50, 15);
    let data = [1.0, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0];
    tb.render(|ui| {
        ui.histogram(&data, 40, 10);
    });
    let line0 = tb.line(0);
    assert!(line0.contains("█") || line0.contains("▁") || line0.len() > 0);
}

#[test]
fn histogram_empty_no_panic() {
    let mut tb = TestBackend::new(40, 10);
    let data: [f64; 0] = [];
    tb.render(|ui| {
        ui.histogram(&data, 30, 8);
    });
}

#[test]
fn bar_chart_styled_horizontal() {
    let mut tb = TestBackend::new(50, 10);
    let bars = vec![
        slt::Bar::new("A", 10.0).color(slt::Color::Cyan),
        slt::Bar::new("B", 20.0).color(slt::Color::Red),
    ];
    tb.render(|ui| {
        ui.bar_chart_styled(&bars, 20, slt::BarDirection::Horizontal);
    });
    tb.assert_contains("A");
    tb.assert_contains("B");
    tb.assert_contains("█");
}

#[test]
fn bar_chart_styled_vertical() {
    let mut tb = TestBackend::new(50, 15);
    let bars = vec![slt::Bar::new("X", 5.0), slt::Bar::new("Y", 10.0)];
    tb.render(|ui| {
        ui.bar_chart_styled(&bars, 8, slt::BarDirection::Vertical);
    });
    tb.assert_contains("X");
    tb.assert_contains("Y");
}

#[test]
fn bar_chart_grouped_renders() {
    let mut tb = TestBackend::new(50, 15);
    let groups = vec![
        slt::BarGroup::new(
            "G1",
            vec![slt::Bar::new("a", 10.0), slt::Bar::new("b", 20.0)],
        ),
        slt::BarGroup::new(
            "G2",
            vec![slt::Bar::new("a", 15.0), slt::Bar::new("b", 25.0)],
        ),
    ];
    tb.render(|ui| {
        ui.bar_chart_grouped(&groups, 20);
    });
    tb.assert_contains("G1");
    tb.assert_contains("G2");
}

#[test]
fn sparkline_styled_renders() {
    let mut tb = TestBackend::new(40, 5);
    let data: Vec<(f64, Option<slt::Color>)> = vec![
        (10.0, Some(slt::Color::Green)),
        (20.0, Some(slt::Color::Red)),
        (f64::NAN, None),
        (15.0, None),
    ];
    tb.render(|ui| {
        ui.sparkline_styled(&data, 10);
    });
}

// ── Korean / CJK text handling ──────────────────────────────────

#[test]
fn text_input_korean_char_insert() {
    let mut tb = TestBackend::new(40, 5);
    let mut input = TextInputState::new();
    let events = slt::EventBuilder::new().key('한').key('글').build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.text_input(&mut input);
    });
    assert_eq!(input.value, "한글");
    assert_eq!(input.cursor, 2);
}

#[test]
fn text_input_korean_backspace() {
    let mut tb = TestBackend::new(40, 5);
    let mut input = TextInputState::new();
    input.value = "한글".into();
    input.cursor = 2;
    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Backspace)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.text_input(&mut input);
    });
    assert_eq!(input.value, "한");
    assert_eq!(input.cursor, 1);
}

#[test]
fn text_input_korean_renders_cursor() {
    let mut tb = TestBackend::new(40, 5);
    let mut input = TextInputState::new();
    input.value = "한글".into();
    input.cursor = 1;
    tb.render_with_events(Vec::new(), 0, 1, |ui| {
        ui.text_input(&mut input);
    });
    tb.assert_contains("한");
    tb.assert_contains("글");
    tb.assert_contains("▎");
}

#[test]
fn text_input_delete_forward() {
    let mut tb = TestBackend::new(40, 5);
    let mut input = TextInputState::new();
    input.value = "abc".into();
    input.cursor = 1;
    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Delete)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.text_input(&mut input);
    });
    assert_eq!(input.value, "ac");
    assert_eq!(input.cursor, 1);
}

#[test]
fn text_input_paste_inserts_text() {
    let mut tb = TestBackend::new(40, 5);
    let mut input = TextInputState::new();
    input.value = "ab".into();
    input.cursor = 1;
    let events = slt::EventBuilder::new().paste("XY").build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.text_input(&mut input);
    });
    assert_eq!(input.value, "aXYb");
    assert_eq!(input.cursor, 3);
}

#[test]
fn text_input_paste_korean() {
    let mut tb = TestBackend::new(40, 5);
    let mut input = TextInputState::new();
    let events = slt::EventBuilder::new().paste("안녕하세요").build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.text_input(&mut input);
    });
    assert_eq!(input.value, "안녕하세요");
    assert_eq!(input.cursor, 5);
}

#[test]
fn textarea_paste_with_newlines() {
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();
    let events = slt::EventBuilder::new()
        .paste("line1\nline2\nline3")
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["line1", "line2", "line3"]);
    assert_eq!(state.cursor_row, 2);
    assert_eq!(state.cursor_col, 5);
}

#[test]
fn textarea_delete_forward() {
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();
    state.lines = vec!["hello".into(), "world".into()];
    state.cursor_row = 0;
    state.cursor_col = 5;
    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Delete)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["helloworld"]);
}

#[test]
fn textarea_delete_forward_mid_line() {
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();
    state.lines = vec!["한글입력".into()];
    state.cursor_row = 0;
    state.cursor_col = 1;
    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Delete)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["한입력"]);
    assert_eq!(state.cursor_col, 1);
}

#[test]
fn text_input_mixed_width_cursor_navigation() {
    let mut tb = TestBackend::new(40, 5);
    let mut input = TextInputState::new();
    input.value = "A한B".into();
    input.cursor = 0;

    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Right)
        .key_code(slt::KeyCode::Right)
        .key_code(slt::KeyCode::Right)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.text_input(&mut input);
    });
    assert_eq!(input.cursor, 3);

    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Left)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.text_input(&mut input);
    });
    assert_eq!(input.cursor, 2);
}

#[test]
fn text_renders_korean() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.text("한글 텍스트 렌더링");
    });
    tb.assert_contains("한글 텍스트 렌더링");
}

#[test]
fn link_renders_text() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.link("SLT Docs", "https://docs.rs/superlighttui");
    });
    tb.assert_contains("SLT Docs");
}

#[test]
fn link_style_chaining() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.link("Bold Link", "https://example.com").bold();
    });
    tb.assert_contains("Bold Link");
}

#[test]
fn modal_renders_on_top() {
    let mut tb = TestBackend::new(5, 1);
    tb.render(|ui| {
        ui.text("aaaaa");
        ui.modal(|ui| {
            ui.text("TOP");
        });
    });

    let line = tb.line(0);
    assert!(
        line.contains("TOP"),
        "Expected modal content on top, got: {line}"
    );
}

#[test]
fn overlay_renders_content() {
    let mut tb = TestBackend::new(6, 1);
    tb.render(|ui| {
        ui.text("aaaaaa");
        ui.overlay(|ui| {
            ui.text("OVR");
        });
    });

    let line = tb.line(0);
    assert!(
        line.contains("OVR"),
        "Expected overlay content to render, got: {line}"
    );
}

#[test]
fn link_sets_hyperlink_on_cells() {
    let mut tb = TestBackend::new(40, 3);
    tb.render(|ui| {
        ui.link("Click Me", "https://example.com");
    });
    tb.assert_contains("Click Me");
    let cell = tb.buffer().get(0, 0);
    assert_eq!(
        cell.hyperlink.as_deref(),
        Some("https://example.com"),
        "Expected hyperlink URL on link cell, got: {:?}",
        cell.hyperlink
    );
    let empty_cell = tb.buffer().get(20, 0);
    assert!(
        empty_cell.hyperlink.is_none(),
        "Non-link cell should not have hyperlink"
    );
}

#[test]
fn link_default_style_is_underlined_cyan() {
    let mut tb = TestBackend::new(40, 3);
    tb.render(|ui| {
        ui.link("Docs", "https://docs.rs");
    });
    let cell = tb.buffer().get(0, 0);
    assert!(
        cell.style
            .modifiers
            .contains(slt::style::Modifiers::UNDERLINE),
        "Link should be underlined by default"
    );
    assert_eq!(
        cell.style.fg,
        Some(slt::Color::Cyan),
        "Link should be cyan (theme.primary) by default"
    );
}

#[test]
fn modal_dims_background_content() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.text("Background Text");
        ui.modal(|ui| {
            ui.text("Modal Content");
        });
    });
    let bg_cell = tb.buffer().get(0, 0);
    assert!(
        bg_cell.style.modifiers.contains(slt::style::Modifiers::DIM),
        "Background should be dimmed when modal is active, got modifiers: {:?}",
        bg_cell.style.modifiers
    );
    tb.assert_contains("Modal Content");
}

#[test]
fn modal_renders_centered_on_large_screen() {
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        ui.text("background");
        ui.modal(|ui| {
            ui.bordered(slt::Border::Rounded).pad(1).col(|ui| {
                ui.text("Hello Modal");
                if ui.button("OK") {}
            });
        });
    });
    tb.assert_contains("Hello Modal");
    tb.assert_contains("OK");
}

#[test]
fn modal_button_activates_with_enter() {
    use slt::{EventBuilder, KeyCode};
    let mut activated = false;
    let events = EventBuilder::new().key_code(KeyCode::Enter).build();
    let mut tb = TestBackend::new(40, 10);
    tb.render_with_events(events, 0, 1, |ui| {
        ui.modal(|ui| {
            if ui.button("Confirm") {
                activated = true;
            }
        });
    });
    assert!(activated, "Button inside modal should activate with Enter");
}

#[test]
fn textarea_word_wrap_renders_wrapped_lines() {
    let mut tb = TestBackend::new(20, 10);
    let mut state = TextareaState::new().word_wrap(10);
    state.set_value("abcdefghijklmno");
    tb.render(|ui| {
        ui.textarea(&mut state, 5);
    });
    tb.assert_line_contains(0, "abcdefghij");
    tb.assert_line_contains(1, "klmno");
}

#[test]
fn textarea_word_wrap_cursor_down_navigates_visual() {
    use slt::{EventBuilder, KeyCode};
    let mut tb = TestBackend::new(20, 10);
    let mut state = TextareaState::new().word_wrap(5);
    state.set_value("abcdefghij");
    state.cursor_row = 0;
    state.cursor_col = 2;
    let events = EventBuilder::new().key_code(KeyCode::Down).build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.cursor_row, 0);
    assert_eq!(state.cursor_col, 7);
}

#[test]
fn textarea_word_wrap_cursor_up_navigates_visual() {
    use slt::{EventBuilder, KeyCode};
    let mut tb = TestBackend::new(20, 10);
    let mut state = TextareaState::new().word_wrap(5);
    state.set_value("abcdefghij");
    state.cursor_row = 0;
    state.cursor_col = 7;
    let events = EventBuilder::new().key_code(KeyCode::Up).build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.cursor_row, 0);
    assert_eq!(state.cursor_col, 2);
}

#[test]
fn textarea_word_wrap_scroll_follows_cursor() {
    let mut tb = TestBackend::new(20, 10);
    let mut state = TextareaState::new().word_wrap(5);
    state.set_value("abcdefghijklmnopqrstuvwxyz");
    state.cursor_row = 0;
    state.cursor_col = 24;
    tb.render(|ui| {
        ui.textarea(&mut state, 3);
    });
    assert!(state.scroll_offset > 0);
}

#[test]
fn textarea_word_wrap_korean() {
    let mut tb = TestBackend::new(20, 10);
    let mut state = TextareaState::new().word_wrap(8);
    state.set_value("가나다라마바사아");
    tb.render(|ui| {
        ui.textarea(&mut state, 5);
    });
    tb.assert_line_contains(0, "가나다라");
    tb.assert_line_contains(1, "마바사아");
}

#[test]
fn modal_with_max_w_renders_centered() {
    let mut tb = TestBackend::new(80, 20);
    tb.render(|ui| {
        ui.text("bg");
        ui.modal(|ui| {
            ui.bordered(slt::Border::Rounded)
                .pad(1)
                .max_w(30)
                .col(|ui| {
                    ui.text("Center Me");
                });
        });
    });
    for y in 0..20u32 {
        let line = tb.line(y);
        if line.contains("Center Me") {
            let x = line.find("Center Me").unwrap();
            assert!(
                x >= 20,
                "Modal should be centered (x={x}), but appears left-aligned"
            );
            return;
        }
    }
    panic!("Modal content 'Center Me' not found in buffer");
}

#[test]
fn container_bg_propagates_to_text() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.container().bg(slt::Color::Red).col(|ui| {
            ui.text("Hello");
        });
    });
    let cell = tb.buffer().get(0, 0);
    assert_eq!(
        cell.style.bg,
        Some(slt::Color::Red),
        "Text cell should inherit container bg(Red), got: {:?}",
        cell.style.bg
    );
}

#[test]
fn container_bg_propagates_to_border() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.container()
            .bg(slt::Color::Blue)
            .border(slt::Border::Rounded)
            .col(|ui: &mut slt::Context| {
                ui.text("Inside");
            });
    });
    let corner_cell = tb.buffer().get(0, 0);
    assert_eq!(
        corner_cell.style.bg,
        Some(slt::Color::Blue),
        "Border corner cell should inherit container bg(Blue), got: {:?}",
        corner_cell.style.bg
    );
}

#[test]
fn nested_container_bg_inheritance() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.container().bg(slt::Color::Green).col(|ui| {
            ui.container().col(|ui| {
                ui.text("Deep");
            });
        });
    });
    let cell = tb.buffer().get(0, 0);
    assert_eq!(
        cell.style.bg,
        Some(slt::Color::Green),
        "Nested text cell should inherit outer container bg(Green), got: {:?}",
        cell.style.bg
    );
}

#[test]
fn child_bg_overrides_parent_bg() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.container().bg(slt::Color::Red).col(|ui| {
            ui.container().bg(slt::Color::Yellow).col(|ui| {
                ui.text("Override");
            });
        });
    });
    let cell = tb.buffer().get(0, 0);
    assert_eq!(
        cell.style.bg,
        Some(slt::Color::Yellow),
        "Child container bg should override parent bg, got: {:?}",
        cell.style.bg
    );
}

#[test]
fn select_renders_closed() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = SelectState::new(vec!["Apple", "Banana", "Cherry"]);
    state.selected = 1;

    tb.render(|ui| {
        ui.select(&mut state);
    });

    tb.assert_contains("Banana");
}

#[test]
fn select_renders_open() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = SelectState::new(vec!["Apple", "Banana", "Cherry"]);
    state.open = true;

    tb.render(|ui| {
        ui.select(&mut state);
    });

    tb.assert_contains("Apple");
    tb.assert_contains("Banana");
    tb.assert_contains("Cherry");
}

#[test]
fn radio_renders_options() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = RadioState::new(vec!["One", "Two", "Three"]);

    tb.render(|ui| {
        ui.radio(&mut state);
    });

    tb.assert_contains("● One");
    tb.assert_contains("○ Two");
    tb.assert_contains("○ Three");
}

#[test]
fn radio_selected_marker() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = RadioState::new(vec!["One", "Two", "Three"]);
    state.selected = 1;

    tb.render(|ui| {
        ui.radio(&mut state);
    });

    tb.assert_contains("○ One");
    tb.assert_contains("● Two");
    tb.assert_contains("○ Three");
}

#[test]
fn multi_select_renders_options() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = MultiSelectState::new(vec!["One", "Two", "Three"]);

    tb.render(|ui| {
        ui.multi_select(&mut state);
    });

    tb.assert_contains("[ ] One");
    tb.assert_contains("[ ] Two");
    tb.assert_contains("[ ] Three");
}

#[test]
fn multi_select_checked_items() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = MultiSelectState::new(vec!["One", "Two", "Three"]);
    state.selected.insert(0);
    state.selected.insert(2);

    tb.render(|ui| {
        ui.multi_select(&mut state);
    });

    tb.assert_contains("[x] One");
    tb.assert_contains("[ ] Two");
    tb.assert_contains("[x] Three");
}

#[test]
fn tree_renders_root() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = TreeState::new(vec![TreeNode::new("Root")]);

    tb.render(|ui| {
        ui.tree(&mut state);
    });

    tb.assert_contains("Root");
}

#[test]
fn tree_renders_expanded() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = TreeState::new(vec![TreeNode::new("Root")
        .expanded()
        .children(vec![TreeNode::new("Child A"), TreeNode::new("Child B")])]);

    tb.render(|ui| {
        ui.tree(&mut state);
    });

    tb.assert_contains("Root");
    tb.assert_contains("Child A");
    tb.assert_contains("Child B");
}

#[test]
fn tree_renders_collapsed() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = TreeState::new(vec![
        TreeNode::new("Root").children(vec![TreeNode::new("Hidden Child")])
    ]);

    tb.render(|ui| {
        ui.tree(&mut state);
    });

    tb.assert_contains("Root");
    assert!(!tb.to_string().contains("Hidden Child"));
}

#[test]
fn virtual_list_renders_items() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = ListState::new(vec![
        "Item 0", "Item 1", "Item 2", "Item 3", "Item 4", "Item 5",
    ]);

    tb.render(|ui| {
        ui.virtual_list(&mut state, 3, |ui, idx| {
            ui.text(format!("Item {idx}"));
        });
    });

    tb.assert_contains("Item 0");
    tb.assert_contains("Item 1");
    tb.assert_contains("Item 2");
    assert!(!tb.to_string().contains("Item 3"));
}

#[test]
fn command_palette_closed() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = CommandPaletteState::new(vec![
        PaletteCommand::new("Open File", "Open a file from disk"),
        PaletteCommand::new("Save File", "Save current buffer"),
    ]);
    state.open = false;

    tb.render(|ui| {
        let _ = ui.command_palette(&mut state);
    });

    assert!(!tb.to_string().contains("Open File"));
    assert!(!tb.to_string().contains("Save File"));
}

#[test]
fn command_palette_open() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = CommandPaletteState::new(vec![
        PaletteCommand::new("Open File", "Open a file from disk"),
        PaletteCommand::new("Save File", "Save current buffer"),
    ]);
    state.open = true;

    tb.render(|ui| {
        let _ = ui.command_palette(&mut state);
    });

    tb.assert_contains("Open File");
    tb.assert_contains("Save File");
}

#[test]
fn markdown_heading() {
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        ui.markdown("# Hello");
    });
    tb.assert_contains("Hello");
}

#[test]
fn markdown_bold() {
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        ui.markdown("**bold**");
    });
    tb.assert_contains("bold");
}

#[test]
fn markdown_list() {
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        ui.markdown("- item1\n- item2");
    });
    tb.assert_contains("item1");
    tb.assert_contains("item2");
}

#[test]
fn key_seq_matches_sequence() {
    let mut tb = TestBackend::new(80, 24);
    let events = slt::EventBuilder::new().key('g').key('g').build();
    let mut matched = false;

    tb.render_with_events(events, 0, 1, |ui| {
        matched = ui.key_seq("gg");
    });

    assert!(matched);
}

#[test]
fn key_seq_rejects_non_sequence() {
    let mut tb = TestBackend::new(80, 24);
    let events = slt::EventBuilder::new().key('g').key('x').build();
    let mut matched = false;

    tb.render_with_events(events, 0, 1, |ui| {
        matched = ui.key_seq("gg");
    });

    assert!(!matched);
}

#[test]
fn password_masked() {
    let mut tb = TestBackend::new(80, 24);
    let mut input = TextInputState::new();
    input.value = "secret".into();
    input.cursor = input.value.chars().count();
    input.masked = true;

    tb.render(|ui| {
        ui.text_input(&mut input);
    });

    tb.assert_contains("••••••");
    assert!(!tb.to_string().contains("secret"));
}

#[test]
fn password_unmasked() {
    let mut tb = TestBackend::new(80, 24);
    let mut input = TextInputState::new();
    input.value = "secret".into();
    input.cursor = input.value.chars().count();
    input.masked = false;

    tb.render(|ui| {
        ui.text_input(&mut input);
    });

    tb.assert_contains("secret");
}

#[test]
fn percentage_width() {
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        ui.row(|ui| {
            ui.container()
                .w_pct(50)
                .border(slt::Border::Rounded)
                .col(|ui| {
                    ui.text("Half Width");
                });
            ui.container()
                .w_pct(50)
                .border(slt::Border::Rounded)
                .col(|ui| {
                    ui.text("Other Half");
                });
        });
    });

    tb.assert_contains("Half Width");
    tb.assert_contains("Other Half");
}
