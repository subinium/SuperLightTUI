//! Issue #438 / #445: built-in selection bytes through the real flush
//! pipeline. Widget drags emit no reverse video and no OSC 52; plain text
//! drags highlight and copy exactly once.
//!
//! Gated behind the dev-only `pty-test` feature.
#![cfg(feature = "pty-test")]

use slt::{Context, EventBuilder, PtyBackend, ScrollState, SplitPaneState};

const OSC52: &str = "\u{1b}]52;";
const REVERSE: &str = "\u{1b}[7m";

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

fn count(haystack: &[u8], needle: &str) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle.as_bytes())
        .count()
}

fn base64(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(CHARS[(n >> 18) as usize & 63] as char);
        out.push(CHARS[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            CHARS[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn split_view(ui: &mut Context, split: &mut SplitPaneState) {
    let _ = ui.split_pane(
        split,
        |ui| {
            ui.text("LEFTPANE text");
        },
        |ui| {
            ui.text("RIGHTPANE text");
        },
    );
}

fn scroll_view(ui: &mut Context, scroll: &mut ScrollState) {
    let _ = ui.row(|ui| {
        let _ = ui.scrollable(scroll).grow(1).col(|ui| {
            for i in 0..40 {
                ui.text(format!("row {i:02} abcdef"));
            }
        });
        let _ = ui.scrollbar(scroll);
    });
}

#[test]
fn widget_drags_emit_no_highlight_and_no_clipboard_write() {
    // Splitter: press on the divider, drag, release.
    let mut pb = PtyBackend::new(40, 4);
    let mut split = SplitPaneState::new(0.5);
    pb.render(|ui| split_view(ui, &mut split));
    pb.render(|ui| split_view(ui, &mut split));
    // Same layout headlessly, to find the divider column.
    let mut probe = slt::TestBackend::new(40, 4);
    let mut probe_split = SplitPaneState::new(0.5);
    probe.render(|ui| split_view(ui, &mut probe_split));
    let divider = probe
        .line(0)
        .chars()
        .position(|c| c == '│')
        .expect("divider column") as u32;
    for events in [
        EventBuilder::new().click(divider, 0).build(),
        EventBuilder::new().drag(11, 0).drag(9, 1).build(),
        EventBuilder::new().mouse_up(9, 1).build(),
        Vec::new(),
    ] {
        pb.render_with_events(events, |ui| split_view(ui, &mut split));
    }
    assert!(split.ratio < 0.5, "splitter drag still moves the divider");

    // Scrollbar thumb: press, drag, release.
    let mut scroll = ScrollState::new();
    let mut sb = PtyBackend::new(30, 6);
    sb.render(|ui| scroll_view(ui, &mut scroll));
    sb.render(|ui| scroll_view(ui, &mut scroll));
    for events in [
        EventBuilder::new().click(29, 0).build(),
        EventBuilder::new().drag(29, 2).drag(27, 3).build(),
        EventBuilder::new().mouse_up(27, 3).build(),
        Vec::new(),
    ] {
        sb.render_with_events(events, |ui| scroll_view(ui, &mut scroll));
    }
    assert!(scroll.offset > 0, "thumb drag still scrolls");

    for raw in pb.frames_raw().chain(sb.frames_raw()) {
        assert!(!contains(raw, OSC52), "widget drag wrote the clipboard");
        assert!(!contains(raw, REVERSE), "widget drag highlighted text");
    }
}

fn text_view(ui: &mut Context) {
    let _ = ui.col(|ui| {
        ui.text("COPYME selectable text");
        ui.text("another line");
    });
}

#[test]
fn plain_text_drag_highlights_and_copies_once() {
    let mut pb = PtyBackend::new(40, 4);
    pb.render(text_view);
    pb.render(text_view);
    pb.render_with_events(
        EventBuilder::new().click(0, 0).drag(5, 0).build(),
        text_view,
    );
    assert!(
        contains(pb.last_raw(), REVERSE),
        "a plain drag shows reverse video during the drag"
    );
    pb.render_with_events(EventBuilder::new().mouse_up(5, 0).build(), text_view);
    pb.render(text_view);

    let total: usize = pb.frames_raw().map(|raw| count(raw, OSC52)).sum();
    assert_eq!(total, 1, "exactly one OSC 52 payload");
    let payload = format!("{OSC52}c;{}", base64(b"COPYME"));
    assert!(
        pb.frames_raw().any(|raw| contains(raw, &payload)),
        "the copied text is the dragged text"
    );
}

#[test]
fn top_level_text_drag_copies_that_text() {
    let view = |ui: &mut Context| {
        ui.text("TOPLEVEL words here");
    };
    let mut pb = PtyBackend::new(40, 3);
    pb.render(view);
    pb.render(view);
    pb.render_with_events(EventBuilder::new().click(0, 0).drag(7, 0).build(), view);
    assert!(
        contains(pb.last_raw(), REVERSE),
        "top-level text highlights"
    );
    pb.render_with_events(EventBuilder::new().mouse_up(7, 0).build(), view);
    let payload = format!("{OSC52}c;{}", base64(b"TOPLEVEL"));
    assert!(pb.frames_raw().any(|raw| contains(raw, &payload)));
}

fn rows(ui: &mut Context, scroll: &mut ScrollState) {
    let _ = ui.scrollable(scroll).grow(1).col(|ui| {
        for i in 0..40 {
            ui.text(format!("row {i:02} abcdef"));
        }
    });
}

#[test]
fn scrolled_region_copy_matches_the_selected_lines() {
    let mut scroll = ScrollState::new();
    let mut pb = PtyBackend::new(30, 6);
    pb.render(|ui| rows(ui, &mut scroll));
    pb.render(|ui| rows(ui, &mut scroll));
    // Scroll first so the selection lives in content rows 4 and 5.
    let wheel = EventBuilder::new()
        .scroll_down(3, 3)
        .scroll_down(3, 3)
        .scroll_down(3, 3)
        .build();
    pb.render_with_events(wheel, |ui| rows(ui, &mut scroll));
    pb.render(|ui| rows(ui, &mut scroll));
    pb.render_with_events(EventBuilder::new().click(0, 1).drag(8, 2).build(), |ui| {
        rows(ui, &mut scroll)
    });
    pb.render_with_events(EventBuilder::new().mouse_up(8, 2).build(), |ui| {
        rows(ui, &mut scroll)
    });
    let payload = format!("{OSC52}c;{}", base64(b"row 04 abcdef\nrow 05 ab"));
    assert!(
        pb.frames_raw().any(|raw| contains(raw, &payload)),
        "copied text is the selected logical content"
    );
}
