use slt::widgets::{CalendarState, ListState, TableState, TabsState};
use slt::{Border, TestBackend};

#[test]
fn snapshot_text() {
    let mut tb = TestBackend::new(30, 3);
    tb.render(|ui| {
        ui.text("hello, world");
    });
    insta::assert_snapshot!(tb.to_string_trimmed(), @r"hello, world");
}

#[test]
fn snapshot_bordered_col() {
    let mut tb = TestBackend::new(30, 5);
    tb.render(|ui| {
        let _ = ui.bordered(Border::Rounded).col(|ui| {
            ui.text("title");
            ui.text("body");
        });
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}

#[test]
fn snapshot_row_layout() {
    let mut tb = TestBackend::new(30, 3);
    tb.render(|ui| {
        let _ = ui.row(|ui| {
            ui.text("left");
            ui.spacer();
            ui.text("right");
        });
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}

#[test]
fn snapshot_button() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        let _ = ui.button("Click Me");
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}

#[test]
fn snapshot_progress() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.progress(0.5);
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}

#[test]
fn snapshot_tabs() {
    let mut tb = TestBackend::new(40, 5);
    let mut tabs = TabsState::new(vec!["Home", "Settings", "About"]);
    tb.render(|ui| {
        let _ = ui.tabs(&mut tabs);
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}

#[test]
fn snapshot_table() {
    let mut tb = TestBackend::new(50, 10);
    let mut table = TableState::new(
        vec!["Name", "Role", "Status"],
        vec![
            vec!["Alice", "Admin", "Active"],
            vec!["Bob", "Editor", "Away"],
            vec!["Cara", "Viewer", "Active"],
        ],
    );
    tb.render(|ui| {
        let _ = ui.table(&mut table);
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}

#[test]
fn snapshot_table_zebra() {
    let mut tb = TestBackend::new(50, 10);
    let mut table = TableState::new(
        vec!["Name", "Role", "Status"],
        vec![
            vec!["Alice", "Admin", "Active"],
            vec!["Bob", "Editor", "Away"],
            vec!["Cara", "Viewer", "Active"],
        ],
    );
    table.zebra = true;
    tb.render(|ui| {
        let _ = ui.table(&mut table);
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}

#[test]
fn snapshot_calendar() {
    let mut tb = TestBackend::new(40, 12);
    let mut calendar = CalendarState::from_ym(2024, 3);
    tb.render(|ui| {
        let _ = ui.calendar(&mut calendar);
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}

#[test]
fn snapshot_list() {
    let mut tb = TestBackend::new(40, 8);
    let mut list = ListState::new(vec!["Item 1", "Item 2", "Item 3", "Item 4", "Item 5"]);
    tb.render(|ui| {
        let _ = ui.list(&mut list);
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}

#[test]
fn snapshot_separator() {
    let mut tb = TestBackend::new(40, 5);
    tb.render(|ui| {
        ui.text("Top");
        ui.separator();
        ui.text("Bottom");
    });
    insta::assert_snapshot!(tb.to_string_trimmed());
}
