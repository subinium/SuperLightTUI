//! Terminal graphics-protocol detection tests (Kitty, Sixel, iTerm2)
//! against the environment fallback policy in `terminal::capabilities`.

#[cfg(all(test, feature = "crossterm"))]
fn terminal_supports_kitty() -> bool {
    crate::terminal::GraphicsEmissionSupport::for_context(
        crate::terminal::Capabilities::default(),
        true,
    )
    .should_emit_kitty()
}

#[cfg(all(test, feature = "crossterm"))]
fn terminal_supports_sixel() -> bool {
    crate::terminal::GraphicsEmissionSupport::for_context(
        crate::terminal::Capabilities::default(),
        true,
    )
    .should_emit_sprixel(crate::terminal::SprixelProtocol::Sixel)
}

/// Env-fallback detection for the iTerm2 OSC 1337 inline-image protocol
/// (issue #265).
///
/// Consulted as the env-fallback when the runtime capability probe returned
/// unknown, mirroring [`terminal_supports_sixel`]. Matches the `TERM_PROGRAM`
/// identities of terminals that implement OSC 1337 inline images: iTerm2
/// itself, WezTerm (iTerm2-compat mode), Tabby, and mintty. `SLT_FORCE_ITERM=1`
/// forces a positive. Never fires on `xterm-256color`, matching the sixel
/// regression-test parity.
#[cfg(all(test, feature = "crossterm"))]
fn terminal_supports_iterm() -> bool {
    crate::terminal::GraphicsEmissionSupport::for_context(
        crate::terminal::Capabilities::default(),
        true,
    )
    .should_emit_sprixel(crate::terminal::SprixelProtocol::Iterm2)
}

#[cfg(all(test, feature = "crossterm"))]
#[derive(Default)]
struct GraphicsEnv<'a> {
    term: Option<&'a str>,
    term_program: Option<&'a str>,
    force_sixel: bool,
    force_iterm: bool,
    force_kitty: bool,
    tmux: bool,
    sty: bool,
}

#[cfg(all(test, feature = "crossterm"))]
static GRAPHICS_ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(test, feature = "crossterm"))]
#[expect(
    unsafe_code,
    reason = "edition 2024 env mutation; serialised by GRAPHICS_ENV_GUARD"
)]
fn with_graphics_env<F: FnOnce()>(env: GraphicsEnv<'_>, f: F) {
    let _g = GRAPHICS_ENV_GUARD.lock().unwrap_or_else(|e| e.into_inner());
    let prev_term = std::env::var("TERM").ok();
    let prev_program = std::env::var("TERM_PROGRAM").ok();
    let prev_sixel = std::env::var("SLT_FORCE_SIXEL").ok();
    let prev_iterm = std::env::var("SLT_FORCE_ITERM").ok();
    let prev_kitty = std::env::var("SLT_FORCE_KITTY").ok();
    let prev_tmux = std::env::var("TMUX").ok();
    let prev_sty = std::env::var("STY").ok();

    // SAFETY (edition 2024): set_var/remove_var are unsafe because env
    // mutation races concurrent reads. GRAPHICS_ENV_GUARD serializes these
    // tests so no other detection test observes a torn update.
    unsafe {
        match env.term {
            Some(v) => std::env::set_var("TERM", v),
            None => std::env::remove_var("TERM"),
        }
        match env.term_program {
            Some(v) => std::env::set_var("TERM_PROGRAM", v),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
        if env.force_sixel {
            std::env::set_var("SLT_FORCE_SIXEL", "1");
        } else {
            std::env::remove_var("SLT_FORCE_SIXEL");
        }
        if env.force_iterm {
            std::env::set_var("SLT_FORCE_ITERM", "1");
        } else {
            std::env::remove_var("SLT_FORCE_ITERM");
        }
        if env.force_kitty {
            std::env::set_var("SLT_FORCE_KITTY", "1");
        } else {
            std::env::remove_var("SLT_FORCE_KITTY");
        }
        if env.tmux {
            std::env::set_var("TMUX", "/tmp/tmux-1000/default,1,0");
        } else {
            std::env::remove_var("TMUX");
        }
        if env.sty {
            std::env::set_var("STY", "1234.pts-0.host");
        } else {
            std::env::remove_var("STY");
        }
    }

    f();

    unsafe {
        match prev_term {
            Some(v) => std::env::set_var("TERM", v),
            None => std::env::remove_var("TERM"),
        }
        match prev_program {
            Some(v) => std::env::set_var("TERM_PROGRAM", v),
            None => std::env::remove_var("TERM_PROGRAM"),
        }
        match prev_sixel {
            Some(v) => std::env::set_var("SLT_FORCE_SIXEL", v),
            None => std::env::remove_var("SLT_FORCE_SIXEL"),
        }
        match prev_iterm {
            Some(v) => std::env::set_var("SLT_FORCE_ITERM", v),
            None => std::env::remove_var("SLT_FORCE_ITERM"),
        }
        match prev_kitty {
            Some(v) => std::env::set_var("SLT_FORCE_KITTY", v),
            None => std::env::remove_var("SLT_FORCE_KITTY"),
        }
        match prev_tmux {
            Some(v) => std::env::set_var("TMUX", v),
            None => std::env::remove_var("TMUX"),
        }
        match prev_sty {
            Some(v) => std::env::set_var("STY", v),
            None => std::env::remove_var("STY"),
        }
    }
}

#[cfg(all(test, feature = "crossterm"))]
mod kitty_detection_tests {
    use super::{GraphicsEnv, terminal_supports_kitty, with_graphics_env};

    #[test]
    fn kitty_xterm_kitty_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-kitty"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_kitty());
            },
        );
    }

    #[test]
    fn kitty_ghostty_detected_via_term_program() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("ghostty"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_kitty());
            },
        );
    }

    #[test]
    fn kitty_xterm_256color_no_false_positive() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_kitty());
            },
        );
    }

    #[test]
    fn kitty_tmux_blocks_env_fallback() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("wezterm"),
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_kitty());
            },
        );
    }

    #[test]
    fn kitty_screen_term_blocks_env_fallback() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("screen-256color"),
                term_program: Some("ghostty"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_kitty());
            },
        );
    }

    #[test]
    fn kitty_force_env_overrides_tmux() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("screen-256color"),
                term_program: Some("wezterm"),
                force_kitty: true,
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_kitty());
            },
        );
    }
}

#[cfg(all(test, feature = "crossterm"))]
mod sixel_detection_tests {
    use super::{GraphicsEnv, terminal_supports_sixel, with_graphics_env};

    #[test]
    fn sixel_xterm_256color_no_false_positive() {
        // Regression: `term.contains("xterm")` previously matched
        // `xterm-256color` and printed raw escape sequences to screen on
        // macOS Terminal.app, VS Code, and most SSH clients.
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_mlterm_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("mlterm"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_foot_detected_via_term() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("foot"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_foot_detected_via_term_program() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("foot"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_force_env_overrides_negative_term() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                force_sixel: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_substring_match_catches_custom_builds() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("custom-with-sixel"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_wezterm_detected_via_term_program() {
        // Issue #264: WezTerm reports `TERM=xterm-256color` but is a capable
        // image host; it must no longer fall through to `[sixel unsupported]`.
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("wezterm"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_ghostty_detected_via_term_program() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("ghostty"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_mlterm_still_detected_via_term_program() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("mlterm"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_tmux_blocks_env_fallback() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("ghostty"),
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_sixel());
            },
        );
    }

    #[test]
    fn sixel_force_env_overrides_tmux() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("screen-256color"),
                term_program: Some("wezterm"),
                force_sixel: true,
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_sixel());
            },
        );
    }
}

#[cfg(all(test, feature = "crossterm"))]
mod iterm_detection_tests {
    use super::{GraphicsEnv, terminal_supports_iterm, with_graphics_env};

    #[test]
    fn iterm_xterm_256color_no_false_positive() {
        // Parity with `sixel_xterm_256color_no_false_positive`: a plain xterm
        // must never be mistaken for an OSC 1337 host.
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_app_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("iTerm.app"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_wezterm_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("WezTerm"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_tabby_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("Tabby"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_mintty_detected() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("mintty"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_force_env_overrides_negative_term() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                force_iterm: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_unknown_term_program_negative() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("Apple_Terminal"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_tmux_blocks_env_fallback() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("xterm-256color"),
                term_program: Some("iTerm.app"),
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_screen_term_blocks_env_fallback() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("screen.xterm-256color"),
                term_program: Some("Tabby"),
                ..GraphicsEnv::default()
            },
            || {
                assert!(!terminal_supports_iterm());
            },
        );
    }

    #[test]
    fn iterm_force_env_overrides_tmux() {
        with_graphics_env(
            GraphicsEnv {
                term: Some("screen-256color"),
                term_program: Some("iTerm.app"),
                force_iterm: true,
                tmux: true,
                ..GraphicsEnv::default()
            },
            || {
                assert!(terminal_supports_iterm());
            },
        );
    }
}
