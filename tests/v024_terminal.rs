#![cfg(feature = "pty-test")]

use slt::{Buffer, ColorDepth, PtyBackend, Rect, Style, UnderlineStyle};
use std::process::Command;

#[test]
fn graphics_policy_has_visible_fallback_in_isolated_environments() {
    for (mode, extra) in [
        ("fallback", vec![]),
        ("kitty", vec![("TERM", "xterm-kitty")]),
        (
            "fallback",
            vec![("TERM", "xterm-kitty"), ("SLT_DISABLE_KITTY", "1")],
        ),
        (
            "fallback",
            vec![
                ("TERM", "xterm-kitty"),
                ("SLT_FORCE_KITTY", "1"),
                ("SLT_DISABLE_KITTY", "1"),
            ],
        ),
        ("fallback", vec![("TERM", "xterm-kitty"), ("ZELLIJ", "1")]),
        (
            "fallback",
            vec![("TERM_PROGRAM", "kitty"), ("SSH_CONNECTION", "remote")],
        ),
    ] {
        let mut child = Command::new(std::env::current_exe().expect("test executable"));
        child.args(["--exact", "graphics_policy_child", "--nocapture"]);
        for name in [
            "TMUX",
            "STY",
            "ZELLIJ",
            "ZELLIJ_SESSION_NAME",
            "TERM_PROGRAM",
            "SSH_CONNECTION",
            "SSH_TTY",
            "MOSH_IP",
            "SLT_FORCE_KITTY",
            "SLT_DISABLE_KITTY",
            "SLT_FORCE_SIXEL",
            "SLT_FORCE_ITERM",
        ] {
            child.env_remove(name);
        }
        child
            .env("TERM", "xterm-256color")
            .env("SLT_DISABLE_TERMINAL_QUERIES", "1")
            .env("SLT_V024_GRAPHICS_CHILD", mode)
            .envs(extra);
        let output = child.output().expect("graphics subprocess");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}

#[test]
fn graphics_policy_child() {
    let Ok(mode) = std::env::var("SLT_V024_GRAPHICS_CHILD") else {
        return;
    };
    let mut backend = PtyBackend::new(8, 3);
    let rgba = [255, 0, 0, 255].repeat(4);
    backend.render(|ui| {
        let _ = ui.kitty_image(&rgba, 2, 2, 4, 2);
    });
    let raw = String::from_utf8_lossy(backend.last_raw());
    if mode == "kitty" {
        assert!(raw.contains("\x1b_Ga=t,"), "{raw:?}");
    } else {
        assert!(!raw.contains("\x1b_G"), "{raw:?}");
        assert!(
            raw.contains('\u{2580}'),
            "must draw visible fallback: {raw:?}"
        );
    }
}

#[test]
fn extended_underline_transition_leaves_plain_following_text() {
    for underline in [
        UnderlineStyle::Double,
        UnderlineStyle::Curly,
        UnderlineStyle::Dotted,
        UnderlineStyle::Dashed,
    ] {
        let area = Rect::new(0, 0, 4, 1);
        let mut current = Buffer::empty(area);
        let mut previous = Buffer::empty(area);
        current.set_string(0, 0, "A", Style::new().underline_style(underline));
        current.set_string(1, 0, "B", Style::new());
        let mut raw = Vec::new();
        slt::__bench_flush_buffer_diff_mut(
            &mut raw,
            &mut current,
            &mut previous,
            ColorDepth::TrueColor,
        )
        .unwrap();
        let text = String::from_utf8(raw).unwrap();
        assert!(text.contains("A\x1b[24mB"), "{text:?}");
    }
}
