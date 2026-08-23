use super::*;
use crate::FrameState;
use crate::event::Event;

#[test]
fn line_wrap_without_links_compacts_to_rich_text() {
    let mut frame = FrameState::default();
    let mut ctx = Context::new(Vec::<Event>::new(), 80, 24, &mut frame, Theme::dark());

    ctx.line_wrap(|ui| {
        ui.text("hello ");
        ui.text("world").bold();
    });

    assert!(matches!(
        ctx.commands.as_slice(),
        [Command::RichText { .. }]
    ));
}

#[test]
fn line_wrap_with_links_keeps_interactive_commands() {
    let mut frame = FrameState::default();
    let mut ctx = Context::new(Vec::<Event>::new(), 80, 24, &mut frame, Theme::dark());

    ctx.line_wrap(|ui| {
        ui.text("Visit ");
        ui.link("Docs", "https://docs.rs");
    });

    assert!(matches!(ctx.commands.first(), Some(Command::WrapMarker(0))));
    assert!(matches!(
        ctx.commands.get(1),
        Some(Command::BeginContainer(_))
    ));
    assert!(
        ctx.commands
            .iter()
            .any(|cmd| matches!(cmd, Command::Link { text, .. } if text == "Docs"))
    );
    assert!(matches!(ctx.commands.last(), Some(Command::EndContainer)));
}

#[test]
fn line_wrap_with_links_wraps_and_preserves_hyperlink_cells() {
    let mut backend = crate::TestBackend::new(8, 5);
    backend.render(|ui| {
        ui.line_wrap(|ui| {
            ui.text("prefix ");
            ui.link("Docs", "https://docs.rs");
            ui.text(" trailing words");
        });
    });

    let (_, link_y) = backend.find_text("Docs").expect("wrapped link text");
    let (_, trailing_y) = backend
        .find_text("trailing")
        .expect("wrapped trailing text");
    assert!(link_y > 0, "link should move to the next flex line");
    assert!(
        trailing_y > link_y,
        "long trailing text should wrap internally"
    );
    let link_cells = (0..backend.width()).filter(|&x| {
        backend.buffer().get(x, link_y).hyperlink.as_deref() == Some("https://docs.rs")
    });
    assert_eq!(link_cells.count(), 4);
}
