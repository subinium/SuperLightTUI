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

use slt::{Color, ColorDepth, PtyBackend, ScreenState};
use std::sync::Mutex;

static ENV_GUARD: Mutex<()> = Mutex::new(());

#[allow(unsafe_code)]
fn with_env_vars<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
    let _g = ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
    let previous: Vec<(&str, Option<String>)> = vars
        .iter()
        .map(|(name, _)| (*name, std::env::var(name).ok()))
        .collect();

    // SAFETY (edition 2024): env mutation is process-global. ENV_GUARD
    // serializes these PTY tests while the override is active.
    unsafe {
        for (name, value) in vars {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }

    f();

    unsafe {
        for (name, value) in previous {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

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
    // Foreground SGR present. SLT emits color SGR directly so environment
    // variables such as NO_COLOR cannot override an explicit ColorDepth.
    pb.assert_emits("\u{1b}[31m");
    // The glyph itself is printed.
    pb.assert_emits("x");
}

/// A `sixel_image` call emits a `\x1bPq`-wrapped Sixel payload terminated by
/// the String Terminator. Sixel support is normally terminal-detected; force
/// it on so the headless harness exercises the real encode + flush path.
#[test]
fn sixel_image_emits_envelope() {
    with_env_vars(&[("SLT_FORCE_SIXEL", Some("1"))], || {
        let mut pb = PtyBackend::new(20, 2);
        // 2x2 red square (RGBA: 4 pixels x 4 bytes).
        let rgba = [255u8, 0, 0, 255].repeat(4);
        pb.render(|ui| {
            let _ = ui.sixel_image(&rgba, 2, 2, 20, 2);
        });
        pb.assert_emits("\u{1b}Pq");
        pb.assert_emits("\u{1b}\\");
    });
}

/// A `kitty_image` call emits the Kitty graphics APC introducer `\x1b_Ga=`.
#[test]
fn kitty_image_emits_apc() {
    with_env_vars(&[("SLT_FORCE_KITTY", Some("1"))], || {
        let mut pb = PtyBackend::new(20, 4);
        // 2x2 RGBA image.
        let rgba = [0u8, 128, 255, 255].repeat(4);
        pb.render(|ui| {
            let _ = ui.kitty_image(&rgba, 2, 2, 4, 2);
        });
        pb.assert_emits("\u{1b}_Ga=");
    });
}

/// tmux/screen-like environments must not leak image protocol bytes unless
/// explicitly forced. TERM_PROGRAM may describe the outer capable terminal.
#[test]
fn image_protocols_do_not_emit_inside_tmux_without_force() {
    with_env_vars(
        &[
            ("TERM", Some("screen-256color")),
            ("TERM_PROGRAM", Some("WezTerm")),
            ("TMUX", Some("/tmp/tmux-1000/default,1,0")),
            ("SLT_FORCE_KITTY", None),
            ("SLT_FORCE_SIXEL", None),
            ("SLT_FORCE_ITERM", None),
        ],
        || {
            let rgba = [0u8, 128, 255, 255].repeat(4);
            let png = [0x89u8, b'P', b'N', b'G'];
            let mut pb = PtyBackend::new(24, 8);
            pb.render(|ui| {
                let _ = ui.kitty_image(&rgba, 2, 2, 4, 2);
                let _ = ui.sixel_image(&rgba, 2, 2, 4, 2);
                let _ = ui.iterm_image(&png, 4, 2);
            });

            pb.assert_not_emits("\u{1b}_Ga=");
            pb.assert_not_emits("\u{1b}Pq");
            pb.assert_not_emits("\u{1b}]1337;File=");
        },
    );
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

/// Explicit color depth is authoritative even when the environment requests no
/// color. NO_COLOR is honored only by ColorDepth::detect().
#[test]
fn explicit_truecolor_ignores_no_color_environment() {
    with_env_vars(&[("NO_COLOR", Some("1")), ("TERM", Some("dumb"))], || {
        let rgb = Color::Rgb(200, 50, 50);

        let mut truecolor = PtyBackend::new(10, 1).with_color_depth(ColorDepth::TrueColor);
        truecolor.render(|ui| {
            ui.text("x").fg(rgb);
        });
        truecolor.assert_emits("\u{1b}[38;2;200;50;50m");

        let mut no_color = PtyBackend::new(10, 1).with_color_depth(ColorDepth::NoColor);
        no_color.render(|ui| {
            ui.text("x").fg(rgb);
        });
        no_color.assert_not_emits("\u{1b}[38;2;200;50;50m");
        no_color.assert_not_emits("\u{1b}[38;5;");
    });
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

/// Screen navigation changes state immediately without flushing source and
/// destination views into the same terminal frame.
#[test]
fn screen_navigation_emits_one_view_per_frame() {
    let mut pb = PtyBackend::new(24, 2);
    let mut screens = ScreenState::new("home");

    pb.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            ui.text("Home Screen");
            ui.push_screen("settings");
        });
        ui.screen("settings", &mut screens, |ui| {
            ui.text("Settings Screen");
        });
    });

    assert_eq!(screens.current(), "settings");
    pb.assert_emits("Home Screen");
    pb.assert_not_emits("Settings Screen");

    pb.render(|ui| {
        ui.screen("home", &mut screens, |ui| {
            ui.text("Home Screen");
        });
        ui.screen("settings", &mut screens, |ui| {
            ui.text("Settings Screen");
        });
    });

    pb.assert_emits("Settings Screen");
    pb.assert_not_emits("Home Screen");
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
