//! End-to-end escape-byte / image-protocol assertions (issue #274).
//!
//! These tests drive the *real* `Terminal` flush pipeline through
//! [`slt::PtyBackend`] into an in-process byte sink, so the actual escape
//! codes that ship to a terminal — SGR runs, OSC 8 hyperlinks, Sixel and
//! Kitty graphics envelopes, color-depth-downsampled SGR — are asserted as
//! whole frames, not just unit-tested in isolation against `Vec<u8>`.
//!
//! This is the byte/protocol regression tier that complements the plain-text
//! `insta` snapshots in `tests/visual_snapshots.rs` (which explicitly cannot
//! observe image/escape output) and the buffer-only `TestBackend`.
//!
//! Gated behind the dev-only `pty-test` feature: `cargo test --features pty-test`.
#![cfg(feature = "pty-test")]

use slt::{Color, ColorDepth, PtyBackend};

/// A styled glyph emits an SGR escape carrying both the foreground color and
/// the bold attribute through the production flush pipeline.
#[test]
fn styled_text_emits_sgr_run() {
    let mut pb = PtyBackend::new(10, 1);
    pb.render(|ui| {
        ui.text("x").fg(Color::Red).bold();
    });
    // CSI introducer present.
    pb.assert_emits("\u{1b}[");
    // Bold attribute: SGR 1.
    pb.assert_emits("\u{1b}[1m");
    // Foreground SGR present. crossterm renders the named `Color::Red`
    // (→ `DarkRed`) through `SetForegroundColor` as the 256-color form
    // `38;5;1` rather than the legacy `31` — this end-to-end test pins the
    // bytes the shipping pipeline actually emits, which the isolated
    // `to_crossterm_color` unit does not reveal.
    pb.assert_emits("\u{1b}[38;5;1m");
    // The glyph itself is printed.
    pb.assert_emits("x");
}

/// A `sixel_image` call emits a `\x1bPq`-wrapped Sixel payload terminated by
/// the String Terminator. Sixel support is normally terminal-detected; force
/// it on so the headless harness exercises the real encode + flush path.
#[test]
fn sixel_image_emits_envelope() {
    // SAFETY-equivalent note: `set_var` is process-global. This test owns the
    // var for its duration and restores it; it asserts on the forced path.
    let prev = std::env::var("SLT_FORCE_SIXEL").ok();
    std::env::set_var("SLT_FORCE_SIXEL", "1");

    let mut pb = PtyBackend::new(20, 2);
    // 2x2 red square (RGBA: 4 pixels x 4 bytes).
    let rgba = [255u8, 0, 0, 255].repeat(4);
    pb.render(|ui| {
        let _ = ui.sixel_image(&rgba, 2, 2, 20, 2);
    });
    pb.assert_emits("\u{1b}Pq");
    pb.assert_emits("\u{1b}\\");

    match prev {
        Some(v) => std::env::set_var("SLT_FORCE_SIXEL", v),
        None => std::env::remove_var("SLT_FORCE_SIXEL"),
    }
}

/// A `kitty_image` call emits the Kitty graphics APC introducer `\x1b_Ga=`.
#[test]
fn kitty_image_emits_apc() {
    let mut pb = PtyBackend::new(20, 4);
    // 2x2 RGBA image.
    let rgba = [0u8, 128, 255, 255].repeat(4);
    pb.render(|ui| {
        let _ = ui.kitty_image(&rgba, 2, 2, 4, 2);
    });
    pb.assert_emits("\u{1b}_Ga=");
}

/// A hyperlinked text run emits the OSC 8 open sequence `\x1b]8;;<url>`.
#[test]
fn hyperlink_emits_osc8() {
    let mut pb = PtyBackend::new(20, 1);
    pb.render(|ui| {
        ui.link("Docs", "https://docs.rs");
    });
    pb.assert_emits("\u{1b}]8;;");
    pb.assert_emits("https://docs.rs");
}

/// Downsampling the color depth changes the emitted SGR bytes end-to-end:
/// truecolor emits `38;2;r;g;b`, 256-color emits a downsampled `38;5;n`. This
/// proves the depth path is exercised through the real flush, not just in the
/// isolated `to_crossterm_color` unit.
#[test]
fn color_depth_downgrade_changes_sgr() {
    let rgb = Color::Rgb(200, 50, 50);

    let mut truecolor = PtyBackend::new(10, 1).with_color_depth(ColorDepth::TrueColor);
    truecolor.render(|ui| {
        ui.text("x").fg(rgb);
    });
    let true_bytes = truecolor.last_raw().to_vec();

    let mut eight_bit = PtyBackend::new(10, 1).with_color_depth(ColorDepth::EightBit);
    eight_bit.render(|ui| {
        ui.text("x").fg(rgb);
    });
    let eight_bytes = eight_bit.last_raw().to_vec();

    assert_ne!(
        true_bytes, eight_bytes,
        "truecolor and 256-color frames must emit different SGR bytes"
    );
    // The truecolor frame must carry a 24-bit foreground SGR.
    truecolor.assert_emits("\u{1b}[38;2;200;50;50m");
    // The 256-color frame must NOT carry the 24-bit form.
    eight_bit.assert_not_emits("\u{1b}[38;2;200;50;50m");
}

/// Rendering the same frame twice produces byte-identical output — the harness
/// is deterministic and CI-reproducible (mirrors
/// `snapshot_format_stability::stability_deterministic_repeat`).
#[test]
fn pty_backend_is_deterministic() {
    let render_once = || {
        let mut pb = PtyBackend::new(20, 3);
        pb.render(|ui| {
            ui.text("deterministic").fg(Color::Green).bold();
        });
        pb.last_raw().to_vec()
    };
    assert_eq!(render_once(), render_once());
}

/// Two renders on the same backend each capture their own frame.
#[test]
fn frames_raw_accumulates_each_render() {
    let mut pb = PtyBackend::new(10, 1);
    pb.render(|ui| {
        ui.text("a");
    });
    pb.render(|ui| {
        ui.text("b");
    });
    assert_eq!(pb.frames_raw().count(), 2);
}

/// `assert_not_emits` passes when the needle is genuinely absent.
#[test]
fn assert_not_emits_passes_when_absent() {
    let mut pb = PtyBackend::new(10, 1);
    pb.render(|ui| {
        ui.text("plain");
    });
    // No Sixel envelope in a plain-text frame.
    pb.assert_not_emits("\u{1b}Pq");
}

/// `assert_emits` panics with a diagnostic dump when the needle is missing.
#[test]
#[should_panic(expected = "does not emit")]
fn assert_emits_panics_when_missing() {
    let mut pb = PtyBackend::new(10, 1);
    pb.render(|ui| {
        ui.text("plain");
    });
    pb.assert_emits("\u{1b}Pq");
}
