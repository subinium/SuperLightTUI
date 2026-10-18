//! Tests for the `ratatui` widget bridge (issue #470).
#![cfg(feature = "ratatui")]

use ratatui_widgets::block::Block;
use ratatui_widgets::paragraph::Paragraph;
use slt::ratatui_compat::ratatui_core::buffer::CellDiffOption;
use slt::ratatui_compat::ratatui_core::style::{
    Color as RatColor, Modifier as RatModifier, Style as RatStyle,
};
use slt::widgets::ScrollState;
use slt::{Color, Modifiers, Style, TestBackend};

const SLT_COLORS: [Color; 19] = [
    Color::Reset,
    Color::Black,
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::White,
    Color::DarkGray,
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
    Color::LightWhite,
    Color::Rgb(12, 34, 56),
    Color::Indexed(200),
];

const RAT_COLORS: [RatColor; 19] = [
    RatColor::Reset,
    RatColor::Black,
    RatColor::Red,
    RatColor::Green,
    RatColor::Yellow,
    RatColor::Blue,
    RatColor::Magenta,
    RatColor::Cyan,
    RatColor::Gray,
    RatColor::DarkGray,
    RatColor::LightRed,
    RatColor::LightGreen,
    RatColor::LightYellow,
    RatColor::LightBlue,
    RatColor::LightMagenta,
    RatColor::LightCyan,
    RatColor::White,
    RatColor::Rgb(12, 34, 56),
    RatColor::Indexed(200),
];

#[test]
fn colors_round_trip_in_both_directions() {
    for (slt, rat) in SLT_COLORS.into_iter().zip(RAT_COLORS) {
        assert_eq!(RatColor::from(slt), rat, "{slt:?}");
        assert_eq!(Color::from(rat), slt, "{rat:?}");
        assert_eq!(Color::from(RatColor::from(slt)), slt);
        assert_eq!(RatColor::from(Color::from(rat)), rat);
    }
    for i in 0..=255u8 {
        assert_eq!(Color::from(RatColor::Indexed(i)), Color::Indexed(i));
    }
}

#[test]
fn lossless_modifiers_round_trip() {
    let pairs = [
        (Modifiers::BOLD, RatModifier::BOLD),
        (Modifiers::DIM, RatModifier::DIM),
        (Modifiers::ITALIC, RatModifier::ITALIC),
        (Modifiers::UNDERLINE, RatModifier::UNDERLINED),
        (Modifiers::REVERSED, RatModifier::REVERSED),
        (Modifiers::STRIKETHROUGH, RatModifier::CROSSED_OUT),
        (Modifiers::BLINK, RatModifier::SLOW_BLINK),
    ];
    let mut all_slt = Modifiers::NONE;
    let mut all_rat = RatModifier::empty();
    for (slt, rat) in pairs {
        assert_eq!(RatModifier::from(slt), rat, "{slt:?}");
        assert_eq!(Modifiers::from(rat), slt, "{rat:?}");
        all_slt.insert(slt);
        all_rat.insert(rat);
    }
    assert_eq!(RatModifier::from(all_slt), all_rat);
    assert_eq!(Modifiers::from(all_rat), all_slt);
}

#[test]
fn lossy_modifiers_follow_the_documented_table() {
    assert_eq!(Modifiers::from(RatModifier::RAPID_BLINK), Modifiers::BLINK);
    assert_eq!(
        Modifiers::from(RatModifier::SLOW_BLINK | RatModifier::RAPID_BLINK),
        Modifiers::BLINK
    );
    assert_eq!(Modifiers::from(RatModifier::HIDDEN), Modifiers::NONE);
    assert_eq!(RatModifier::from(Modifiers::OVERLINE), RatModifier::empty());
    let bold_overline = Modifiers(Modifiers::BOLD.0 | Modifiers::OVERLINE.0);
    assert_eq!(RatModifier::from(bold_overline), RatModifier::BOLD);
}

#[test]
fn styles_round_trip_and_drop_sub_modifier() {
    let slt = Style::new()
        .fg(Color::Cyan)
        .bg(Color::Rgb(1, 2, 3))
        .bold()
        .italic();
    let rat = RatStyle::from(slt);
    assert_eq!(rat.fg, Some(RatColor::Cyan));
    assert_eq!(rat.bg, Some(RatColor::Rgb(1, 2, 3)));
    assert_eq!(rat.add_modifier, RatModifier::BOLD | RatModifier::ITALIC);
    assert_eq!(Style::from(rat), slt);

    let with_sub = RatStyle::new()
        .fg(RatColor::Red)
        .remove_modifier(RatModifier::BOLD);
    assert_eq!(Style::from(with_sub), Style::new().fg(Color::Red));
}

#[test]
fn bordered_paragraph_renders_inside_a_container() {
    let mut tb = TestBackend::new(12, 5);
    tb.render(|ui| {
        ui.container()
            .w(8)
            .h(4)
            .ratatui_widget(Paragraph::new("ab\ncd").block(Block::bordered()));
    });
    assert_eq!(tb.line(0), "┌──────┐");
    assert_eq!(tb.line(1), "│ab    │");
    assert_eq!(tb.line(2), "│cd    │");
    assert_eq!(tb.line(3), "└──────┘");
    tb.assert_empty_line(4);
}

/// Render a bordered block in a vertically scrolled viewport of height 2.
fn render_scrolled(offset: usize) -> TestBackend {
    let mut tb = TestBackend::new(10, 3);
    let mut scroll = ScrollState::new();
    scroll.offset = offset;
    // Two frames: the first records content height so the offset is honoured.
    for _ in 0..2 {
        tb.render(|ui| {
            let _ = ui.scrollable(&mut scroll).w(8).h(2).col(|ui| {
                ui.container()
                    .w(8)
                    .h(4)
                    .ratatui_widget(Paragraph::new("ab\ncd").block(Block::bordered()));
            });
        });
    }
    tb
}

#[test]
fn scrolled_region_keeps_logical_geometry_and_is_clipped() {
    let tb = render_scrolled(1);
    assert_eq!(tb.line(0), "│ab    │");
    assert_eq!(tb.line(1), "│cd    │");
    tb.assert_empty_line(2);

    let tb = render_scrolled(2);
    assert_eq!(tb.line(0), "│cd    │");
    assert_eq!(tb.line(1), "└──────┘");
    tb.assert_empty_line(2);
}

#[test]
fn nested_horizontal_and_vertical_clip_crops_the_block() {
    let mut tb = TestBackend::new(6, 3);
    let mut vertical = ScrollState::new();
    vertical.offset = 1;
    let mut horizontal = ScrollState::new();
    horizontal.offset_x = 2;
    for _ in 0..2 {
        tb.render(|ui| {
            let _ = ui.scrollable(&mut vertical).w(4).h(2).col(|ui| {
                let _ = ui.scrollable(&mut horizontal).w(4).h(3).row(|ui| {
                    ui.container()
                        .w(6)
                        .h(3)
                        .ratatui_widget(Paragraph::new("wxyz").block(Block::bordered()));
                });
            });
        });
    }
    // Logical block rows: "┌────┐", "│wxyz│", "└────┘"; visible: rows 1..3,
    // columns 2..6.
    assert_eq!(tb.line(0), "xyz│");
    assert_eq!(tb.line(1), "───┘");
    tb.assert_empty_line(2);
}

#[test]
fn writes_past_the_area_do_not_reach_siblings() {
    let mut tb = TestBackend::new(12, 1);
    tb.render(|ui| {
        let _ = ui.row(|ui| {
            ui.container().w(3).h(1).ratatui(|area, buf| {
                buf.set_string(area.x + 2, area.y, "XYZ", RatStyle::new());
                assert!(buf.cell_mut((area.right(), area.y)).is_none());
            });
            ui.text("rest");
        });
    });
    assert_eq!(tb.line(0), "  Xrest");
}

#[test]
fn wide_cjk_graphemes_keep_slt_continuation_cells() {
    let mut tb = TestBackend::new(8, 1);
    tb.render(|ui| {
        ui.container()
            .w(6)
            .h(1)
            .ratatui_widget(Paragraph::new("한글ab"));
    });
    assert_eq!(tb.line(0), "한글ab");
    let buf = tb.buffer();
    assert_eq!(buf.get(0, 0).symbol.as_str(), "한");
    assert!(buf.get(1, 0).is_continuation());
    assert_eq!(buf.get(2, 0).symbol.as_str(), "글");
    assert!(buf.get(3, 0).is_continuation());
    assert_eq!(buf.get(4, 0).symbol.as_str(), "a");
}

/// Render "한글" (4 logical columns) through a 2-column horizontal viewport.
fn render_wide_scrolled(offset_x: usize) -> TestBackend {
    let mut tb = TestBackend::new(4, 1);
    let mut scroll = ScrollState::new();
    scroll.offset_x = offset_x;
    for _ in 0..2 {
        tb.render(|ui| {
            let _ = ui.scrollable(&mut scroll).w(2).h(1).row(|ui| {
                ui.container()
                    .w(4)
                    .h(1)
                    .ratatui_widget(Paragraph::new("한글"));
            });
        });
    }
    tb
}

#[test]
fn wide_grapheme_cut_by_clip_is_blanked_not_split() {
    // Columns 1..3: the hidden half of "한" and the leading half of "글".
    // The viewport's own scroll arrows sit underneath and must be covered.
    let tb = render_wide_scrolled(1);
    let buf = tb.buffer();
    assert_eq!(buf.get(0, 0).symbol.as_str(), " ");
    assert_eq!(buf.get(1, 0).symbol.as_str(), " ");
    tb.assert_empty_line(0);

    let tb = render_wide_scrolled(2);
    assert_eq!(tb.line(0), "글");
    assert!(tb.buffer().get(1, 0).is_continuation());
}

#[test]
fn parent_background_shows_through_untouched_style() {
    let mut tb = TestBackend::new(6, 1);
    tb.render(|ui| {
        let _ = ui.container().bg(Color::Blue).w(6).h(1).col(|ui| {
            ui.container()
                .w(6)
                .h(1)
                .ratatui_widget(Paragraph::new("hi").style(RatStyle::new().fg(RatColor::Red)));
        });
    });
    let cell = tb.buffer().get(0, 0);
    assert_eq!(cell.symbol.as_str(), "h");
    assert_eq!(cell.style.fg, Some(Color::Red));
    assert_eq!(cell.style.bg, Some(Color::Blue));
    assert_eq!(tb.buffer().get(5, 0).style.bg, Some(Color::Blue));
}

#[test]
fn modifiers_and_hidden_text_map_onto_cells() {
    let mut tb = TestBackend::new(8, 2);
    tb.render(|ui| {
        ui.container().w(8).h(2).ratatui(|area, buf| {
            buf.set_string(
                area.x,
                area.y,
                "bold",
                RatStyle::new().add_modifier(RatModifier::BOLD | RatModifier::RAPID_BLINK),
            );
            buf.set_string(
                area.x,
                area.y + 1,
                "secret",
                RatStyle::new().add_modifier(RatModifier::HIDDEN),
            );
        });
    });
    let bold = tb.buffer().get(0, 0);
    assert_eq!(bold.symbol.as_str(), "b");
    assert!(bold.style.modifiers.contains(Modifiers::BOLD));
    assert!(bold.style.modifiers.contains(Modifiers::BLINK));
    tb.assert_empty_line(1);
}

#[test]
fn skip_cells_are_not_copied() {
    let mut tb = TestBackend::new(4, 1);
    tb.render(|ui| {
        ui.container().w(4).h(1).ratatui(|area, buf| {
            buf.set_string(area.x, area.y, "WXYZ", RatStyle::new());
            if let Some(cell) = buf.cell_mut((area.x + 1, area.y)) {
                cell.set_diff_option(CellDiffOption::Skip);
            }
        });
    });
    assert_eq!(tb.line(0), "W YZ");
}

#[test]
fn scratch_buffer_is_reset_between_frames() {
    let mut tb = TestBackend::new(8, 1);
    tb.render(|ui| {
        ui.container()
            .w(8)
            .h(1)
            .ratatui_widget(Paragraph::new("hello"));
    });
    assert_eq!(tb.line(0), "hello");
    tb.render(|ui| {
        ui.container()
            .w(8)
            .h(1)
            .ratatui_widget(Paragraph::new("hi"));
    });
    assert_eq!(tb.line(0), "hi");
}
