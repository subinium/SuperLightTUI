//! Lock the format of [`Buffer::snapshot_format`] (#231).
//!
//! These tests pin the byte-exact output of the snapshot serializer for a
//! curated set of hand-crafted scenarios. They are intentionally brittle —
//! changing the format intentionally requires updating these strings, and
//! changing it accidentally fails CI before the change can land.
//!
//! See the rustdoc on [`slt::Buffer::snapshot_format`] for the format spec
//! and stability guarantees.

use slt::Rect;
use slt::buffer::Buffer;
use slt::style::{Color, Modifiers, Style};

/// Default-style buffer renders trailing spaces verbatim, no markers.
#[test]
fn stability_default_only() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 6, 2));
    buf.set_string(0, 0, "abc", Style::new());
    buf.set_string(0, 1, "xy", Style::new());
    let snap = buf.snapshot_format();
    assert_eq!(snap, "abc   \nxy    ");
}

/// Mixed-color row: red prefix, default gap, blue suffix.
#[test]
fn stability_color_runs() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 8, 1));
    buf.set_string(0, 0, "ab", Style::new().fg(Color::Red));
    // gap (cells 2..4 are default)
    buf.set_string(4, 0, "cd", Style::new().fg(Color::Blue));
    let snap = buf.snapshot_format();
    assert_eq!(snap, "[fg=red]\"ab\"[/]  [fg=blue]\"cd\"[/]  ");
}

/// Bold + italic + underline combination renders modifiers in canonical order.
#[test]
fn stability_multiple_modifiers_canonical_order() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
    let style = Style::new().underline().italic().bold();
    buf.set_string(0, 0, "abcd", style);
    let snap = buf.snapshot_format();
    assert_eq!(snap, "[bold,italic,underline]\"abcd\"[/]");
}

/// RGB foreground with a background color renders fg= then bg= in that order.
#[test]
fn stability_fg_and_bg_order() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
    let style = Style::new()
        .fg(Color::Rgb(255, 0, 171))
        .bg(Color::Rgb(0, 0, 0));
    buf.set_string(0, 0, "xyz", style);
    let snap = buf.snapshot_format();
    assert_eq!(snap, "[fg=#ff00ab,bg=#000000]\"xyz\"[/]");
}

/// Indexed palette colors emit `idx<N>` short codes.
#[test]
fn stability_indexed_palette() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
    buf.set_string(0, 0, "ok", Style::new().fg(Color::Indexed(208)));
    let snap = buf.snapshot_format();
    assert_eq!(snap, "[fg=idx208]\"ok\"[/]");
}

/// Embedded backslash and quote are escaped in the styled segment.
#[test]
fn stability_escapes_quote_and_backslash() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
    buf.set_string(0, 0, "a\"b\\", Style::new().bold());
    let snap = buf.snapshot_format();
    assert_eq!(snap, "[bold]\"a\\\"b\\\\\"[/]");
}

/// Determinism: two calls produce byte-equal output for the same buffer.
#[test]
fn stability_deterministic_repeat() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 6, 2));
    buf.set_string(0, 0, "hi", Style::new().fg(Color::Cyan).bold());
    buf.set_string(0, 1, "lo", Style::new().bg(Color::Yellow));
    let a = buf.snapshot_format();
    let b = buf.snapshot_format();
    assert_eq!(a, b);
}

/// Empty cells (default Cell with empty symbol after wide-char blanking)
/// render as a single space — they don't break the snapshot.
#[test]
fn stability_wide_char_trailing_blanked() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 4, 1));
    // Wide char "世" occupies cols 0-1; the trailing cell at col 1 is blanked
    // (empty symbol). snapshot_format must treat that as a single space.
    buf.set_string(0, 0, "世", Style::new());
    let snap = buf.snapshot_format();
    // Expected: "世" (col 0) + " " (col 1 blanked) + "  " (cols 2-3 default).
    // Total: 1 wide char + 3 spaces.
    assert_eq!(snap, "世   ");
}

/// Plain explicit Style construction with all-default fields == no markers.
#[test]
fn stability_explicit_default_style_no_markers() {
    let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
    let style = Style {
        fg: None,
        bg: None,
        modifiers: Modifiers::NONE,
        ..Style::new()
    };
    buf.set_string(0, 0, "abc", style);
    let snap = buf.snapshot_format();
    assert_eq!(snap, "abc");
}
