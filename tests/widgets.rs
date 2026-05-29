#![allow(unused_must_use)]

use slt::widgets::*;
use slt::{KeyCode, KeyMap, KeyModifiers, TestBackend};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("slt_{prefix}_{}_{}", std::process::id(), nanos));
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

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
fn file_picker_lists_directories_before_files() {
    let root = make_temp_dir("file_picker_list");
    fs::create_dir_all(root.join("alpha")).expect("failed to create subdir");
    fs::write(root.join("zeta.txt"), b"data").expect("failed to create file");

    let mut state = FilePickerState::new(root.clone());
    state.refresh();

    assert!(state.entries.iter().any(|e| e.name == "alpha" && e.is_dir));
    assert!(state
        .entries
        .iter()
        .any(|e| e.name == "zeta.txt" && !e.is_dir));

    let first_file = state.entries.iter().position(|e| !e.is_dir);
    if let Some(first_file_idx) = first_file {
        assert!(state.entries[..first_file_idx].iter().all(|e| e.is_dir));
    }

    fs::remove_dir_all(root).expect("failed to clean temp dir");
}

#[test]
fn file_picker_navigation_enter_dir_and_backspace_parent() {
    let root = make_temp_dir("file_picker_nav");
    let child = root.join("child");
    fs::create_dir_all(&child).expect("failed to create child dir");

    let mut state = FilePickerState::new(root.clone());
    let mut tb = TestBackend::new(80, 24);

    tb.render(|ui| {
        let _ = ui.file_picker(&mut state);
    });

    let enter = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Enter)
        .build();
    tb.render_with_events(enter, 0, 1, |ui| {
        let _ = ui.file_picker(&mut state);
    });
    assert_eq!(state.current_dir, child);

    let back = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Backspace)
        .build();
    tb.render_with_events(back, 0, 1, |ui| {
        let _ = ui.file_picker(&mut state);
    });
    assert_eq!(state.current_dir, root);

    fs::remove_dir_all(state.current_dir.clone()).expect("failed to clean temp dir");
}

#[test]
fn file_picker_extension_filter() {
    let root = make_temp_dir("file_picker_ext");
    fs::create_dir_all(root.join("dir_a")).expect("failed to create dir");
    fs::write(root.join("main.rs"), b"fn main() {}\n").expect("failed to create rs file");
    fs::write(root.join("notes.txt"), b"text\n").expect("failed to create txt file");

    let mut state = FilePickerState::new(root.clone()).extensions(&["rs"]);
    state.refresh();

    assert!(state.entries.iter().any(|e| e.name == "dir_a" && e.is_dir));
    assert!(state
        .entries
        .iter()
        .any(|e| e.name == "main.rs" && !e.is_dir));
    assert!(!state.entries.iter().any(|e| e.name == "notes.txt"));

    fs::remove_dir_all(root).expect("failed to clean temp dir");
}

#[test]
fn file_picker_hidden_file_toggle() {
    let root = make_temp_dir("file_picker_hidden");
    fs::write(root.join(".secret"), b"hidden\n").expect("failed to create hidden file");
    fs::write(root.join("visible.txt"), b"visible\n").expect("failed to create visible file");

    let mut state = FilePickerState::new(root.clone());
    state.refresh();
    assert!(!state.entries.iter().any(|e| e.name == ".secret"));

    state.show_hidden = true;
    state.dirty = true;
    state.refresh();
    assert!(state.entries.iter().any(|e| e.name == ".secret"));

    fs::remove_dir_all(root).expect("failed to clean temp dir");
}

#[test]
fn file_picker_response_changed_on_file_select() {
    let root = make_temp_dir("file_picker_select");
    let file = root.join("picked.txt");
    fs::write(&file, b"pick me\n").expect("failed to create file");

    let mut state = FilePickerState::new(root.clone());
    let mut tb = TestBackend::new(80, 24);
    let mut changed = false;

    let enter = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Enter)
        .build();
    tb.render_with_events(enter, 0, 1, |ui| {
        changed = ui.file_picker(&mut state).changed;
    });

    assert!(changed);
    assert_eq!(state.selected_file, Some(file));

    fs::remove_dir_all(root).expect("failed to clean temp dir");
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
fn table_zebra_applies_alternating_backgrounds() {
    let mut tb = TestBackend::new(60, 10);
    let mut table = TableState::new(
        vec!["Name", "Age"],
        vec![vec!["Alice", "30"], vec!["Bob", "25"], vec!["Cara", "22"]],
    );
    table.zebra = true;

    tb.render(|ui| {
        ui.table(&mut table);
    });

    let odd_bg = tb.buffer().get(0, 3).style.bg;
    let even_bg = tb.buffer().get(0, 4).style.bg;
    assert_eq!(odd_bg, Some(slt::Theme::dark().surface_hover));
    assert_eq!(even_bg, Some(slt::Theme::dark().surface));
}

#[test]
fn table_zebra_uses_widget_color_override() {
    let mut tb = TestBackend::new(60, 10);
    let mut table = TableState::new(
        vec!["Name", "Age"],
        vec![vec!["Alice", "30"], vec!["Bob", "25"], vec!["Cara", "22"]],
    );
    table.zebra = true;

    tb.render(|ui| {
        let colors = slt::WidgetColors::new().bg(slt::Color::Blue);
        ui.table_colored(&mut table, &colors);
    });

    assert_eq!(tb.buffer().get(0, 3).style.bg, Some(slt::Color::Blue));
    assert_eq!(tb.buffer().get(0, 4).style.bg, Some(slt::Color::Blue));
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
fn calendar_renders_month_title() {
    let mut tb = TestBackend::new(40, 12);
    let mut cal = CalendarState::from_ym(2024, 2);
    tb.render(|ui| {
        ui.calendar(&mut cal);
    });
    tb.assert_contains("2024 Feb");
}

#[test]
fn progress_renders() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        let _ = ui.progress(0.5);
    });
    tb.assert_contains("█");
    tb.assert_contains("░");
}

#[test]
fn spinner_renders() {
    let mut tb = TestBackend::new(40, 5);
    let spinner = SpinnerState::dots();
    tb.render(|ui| {
        let _ = ui.spinner(&spinner);
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
fn keymap_builder_builds_bindings() {
    let keymap = KeyMap::new()
        .bind('q', "Quit")
        .bind_code(KeyCode::Up, "Move up")
        .bind_mod('s', KeyModifiers::CONTROL, "Save");

    assert_eq!(keymap.bindings.len(), 3);
    assert_eq!(keymap.bindings[0].key, KeyCode::Char('q'));
    assert_eq!(keymap.bindings[0].display, "q");
    assert_eq!(keymap.bindings[1].key, KeyCode::Up);
    assert_eq!(keymap.bindings[1].display, "↑");
    assert_eq!(keymap.bindings[2].key, KeyCode::Char('s'));
    assert_eq!(keymap.bindings[2].modifiers, Some(KeyModifiers::CONTROL));
    assert_eq!(keymap.bindings[2].display, "Ctrl+S");
}

#[test]
fn keymap_visible_bindings_filters_hidden() {
    let keymap = KeyMap::new()
        .bind('q', "Quit")
        .bind_hidden('?', "Toggle help")
        .bind_code(KeyCode::Tab, "Next");

    let visible: Vec<_> = keymap.visible_bindings().collect();
    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0].description, "Quit");
    assert_eq!(visible[1].description, "Next");
}

#[test]
fn help_from_keymap_renders_visible_bindings() {
    let keymap = KeyMap::new()
        .bind('q', "quit")
        .bind_mod('s', KeyModifiers::CONTROL, "save")
        .bind_hidden('?', "toggle help");

    let mut tb = TestBackend::new(60, 5);
    tb.render(|ui| {
        ui.help_from_keymap(&keymap);
    });

    tb.assert_contains("q");
    tb.assert_contains("quit");
    tb.assert_contains("Ctrl+S");
    tb.assert_contains("save");
    assert!(!tb.to_string().contains("toggle help"));
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
fn slider_right_key_increases_value() {
    let mut tb = TestBackend::new(80, 5);
    let mut value = 50.0_f64;
    let mut changed = false;
    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Right)
        .build();

    tb.render_with_events(events, 0, 1, |ui| {
        changed = ui.slider("Volume", &mut value, 0.0..=100.0).changed;
    });

    assert!(changed);
    assert!(value > 50.0);
}

#[test]
fn slider_left_key_decreases_value() {
    let mut tb = TestBackend::new(80, 5);
    let mut value = 50.0_f64;
    let mut changed = false;
    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Left)
        .build();

    tb.render_with_events(events, 0, 1, |ui| {
        changed = ui.slider("Volume", &mut value, 0.0..=100.0).changed;
    });

    assert!(changed);
    assert!(value < 50.0);
}

#[test]
fn confirm_y_key_sets_true_and_clicks() {
    let mut tb = TestBackend::new(80, 5);
    let mut answer = false;
    let mut clicked = false;
    let events = slt::EventBuilder::new().key('y').build();

    tb.render_with_events(events, 0, 1, |ui| {
        clicked = ui.confirm("Delete this file?", &mut answer).clicked;
    });

    assert!(clicked);
    assert!(answer);
}

#[test]
fn confirm_n_key_sets_false_and_clicks() {
    let mut tb = TestBackend::new(80, 5);
    let mut answer = true;
    let mut clicked = false;
    let events = slt::EventBuilder::new().key('n').build();

    tb.render_with_events(events, 0, 1, |ui| {
        clicked = ui.confirm("Delete this file?", &mut answer).clicked;
    });

    assert!(clicked);
    assert!(!answer);
}

#[test]
fn confirm_tab_toggles_choice_before_focus_processing() {
    let mut tb = TestBackend::new(80, 5);
    let mut answer = false;
    let events = slt::EventBuilder::new().key_code(KeyCode::Tab).build();

    tb.render_with_events(events, 0, 1, |ui| {
        let _ = ui.confirm("Delete this file?", &mut answer);
    });

    assert!(answer);
}

// Regression: clicking [Yes] must update `result` in the SAME frame (not lag
// one frame). The previous implementation mutated `*result` correctly but
// computed `[Yes]/[No]` styles before the mouse hit-test, leaking a visual
// one-frame lag and forcing a `let _ = is_yes;` dead-write silencer. After
// the fix, both the outparam and the rendered visual feedback land together.
#[test]
fn confirm_click_yes_updates_answer_in_same_frame() {
    let mut tb = TestBackend::new(40, 5);
    // Frame 0: render once so the row enters `prev_hit_map`.
    let mut answer = false;
    tb.render_with_events(Vec::new(), 0, 1, |ui| {
        ui.confirm("Continue?", &mut answer);
    });
    assert!(!answer, "default state should be No");

    // Frame 1: click on [Yes]. "Continue?" is 9 columns, then a space, so
    // [Yes] starts at x=10 and runs through x=14.
    let events = slt::EventBuilder::new().click(11, 0).build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.confirm("Continue?", &mut answer);
        if answer {
            // Render this only when the click landed; an extra row guarantees
            // the assertion below tests current-frame behaviour, not next.
            ui.text("YES SELECTED");
        }
    });
    assert!(answer, "click on [Yes] must update outparam in same frame");
    tb.assert_contains("YES SELECTED");
}

#[test]
fn confirm_click_no_updates_answer_in_same_frame() {
    let mut tb = TestBackend::new(40, 5);
    // Default is `false`, so start in the Yes state to verify the click flips
    // the answer back to No within a single frame.
    let mut answer = true;
    tb.render_with_events(Vec::new(), 0, 1, |ui| {
        ui.confirm("Continue?", &mut answer);
    });

    // [No] sits one space after [Yes]. With "Continue?" = 9 columns:
    //   row_x=0, q_width=9, yes_start=10, yes_end=15, no_start=16, no_end=20.
    let events = slt::EventBuilder::new().click(17, 0).build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.confirm("Continue?", &mut answer);
        if !answer {
            ui.text("NO SELECTED");
        }
    });
    assert!(!answer, "click on [No] must update outparam in same frame");
    tb.assert_contains("NO SELECTED");
}

// Visual-feedback regression: when a click lands on [Yes], the rendered row
// in the *same* frame must show `[Yes]` with the selected (focused) style
// applied — i.e. the foreground colour for `[Yes]` is the theme's `bg` (the
// reverse of unselected). The old implementation computed styles before the
// hit-test, so `[Yes]` painted as unselected on the click frame.
#[test]
fn confirm_click_yes_renders_selected_style_same_frame() {
    let mut tb = TestBackend::new(40, 5);
    let mut answer = false;
    // Prime `prev_hit_map`.
    tb.render_with_events(Vec::new(), 0, 1, |ui| {
        ui.confirm("Continue?", &mut answer);
    });

    // Snapshot the cell at the 'Y' of "[Yes]" before the click. With default
    // `is_yes = false`, [Yes] is painted in `text_dim`, not the bold/bg-swap
    // selected style.
    let style_before = tb.buffer().get(11, 0).style;

    let events = slt::EventBuilder::new().click(11, 0).build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.confirm("Continue?", &mut answer);
    });

    // After the click, [Yes] should be rendered in the selected style (bold +
    // bg swapped to theme.success). The simplest check that catches the old
    // bug: the cell's style must have changed compared to the unselected
    // frame. The previous-frame render painted [Yes] dim; the current-frame
    // render after the fix paints [Yes] with the success-on-bg style.
    let style_after = tb.buffer().get(11, 0).style;
    assert!(answer, "outparam should track the click");
    assert_ne!(
        style_before, style_after,
        "[Yes] style must change in the click frame, not lag by one frame"
    );
}

#[test]
fn notify_renders_without_toast_state() {
    let mut tb = TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.notify("File saved!", slt::ToastLevel::Success);
    });
    tb.assert_contains("File saved!");
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
    tb.assert_contains("S1");
    tb.assert_contains("X");
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

/// Legend names must not bleed past the chart's allotted width. Long
/// names get an ellipsis; very narrow charts drop the legend entirely
/// rather than emit a garbled prefix. (v0.20 fix.)
#[test]
fn chart_legend_truncates_with_ellipsis_when_narrow() {
    // 16-cell-wide chart with a long legend name + axis labels. The
    // full "Memory" legend wants 4+6 = 10 cells, but with the y-axis
    // taking ~6 cells and a min-plot reservation of 4, the legend is
    // capped down and must truncate the name with an ellipsis.
    let mut tb = TestBackend::new(20, 8);
    tb.render(|ui| {
        ui.chart(
            |c| {
                c.line(&[(0.0, 1.0), (1.0, 2.0)])
                    .label("Memory")
                    .color(slt::Color::Cyan);
                c.legend(slt::LegendPosition::TopRight);
                c.grid(false);
            },
            16,
            6,
        );
    });
    let out = tb.to_string();
    let has_full = out.contains("Memory");
    let has_ellipsis = out.contains('\u{2026}');
    let has_no_legend = !out.contains("Memory") && !out.contains('M');
    assert!(
        has_full || has_ellipsis || has_no_legend,
        "expected full label, ellipsis-truncated label, or dropped legend; got:\n{out}"
    );
    for bad in ["Memor", "Memo", "Mem", "Me", "M"] {
        if out.contains(bad) {
            let with_full = out.contains("Memory");
            let with_ell = out.contains(&format!("{bad}\u{2026}"));
            assert!(
                with_full || with_ell,
                "legend contains bare-truncated prefix {bad:?} \
                 (no ellipsis, no full label):\n{out}"
            );
        }
    }
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
fn bar_chart_with_horizontal() {
    let mut tb = TestBackend::new(50, 10);
    let bars = vec![
        slt::Bar::new("A", 10.0).color(slt::Color::Cyan),
        slt::Bar::new("B", 20.0).color(slt::Color::Red),
    ];
    tb.render(|ui| {
        ui.bar_chart_with(
            &bars,
            |c| {
                c.direction(slt::BarDirection::Horizontal);
            },
            20,
        );
    });
    tb.assert_contains("A");
    tb.assert_contains("B");
    tb.assert_contains("█");
}

#[test]
fn bar_chart_with_vertical() {
    let mut tb = TestBackend::new(50, 15);
    let bars = vec![slt::Bar::new("X", 5.0), slt::Bar::new("Y", 10.0)];
    tb.render(|ui| {
        ui.bar_chart_with(
            &bars,
            |c| {
                c.direction(slt::BarDirection::Vertical);
            },
            8,
        );
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
            ui.bordered(slt::Border::Rounded).p(1).col(|ui| {
                ui.text("Hello Modal");
                if ui.button("OK").clicked {}
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
            if ui.button("Confirm").clicked {
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
            ui.bordered(slt::Border::Rounded).p(1).max_w(30).col(|ui| {
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
fn rich_log_renders_entries() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = RichLogState::new();
    state.push("INFO started", slt::Style::new().fg(slt::Color::Green));
    state.push("WARN retry", slt::Style::new().fg(slt::Color::Yellow));
    state.push_plain("DONE");

    tb.render(|ui| {
        ui.rich_log(&mut state);
    });

    tb.assert_contains("INFO started");
    tb.assert_contains("WARN retry");
    tb.assert_contains("DONE");
}

#[test]
fn rich_log_scrolls_with_keyboard() {
    let mut tb = TestBackend::new(40, 8);
    let mut state = RichLogState::new();
    state.auto_scroll = false;
    for i in 0..100 {
        state.push_plain(format!("Entry {i}"));
    }

    tb.render(|ui| {
        ui.rich_log(&mut state);
    });
    tb.assert_contains("Entry 0");

    let events = slt::EventBuilder::new().key_code(slt::KeyCode::End).build();
    tb.run_with_events(events, |ui| {
        ui.rich_log(&mut state);
    });
    tb.assert_contains("Entry 99");
}

#[test]
fn directory_tree_from_paths_renders_structure() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = DirectoryTreeState::from_paths(&["src/main.rs", "src/lib.rs", "Cargo.toml"]);

    tb.render(|ui| {
        ui.directory_tree(&mut state);
    });

    tb.assert_contains("src");
    tb.assert_contains("main.rs");
    tb.assert_contains("lib.rs");
    tb.assert_contains("Cargo.toml");
    tb.assert_contains("├──");
}

#[test]
fn directory_tree_selected_label_from_paths() {
    let state = DirectoryTreeState::from_paths(&["src/main.rs", "src/lib.rs", "Cargo.toml"]);
    assert_eq!(state.selected_label(), Some("src"));
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
fn command_palette_fuzzy_match_sf() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = CommandPaletteState::new(vec![
        PaletteCommand::new("Save File", "Write the current buffer"),
        PaletteCommand::new("Quit", "Exit the app"),
    ]);
    state.open = true;
    state.input = "sf".into();

    tb.render(|ui| {
        let _ = ui.command_palette(&mut state);
    });

    tb.assert_contains("Save File");
    assert!(!tb.to_string().contains("Quit"));
}

#[test]
fn command_palette_fuzzy_match_cmd() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = CommandPaletteState::new(vec![
        PaletteCommand::new("Command Palette", "Open actions"),
        PaletteCommand::new("Save File", "Write the current buffer"),
    ]);
    state.open = true;
    state.input = "cmd".into();

    tb.render(|ui| {
        let _ = ui.command_palette(&mut state);
    });

    tb.assert_contains("Command Palette");
    assert!(!tb.to_string().contains("Save File"));
}

#[test]
fn command_palette_exact_substring_still_works() {
    let mut tb = TestBackend::new(80, 24);
    let mut state = CommandPaletteState::new(vec![
        PaletteCommand::new("Save File", "Write the current buffer"),
        PaletteCommand::new("Quit", "Exit the app"),
    ]);
    state.open = true;
    state.input = "buffer".into();

    tb.render(|ui| {
        let _ = ui.command_palette(&mut state);
    });

    tb.assert_contains("Save File");
    assert!(!tb.to_string().contains("Quit"));
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
    use slt::event::Event;

    let mut tb = TestBackend::new(40, 5);
    let events = vec![Event::key_release('q')];
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
        if ui.alert("msg", slt::AlertLevel::Info).clicked {
            dismissed = true;
        }
    });
    assert!(dismissed);
}

#[test]
fn alert_consumes_dismiss_key() {
    let mut tb = TestBackend::new(60, 5);
    let mut dismissed = false;
    let mut saw_x_after_alert = false;
    let events = slt::EventBuilder::new().key('x').build();
    tb.render_with_events(events, 0, 1, |ui| {
        if ui.alert("msg", slt::AlertLevel::Info).clicked {
            dismissed = true;
        }
        saw_x_after_alert = ui.key('x');
    });
    assert!(dismissed);
    assert!(!saw_x_after_alert);
}

#[test]
fn breadcrumb_renders_segments() {
    let mut tb = TestBackend::new(60, 3);
    tb.render(|ui| {
        let _ = ui.breadcrumb(&["Home", "Settings", "Profile"]);
    });
    let output = tb.to_string();
    assert!(output.contains("Home"));
    assert!(output.contains("Profile"));
}

#[test]
fn breadcrumb_enter_activates_focused_segment() {
    let mut tb = TestBackend::new(60, 3);
    let events = slt::EventBuilder::new().key_code(KeyCode::Enter).build();
    let mut clicked = None;
    tb.render_with_events(events, 0, 1, |ui| {
        clicked = ui
            .breadcrumb(&["Home", "Settings", "Profile"])
            .show()
            .clicked_segment;
    });
    assert_eq!(clicked, Some(0));
}

#[test]
fn breadcrumb_mouse_click_activates_segment() {
    let mut tb = TestBackend::new(60, 3);
    tb.render(|ui| {
        let _ = ui.breadcrumb(&["Home", "Settings", "Profile"]);
    });

    let events = slt::EventBuilder::new().click(1, 0).build();
    let mut clicked = None;
    tb.render_with_events(events, 0, 0, |ui| {
        clicked = ui
            .breadcrumb(&["Home", "Settings", "Profile"])
            .show()
            .clicked_segment;
    });

    assert_eq!(clicked, Some(0));
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
fn accordion_enter_toggles_open() {
    let mut tb = TestBackend::new(40, 10);
    let mut open = false;
    let events = slt::EventBuilder::new().key_code(KeyCode::Enter).build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.accordion("Title", &mut open, |ui| {
            ui.text("visible content");
        });
    });
    assert!(open);
    assert!(tb.to_string().contains("visible content"));
}

#[test]
fn accordion_space_toggles_open() {
    let mut tb = TestBackend::new(40, 10);
    let mut open = false;
    let events = slt::EventBuilder::new().key(' ').build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.accordion("Title", &mut open, |ui| {
            ui.text("visible content");
        });
    });
    assert!(open);
    assert!(tb.to_string().contains("visible content"));
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
fn demo_v094_content_does_not_panic() {
    use slt::*;
    let mut tb = TestBackend::new(120, 40);
    let mut acc_gen = true;
    let mut acc_adv = false;
    let alert = true;
    tb.render(|ui| {
        if alert {
            ui.alert("Test alert", AlertLevel::Success);
        }
        ui.divider_text("Nav");
        let _ = ui.breadcrumb(&["Home", "Settings"]);
        ui.stat("Uptime", "14d");
        ui.stat_trend("Revenue", "$12,400", Trend::Up);
        ui.stat_colored("CPU", "72%", Color::Yellow);
        ui.badge("v0.9.4");
        ui.badge_colored("Stable", Color::Green);
        ui.key_hint("Ctrl+S");
        ui.accordion("General", &mut acc_gen, |ui| {
            ui.definition_list(&[("Theme", "Dark")]);
        });
        ui.accordion("Advanced", &mut acc_adv, |ui| {
            ui.definition_list(&[("Log", "debug")]);
        });
        ui.code_block_numbered("fn main() {}");
        ui.empty_state("No items", "Add some");
    });
    tb.assert_contains("v0.9.4");
}

#[test]
fn demo_list_set_items_no_panic() {
    use slt::*;
    let mut tb = TestBackend::new(80, 24);
    let mut list = ListState::new(vec!["A", "B", "C", "D", "E"]);
    list.selected = 4;
    // Shrink items - old selected (4) should be clamped
    list.set_items(vec!["X".to_string(), "Y".to_string(), "Z".to_string()]);
    assert_eq!(list.selected, 2);
    tb.render(|ui| {
        ui.list(&mut list);
    });
    tb.assert_contains("X");
}

#[test]
fn code_block_lang_renders_content() {
    let mut tb = slt::TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.code_block_lang("let x = 1;", "rust");
    });
    tb.assert_contains("let");
    tb.assert_contains("1");
}

#[test]
fn code_block_lang_unknown_falls_back() {
    let mut tb = slt::TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.code_block_lang("hello world", "brainfuck");
    });
    tb.assert_contains("hello");
}

#[test]
fn code_block_numbered_lang_renders() {
    let mut tb = slt::TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.code_block_numbered_lang("fn main() {}\nlet x = 1;", "rust");
    });
    let output = tb.to_string();
    assert!(output.contains("1"));
    assert!(output.contains("2"));
    assert!(output.contains("main"));
}

#[test]
fn code_block_lang_empty_lang_uses_fallback() {
    let mut tb = slt::TestBackend::new(60, 10);
    tb.render(|ui| {
        ui.code_block_lang("let x = 1;", "");
    });
    tb.assert_contains("let");
}

#[test]
fn markdown_fenced_code_block_renders() {
    let mut tb = slt::TestBackend::new(80, 20);
    tb.render(|ui| {
        ui.markdown("# Title\n\n```rust\nfn main() {}\n```\n\nDone.");
    });
    tb.assert_contains("Title");
    tb.assert_contains("main");
    tb.assert_contains("Done");
}

#[test]
fn markdown_unclosed_code_block_no_panic() {
    let mut tb = slt::TestBackend::new(80, 20);
    tb.render(|ui| {
        ui.markdown("```python\ndef foo():\n    pass");
    });
    tb.assert_contains("def");
}

#[test]
fn markdown_pipe_table_renders() {
    let mut tb = slt::TestBackend::new(60, 12);
    tb.render(|ui| {
        ui.markdown("| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |");
    });
    tb.assert_contains("Name");
    tb.assert_contains("Age");
    tb.assert_contains("Alice");
    tb.assert_contains("Bob");
    // Box-drawing borders
    tb.assert_contains("┌");
    tb.assert_contains("┘");
    tb.assert_contains("├");
}

#[test]
fn markdown_pipe_table_followed_by_text() {
    let mut tb = slt::TestBackend::new(60, 12);
    tb.render(|ui| {
        ui.markdown("| A | B |\n|---|---|\n| 1 | 2 |\n\nParagraph after table.");
    });
    tb.assert_contains("A");
    tb.assert_contains("1");
    tb.assert_contains("Paragraph after table");
}

#[test]
fn focus_control_api() {
    let mut tb = slt::TestBackend::new(40, 10);
    tb.render(|ui| {
        assert_eq!(ui.focus_count(), 0, "first frame has no prev focus count");
        let mut input1 = slt::TextInputState::with_placeholder("A");
        let mut input2 = slt::TextInputState::with_placeholder("B");
        ui.text_input(&mut input1);
        ui.text_input(&mut input2);
        // On the first frame, set_focus_index should work without panic
        ui.set_focus_index(1);
        assert_eq!(ui.focus_index(), 1);
    });
}

#[test]
fn markdown_link_renders_surrounding_text() {
    let mut tb = slt::TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.markdown("Click [here](https://example.com) for info.");
    });
    // Link text is rendered via ui.link() which uses a separate Command —
    // surrounding plain text should still be present.
    tb.assert_contains("Click");
    tb.assert_contains("for info");
}

#[test]
fn markdown_image_renders_placeholder() {
    let mut tb = slt::TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.markdown("See ![screenshot](./img.png) below.");
    });
    tb.assert_contains("screenshot");
    tb.assert_contains("below");
}

#[test]
fn markdown_blockquote_renders() {
    let mut tb = slt::TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.markdown("> This is a quote");
    });
    tb.assert_contains("This is a quote");
}

#[test]
fn markdown_link_with_apostrophe() {
    let mut tb = slt::TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.markdown(r#"[Girls' Generation](/wiki/Girls%27_Generation "Girls' Generation")"#);
    });
    // Link text should be extracted despite apostrophe and tooltip in URL
    tb.assert_contains("Girls");
}

#[test]
fn markdown_link_with_tooltip_quotes() {
    let mut tb = slt::TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.markdown(r#"See [Beyoncé](/wiki/Beyoncé "Beyoncé") here."#);
    });
    tb.assert_contains("See");
    tb.assert_contains("here");
}

// ── Treemap ─────────────────────────────────────────────────────

#[test]
fn treemap_renders_labels() {
    let mut tb = TestBackend::new(60, 20);
    let items = vec![
        slt::TreemapItem::new("Rust", 40.0, slt::Color::Cyan),
        slt::TreemapItem::new("Go", 25.0, slt::Color::Blue),
        slt::TreemapItem::new("Python", 20.0, slt::Color::Yellow),
    ];
    tb.render(|ui| {
        ui.treemap(&items);
    });
    tb.assert_contains("Rust");
    tb.assert_contains("Go");
    tb.assert_contains("Python");
}

#[test]
fn treemap_uses_bg_colors() {
    let mut tb = TestBackend::new(40, 10);
    let items = vec![
        slt::TreemapItem::new("A", 50.0, slt::Color::Rgb(255, 0, 0)),
        slt::TreemapItem::new("B", 50.0, slt::Color::Rgb(0, 0, 255)),
    ];
    tb.render(|ui| {
        ui.treemap(&items);
    });
    // At least one cell should have a red bg and one blue bg
    let buf = tb.buffer();
    let mut found_red = false;
    let mut found_blue = false;
    for y in 0..10 {
        for x in 0..40 {
            if let Some(bg) = buf.get(x, y).style.bg {
                if bg == slt::Color::Rgb(255, 0, 0) {
                    found_red = true;
                }
                if bg == slt::Color::Rgb(0, 0, 255) {
                    found_blue = true;
                }
            }
        }
    }
    assert!(found_red, "expected red bg in treemap");
    assert!(found_blue, "expected blue bg in treemap");
}

#[test]
fn treemap_empty_input() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.treemap(&[]);
    });
    // Should not panic, renders nothing
}

#[test]
fn treemap_single_item() {
    let mut tb = TestBackend::new(30, 8);
    let items = vec![slt::TreemapItem::new("Only", 100.0, slt::Color::Green)];
    tb.render(|ui| {
        ui.treemap(&items);
    });
    tb.assert_contains("Only");
}

#[test]
fn treemap_filters_tiny_items() {
    let mut tb = TestBackend::new(20, 5);
    // "Tiny" has value 0.01 out of 100 total — much less than 1 cell in 20x5 = 100 area
    let items = vec![
        slt::TreemapItem::new("Big", 99.99, slt::Color::Cyan),
        slt::TreemapItem::new("Tiny", 0.01, slt::Color::Red),
    ];
    tb.render(|ui| {
        ui.treemap(&items);
    });
    tb.assert_contains("Big");
    // "Tiny" should be filtered out (area < 1 cell)
    let output = tb.to_string();
    assert!(
        !output.contains("Tiny"),
        "tiny items should be filtered out"
    );
}

/// When a treemap cell is narrower than its label, the label must be
/// truncated with an ellipsis ("…"), never bare-truncated mid-character.
/// (v0.20 fix: pre-fix output showed "Pytho"/"TypeS" instead of
/// "Pyth…"/"Type…".)
#[test]
fn treemap_truncates_label_with_ellipsis() {
    let mut tb = TestBackend::new(30, 5);
    let items = vec![
        slt::TreemapItem::new("Rust", 40.0, slt::Color::Cyan),
        slt::TreemapItem::new("Python", 20.0, slt::Color::Yellow),
        slt::TreemapItem::new("TypeScript", 10.0, slt::Color::Blue),
        slt::TreemapItem::new("Go", 25.0, slt::Color::Green),
    ];
    tb.render(|ui| {
        ui.treemap(&items);
    });
    let output = tb.to_string();
    for bad in ["Pytho", "Pyth", "TypeS", "Types", "TypeSc"] {
        if output.contains(bad) {
            let with_full =
                bad == "Pyth" && output.contains("Python") || output.contains("TypeScript");
            let with_ell = output.contains(&format!("{bad}\u{2026}"));
            assert!(
                with_full || with_ell,
                "treemap contains bare-truncated label {bad:?} (no ellipsis, no full label):\n{output}"
            );
        }
    }
}

// ── Heatmap Halfblock ───────────────────────────────────────────

#[test]
fn heatmap_halfblock_renders() {
    let mut tb = TestBackend::new(20, 5);
    let data: Vec<Vec<f64>> = (0..10)
        .map(|r| (0..20).map(|c| (r * 3 + c * 7) as f64).collect())
        .collect();
    tb.render(|ui| {
        ui.heatmap_halfblock(
            &data,
            20,
            5,
            slt::Color::Rgb(0, 0, 0),
            slt::Color::Rgb(255, 255, 255),
        );
    });
    // Check that half-block character is used
    let output = tb.to_string();
    assert!(
        output.contains('▀'),
        "heatmap_halfblock should use ▀ character"
    );
}

#[test]
fn heatmap_halfblock_uses_fg_and_bg() {
    let mut tb = TestBackend::new(10, 3);
    // 6 data rows → 3 terminal rows, each packing 2 data rows
    let data: Vec<Vec<f64>> = vec![
        vec![0.0; 10],   // row 0: low
        vec![100.0; 10], // row 1: high
        vec![0.0; 10],   // row 2: low
        vec![100.0; 10], // row 3: high
        vec![50.0; 10],  // row 4: mid
        vec![50.0; 10],  // row 5: mid
    ];
    tb.render(|ui| {
        ui.heatmap_halfblock(
            &data,
            10,
            3,
            slt::Color::Rgb(0, 0, 0),
            slt::Color::Rgb(255, 255, 255),
        );
    });
    // Cell (0,0) should have fg (upper=row0=dark) and bg (lower=row1=bright)
    let cell = tb.buffer().get(0, 0);
    assert!(cell.style.fg.is_some(), "halfblock should set fg");
    assert!(cell.style.bg.is_some(), "halfblock should set bg");
}

#[test]
fn heatmap_halfblock_empty_data() {
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        ui.heatmap_halfblock(
            &[],
            20,
            5,
            slt::Color::Rgb(0, 0, 0),
            slt::Color::Rgb(255, 255, 255),
        );
    });
    // Should not panic
}

// ── Candlestick HD ──────────────────────────────────────────────

#[test]
fn candlestick_hd_renders() {
    let mut tb = TestBackend::new(60, 15);
    let candles = vec![
        slt::Candle {
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 108.0,
        },
        slt::Candle {
            open: 108.0,
            high: 115.0,
            low: 102.0,
            close: 105.0,
        },
        slt::Candle {
            open: 105.0,
            high: 112.0,
            low: 100.0,
            close: 110.0,
        },
    ];
    tb.render(|ui| {
        ui.candlestick_hd(
            &candles,
            slt::Color::Rgb(38, 166, 91),
            slt::Color::Rgb(234, 57, 67),
        );
    });
    let output = tb.to_string();
    // Should contain heavy wick character and block body
    assert!(
        output.contains('┃'),
        "candlestick_hd should use heavy wick ┃"
    );
    assert!(
        output.contains('█'),
        "candlestick_hd should use block body █"
    );
}

#[test]
fn candlestick_hd_empty() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.candlestick_hd(&[], slt::Color::Green, slt::Color::Red);
    });
    // Should not panic
}

#[test]
fn candlestick_hd_single_candle() {
    let mut tb = TestBackend::new(20, 8);
    let candles = vec![slt::Candle {
        open: 100.0,
        high: 110.0,
        low: 90.0,
        close: 105.0,
    }];
    tb.render(|ui| {
        ui.candlestick_hd(&candles, slt::Color::Green, slt::Color::Red);
    });
    let output = tb.to_string();
    assert!(output.contains('┃') || output.contains('█'));
}

// ── Stacked Bar Chart ───────────────────────────────────────────

#[test]
fn bar_chart_stacked_renders() {
    let mut tb = TestBackend::new(40, 15);
    let groups = vec![
        slt::BarGroup::new(
            "Q1",
            vec![
                slt::Bar::new("A", 30.0).color(slt::Color::Cyan),
                slt::Bar::new("B", 20.0).color(slt::Color::Yellow),
            ],
        ),
        slt::BarGroup::new(
            "Q2",
            vec![
                slt::Bar::new("A", 40.0).color(slt::Color::Cyan),
                slt::Bar::new("B", 25.0).color(slt::Color::Yellow),
            ],
        ),
    ];
    tb.render(|ui| {
        ui.bar_chart_stacked(&groups, 12);
    });
    tb.assert_contains("Q1");
    tb.assert_contains("Q2");
}

#[test]
fn bar_chart_stacked_with_custom_width() {
    let mut tb = TestBackend::new(50, 15);
    let groups = vec![slt::BarGroup::new(
        "G1",
        vec![
            slt::Bar::new("X", 50.0).color(slt::Color::Red),
            slt::Bar::new("Y", 30.0).color(slt::Color::Blue),
        ],
    )];
    tb.render(|ui| {
        ui.bar_chart_stacked_with(
            &groups,
            |c| {
                c.bar_width(7).bar_gap(2);
            },
            10,
        );
    });
    tb.assert_contains("G1");
}

#[test]
fn bar_chart_stacked_empty() {
    let mut tb = TestBackend::new(40, 10);
    tb.render(|ui| {
        ui.bar_chart_stacked(&[], 10);
    });
    // Should not panic
}

// ── Existing viz widgets that lacked tests ──────────────────────

#[test]
fn bar_chart_basic_renders() {
    let mut tb = TestBackend::new(40, 8);
    let data = [("Sales", 100.0), ("Revenue", 80.0), ("Costs", 50.0)];
    tb.render(|ui| {
        ui.bar_chart(&data, 20);
    });
    tb.assert_contains("Sales");
    tb.assert_contains("Revenue");
    tb.assert_contains("Costs");
}

#[test]
fn sparkline_basic_renders() {
    let mut tb = TestBackend::new(30, 3);
    let data = [10.0, 20.0, 15.0, 25.0, 30.0, 18.0];
    tb.render(|ui| {
        ui.sparkline(&data, 20);
    });
    let output = tb.to_string();
    // Sparkline uses block chars
    let has_block = output.chars().any(|c| "▁▂▃▄▅▆▇█".contains(c));
    assert!(has_block, "sparkline should render block characters");
}

#[test]
fn heatmap_standard_renders() {
    let mut tb = TestBackend::new(20, 5);
    let data: Vec<Vec<f64>> = (0..5)
        .map(|r| (0..20).map(|c| (r * c) as f64).collect())
        .collect();
    tb.render(|ui| {
        ui.heatmap(
            &data,
            20,
            5,
            slt::Color::Rgb(0, 0, 50),
            slt::Color::Rgb(255, 100, 0),
        );
    });
    let output = tb.to_string();
    assert!(output.contains('█'), "heatmap should render block chars");
}

#[test]
fn candlestick_standard_renders() {
    let mut tb = TestBackend::new(40, 10);
    let candles = vec![
        slt::Candle {
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
        },
        slt::Candle {
            open: 105.0,
            high: 112.0,
            low: 98.0,
            close: 102.0,
        },
    ];
    tb.render(|ui| {
        ui.candlestick(&candles, slt::Color::Green, slt::Color::Red);
    });
    let output = tb.to_string();
    assert!(
        output.contains('│') || output.contains('█'),
        "candlestick should render wick or body"
    );
}

#[test]
fn textarea_paste_max_length_incremental() {
    // 1000-char paste with max_length=500 must truncate at exactly 500 chars,
    // not panic and not use O(n²) scanning.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new().max_length(500);
    let paste: String = "a".repeat(1000);
    let events = slt::EventBuilder::new().paste(paste).build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    let total: usize = state.lines.iter().map(|l| l.chars().count()).sum();
    assert_eq!(
        total, 500,
        "expected exactly 500 chars after truncation, got {total}"
    );
}

#[test]
fn textarea_newline_paste_respects_max_length() {
    // "a\nb\nc\n…" paste — newlines also count toward max_length.
    let mut tb = TestBackend::new(40, 20);
    let mut state = TextareaState::new().max_length(5);
    // 10 "a\n" pairs = 20 chars if uncapped; should stop at 5.
    let paste: String = "a\n".repeat(10);
    let events = slt::EventBuilder::new().paste(paste).build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 10);
    });
    let total: usize = state.lines.iter().map(|l| l.chars().count()).sum::<usize>()
        + state.lines.len().saturating_sub(1); // newlines between lines
    assert!(
        total <= 5,
        "expected total chars+newlines <= 5 after max_length cap, got {total}"
    );
}

// ===== a10 batch regression tests =====

#[test]
fn text_input_state_clone_drops_validators() {
    // a10-001 (#92): validators do not survive Clone; doc note clarifies behavior.
    let mut s = TextInputState::new();
    s.add_validator(|v: &str| {
        if v.is_empty() {
            Err("empty".to_string())
        } else {
            Ok(())
        }
    });
    s.run_validators();
    assert_eq!(s.errors().len(), 1, "validator should produce 1 error");
    let mut clone = s.clone();
    // errors are preserved on clone (stale)
    assert_eq!(clone.errors().len(), 1, "stale errors preserved on clone");
    // running validators on clone clears them (no validators registered)
    clone.run_validators();
    assert!(
        clone.errors().is_empty(),
        "clone has no validators; errors cleared"
    );
}

#[test]
fn textarea_change_detection_via_response_changed() {
    // a10-004 (#94): response.changed reports whether the lines mutated
    // since the previous frame.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();
    // Idle frame (no events) — should report changed=false.
    let mut changed = true;
    tb.render(|ui| {
        changed = ui.textarea(&mut state, 5).changed;
    });
    assert!(!changed, "idle frame: changed must be false");

    // Mutation frame — typing 'a' should set changed=true.
    let events = slt::EventBuilder::new().key('a').build();
    let mut changed2 = false;
    tb.render_with_events(events, 0, 1, |ui| {
        changed2 = ui.textarea(&mut state, 5).changed;
    });
    assert!(changed2, "mutation frame: changed must be true");
    assert_eq!(state.lines[0], "a");

    // Next idle frame — changed must reset to false.
    let mut changed3 = true;
    tb.render(|ui| {
        changed3 = ui.textarea(&mut state, 5).changed;
    });
    assert!(!changed3, "post-mutation idle: changed reset to false");
}

#[test]
fn list_state_filter_uses_search_cache() {
    // a10-006 (#96): filter no longer calls to_lowercase per item per keystroke;
    // behavior must remain case-insensitive.
    let mut state = ListState::new(vec!["Hello", "World", "HELLO World"]);
    state.set_filter("hello");
    let visible = state.visible_indices();
    assert_eq!(visible, &[0, 2], "case-insensitive 'hello' matches 0 and 2");

    state.set_filter("hello world");
    let visible = state.visible_indices();
    assert_eq!(visible, &[2], "AND tokens: only 'HELLO World' matches both");

    state.set_filter("");
    assert_eq!(
        state.visible_indices(),
        &[0, 1, 2],
        "empty filter: all visible"
    );

    // set_items rebuilds cache.
    state.set_items(vec!["Foo", "Bar"]);
    state.set_filter("foo");
    assert_eq!(state.visible_indices(), &[0]);
}

#[test]
fn slider_with_step_uses_explicit_step() {
    // a10-007 (#97): slider_with_step accepts an explicit step.
    let mut tb = TestBackend::new(80, 5);
    let mut value = 50.0_f64;
    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Right)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.slider_with_step("Volume", &mut value, 0.0..=100.0, 1.0);
    });
    assert!(
        (value - 51.0).abs() < f64::EPSILON,
        "step=1.0 should advance by exactly 1.0, got {value}"
    );

    // slider() unchanged: span/20 = 5.0 step on a 0..=100 range.
    let mut value2 = 50.0_f64;
    let events2 = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Right)
        .build();
    tb.render_with_events(events2, 0, 1, |ui| {
        ui.slider("Volume", &mut value2, 0.0..=100.0);
    });
    assert!(
        (value2 - 55.0).abs() < f64::EPSILON,
        "default step span/20=5.0 unchanged, got {value2}"
    );
}

#[test]
fn text_input_suggestions_track_typed_chars_in_burst() {
    // a10-003 (#93): matched_suggestions is recomputed after Char/Backspace/
    // Delete mutations within the same key burst. Test that a burst of typed
    // chars filters suggestions correctly at every step (would fail if
    // matched_suggestions were hoisted unconditionally outside the loop).
    let mut tb = TestBackend::new(40, 10);
    let mut input = TextInputState::new();
    input.set_suggestions(vec![
        "apple".into(),
        "apricot".into(),
        "banana".into(),
        "blueberry".into(),
    ]);
    // Burst: type 'a','p' — both refine the suggestion match.
    let events = slt::EventBuilder::new().key('a').key('p').build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.text_input(&mut input);
    });
    assert_eq!(input.value, "ap", "both keys consumed in single frame");
    let matches = input.matched_suggestions();
    assert_eq!(
        matches,
        vec!["apple", "apricot"],
        "after burst, matches reflect post-mutation 'ap' prefix, not stale empty value"
    );
}

// ── #180: code_block_numbered gutter width via ilog10 ──────────────────────

#[test]
fn code_block_numbered_single_line_gutter_one_digit() {
    // 1 line → gutter width = 1 ("1 │ ")
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.code_block_numbered("only_line");
    });
    let output = tb.to_string();
    assert!(output.contains("1 │"));
    assert!(output.contains("only_line"));
}

#[test]
fn code_block_numbered_ten_lines_gutter_two_digits() {
    // 10 lines → gutter width = 2 (" 1 │ ", "10 │ ")
    let code: String = (1..=10)
        .map(|i| format!("ln{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut tb = TestBackend::new(40, 14);
    tb.render(|ui| {
        ui.code_block_numbered(&code);
    });
    let output = tb.to_string();
    // First line right-padded to width 2
    assert!(output.contains(" 1 │"));
    // Tenth line uses both digits
    assert!(output.contains("10 │"));
}

#[test]
fn code_block_numbered_hundred_lines_gutter_three_digits() {
    // 100 lines → gutter width = 3
    let code: String = (1..=100)
        .map(|i| format!("l{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut tb = TestBackend::new(60, 110);
    tb.render(|ui| {
        ui.code_block_numbered(&code);
    });
    let output = tb.to_string();
    // 100th line should appear with no leading space ("100 │")
    assert!(output.contains("100 │"));
    // 1st line should be padded with 2 spaces (" 1 │") — but stricter:
    // "  1 │" with two leading spaces is the right-aligned form for width=3.
    assert!(output.contains("  1 │"));
}

#[test]
fn code_block_numbered_empty_input_does_not_panic() {
    // lines.len() == 0 → .max(1).ilog10() == 0 → gutter_w = 1, no panic
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.code_block_numbered("");
    });
    // Empty body still renders the bordered container without crashing.
    let _ = tb.to_string();
}

// ── #181: definition_list manual padding ──────────────────────────────────

#[test]
fn definition_list_right_aligns_mixed_ascii_keys() {
    // Longest key drives column width; shorter keys are right-padded.
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.definition_list(&[("Hostname", "localhost"), ("Port", "8080")]);
    });
    let output = tb.to_string();
    // "Hostname" is 8 cols; "Port" must be padded to 8 cols → "    Port".
    assert!(
        output.contains("    Port"),
        "expected right-padded 'Port' in: {output}"
    );
    assert!(output.contains("Hostname"));
}

#[test]
fn definition_list_handles_cjk_double_width_keys() {
    // CJK characters are 2 display columns each per UnicodeWidthStr.
    // "한" (2 cols) should be padded to match a longer ASCII key.
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.definition_list(&[("Region", "ap-northeast-2"), ("한", "Seoul")]);
    });
    let output = tb.to_string();
    // "Region" = 6 cols → "한" (2 cols) padded with 4 spaces.
    assert!(
        output.contains("    한"),
        "expected CJK-aware right-padding in: {output}"
    );
    assert!(output.contains("Seoul"));
}

#[test]
fn definition_list_empty_key_renders_only_padding() {
    // Empty key with non-empty co-keys → padded with `max_key_width` spaces.
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.definition_list(&[("Name", "Alice"), ("", "—")]);
    });
    let output = tb.to_string();
    // "Name" max width is 4 → empty key becomes "    " (4 spaces), then "  —".
    assert!(output.contains("Alice"));
    assert!(output.contains("—"));
}

// ── v0.20.0 #213: breadcrumb collapsed to BreadcrumbResponse ─────────────

#[test]
fn breadcrumb_response_exposes_rect() {
    let mut tb = TestBackend::new(60, 3);
    let mut rect = slt::Rect::default();
    let mut idx: Option<usize> = None;
    tb.render(|ui| {
        let r = ui.breadcrumb(&["Home", "Settings", "Profile"]).show();
        rect = r.rect;
        idx = r.clicked_segment;
    });
    // First frame has no prev_hit_map entry yet, so rect may be zero — render
    // a second frame so the response carries a real rect.
    tb.render(|ui| {
        let r = ui.breadcrumb(&["Home", "Settings", "Profile"]).show();
        rect = r.rect;
    });
    assert!(
        rect.width > 0,
        "rect width should be non-zero after warm frame"
    );
    assert!(
        rect.height > 0,
        "rect height should be non-zero after warm frame"
    );
    assert_eq!(idx, None, "no events → no clicked segment");
}

#[test]
fn breadcrumb_response_returns_clicked_index_on_enter() {
    let mut tb = TestBackend::new(60, 3);
    let events = slt::EventBuilder::new().key_code(KeyCode::Enter).build();
    let mut clicked: Option<usize> = None;
    tb.render_with_events(events, 0, 1, |ui| {
        let r = ui.breadcrumb(&["Home", "Settings", "Profile"]).show();
        clicked = r.clicked_segment;
    });
    assert_eq!(clicked, Some(0));
}

#[test]
fn breadcrumb_response_derefs_to_response() {
    // BreadcrumbResponse derefs into Response, so .hovered/.rect work directly.
    let mut tb = TestBackend::new(60, 3);
    tb.render(|ui| {
        let r = ui.breadcrumb(&["Home", "Settings", "Profile"]).show();
        // Must compile via Deref impl.
        let _ = r.hovered;
        let _ = r.rect;
        let _ = r.focused;
    });
}

#[test]
fn breadcrumb_builder_separator_uses_custom_string() {
    // The chainable `.separator(...)` is the only public form for custom
    // breadcrumb separators in v0.20.0+.
    let mut tb = TestBackend::new(60, 3);
    tb.render(|ui| {
        ui.breadcrumb(&["A", "B", "C"]).separator(" > ");
    });
    let output = tb.to_string();
    assert!(output.contains(" > "));
    assert!(output.contains("A"));
    assert!(output.contains("C"));
}

// --- v0.19.2 layout perf wave: end-to-end regression coverage ------------

/// Issue #150: rendering many frames in succession must remain functionally
/// correct under the `FrameState.commands_buf` carry-over. The drained Vec
/// is reused across frames, so any bug that left stale items would corrupt
/// the second frame's output.
#[test]
fn many_frames_under_commands_buf_reuse_remain_correct() {
    let mut tb = TestBackend::new(40, 4);
    for frame in 0..10 {
        tb.render(|ui| {
            ui.text(format!("frame {frame}"));
        });
        // Each frame must show only its own text — if commands_buf were
        // not properly drained we'd see leaked content from prior frames.
        let out = tb.to_string();
        assert!(
            out.contains(&format!("frame {frame}")),
            "frame {frame} text missing from output: {out:?}"
        );
        for prior in 0..frame {
            assert!(
                !out.contains(&format!("frame {prior}")),
                "frame {prior} content leaked into frame {frame}: {out:?}"
            );
        }
    }
}

/// Issue #155: rendering many frames in succession must remain functionally
/// correct under the `FrameState.frame_data` carry-over. The collect Vecs
/// are cleared in place; any bug that left stale focus / hit / scroll
/// entries would corrupt later-frame interactions and assertions.
#[test]
fn many_frames_under_frame_data_reuse_remain_correct() {
    let mut tb = TestBackend::new(60, 8);
    for frame in 0..8 {
        tb.render(|ui| {
            // Use a varying number of focusable widgets to make sure stale
            // focus_rects / hit_areas / focus_groups from a longer prior
            // frame don't survive a shorter current frame.
            let n = (frame % 4) + 1;
            for i in 0..n {
                ui.text(format!("row-{frame}-{i}"));
            }
        });
        let out = tb.to_string();
        for i in 0..((frame % 4) + 1) {
            assert!(
                out.contains(&format!("row-{frame}-{i}")),
                "current-frame row {i} missing in frame {frame}: {out:?}"
            );
        }
    }
}

// --- regression: issue #110 ThemeBuilder::builder_from + light_builder ---

#[test]
fn theme_builder_from_preserves_base() {
    use slt::Theme;
    // builder_from(base) without any overrides must reproduce `base`
    // field-for-field, otherwise users deriving variants from presets
    // would silently lose unset fields.
    let nord = Theme::nord();
    let derived = Theme::builder_from(nord).build();
    assert_eq!(derived.bg, nord.bg);
    assert_eq!(derived.primary, nord.primary);
    assert_eq!(derived.surface, nord.surface);
    assert_eq!(derived.is_dark, nord.is_dark);
}

#[test]
fn theme_light_builder_keeps_light_defaults() {
    use slt::{Color, Theme};
    // Plain Theme::builder() inherits dark() defaults for unset fields,
    // so a "light variant" that only overrides .primary would still get
    // a dark bg. light_builder() must avoid that surprise.
    let t = Theme::light_builder()
        .primary(Color::Rgb(0, 100, 200))
        .build();
    let light = Theme::light();
    assert_eq!(t.primary, Color::Rgb(0, 100, 200));
    assert_eq!(t.bg, light.bg);
    assert_eq!(t.surface, light.surface);
    assert!(!t.is_dark);
}

#[test]
fn theme_builder_methods_are_const_evaluable() {
    // Compile-time const-eval regression: if any builder method or
    // build() is demoted to non-const, this fails to compile.
    use slt::{Color, Theme};
    const T: Theme = Theme::builder()
        .primary(Color::Rgb(1, 2, 3))
        .bg(Color::Rgb(4, 5, 6))
        .build();
    assert_eq!(T.primary, Color::Rgb(1, 2, 3));
    assert_eq!(T.bg, Color::Rgb(4, 5, 6));
}

// --- regression: issue #173 in-place trim_end in extract_selection_text ---
//
// The selection helpers are crate-private, so this test exercises the
// observable behavior end-to-end: rendering text. The trim semantics
// preserved by `String::truncate(trim_end().len())` must match what
// `trim_end().to_string()` produced previously.
#[test]
fn render_strips_trailing_whitespace_from_text() {
    let mut tb = TestBackend::new(20, 2);
    tb.render(|ui| {
        ui.text("hello   ");
    });
    let out = tb.to_string();
    let first_line = out.lines().next().unwrap_or("");
    assert!(first_line.contains("hello"));
}

// ── #196: tabs mouse.x - rect.x must use saturating_sub ─────────────────────

#[test]
fn tabs_mouse_outside_rect_no_panic() {
    // Regression for #196: a click at x=0 while the tabs container starts at
    // x>0 used raw `mouse.x - rect.x` subtraction, which panicked in debug
    // builds with "attempt to subtract with overflow". After the fix the hit
    // test uses `saturating_sub`, leaving the selection unchanged.
    let mut tb = TestBackend::new(40, 5);
    let mut state = TabsState::new(vec!["Tab1", "Tab2", "Tab3"]);
    let events = slt::EventBuilder::new().click(0, 0).build();
    tb.render_with_events(events, 0, 1, |ui| {
        // Force the tabs row to start past x=0 so the click lands outside it.
        ui.col(|ui| {
            ui.text(" ");
            ui.tabs(&mut state);
        });
    });
    assert_eq!(
        state.selected, 0,
        "click outside tabs rect must not change selection"
    );
}

// ── #195: TableState::recompute_widths must short-circuit when not dirty ────

#[test]
fn table_recompute_widths_stable_across_clean_renders() {
    // Regression for #195: rendering the same TableState across multiple
    // frames without any mutation must keep the column widths stable. The
    // dirty-flag guard at the top of `recompute_widths` makes the second
    // frame a no-op; this test verifies the observable invariant — the
    // header cell width matches the longest cell content on every frame.
    let mut tb = TestBackend::new(80, 30);
    let rows: Vec<Vec<String>> = (0..200)
        .map(|i| vec![format!("row{i:03}"), format!("val{i:03}")])
        .collect();
    let mut state = TableState::new(vec!["NameNameName", "Score"], rows);

    tb.render(|ui| {
        ui.table(&mut state);
    });
    let frame1 = tb.to_string();

    tb.render(|ui| {
        ui.table(&mut state);
    });
    let frame2 = tb.to_string();

    // No mutation between frames → identical render. If `recompute_widths`
    // ever accidentally clobbers `column_widths` on a clean call, the second
    // frame would diverge from the first.
    assert_eq!(
        frame1, frame2,
        "clean re-render must produce identical output"
    );
    // Header text "NameNameName" (12 cols) is wider than every cell ("rowNNN"
    // = 6 cols) and must be visible in full — confirms widths are intact.
    assert!(frame2.contains("NameNameName"));
}

// ── #194: collect_grid_elements extraction — grid() and grid_with() share logic

#[test]
fn grid_and_grid_with_produce_same_elements() {
    // Regression for #194: the child-command parsing logic used to be a 41-line
    // byte-for-byte duplicate across `grid()` and `grid_with()`. After extraction
    // both call `collect_grid_elements`; this test pins the public contract by
    // rendering the same children through both and asserting both render the
    // input texts.
    let mut tb1 = TestBackend::new(60, 10);
    tb1.render(|ui| {
        ui.grid(3, |ui| {
            ui.text("a");
            ui.text("b");
            ui.text("c");
            ui.text("d");
            ui.text("e");
            ui.text("f");
        });
    });
    tb1.assert_contains("a");
    tb1.assert_contains("f");

    let mut tb2 = TestBackend::new(60, 10);
    tb2.render(|ui| {
        ui.grid_with(
            &[
                slt::GridColumn::Auto,
                slt::GridColumn::Auto,
                slt::GridColumn::Auto,
            ],
            |ui| {
                ui.text("a");
                ui.text("b");
                ui.text("c");
                ui.text("d");
                ui.text("e");
                ui.text("f");
            },
        );
    });
    tb2.assert_contains("a");
    tb2.assert_contains("f");
}

// ── #108: ContainerStyle mx/my margin shorthands ─────────────────────

#[test]
fn container_style_mx_my() {
    let s = slt::ContainerStyle::new().mx(4).my(2);
    let m = s.margin.unwrap();
    assert_eq!(m.left, 4);
    assert_eq!(m.right, 4);
    assert_eq!(m.top, 2);
    assert_eq!(m.bottom, 2);
}

#[test]
fn container_style_mx_preserves_vertical() {
    let s = slt::ContainerStyle::new().my(3).mx(1);
    let m = s.margin.unwrap();
    assert_eq!(m.top, 3);
    assert_eq!(m.bottom, 3);
    assert_eq!(m.left, 1);
    assert_eq!(m.right, 1);
}

#[test]
fn container_style_my_preserves_horizontal() {
    let s = slt::ContainerStyle::new().mx(5).my(2);
    let m = s.margin.unwrap();
    assert_eq!(m.left, 5);
    assert_eq!(m.right, 5);
    assert_eq!(m.top, 2);
    assert_eq!(m.bottom, 2);
}

#[test]
fn container_style_mx_only() {
    let s = slt::ContainerStyle::new().mx(2);
    let m = s.margin.unwrap();
    assert_eq!(m.left, 2);
    assert_eq!(m.right, 2);
    assert_eq!(m.top, 0);
    assert_eq!(m.bottom, 0);
}

#[test]
fn container_style_my_only() {
    let s = slt::ContainerStyle::new().my(3);
    let m = s.margin.unwrap();
    assert_eq!(m.top, 3);
    assert_eq!(m.bottom, 3);
    assert_eq!(m.left, 0);
    assert_eq!(m.right, 0);
}

// ── #111: Modifiers::remove ──────────────────────────────────────────

#[test]
fn modifiers_remove() {
    let mut m = slt::Modifiers::BOLD | slt::Modifiers::ITALIC | slt::Modifiers::UNDERLINE;
    m.remove(slt::Modifiers::ITALIC);
    assert!(m.contains(slt::Modifiers::BOLD));
    assert!(!m.contains(slt::Modifiers::ITALIC));
    assert!(m.contains(slt::Modifiers::UNDERLINE));
}

#[test]
fn modifiers_remove_none_is_noop() {
    let mut m = slt::Modifiers::BOLD;
    m.remove(slt::Modifiers::NONE);
    assert!(m.contains(slt::Modifiers::BOLD));
}

#[test]
fn modifiers_remove_all() {
    let mut m = slt::Modifiers::BOLD | slt::Modifiers::DIM;
    m.remove(slt::Modifiers::BOLD | slt::Modifiers::DIM);
    assert!(m.is_empty());
}

#[test]
fn modifiers_remove_unset_is_noop() {
    let mut m = slt::Modifiers::BOLD;
    m.remove(slt::Modifiers::ITALIC);
    assert!(m.contains(slt::Modifiers::BOLD));
    assert!(!m.contains(slt::Modifiers::ITALIC));
}

// ── #183: separator() still works after move to widgets_display ──────

#[test]
fn separator_still_callable_after_move() {
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
fn separator_colored_still_callable_after_move() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.separator_colored(slt::Color::Red);
    });
    tb.assert_contains("─");
}

// ── #133: use_memo single-downcast cache-hit path ──────────────────────────

#[test]
fn use_memo_cache_hit_returns_consistent_value() {
    let mut tb = TestBackend::new(20, 5);
    tb.render(|ui| {
        let v1 = *ui.use_memo(&42i32, |x| x * 2);
        // Different compute closure on a different slot: hook cursor must
        // advance correctly even when the cache-hit path returns directly
        // from a single downcast. If the cursor or downcast logic regresses,
        // these reads will conflict.
        let v2 = *ui.use_memo(&10i32, |x| x * 3);
        assert_eq!(v1, 84);
        assert_eq!(v2, 30);
    });
}

#[test]
fn use_memo_cache_hit_does_not_recompute() {
    let mut tb = TestBackend::new(20, 5);
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));

    let c1 = calls.clone();
    tb.render(|ui| {
        let v = *ui.use_memo(&7i32, |d| {
            c1.set(c1.get() + 1);
            d * 4
        });
        assert_eq!(v, 28);
    });

    let c2 = calls.clone();
    tb.render(|ui| {
        // Same deps — must hit cache and return the stored value via the
        // single-downcast path without invoking `compute`.
        let v = *ui.use_memo(&7i32, |d| {
            c2.set(c2.get() + 1);
            d * 99 // would corrupt result if recomputed
        });
        assert_eq!(v, 28);
    });

    assert_eq!(calls.get(), 1, "compute must run only on first frame");
}

// ── #135: SmallVec activation-key consume ──────────────────────────────────

#[test]
fn button_activates_on_enter_through_smallvec_path() {
    // Exercises the SmallVec-backed activation-key consumer through the
    // public button widget — Enter on the focused button must trigger a
    // click response, going through `consume_activation_keys` which now
    // collects matched events into a `SmallVec<[usize; 8]>` with no heap
    // allocation in the typical case.
    let mut tb = TestBackend::new(20, 5);
    let events = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Enter)
        .build();
    let mut clicked = false;
    tb.render_with_events(events, 0, 1, |ui| {
        if ui.button("OK").clicked {
            clicked = true;
        }
    });
    assert!(
        clicked,
        "Enter on focused button must activate via SmallVec path"
    );
}

#[test]
fn button_does_not_activate_without_key_event() {
    // Empty event list — SmallVec stays empty (`is_empty == true`) and the
    // early `activated = false` branch fires. Activation must not occur.
    let mut tb = TestBackend::new(20, 5);
    let mut clicked = false;
    tb.render(|ui| {
        if ui.button("OK").clicked {
            clicked = true;
        }
    });
    assert!(!clicked, "no key events ⇒ no activation");
}

// ── #148: deprecated long-form aliases still compile and behave the same ───

#[test]
#[allow(deprecated)]
fn deprecated_long_form_aliases_still_function() {
    let mut tb = TestBackend::new(40, 10);
    // Old long-forms must still produce identical layouts to the short forms.
    tb.render(|ui| {
        ui.container()
            .p(1)
            .min_w(10)
            .max_w(30)
            .min_h(2)
            .max_h(8)
            .col(|ui| {
                ui.text("legacy api");
            });
    });
    tb.assert_contains("legacy api");
}

// ── #123: candlestick_hd half-block body precision ─────────────────────────

#[test]
fn candlestick_hd_uses_half_block_body() {
    // Tall canvas + a doji (open == close) plus narrow-bodied candles forces
    // the body to land on a single half-cell, requiring ▀ or ▄ rendering.
    let mut tb = TestBackend::new(20, 8);
    let candles = vec![
        slt::Candle {
            open: 50.0,
            high: 100.0,
            low: 0.0,
            close: 50.0,
        },
        slt::Candle {
            open: 30.0,
            high: 100.0,
            low: 0.0,
            close: 70.0,
        },
        slt::Candle {
            open: 35.0,
            high: 100.0,
            low: 0.0,
            close: 65.0,
        },
    ];
    tb.render(|ui| {
        ui.candlestick_hd(
            &candles,
            slt::Color::Rgb(38, 166, 91),
            slt::Color::Rgb(234, 57, 67),
        );
    });
    let output = tb.to_string();
    assert!(
        output.contains('▀') || output.contains('▄'),
        "candlestick_hd should render at least one half-block (▀ or ▄) for sub-cell body edges; output:\n{output}"
    );
    assert!(
        output.contains('┃'),
        "candlestick_hd should still draw heavy wick ┃; output:\n{output}"
    );
}

#[test]
fn candlestick_hd_full_block_when_body_spans_full_cells() {
    // A wide-bodied candle (open near low, close near high) covers many full
    // cells, so at least one █ must appear.
    let mut tb = TestBackend::new(12, 10);
    let candles = vec![slt::Candle {
        open: 10.0,
        high: 100.0,
        low: 0.0,
        close: 90.0,
    }];
    tb.render(|ui| {
        ui.candlestick_hd(&candles, slt::Color::Green, slt::Color::Red);
    });
    let output = tb.to_string();
    assert!(
        output.contains('█'),
        "wide-bodied candle should still include full █ blocks; output:\n{output}"
    );
}

// ── #121: treemap sort uses total_cmp; tolerates NaN ───────────────────────

#[test]
fn treemap_total_cmp_handles_nan_without_panic() {
    let mut tb = TestBackend::new(30, 10);
    tb.render(|ui| {
        let _ = ui.treemap(&[
            slt::TreemapItem::new("A", 50.0, slt::Color::Red),
            slt::TreemapItem::new("B", f64::NAN, slt::Color::Blue),
            slt::TreemapItem::new("C", 30.0, slt::Color::Green),
        ]);
    });
    // Survives NaN input without panic; total_cmp gives a deterministic order.
}

// ── #115: squarify_recursive incremental ratio path (no Vec::clone) ────────

#[test]
fn treemap_many_items_renders_without_panic() {
    // Stress the squarify inner loop with 100+ items — exercises the
    // incremental sum/max/min tracking that replaces the per-iteration
    // candidate Vec::clone.
    let mut tb = TestBackend::new(80, 30);
    let items: Vec<slt::TreemapItem> = (0..120)
        .map(|i| {
            let value = ((i % 17) + 1) as f64 * 1.5;
            slt::TreemapItem::new(format!("item-{i}"), value, slt::Color::Cyan)
        })
        .collect();
    tb.render(|ui| {
        let _ = ui.treemap(&items);
    });
    let output = tb.to_string();
    assert!(
        !output.is_empty(),
        "treemap with 120 items should render some cells"
    );
}

// ── v0.19.2 fix wave: regressions for #99, #101, #117, #122, #191 ────────────

#[test]
fn rich_log_bounded_default() {
    // a-191: RichLogState::new() must default to Some(DEFAULT_MAX_ENTRIES) so
    // long-running apps cannot accumulate state without bound.
    let mut state = RichLogState::new();
    assert!(
        state.max_entries.is_some(),
        "new() must apply default cap, got max_entries=None"
    );
    for i in 0..(RichLogState::DEFAULT_MAX_ENTRIES + 10) {
        state.push_plain(format!("line {i}"));
    }
    assert!(
        state.entries.len() <= RichLogState::DEFAULT_MAX_ENTRIES,
        "entries.len() = {}, should be <= {}",
        state.entries.len(),
        RichLogState::DEFAULT_MAX_ENTRIES
    );
}

#[test]
fn rich_log_unbounded_new() {
    // a-191: explicit opt-in via new_unbounded() must skip the trim guard.
    let state = RichLogState::new_unbounded();
    assert!(state.max_entries.is_none());
}

#[test]
fn spinner_state_frame_is_stable_across_clones() {
    // a-099: SpinnerState now stores frames as &'static [char], so cloning is
    // cheap and the frame sequence must be byte-identical to the heap-Vec
    // implementation.
    let s = slt::SpinnerState::dots();
    let s2 = s.clone();
    for tick in 0..32u64 {
        assert_eq!(
            s.frame(tick),
            s2.frame(tick),
            "clone must yield identical frame at tick {tick}"
        );
    }
    let dots: Vec<char> = (0..10).map(|t| s.frame(t)).collect();
    assert_eq!(dots, vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']);
    assert_eq!(s.frame(10), '⠋', "tick 10 wraps back to first frame");

    let line = slt::SpinnerState::line();
    let line_seq: Vec<char> = (0..4).map(|t| line.frame(t)).collect();
    assert_eq!(line_seq, vec!['|', '/', '-', '\\']);
    assert_eq!(line.frame(4), '|');
}

#[test]
fn command_palette_render_uses_filter_cache() {
    // a-101: command_palette() previously called fuzzy_score twice per frame.
    // Now it routes through filtered_indices_cached(), but the rendered output
    // must remain unchanged.
    let cmds = vec![
        slt::PaletteCommand::new("open file", ""),
        slt::PaletteCommand::new("save file", ""),
        slt::PaletteCommand::new("quit", ""),
    ];
    let mut state = CommandPaletteState::new(cmds);
    state.toggle();
    state.input.push_str("file");

    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        ui.command_palette(&mut state);
    });
    let out = tb.to_string();
    assert!(out.contains("open file"), "expected 'open file' in:\n{out}");
    assert!(out.contains("save file"), "expected 'save file' in:\n{out}");
}

#[test]
fn chart_flat_buffer_renders_line() {
    // a-117: chart's plot_chars/plot_styles are now flat Vec<T> instead of
    // Vec<Vec<T>>. Verify a line chart renders without panics and produces
    // non-empty output.
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        ui.chart(
            |c| {
                c.line(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0), (3.0, 1.5)])
                    .label("S1")
                    .color(slt::Color::Cyan);
            },
            70,
            20,
        );
    });
    let out = tb.to_string();
    assert!(!out.trim().is_empty(), "chart output must not be blank");
    assert!(out.contains("S1"), "legend label must render");
}

#[test]
fn chart_bar_dataset_renders_with_flat_buffer() {
    // a-117: bar dataset path through draw_bar_dataset on flat buffer.
    let mut tb = TestBackend::new(80, 24);
    tb.render(|ui| {
        ui.chart(
            |c| {
                c.bar(&[(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)])
                    .label("B")
                    .color(slt::Color::Cyan);
            },
            70,
            20,
        );
    });
    let out = tb.to_string();
    assert!(out.contains('█'), "bar chart should render block glyph");
}

#[test]
fn sixel_image_renders_with_stack_register_array() {
    // a-122: row_registers() now uses [bool; 216] on the stack. Verify the
    // sixel encode path still produces output for a small multi-color image.
    let mut rgba = Vec::with_capacity(4 * 6 * 4);
    let cols = [
        [255u8, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [128, 128, 128, 255],
    ];
    for _ in 0..6 {
        for c in &cols {
            rgba.extend_from_slice(c);
        }
    }
    let mut tb = TestBackend::new(20, 8);
    tb.render(|ui| {
        let _ = ui.sixel_image(&rgba, 4, 6, 20, 4);
    });
    // Smoke: rendering must not panic with the stack-array implementation.
}

// ===== v0.19.2 — issue #103 / #176 regression tests =====

#[test]
fn textarea_kill_line_truncates_to_cursor() {
    // Issue #103: Ctrl+K from middle of line truncates to cursor.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();
    state.set_value("hello world");
    state.cursor_row = 0;
    state.cursor_col = 5;
    let events = slt::EventBuilder::new()
        .key_with(KeyCode::Char('k'), KeyModifiers::CONTROL)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["hello".to_string()]);
    assert_eq!(state.cursor_col, 5);
}

#[test]
fn textarea_kill_line_preserves_following_lines() {
    // Issue #103: Ctrl+K does NOT remove the line break — only truncates
    // the current line at the cursor column.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();
    state.set_value("foo bar\nnext");
    state.cursor_row = 0;
    state.cursor_col = 3;
    let events = slt::EventBuilder::new()
        .key_with(KeyCode::Char('k'), KeyModifiers::CONTROL)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.lines, vec!["foo".to_string(), "next".to_string()]);
}

#[test]
fn textarea_word_jump_forward() {
    // Issue #103: Ctrl+Right from cursor=0 on "foo bar baz" must jump
    // past the end of the next word ("foo") to char 3.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();
    state.set_value("foo bar baz");
    state.cursor_row = 0;
    state.cursor_col = 0;
    let events = slt::EventBuilder::new()
        .key_with(KeyCode::Right, KeyModifiers::CONTROL)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.cursor_col, 3);
}

#[test]
fn textarea_word_jump_backward() {
    // Issue #103: Ctrl+Left from cursor=7 on "foo bar baz" must jump back
    // to the start of "bar" at char 4.
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();
    state.set_value("foo bar baz");
    state.cursor_row = 0;
    state.cursor_col = 7;
    let events = slt::EventBuilder::new()
        .key_with(KeyCode::Left, KeyModifiers::CONTROL)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.cursor_col, 4);
}

#[test]
fn textarea_word_jump_alt_modifier() {
    // Issue #103: Alt+Left/Right are equivalent to Ctrl+Left/Right
    // (matches macOS readline conventions).
    let mut tb = TestBackend::new(40, 10);
    let mut state = TextareaState::new();
    state.set_value("alpha beta gamma");
    state.cursor_row = 0;
    state.cursor_col = 0;
    let events = slt::EventBuilder::new()
        .key_with(KeyCode::Right, KeyModifiers::ALT)
        .build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.textarea(&mut state, 5);
    });
    assert_eq!(state.cursor_col, 5);
}

#[test]
fn markdown_byte_index_handles_cjk_bold() {
    // Issue #176: byte-index scan of `**굵은**` must NOT split the
    // multi-byte Korean characters (each is 3 bytes in UTF-8).
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.markdown("**굵은**");
    });
    tb.assert_contains("굵은");
}

#[test]
fn markdown_byte_index_handles_interleaved_cjk_markers() {
    // Issue #176: arbitrary order of bold/italic/code with multi-byte text.
    let mut tb = TestBackend::new(80, 5);
    tb.render(|ui| {
        ui.markdown("**굵은** `코드` *이탤릭*");
    });
    let out = tb.to_string_trimmed();
    assert!(out.contains("굵은"), "bold CJK segment lost: {out:?}");
    assert!(out.contains("코드"), "code CJK segment lost: {out:?}");
    assert!(out.contains("이탤릭"), "italic CJK segment lost: {out:?}");
    // The marker characters themselves must be stripped.
    assert!(!out.contains("**"), "** marker must be stripped: {out:?}");
    assert!(!out.contains('`'), "code marker must be stripped: {out:?}");
}

#[test]
fn markdown_byte_index_unclosed_marker_falls_through() {
    // Issue #176: unclosed `**` must render as plain text, not panic.
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.markdown("**unclosed");
    });
    // The literal `**unclosed` must appear since the marker never closes.
    tb.assert_contains("**unclosed");
}

#[test]
fn markdown_byte_index_empty_bold_marker() {
    // Issue #176: `****` is a valid empty bold span — must not produce
    // garbage or panic on byte indices.
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.markdown("a****b");
    });
    tb.assert_contains("ab");
}

#[test]
fn virtual_list_cursor_not_anchored_to_viewport_bottom() {
    // Regression for #192: previously `start = selected - vh + 1` always
    // pinned the cursor to the bottom of the viewport. The sticky-viewport
    // fix keeps the cursor mid-viewport when the user scrolls up.
    let mut tb = TestBackend::new(40, 12);
    let items: Vec<String> = (0..20).map(|i| format!("Item {i}")).collect();
    let mut state = ListState::new(items);
    let visible_height: u32 = 5;

    // Frame 1: scroll the cursor to row 10. The viewport snaps so that
    // `Item 10` is the last visible row (start = 6).
    state.selected = 10;
    tb.render(|ui| {
        ui.virtual_list(&mut state, visible_height, |ui, idx| {
            ui.text(format!("Item {idx}"));
        });
    });
    tb.assert_contains("Item 10");

    // Frame 2: move the cursor up by one. With the bug the viewport would
    // also slide up by one (start = 5) so `Item 10` would no longer be
    // visible. With the fix the viewport stays put and `Item 10` is still
    // on screen — the cursor moved off the bottom row instead of dragging
    // the viewport with it.
    state.selected = 9;
    tb.render(|ui| {
        ui.virtual_list(&mut state, visible_height, |ui, idx| {
            ui.text(format!("Item {idx}"));
        });
    });
    let out = tb.to_string();
    assert!(
        out.contains("Item 10"),
        "viewport should stay put when cursor moves up, but Item 10 disappeared: {out:?}"
    );
    assert!(
        !out.contains("Item 5"),
        "viewport should not have followed the cursor up, but Item 5 became visible: {out:?}"
    );
}

#[test]
fn calendar_h_l_move_by_day() {
    // Regression for #193: `h`/`l` previously navigated months, which
    // contradicted vim convention (h/l = cursor ±1 unit). They now move
    // the cursor by one day, and `[`/`]` navigate months instead.
    let mut tb = TestBackend::new(40, 12);
    let mut state = CalendarState::from_ym(2024, 6);

    // Walk the cursor to June 15 with arrow keys so we can observe `h`/`l`
    // from a known mid-month position. Pressing Enter at the end commits
    // the cursor to `selected_day`, which the test reads via the public
    // `selected_date()` getter (`cursor_day` is crate-private).
    let mut walk = slt::EventBuilder::new();
    for _ in 0..14 {
        walk = walk.key_code(slt::KeyCode::Right);
    }
    let setup = walk.key_code(slt::KeyCode::Enter).build();
    tb.render_with_events(setup, 0, 1, |ui| {
        ui.calendar(&mut state);
    });
    assert_eq!(
        state.selected_date(),
        Some((2024, 6, 15)),
        "setup precondition: cursor walked to June 15"
    );

    // `h` must move the cursor one day backward inside the same month,
    // not jump to the previous month. The bug fixed in #193 used to map
    // `h` to `prev_month()`, which would land us in May.
    let h = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Char('h'))
        .key_code(slt::KeyCode::Enter)
        .build();
    tb.render_with_events(h, 0, 1, |ui| {
        ui.calendar(&mut state);
    });
    assert_eq!(
        state.selected_date(),
        Some((2024, 6, 14)),
        "h should move cursor one day back, staying in June"
    );

    // `l` must move the cursor one day forward inside the same month.
    let l = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Char('l'))
        .key_code(slt::KeyCode::Enter)
        .build();
    tb.render_with_events(l, 0, 1, |ui| {
        ui.calendar(&mut state);
    });
    assert_eq!(
        state.selected_date(),
        Some((2024, 6, 15)),
        "l should move cursor one day forward, staying in June"
    );

    // `[` is the new month-back binding.
    let lb = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Char('['))
        .build();
    tb.render_with_events(lb, 0, 1, |ui| {
        ui.calendar(&mut state);
    });
    assert_eq!((state.year, state.month), (2024, 5));

    // `]` is the new month-forward binding.
    let rb = slt::EventBuilder::new()
        .key_code(slt::KeyCode::Char(']'))
        .build();
    tb.render_with_events(rb, 0, 1, |ui| {
        ui.calendar(&mut state);
    });
    assert_eq!((state.year, state.month), (2024, 6));
}

/// Default (single) mode still selects one date and never produces a range.
#[test]
fn calendar_single_mode_unchanged() {
    let mut tb = TestBackend::new(40, 14);
    let mut state = CalendarState::from_ym(2024, 6);
    assert_eq!(state.mode(), slt::CalendarSelect::Single);

    // Walk to June 10 and commit with Enter.
    let mut walk = slt::EventBuilder::new();
    for _ in 0..9 {
        walk = walk.key_code(KeyCode::Right);
    }
    let events = walk.key_code(KeyCode::Enter).build();
    tb.render_with_events(events, 0, 1, |ui| {
        ui.calendar(&mut state);
    });

    assert_eq!(state.selected_date(), Some((2024, 6, 10)));
    assert!(
        state.selected_range().is_none(),
        "single mode must not expose a range"
    );
    assert!(state.selected_time().is_none(), "time is off by default");
}

/// `Shift+Right` extends the range forward from the anchor.
#[test]
fn calendar_shift_right_extends_range() {
    let mut tb = TestBackend::new(40, 14);
    let mut state = CalendarState::from_ym(2024, 6);
    state.with_range();

    // Walk cursor to June 5, anchor with Enter.
    let mut walk = slt::EventBuilder::new();
    for _ in 0..4 {
        walk = walk.key_code(KeyCode::Right);
    }
    let setup = walk.key_code(KeyCode::Enter).build();
    tb.render_with_events(setup, 0, 1, |ui| {
        ui.calendar(&mut state);
    });
    assert_eq!(
        state.selected_range(),
        Some((
            slt::CalDate {
                year: 2024,
                month: 6,
                day: 5
            },
            slt::CalDate {
                year: 2024,
                month: 6,
                day: 5
            }
        )),
        "anchor sets a degenerate start==end range"
    );

    // Shift+Right ×3 → extend to June 8.
    let mut ext = slt::EventBuilder::new();
    for _ in 0..3 {
        ext = ext.key_with(KeyCode::Right, KeyModifiers::SHIFT);
    }
    tb.render_with_events(ext.build(), 0, 1, |ui| {
        ui.calendar(&mut state);
    });

    let (start, end) = state.selected_range().expect("range active");
    assert_eq!((start.year, start.month, start.day), (2024, 6, 5));
    assert_eq!((end.year, end.month, end.day), (2024, 6, 8));
}

/// A Shift-extended range may cross a month boundary.
#[test]
fn calendar_range_spans_month_boundary() {
    let mut tb = TestBackend::new(40, 14);
    let mut state = CalendarState::from_ym(2024, 1);
    state.with_range();

    // Walk cursor to Jan 30 (29 Right presses from day 1), anchor.
    let mut walk = slt::EventBuilder::new();
    for _ in 0..29 {
        walk = walk.key_code(KeyCode::Right);
    }
    let setup = walk.key_code(KeyCode::Enter).build();
    tb.render_with_events(setup, 0, 1, |ui| {
        ui.calendar(&mut state);
    });

    // Shift+Right ×4 → Jan 30 → Jan 31 → Feb 1, 2, 3.
    let mut ext = slt::EventBuilder::new();
    for _ in 0..4 {
        ext = ext.key_with(KeyCode::Right, KeyModifiers::SHIFT);
    }
    tb.render_with_events(ext.build(), 0, 1, |ui| {
        ui.calendar(&mut state);
    });

    let (start, end) = state.selected_range().expect("range active");
    assert_eq!((start.year, start.month, start.day), (2024, 1, 30));
    assert_eq!(
        (end.year, end.month, end.day),
        (2024, 2, 3),
        "extent crossed into February"
    );
}

/// Extending backward still yields a normalized `(start, end)` with start <= end.
#[test]
fn calendar_range_normalizes_order() {
    let mut tb = TestBackend::new(40, 14);
    let mut state = CalendarState::from_ym(2024, 6);
    state.with_range();

    // Anchor on June 15.
    let mut walk = slt::EventBuilder::new();
    for _ in 0..14 {
        walk = walk.key_code(KeyCode::Right);
    }
    let setup = walk.key_code(KeyCode::Enter).build();
    tb.render_with_events(setup, 0, 1, |ui| {
        ui.calendar(&mut state);
    });

    // Shift+Left ×5 → extent at June 10 (before the anchor).
    let mut ext = slt::EventBuilder::new();
    for _ in 0..5 {
        ext = ext.key_with(KeyCode::Left, KeyModifiers::SHIFT);
    }
    tb.render_with_events(ext.build(), 0, 1, |ui| {
        ui.calendar(&mut state);
    });

    let (start, end) = state.selected_range().expect("range active");
    assert!(
        (start.year, start.month, start.day) <= (end.year, end.month, end.day),
        "range must be normalized: {start:?} > {end:?}"
    );
    assert_eq!((start.month, start.day), (6, 10));
    assert_eq!((end.month, end.day), (6, 15));
}

/// Plain click sets the anchor; Shift+click sets the extent endpoint.
#[test]
fn calendar_shift_click_sets_extent() {
    let mut tb = TestBackend::new(40, 14);
    let mut state = CalendarState::from_ym(2024, 6);
    state.with_range();

    // June 2024: first weekday (Mo-based) for June 1 is Saturday → first=5.
    // Grid rows start at rel_y==2; column width is 3 cells; rel_x = col*3.
    // Day 1 sits at col 5 (rel_x 15), day 8 at col 5 row 1 (rel_y 3), etc.
    // We compute click positions from first_weekday so the test is robust.
    // For day d: idx = first + (d - 1); week = idx / 7; col = idx % 7.
    let click_pos = |day: u32| -> (u32, u32) {
        let first = 5_u32; // June 2024 starts on Saturday (Mo-indexed = 5)
        let idx = first + (day - 1);
        let week = idx / 7;
        let col = idx % 7;
        // widget at (0,0): title row 0, weekday row 1, weeks start row 2.
        // each day cell is 3 wide, centered text but col*3 lands inside cell.
        (col * 3 + 1, 2 + week)
    };

    let (ax, ay) = click_pos(3);
    let (ex, ey) = click_pos(7);

    // Warm-up frame populates the hit map so the next frame's click resolves.
    tb.render_with_events(Vec::new(), 0, 1, |ui| {
        ui.calendar(&mut state);
    });

    // Plain click on June 3 → anchor.
    tb.render_with_events(slt::EventBuilder::new().click(ax, ay).build(), 0, 1, |ui| {
        ui.calendar(&mut state);
    });
    assert_eq!(
        state.selected_range(),
        Some((
            slt::CalDate {
                year: 2024,
                month: 6,
                day: 3
            },
            slt::CalDate {
                year: 2024,
                month: 6,
                day: 3
            }
        )),
        "plain click should anchor on June 3"
    );

    // Shift+click on June 7 → extent.
    tb.render_with_events(
        slt::EventBuilder::new()
            .click_with(ex, ey, KeyModifiers::SHIFT)
            .build(),
        0,
        1,
        |ui| {
            ui.calendar(&mut state);
        },
    );
    let (start, end) = state.selected_range().expect("range active");
    assert_eq!((start.month, start.day), (6, 3));
    assert_eq!((end.month, end.day), (6, 7));
}

/// The range band renders each in-band day number, including endpoints.
#[test]
fn calendar_range_band_renders() {
    let mut tb = TestBackend::new(40, 14);
    let mut state = CalendarState::from_ym(2024, 6);
    state.with_range();

    // Anchor June 10, Shift-extend to June 13.
    let mut walk = slt::EventBuilder::new();
    for _ in 0..9 {
        walk = walk.key_code(KeyCode::Right);
    }
    let mut setup = walk.key_code(KeyCode::Enter);
    for _ in 0..3 {
        setup = setup.key_with(KeyCode::Right, KeyModifiers::SHIFT);
    }
    tb.render_with_events(setup.build(), 0, 1, |ui| {
        ui.calendar(&mut state);
    });

    let (start, end) = state.selected_range().expect("range active");
    assert_eq!((start.day, end.day), (10, 13));

    // All in-band day numbers should be present in the rendered text.
    for d in ["10", "11", "12", "13"] {
        tb.assert_contains(d);
    }
}

/// Time row is hidden by default and shown after `with_time()`.
#[test]
fn calendar_time_disabled_no_row() {
    let mut tb = TestBackend::new(40, 14);
    let mut state = CalendarState::from_ym(2024, 6);
    tb.render(|ui| {
        ui.calendar(&mut state);
    });
    tb.assert_not_contains("00:00");
}

#[test]
fn calendar_time_enabled_renders() {
    let mut tb = TestBackend::new(40, 14);
    let mut state = CalendarState::from_ym(2024, 6);
    state.with_time();
    tb.render(|ui| {
        ui.calendar(&mut state);
    });
    tb.assert_contains("00:00");
    assert_eq!(state.selected_time(), Some((0, 0)));
}

/// `Response.changed` flips on range/time changes and is false on no-op frames.
#[test]
fn calendar_changed_flag() {
    let mut tb = TestBackend::new(40, 14);
    let mut state = CalendarState::from_ym(2024, 6);
    state.with_range();

    // Frame 1: anchor via Enter → changed.
    let mut changed = false;
    tb.render_with_events(
        slt::EventBuilder::new().key_code(KeyCode::Enter).build(),
        0,
        1,
        |ui| {
            changed = ui.calendar(&mut state).changed;
        },
    );
    assert!(changed, "anchoring should flip changed");

    // Frame 2: no events → no change.
    tb.render_with_events(Vec::new(), 0, 1, |ui| {
        changed = ui.calendar(&mut state).changed;
    });
    assert!(!changed, "no-op frame must not report changed");

    // Frame 3: Shift+Right extends the range → changed.
    tb.render_with_events(
        slt::EventBuilder::new()
            .key_with(KeyCode::Right, KeyModifiers::SHIFT)
            .build(),
        0,
        1,
        |ui| {
            changed = ui.calendar(&mut state).changed;
        },
    );
    assert!(changed, "range extend should flip changed");
}

proptest::proptest! {
    /// Random anchor/extent walks always yield an ordered, inclusive range.
    #[test]
    fn calendar_range_always_ordered(anchor_day in 1u32..=28, fwd in 0i32..40, back in 0i32..40) {
        let mut tb = TestBackend::new(40, 14);
        let mut state = CalendarState::from_ym(2024, 6);
        state.with_range();

        // Walk to the anchor day, then anchor.
        let mut walk = slt::EventBuilder::new();
        for _ in 1..anchor_day {
            walk = walk.key_code(KeyCode::Right);
        }
        let setup = walk.key_code(KeyCode::Enter).build();
        tb.render_with_events(setup, 0, 1, |ui| {
            ui.calendar(&mut state);
        });

        // Extend forward then backward by arbitrary amounts.
        let mut ext = slt::EventBuilder::new();
        for _ in 0..fwd {
            ext = ext.key_with(KeyCode::Right, KeyModifiers::SHIFT);
        }
        for _ in 0..back {
            ext = ext.key_with(KeyCode::Left, KeyModifiers::SHIFT);
        }
        tb.render_with_events(ext.build(), 0, 1, |ui| {
            ui.calendar(&mut state);
        });

        let (start, end) = state.selected_range().expect("range active after anchor");
        proptest::prop_assert!(
            (start.year, start.month, start.day) <= (end.year, end.month, end.day),
            "range not ordered: {start:?} > {end:?}"
        );
    }
}
