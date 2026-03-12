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
fn text_bold_renders() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.text("bold text").bold();
    });
    tb.assert_contains("bold text");
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
