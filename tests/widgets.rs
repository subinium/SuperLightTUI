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
fn use_state_persists_across_renders() {
    let mut tb = TestBackend::new(40, 10);

    tb.render(|ui| {
        let s = ui.use_state(|| 42i32);
        assert_eq!(*s.get(ui), 42);
    });

    tb.render(|ui| {
        let s = ui.use_state(|| 0i32);
        assert_eq!(*s.get(ui), 42);
    });
}

#[test]
fn use_state_mutation_persists() {
    let mut tb = TestBackend::new(40, 10);

    tb.render(|ui| {
        let s = ui.use_state(|| 10i32);
        *s.get_mut(ui) = 99;
    });

    tb.render(|ui| {
        let s = ui.use_state(|| 0i32);
        assert_eq!(*s.get(ui), 99);
    });
}

#[test]
fn use_memo_caches_when_deps_unchanged() {
    let mut tb = TestBackend::new(40, 10);
    let call_count = std::rc::Rc::new(std::cell::Cell::new(0));

    let first = call_count.clone();
    tb.render(|ui| {
        let val = ui.use_memo(&5i32, |d| {
            first.set(first.get() + 1);
            d * 2
        });
        assert_eq!(*val, 10);
    });

    let second = call_count.clone();
    tb.render(|ui| {
        let val = ui.use_memo(&5i32, |d| {
            second.set(second.get() + 1);
            d * 2
        });
        assert_eq!(*val, 10);
    });

    assert_eq!(call_count.get(), 1);
}

#[test]
fn use_memo_recomputes_on_dep_change() {
    let mut tb = TestBackend::new(40, 10);
    let call_count = std::rc::Rc::new(std::cell::Cell::new(0));

    let first = call_count.clone();
    tb.render(|ui| {
        let val = ui.use_memo(&3i32, |d| {
            first.set(first.get() + 1);
            d * 10
        });
        assert_eq!(*val, 30);
    });

    let second = call_count.clone();
    tb.render(|ui| {
        let val = ui.use_memo(&7i32, |d| {
            second.set(second.get() + 1);
            d * 10
        });
        assert_eq!(*val, 70);
    });

    assert_eq!(call_count.get(), 2);
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
fn list_filter_single_token() {
    let mut list = ListState::new(vec!["deploy failed", "health check", "deploy success"]);
    list.set_filter("deploy");
    assert_eq!(list.visible_indices(), &[0, 2]);
}

#[test]
fn list_filter_multi_token() {
    let mut list = ListState::new(vec![
        "error deploy failed",
        "deploy success",
        "error health check",
    ]);
    list.set_filter("error deploy");
    assert_eq!(list.visible_indices(), &[0]);
}

#[test]
fn list_filter_no_match() {
    let mut list = ListState::new(vec!["alpha", "beta", "gamma"]);
    list.set_filter("zzz");
    assert_eq!(list.visible_indices(), &[]);
}

#[test]
fn list_filter_empty_shows_all() {
    let mut list = ListState::new(vec!["alpha", "beta", "gamma"]);
    list.set_filter("alpha");
    list.set_filter("");
    assert_eq!(list.visible_indices(), &[0, 1, 2]);
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
fn table_sort_ascending() {
    let mut table = TableState::new(
        vec!["Name", "Score"],
        vec![
            vec!["Bob", "10"],
            vec!["Alice", "20"],
            vec!["Charlie", "30"],
        ],
    );
    table.sort_by(0);
    assert_eq!(table.visible_indices(), &[1, 0, 2]);

    let mut tb = TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.table(&mut table);
    });
    tb.assert_contains("▲");
}

#[test]
fn table_sort_descending_toggle() {
    let mut table = TableState::new(
        vec!["Name", "Score"],
        vec![
            vec!["Bob", "10"],
            vec!["Alice", "20"],
            vec!["Charlie", "30"],
        ],
    );
    table.toggle_sort(0);
    assert_eq!(table.visible_indices(), &[1, 0, 2]);
    table.toggle_sort(0);
    assert_eq!(table.visible_indices(), &[2, 0, 1]);
}

#[test]
fn table_sort_numeric() {
    let mut table = TableState::new(
        vec!["Name", "Value"],
        vec![vec!["A", "2"], vec!["B", "10"], vec!["C", "1"]],
    );
    table.sort_by(1);
    assert_eq!(table.visible_indices(), &[2, 0, 1]);
}

#[test]
fn table_filter_basic() {
    let mut table = TableState::new(
        vec!["Name", "City"],
        vec![
            vec!["Alice", "Seoul"],
            vec!["Bob", "Busan"],
            vec!["Lila", "Jeju"],
        ],
    );
    table.set_filter("li");
    assert_eq!(table.visible_indices(), &[0, 2]);
}

#[test]
fn table_filter_case_insensitive() {
    let mut table = TableState::new(
        vec!["Name", "City"],
        vec![vec!["Alice", "Seoul"], vec!["Bob", "Busan"]],
    );
    table.set_filter("ALICE");
    assert_eq!(table.visible_indices(), &[0]);
}

#[test]
fn table_filter_no_match() {
    let mut table = TableState::new(
        vec!["Name", "City"],
        vec![vec!["Alice", "Seoul"], vec!["Bob", "Busan"]],
    );
    table.set_filter("zzz");
    assert_eq!(table.visible_indices(), &[]);
    assert_eq!(table.selected_row(), None);
}

#[test]
fn table_filter_multi_token_cross_column() {
    let mut table = TableState::new(
        vec!["Level", "Message"],
        vec![
            vec!["ERROR", "deploy failed"],
            vec!["INFO", "deploy success"],
            vec!["ERROR", "health check ok"],
        ],
    );
    table.set_filter("ERROR deploy");
    assert_eq!(table.visible_indices(), &[0]);
}

#[test]
fn table_filter_multi_token_same_column() {
    let mut table = TableState::new(
        vec!["Name", "City"],
        vec![vec!["Alice", "Seoul"], vec!["Bob", "Busan"]],
    );
    table.set_filter("Ali ce");
    assert_eq!(table.visible_indices(), &[0]);
}

#[test]
fn table_filter_single_token_unchanged() {
    let mut table = TableState::new(
        vec!["Name", "City"],
        vec![
            vec!["Alice", "Seoul"],
            vec!["Bob", "Busan"],
            vec!["Lila", "Jeju"],
        ],
    );
    table.set_filter("li");
    assert_eq!(table.visible_indices(), &[0, 2]);
}

#[test]
fn table_filter_whitespace_only_shows_all() {
    let mut table = TableState::new(
        vec!["Name", "City"],
        vec![vec!["Alice", "Seoul"], vec!["Bob", "Busan"]],
    );
    table.set_filter("   ");
    assert_eq!(table.visible_indices(), &[0, 1]);
}

#[test]
fn table_pagination_basic() {
    let mut table = TableState::new(
        vec!["Name", "Value"],
        vec![
            vec!["A", "1"],
            vec!["B", "2"],
            vec!["C", "3"],
            vec!["D", "4"],
            vec!["E", "5"],
        ],
    );
    table.page_size = 2;
    assert_eq!(table.total_pages(), 3);
    assert_eq!(table.page, 0);

    let mut tb = TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.table(&mut table);
    });
    tb.assert_contains("Page 1/3");

    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::PageDown)
        .build();
    tb.run_with_events(events, |ui| {
        ui.table(&mut table);
    });

    assert_eq!(table.page, 1);
}

#[test]
fn table_pagination_last_page() {
    let mut table = TableState::new(
        vec!["Name", "Value"],
        vec![vec!["A", "1"], vec!["B", "2"], vec!["C", "3"]],
    );
    table.page_size = 2;
    table.next_page();
    table.next_page();
    assert_eq!(table.page, 1);
    assert_eq!(table.total_pages(), 2);
    table.prev_page();
    assert_eq!(table.page, 0);
}

#[test]
fn table_sort_and_filter_combined() {
    let mut table = TableState::new(
        vec!["Name", "Value"],
        vec![vec!["Alpha", "20"], vec!["Beta", "3"], vec!["Alfred", "10"]],
    );
    table.sort_by(1);
    table.set_filter("al");
    assert_eq!(table.visible_indices(), &[2, 0]);
}

#[test]
fn table_selected_row_with_sort() {
    let mut table = TableState::new(
        vec!["Name", "Value"],
        vec![vec!["Bob", "2"], vec!["Alice", "1"], vec!["Carol", "3"]],
    );
    table.sort_by(0);
    table.selected = 1;
    let selected = table
        .selected_row()
        .expect("expected selected row after sorting");
    assert_eq!(selected[0], "Bob");
}

#[test]
fn table_backward_compat() {
    let mut table = TableState::new(
        vec!["Name", "Age"],
        vec![vec!["Alice", "30"], vec!["Bob", "25"]],
    );

    assert_eq!(table.sort_column, None);
    assert!(table.sort_ascending);
    assert_eq!(table.filter, "");
    assert_eq!(table.page, 0);
    assert_eq!(table.page_size, 0);
    assert_eq!(table.visible_indices(), &[0, 1]);

    table.selected = 1;
    let selected = table
        .selected_row()
        .expect("expected selected row in default behavior");
    assert_eq!(selected[0], "Bob");

    let mut tb = TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.table(&mut table);
    });
    let output = tb.to_string();
    assert!(!output.contains("▲"));
    assert!(!output.contains("▼"));
    assert!(!output.contains("Page "));
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
fn group_hover_bg_applied() {
    let mut tb = TestBackend::new(40, 10);
    let events = slt::EventBuilder::new().click(5, 2).build();
    tb.run_with_events(events, |ui| {
        ui.group("card").group_hover_bg(slt::Color::Blue).col(|ui| {
            ui.text("Card content");
        });
    });
    tb.assert_contains("Card content");
}

#[test]
fn group_renders_normally_without_hover() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.group("card").bg(slt::Color::Black).col(|ui| {
            ui.text("Normal");
        });
    });
    tb.assert_contains("Normal");
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
fn error_boundary_restores_focus_count() {
    let mut tb = TestBackend::new(40, 6);

    tb.render(|ui| {
        ui.error_boundary_with(
            |ui| {
                let _ = ui.register_focusable();
                panic!("focus panic");
            },
            |ui, _| {
                ui.text("Recovered");
            },
        );
    });

    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Tab)
        .key_code(slt::KeyCode::Enter)
        .build();
    let mut first_clicked = false;
    let mut second_clicked = false;

    tb.render_with_events(events, 0, 2, |ui| {
        if ui.button("First") {
            first_clicked = true;
        }
        if ui.button("Second") {
            second_clicked = true;
        }
    });

    assert!(
        !first_clicked,
        "Tab should move focus away from the first button"
    );
    assert!(
        second_clicked,
        "Second button should receive focus and activate after Tab"
    );
}

#[test]
fn error_boundary_restores_hook_cursor() {
    let mut tb = TestBackend::new(40, 6);

    tb.render(|ui| {
        ui.error_boundary_with(
            |ui| {
                let state = ui.use_state(|| 7i32);
                assert_eq!(*state.get(ui), 7);
                panic!("hook panic");
            },
            |ui, _| {
                ui.text("Recovered");
            },
        );
    });

    tb.render(|ui| {
        let state = ui.use_state(|| String::from("hook-ok"));
        assert_eq!(state.get(ui).as_str(), "hook-ok");
        ui.text(state.get(ui).clone());
    });

    tb.assert_contains("hook-ok");
}

#[test]
fn error_boundary_restores_modal_active() {
    let mut tb = TestBackend::new(40, 6);
    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Enter)
        .build();
    let mut clicked_after_recovery = false;

    tb.render_with_events(events, 0, 0, |ui| {
        ui.error_boundary_with(
            |ui| {
                ui.modal(|ui| {
                    ui.text("Modal");
                    panic!("modal panic");
                });
            },
            |ui, _| {
                ui.text("Recovered");
            },
        );

        if ui.button("After panic") {
            clicked_after_recovery = true;
        }
    });

    assert!(
        clicked_after_recovery,
        "Modal state should be reset so later widgets can receive focus"
    );

    tb.render(|ui| {
        ui.text("no dim");
    });
    let cell = tb.buffer().get(0, 0);
    assert!(
        !cell.style.modifiers.contains(slt::Modifiers::DIM),
        "Second frame should not be dimmed after modal panic"
    );
}

#[test]
fn error_boundary_restores_group_stack() {
    let mut tb = TestBackend::new(40, 8);

    tb.render(|ui| {
        ui.error_boundary_with(
            |ui| {
                ui.group("broken").col(|_| {
                    panic!("group panic");
                });
            },
            |ui, _| {
                ui.text("Recovered");
            },
        );
        ui.group("safe").col(|ui| {
            ui.text("safe group");
        });
    });
    tb.assert_contains("safe group");

    tb.render(|ui| {
        ui.group("next").col(|ui| {
            ui.text("next group");
        });
    });
    tb.assert_contains("next group");
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
fn scatter_renders_points() {
    let mut tb = TestBackend::new(60, 20);
    tb.render(|ui| {
        ui.scatter(&[(1.0, 2.0), (3.0, 4.0), (5.0, 1.0)], 50, 16);
    });
    assert!(!tb.to_string().trim().is_empty());
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
    assert!(line0.contains("█") || line0.contains("▁") || !line0.is_empty());
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
fn dark_mode_bg_applied() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.set_dark_mode(true);
        ui.container()
            .bg(slt::Color::White)
            .dark_bg(slt::Color::Black)
            .col(|ui| {
                ui.text("Dark");
            });
    });

    let cell = tb.buffer().get(0, 0);
    assert_eq!(cell.style.bg, Some(slt::Color::Black));
}

#[test]
fn dark_mode_off_uses_normal_bg() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.set_dark_mode(false);
        ui.container()
            .bg(slt::Color::White)
            .dark_bg(slt::Color::Black)
            .col(|ui| {
                ui.text("Light");
            });
    });

    let cell = tb.buffer().get(0, 0);
    assert_eq!(cell.style.bg, Some(slt::Color::White));
}

#[test]
fn responsive_md_w_applied_at_80_cols() {
    let mut tb = TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.row(|ui| {
            ui.container().w(20).md_w(40).h(1).col(|_ui| {});
            ui.text("X");
        });
    });

    let line = tb.line(0);
    let x = line.find('X').expect("marker should be rendered");
    assert_eq!(x, 40, "md_w(40) should override base w(20) at 80 cols");
}

#[test]
fn responsive_sm_w_ignored_at_80_cols() {
    let mut tb = TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.row(|ui| {
            ui.container().w(20).sm_w(40).h(1).col(|_ui| {});
            ui.text("X");
        });
    });

    let line = tb.line(0);
    let x = line.find('X').expect("marker should be rendered");
    assert_eq!(x, 20, "sm_w(40) should be ignored at 80 cols (Md)");
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
fn command_palette_filter_single_token() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = CommandPaletteState::new(vec![
        PaletteCommand::new("Open File", "Open a file from disk"),
        PaletteCommand::new("Save File", "Save current buffer"),
        PaletteCommand::new("Quit", "Exit the application"),
    ]);
    state.open = true;
    state.input = "open".into();
    tb.render(|ui| {
        let _ = ui.command_palette(&mut state);
    });
    tb.assert_contains("Open File");
    assert!(!tb.to_string().contains("Save File"));
    assert!(!tb.to_string().contains("Quit"));
}

#[test]
fn command_palette_filter_multi_token_cross_field() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = CommandPaletteState::new(vec![
        PaletteCommand::new("Open File", "Open a file from disk"),
        PaletteCommand::new("Save File", "Save current buffer"),
        PaletteCommand::new("Quit", "Exit the application"),
    ]);
    state.open = true;
    state.input = "save buffer".into();
    tb.render(|ui| {
        let _ = ui.command_palette(&mut state);
    });
    tb.assert_contains("Save File");
    assert!(!tb.to_string().contains("Open File"));
    assert!(!tb.to_string().contains("Quit"));
}

#[test]
fn command_palette_filter_multi_token_no_match() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = CommandPaletteState::new(vec![
        PaletteCommand::new("Open File", "Open a file from disk"),
        PaletteCommand::new("Save File", "Save current buffer"),
    ]);
    state.open = true;
    state.input = "open buffer".into();
    tb.render(|ui| {
        let _ = ui.command_palette(&mut state);
    });
    assert!(!tb.to_string().contains("Open File"));
    assert!(!tb.to_string().contains("Save File"));
}

#[test]
fn command_palette_filter_whitespace_shows_all() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = CommandPaletteState::new(vec![
        PaletteCommand::new("Open File", "Open a file from disk"),
        PaletteCommand::new("Save File", "Save current buffer"),
    ]);
    state.open = true;
    state.input = "   ".into();
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

#[test]
fn line_renders_inline_text() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.line(|ui| {
            ui.text("hello ");
            ui.text("world");
        });
    });
    tb.assert_contains("hello world");
}

#[test]
fn line_preserves_different_styles() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.line(|ui| {
            ui.text("normal ");
            ui.text("bold").bold();
        });
    });
    tb.assert_contains("normal bold");
    let buf = tb.buffer();
    let bold_cell = buf.get(7, 0);
    assert!(
        bold_cell.style.modifiers.contains(slt::Modifiers::BOLD),
        "expected bold modifier on 'b' at x=7"
    );
    let normal_cell = buf.get(0, 0);
    assert!(
        !normal_cell.style.modifiers.contains(slt::Modifiers::BOLD),
        "expected no bold on 'n' at x=0"
    );
}

#[test]
fn line_with_fg_colors() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.line(|ui| {
            ui.text("Status: ");
            ui.text("Online").fg(slt::Color::Green);
        });
    });
    tb.assert_contains("Status: Online");
    let buf = tb.buffer();
    let green_cell = buf.get(8, 0);
    assert_eq!(green_cell.style.fg, Some(slt::Color::Green));
}

#[test]
fn line_in_container_builder() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.container().border(slt::Border::Rounded).line(|ui| {
            ui.text("a");
            ui.text("b").bold();
        });
    });
    tb.assert_contains("ab");
}

#[test]
fn markdown_inline_bold_styled() {
    let mut tb = TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.markdown("This is **bold** text");
    });
    tb.assert_contains("This is bold text");
    let buf = tb.buffer();
    let b_cell = buf.get(8, 0);
    assert!(
        b_cell.style.modifiers.contains(slt::Modifiers::BOLD),
        "expected bold on 'b' at x=8"
    );
    let t_cell = buf.get(0, 0);
    assert!(
        !t_cell.style.modifiers.contains(slt::Modifiers::BOLD),
        "expected no bold on 'T' at x=0"
    );
}

#[test]
fn markdown_inline_code_styled() {
    let mut tb = TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.markdown("Use `slt::run` here");
    });
    tb.assert_contains("Use slt::run here");
}

#[test]
fn markdown_inline_italic_styled() {
    let mut tb = TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.markdown("This is *italic* text");
    });
    tb.assert_contains("This is italic text");
    let buf = tb.buffer();
    let i_cell = buf.get(8, 0);
    assert!(
        i_cell.style.modifiers.contains(slt::Modifiers::ITALIC),
        "expected italic on 'i' at x=8"
    );
}

#[test]
fn markdown_list_with_bold() {
    let mut tb = TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.markdown("- a **bold** item");
    });
    tb.assert_contains("bold");
    tb.assert_contains("item");
}

#[test]
fn line_wrap_wraps_segments() {
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        ui.line_wrap(|ui| {
            ui.text("hello ");
            ui.text("world ").bold();
            ui.text("this wraps");
        });
    });
    tb.assert_contains("hello");
    tb.assert_contains("world");
    let output = tb.to_string();
    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.len() >= 2,
        "expected wrapping into 2+ lines, got {lines:?}"
    );
}

#[test]
fn line_wrap_preserves_styles_across_lines() {
    let mut tb = TestBackend::new(15, 5);
    tb.render(|ui| {
        ui.line_wrap(|ui| {
            ui.text("aaa ");
            ui.text("bbb").bold();
            ui.text(" ccc ddd");
        });
    });
    tb.assert_contains("bbb");
    let buf = tb.buffer();
    let b_cell = buf.get(4, 0);
    assert!(
        b_cell.style.modifiers.contains(slt::Modifiers::BOLD),
        "expected bold on 'b' at x=4"
    );
}

#[test]
fn line_wrap_single_line_no_wrap() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.line_wrap(|ui| {
            ui.text("short ");
            ui.text("text");
        });
    });
    tb.assert_contains("short text");
}

#[test]
fn border_dashed_renders() {
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        ui.bordered(slt::Border::Dashed).col(|ui| {
            ui.text("dashed");
        });
    });
    tb.assert_contains("dashed");
    let output = tb.to_string();
    assert!(
        output.contains('┄'),
        "Should contain dashed horizontal char"
    );
}

#[test]
fn border_dashed_thick_renders() {
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        ui.bordered(slt::Border::DashedThick).col(|ui| {
            ui.text("thick");
        });
    });
    tb.assert_contains("thick");
    let output = tb.to_string();
    assert!(
        output.contains('┅'),
        "Should contain thick dashed horizontal char"
    );
}

#[test]
fn key_event_kind_default_is_press() {
    use slt::{EventBuilder, KeyEventKind};

    let events = EventBuilder::new().key('a').build();
    for event in &events {
        if let slt::Event::Key(ke) = event {
            assert_eq!(ke.kind, KeyEventKind::Press);
        }
    }
}

#[test]
fn key_release_not_matched_by_key() {
    use slt::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    let mut tb = TestBackend::new(40, 5);
    let events = vec![Event::Key(KeyEvent {
        code: KeyCode::Char('q'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Release,
    })];
    let mut pressed = false;
    tb.run_with_events(events, |ui| {
        if ui.key('q') {
            pressed = true;
        }
        ui.text("test");
    });
    assert!(!pressed, "key() should NOT match Release events");
}

#[test]
fn color_downsample_truecolor_passthrough() {
    use slt::{Color, ColorDepth};

    let c = Color::Rgb(123, 45, 67);
    assert_eq!(c.downsampled(ColorDepth::TrueColor), c);
}

#[test]
fn color_downsample_eightbit_converts_rgb() {
    use slt::{Color, ColorDepth};

    let c = Color::Rgb(255, 0, 0);
    let d = c.downsampled(ColorDepth::EightBit);
    match d {
        Color::Indexed(_) => {}
        _ => panic!("Expected Indexed color, got {:?}", d),
    }
}

#[test]
fn color_downsample_basic_converts_rgb() {
    use slt::{Color, ColorDepth};

    let c = Color::Rgb(255, 0, 0);
    let d = c.downsampled(ColorDepth::Basic);
    assert_eq!(d, Color::Red, "Pure red RGB should map to Red");
}

#[test]
fn color_downsample_basic_named_passthrough() {
    use slt::{Color, ColorDepth};

    assert_eq!(Color::Green.downsampled(ColorDepth::Basic), Color::Green);
    assert_eq!(Color::Reset.downsampled(ColorDepth::Basic), Color::Reset);
}

#[test]
fn scrollbar_renders_thumb() {
    let mut tb = TestBackend::new(40, 10);
    let mut scroll = ScrollState::new();
    tb.render(|ui| {
        ui.container().h(8).row(|ui| {
            ui.scrollable(&mut scroll).grow(1).h(8).col(|ui| {
                for i in 0..50 {
                    ui.text(format!("Line {i}"));
                }
            });
            ui.scrollbar(&scroll);
        });
    });
    tb.render(|ui| {
        ui.container().h(8).row(|ui| {
            ui.scrollable(&mut scroll).grow(1).h(8).col(|ui| {
                for i in 0..50 {
                    ui.text(format!("Line {i}"));
                }
            });
            ui.scrollbar(&scroll);
        });
    });
    let output = tb.to_string();
    assert!(output.contains("Line 0"));
}

#[test]
fn scrollbar_no_render_when_content_fits() {
    let mut tb = TestBackend::new(40, 10);
    let mut scroll = ScrollState::new();
    tb.render(|ui| {
        ui.container().h(8).row(|ui| {
            ui.scrollable(&mut scroll).grow(1).h(8).col(|ui| {
                ui.text("short content");
            });
            ui.scrollbar(&scroll);
        });
    });
    tb.render(|ui| {
        ui.container().h(8).row(|ui| {
            ui.scrollable(&mut scroll).grow(1).h(8).col(|ui| {
                ui.text("short content");
            });
            ui.scrollbar(&scroll);
        });
    });
    let output = tb.to_string();
    assert!(!scroll.can_scroll_down());
    assert!(
        !output.contains('█'),
        "No thumb when content fits in viewport"
    );
}

#[test]
fn breakpoint_xs_under_40() {
    use slt::Breakpoint;

    let mut tb = TestBackend::new(30, 10);
    let mut bp = Breakpoint::Md;
    tb.render(|ui| {
        bp = ui.breakpoint();
    });
    assert_eq!(bp, Breakpoint::Xs);
}

#[test]
fn breakpoint_md_at_80() {
    use slt::Breakpoint;

    let mut tb = TestBackend::new(80, 24);
    let mut bp = Breakpoint::Xs;
    tb.render(|ui| {
        bp = ui.breakpoint();
    });
    assert_eq!(bp, Breakpoint::Md);
}

#[test]
fn breakpoint_xl_at_160() {
    use slt::Breakpoint;

    let mut tb = TestBackend::new(160, 24);
    let mut bp = Breakpoint::Xs;
    tb.render(|ui| {
        bp = ui.breakpoint();
    });
    assert_eq!(bp, Breakpoint::Xl);
}

#[test]
fn copy_to_clipboard_sets_field() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.copy_to_clipboard("test data");
        ui.text("clipboard");
    });
    tb.assert_contains("clipboard");
}

#[test]
fn streaming_text_renders_content() {
    let mut tb = TestBackend::new(40, 5);
    let mut state = StreamingTextState::new();
    state.push("Hello AI");
    tb.render(|ui| {
        ui.streaming_text(&mut state);
    });
    tb.assert_contains("Hello AI");
}

#[test]
fn streaming_text_shows_cursor_when_active() {
    let mut tb = TestBackend::new(40, 5);
    let mut state = StreamingTextState::new();
    state.start();
    state.push("typing");
    tb.render(|ui| {
        ui.streaming_text(&mut state);
    });
    let output = tb.to_string();
    assert!(output.contains("typing"), "Content should be visible");
}

#[test]
fn streaming_text_start_clears() {
    let mut state = StreamingTextState::new();
    state.push("old");
    state.start();
    assert!(state.content.is_empty());
    assert!(state.streaming);
}

#[test]
fn tool_approval_renders_pending() {
    let mut tb = TestBackend::new(60, 10);
    let mut tool = ToolApprovalState::new("read_file", "Read config.toml");
    tb.render(|ui| {
        ui.tool_approval(&mut tool);
    });
    tb.assert_contains("read_file");
    tb.assert_contains("Read config.toml");
    tb.assert_contains("Approve");
    tb.assert_contains("Reject");
}

#[test]
fn tool_approval_action_default_pending() {
    use slt::ApprovalAction;

    let tool = ToolApprovalState::new("test", "desc");
    assert_eq!(tool.action, ApprovalAction::Pending);
}

#[test]
fn context_bar_renders_items() {
    use slt::widgets::ContextItem;

    let mut tb = TestBackend::new(60, 5);
    let items = vec![
        ContextItem::new("main.rs", 1200),
        ContextItem::new("lib.rs", 800),
    ];
    tb.render(|ui| {
        ui.context_bar(&items);
    });
    tb.assert_contains("main.rs");
    tb.assert_contains("lib.rs");
}

#[test]
fn context_bar_empty_no_render() {
    use slt::widgets::ContextItem;

    let mut tb = TestBackend::new(40, 5);
    let items: Vec<ContextItem> = vec![];
    tb.render(|ui| {
        ui.context_bar(&items);
    });
    let output = tb.to_string();
    assert!(!output.contains("main.rs"));
}

#[test]
fn halfblock_image_from_rgb_renders() {
    use slt::HalfBlockImage;

    let rgb = vec![255u8; 4 * 2 * 3];
    let img = HalfBlockImage::from_rgb(&rgb, 4, 1);
    assert_eq!(img.width, 4);
    assert_eq!(img.height, 1);
    assert_eq!(img.pixels.len(), 4);

    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        ui.image(&img);
    });
    let output = tb.to_string();
    assert!(output.contains('▀'), "Should render half-block chars");
}

#[test]
fn halfblock_image_zero_size_no_panic() {
    use slt::HalfBlockImage;

    let img = HalfBlockImage::from_rgb(&[], 0, 0);
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        ui.image(&img);
    });
}

#[test]
fn theme_builder_defaults_from_dark() {
    let theme = slt::Theme::builder().build();
    let dark = slt::Theme::dark();

    assert_eq!(theme.primary, dark.primary);
    assert_eq!(theme.secondary, dark.secondary);
    assert_eq!(theme.accent, dark.accent);
    assert_eq!(theme.text, dark.text);
    assert_eq!(theme.text_dim, dark.text_dim);
    assert_eq!(theme.border, dark.border);
    assert_eq!(theme.bg, dark.bg);
    assert_eq!(theme.success, dark.success);
    assert_eq!(theme.warning, dark.warning);
    assert_eq!(theme.error, dark.error);
    assert_eq!(theme.selected_bg, dark.selected_bg);
    assert_eq!(theme.selected_fg, dark.selected_fg);
    assert_eq!(theme.surface, dark.surface);
    assert_eq!(theme.surface_hover, dark.surface_hover);
    assert_eq!(theme.surface_text, dark.surface_text);
}

#[test]
fn theme_builder_overrides() {
    let theme = slt::Theme::builder()
        .primary(slt::Color::Red)
        .text(slt::Color::Green)
        .build();
    let dark = slt::Theme::dark();

    assert_eq!(theme.primary, slt::Color::Red);
    assert_eq!(theme.text, slt::Color::Green);
    assert_eq!(theme.accent, dark.accent);
    assert_eq!(theme.surface_text, dark.surface_text);
}

#[test]
fn draw_raw_renders_to_buffer() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.container().w(10).h(3).draw(|buf, rect| {
            buf.set_char(rect.x, rect.y, 'X', slt::Style::new());
            buf.set_string(rect.x + 1, rect.y, "raw", slt::Style::new());
        });
    });
    tb.assert_contains("Xraw");
}

#[test]
fn draw_raw_respects_constraints() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.container().w(5).h(2).draw(|buf, rect| {
            assert_eq!(rect.width, 5);
            assert_eq!(rect.height, 2);
            for x in rect.x..rect.right() {
                buf.set_char(x, rect.y, '#', slt::Style::new());
            }
        });
    });
    tb.assert_contains("#####");
}

#[test]
fn draw_raw_clips_outside_rect() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.container().w(3).h(1).draw(|buf, rect| {
            buf.set_string(rect.x, rect.y, "ABCDEFGH", slt::Style::new());
        });
    });
    let output = tb.to_string();
    assert!(output.contains("ABC"));
    assert!(!output.contains("ABCDEFGH"));
}

#[test]
fn draw_raw_with_grow_fills_available_width() {
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        ui.col(|ui| {
            ui.container().grow(1).h(3).draw(|buf, rect| {
                assert!(rect.width > 0);
                assert_eq!(rect.height, 3);
                buf.set_char(rect.x, rect.y, 'G', slt::Style::new());
            });
        });
    });
    tb.assert_contains("G");
}

#[test]
fn draw_raw_alongside_normal_widgets() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.col(|ui| {
            ui.text("above");
            ui.container().w(10).h(1).draw(|buf, rect| {
                buf.set_string(rect.x, rect.y, "drawn", slt::Style::new());
            });
            ui.text("below");
        });
    });
    let output = tb.to_string();
    assert!(output.contains("above"));
    assert!(output.contains("drawn"));
    assert!(output.contains("below"));
}

#[test]
fn draw_raw_with_fixed_size() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.container().w(12).h(5).draw(|buf, rect| {
            assert_eq!(rect.width, 12);
            assert_eq!(rect.height, 5);
            buf.set_char(rect.x, rect.y, 'I', slt::Style::new());
        });
    });
    tb.assert_contains("I");
}

#[test]
fn draw_raw_styled_content() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.container().w(5).h(1).draw(|buf, rect| {
            let style = slt::Style::new().fg(slt::Color::Red).bold();
            buf.set_char(rect.x, rect.y, 'R', style);
        });
    });
    let cell = tb.buffer().get(0, 0);
    assert_eq!(cell.symbol, "R");
    assert_eq!(cell.style.fg, Some(slt::Color::Red));
    assert!(cell.style.modifiers.contains(slt::Modifiers::BOLD));
}

#[test]
fn draw_raw_multiple_regions() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.row(|ui| {
            ui.container().w(5).h(1).draw(|buf, rect| {
                buf.set_string(rect.x, rect.y, "AAA", slt::Style::new());
            });
            ui.container().w(5).h(1).draw(|buf, rect| {
                buf.set_string(rect.x, rect.y, "BBB", slt::Style::new());
            });
        });
    });
    let output = tb.to_string();
    assert!(output.contains("AAA"));
    assert!(output.contains("BBB"));
}

#[test]
fn collect_all_focus_rects_match_tab_navigation() {
    let mut tb = TestBackend::new(40, 10);
    let events = slt::EventBuilder::new().key_code(slt::KeyCode::Tab).build();
    tb.run_with_events(events, |ui| {
        ui.col(|ui| {
            let mut input1 = slt::TextInputState::new();
            ui.text_input(&mut input1);
            let mut input2 = slt::TextInputState::new();
            ui.text_input(&mut input2);
        });
    });
}

#[test]
fn collect_all_scroll_works_after_merge() {
    let mut tb = TestBackend::new(40, 10);
    let mut scroll = slt::ScrollState::new();
    tb.render(|ui| {
        ui.scrollable(&mut scroll).h(5).col(|ui| {
            for i in 0..20 {
                ui.text(format!("Line {i}"));
            }
        });
    });
    tb.assert_contains("Line 0");
}

#[test]
fn divider_text_renders_label() {
    let mut tb = TestBackend::new(40, 3);
    tb.render(|ui| {
        ui.divider_text("Settings");
    });
    tb.assert_contains("Settings");
    tb.assert_contains("─");
}

#[test]
fn alert_renders_with_icon() {
    let mut tb = TestBackend::new(60, 3);
    tb.render(|ui| {
        ui.alert("Test message", slt::AlertLevel::Success);
    });
    tb.assert_contains("✓");
    tb.assert_contains("Test message");
    tb.assert_contains("[×]");
}

#[test]
fn alert_dismiss_on_key() {
    let mut tb = TestBackend::new(60, 5);
    let mut dismissed = false;
    let events = slt::EventBuilder::new().key('x').build();
    tb.run_with_events(events, |ui| {
        if ui.alert("msg", slt::AlertLevel::Info) {
            dismissed = true;
        }
    });
    assert!(dismissed);
}

#[test]
fn breadcrumb_renders_segments() {
    let mut tb = TestBackend::new(60, 3);
    tb.render(|ui| {
        ui.breadcrumb(&["Home", "Settings", "Profile"]);
    });
    let output = tb.to_string();
    assert!(output.contains("Home"));
    assert!(output.contains("Profile"));
}

#[test]
fn accordion_closed_hides_content() {
    let mut tb = TestBackend::new(40, 10);
    let mut open = false;
    tb.render(|ui| {
        ui.accordion("Title", &mut open, |ui| {
            ui.text("hidden content");
        });
    });
    let output = tb.to_string();
    assert!(output.contains("▸"));
    assert!(output.contains("Title"));
    assert!(!output.contains("hidden content"));
}

#[test]
fn accordion_open_shows_content() {
    let mut tb = TestBackend::new(40, 10);
    let mut open = true;
    tb.render(|ui| {
        ui.accordion("Title", &mut open, |ui| {
            ui.text("visible content");
        });
    });
    let output = tb.to_string();
    assert!(output.contains("▾"));
    assert!(output.contains("visible content"));
}

#[test]
fn badge_renders_label() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.badge("v0.9");
    });
    tb.assert_contains("v0.9");
}

#[test]
fn badge_colored_has_bg() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.badge_colored("OK", slt::Color::Green);
    });
    let cell = tb.buffer().get(1, 0);
    assert_eq!(cell.style.bg, Some(slt::Color::Green));
}

#[test]
fn key_hint_renders_reversed() {
    let mut tb = TestBackend::new(20, 3);
    tb.render(|ui| {
        ui.key_hint("Ctrl+S");
    });
    tb.assert_contains("Ctrl+S");
    let cell = tb.buffer().get(1, 0);
    assert!(cell.style.modifiers.contains(slt::Modifiers::REVERSED));
}

#[test]
fn stat_renders_label_and_value() {
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        ui.stat("CPU", "72%");
    });
    let output = tb.to_string();
    assert!(output.contains("CPU"));
    assert!(output.contains("72%"));
}

#[test]
fn stat_trend_shows_arrow() {
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        ui.stat_trend("Rev", "$100", slt::Trend::Up);
    });
    tb.assert_contains("↑");
}

#[test]
fn definition_list_aligns_keys() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.definition_list(&[("Host", "localhost"), ("Port", "8080")]);
    });
    let output = tb.to_string();
    assert!(output.contains("Host"));
    assert!(output.contains("localhost"));
    assert!(output.contains("Port"));
    assert!(output.contains("8080"));
}

#[test]
fn empty_state_renders_centered() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.empty_state("No data", "Add items to begin");
    });
    let output = tb.to_string();
    assert!(output.contains("No data"));
    assert!(output.contains("Add items"));
}

#[test]
fn code_block_renders_code() {
    let mut tb = TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.code_block("let x = 1;");
    });
    tb.assert_contains("let");
    tb.assert_contains("1");
}

#[test]
fn code_block_numbered_has_line_numbers() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.code_block_numbered("line1\nline2\nline3");
    });
    let output = tb.to_string();
    assert!(output.contains("1"));
    assert!(output.contains("2"));
    assert!(output.contains("3"));
    assert!(output.contains("line1"));
}

#[test]
fn theme_light_has_is_dark_false() {
    use slt::Theme;
    let light_theme = Theme::light();
    assert!(
        !light_theme.is_dark,
        "Theme::light() should have is_dark=false"
    );
}

#[test]
fn theme_dark_has_is_dark_true() {
    use slt::Theme;
    let dark_theme = Theme::dark();
    assert!(dark_theme.is_dark, "Theme::dark() should have is_dark=true");
}

#[test]
fn theme_builder_defaults_to_dark() {
    use slt::Theme;
    let theme = Theme::builder().build();
    assert!(theme.is_dark, "ThemeBuilder should default to is_dark=true");
}

#[test]
fn theme_builder_can_set_is_dark_false() {
    use slt::Theme;
    let theme = Theme::builder().is_dark(false).build();
    assert!(
        !theme.is_dark,
        "ThemeBuilder.is_dark(false) should set is_dark=false"
    );
}

#[test]
fn dev_warning_framework_works() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.text("hello");
    });
    assert!(tb.dev_warnings().is_empty());
}

#[test]
fn dev_warning_hook_count_change() {
    let mut tb = TestBackend::new(40, 5);
    let show_second = true;
    tb.render(|ui| {
        let _ = ui.use_state(|| 0i32);
        let _ = ui.use_state(|| 0i32);
        ui.text("frame 1");
    });
    assert!(tb.dev_warnings().is_empty(), "no warning on first frame");

    tb.render(|ui| {
        let _ = ui.use_state(|| 0i32);
        if show_second && false {
            let _ = ui.use_state(|| 0i32);
        }
        ui.text("frame 2");
    });
    let warnings = tb.dev_warnings();
    assert!(
        !warnings.is_empty(),
        "expected hook count change warning on frame 2"
    );
    assert!(
        warnings[0].contains("Hook call count changed"),
        "warning message should mention hook count change, got: {}",
        warnings[0]
    );
}

#[test]
fn dev_warning_hook_count_stable_no_warning() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        let _ = ui.use_state(|| 0i32);
        let _ = ui.use_state(|| 0i32);
        ui.text("frame 1");
    });
    assert!(tb.dev_warnings().is_empty(), "no warning on first frame");

    tb.render(|ui| {
        let _ = ui.use_state(|| 0i32);
        let _ = ui.use_state(|| 0i32);
        ui.text("frame 2");
    });
    assert!(
        tb.dev_warnings().is_empty(),
        "no warning when hook count is stable"
    );
}

#[test]
fn dev_warning_first_frame_no_warning() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        let _ = ui.use_state(|| 0i32);
        let _ = ui.use_state(|| 0i32);
        let _ = ui.use_state(|| 0i32);
        ui.text("first frame with hooks");
    });
    assert!(
        tb.dev_warnings().is_empty(),
        "no warning on first frame even with hooks"
    );
}
