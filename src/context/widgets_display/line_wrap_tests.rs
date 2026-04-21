use super::*;
use crate::event::Event;
use crate::FrameState;

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

    assert!(matches!(
        ctx.commands.first(),
        Some(Command::BeginContainer(_))
    ));
    assert!(ctx
        .commands
        .iter()
        .any(|cmd| matches!(cmd, Command::Link { text, .. } if text == "Docs")));
    assert!(matches!(ctx.commands.last(), Some(Command::EndContainer)));
}
