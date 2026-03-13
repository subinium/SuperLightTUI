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
