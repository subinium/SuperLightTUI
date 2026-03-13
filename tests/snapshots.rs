use slt::{Border, TestBackend};

#[test]
fn snapshot_text() {
    let mut tb = TestBackend::new(30, 3);
    tb.render(|ui| {
        ui.text("hello, world");
    });
    insta::assert_snapshot!(tb, @r"hello, world");
}

#[test]
fn snapshot_bordered_col() {
    let mut tb = TestBackend::new(30, 5);
    tb.render(|ui| {
        ui.bordered(Border::Rounded).col(|ui| {
            ui.text("title");
            ui.text("body");
        });
    });
    insta::assert_snapshot!(tb);
}

#[test]
fn snapshot_row_layout() {
    let mut tb = TestBackend::new(30, 3);
    tb.render(|ui| {
        ui.row(|ui| {
            ui.text("left");
            ui.spacer();
            ui.text("right");
        });
    });
    insta::assert_snapshot!(tb);
}

#[test]
fn snapshot_button() {
    let mut tb = TestBackend::new(30, 3);
    tb.render(|ui| {
        ui.button("Click me");
    });
    insta::assert_snapshot!(tb);
}
