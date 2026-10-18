//! Capability model, blitter ladder and host / multiplexer policy tests.

use super::*;
use crate::context::terminal_is_remote_env;

#[test]
fn graphics_emission_requires_protocol_support() {
    let unsupported = GraphicsEmissionSupport {
        real_terminal: true,
        capabilities: Capabilities::default(),
        force_kitty: false,
        force_sixel: false,
        force_iterm: false,
    };
    assert!(!unsupported.should_emit_kitty());
    assert!(!unsupported.should_emit_sprixel(SprixelProtocol::Sixel));
    assert!(!unsupported.should_emit_sprixel(SprixelProtocol::Iterm2));
    assert!(!unsupported.should_emit_sprixel(SprixelProtocol::Unknown));

    let sixel = GraphicsEmissionSupport {
        capabilities: Capabilities {
            sixel: true,
            ..Capabilities::default()
        },
        ..unsupported
    };
    assert!(sixel.should_emit_sprixel(SprixelProtocol::Sixel));

    let forced = GraphicsEmissionSupport {
        force_kitty: true,
        force_iterm: true,
        ..unsupported
    };
    assert!(forced.should_emit_kitty());
    assert!(forced.should_emit_sprixel(SprixelProtocol::Iterm2));
}

// ---- Capability probe / blitter ladder (issue #264) ----

#[test]
fn blitter_support_default_is_conservative() {
    let b = BlitterSupport::default();
    assert!(b.half);
    assert!(b.quad);
    assert!(!b.sextant);
}

#[test]
fn capabilities_default_is_all_false_but_half_block() {
    let c = Capabilities::default();
    assert!(!c.truecolor);
    assert!(!c.sixel);
    assert!(!c.iterm2);
    assert!(!c.kitty_graphics);
    assert!(!c.kitty_keyboard);
    assert!(!c.sync_output);
    // With nothing negotiated the ladder must still resolve to half-block.
    assert_eq!(c.best_blitter(), Blitter::HalfBlock);
}

#[test]
fn best_blitter_ladder_table() {
    let kitty = Capabilities {
        kitty_graphics: true,
        ..Default::default()
    };
    assert_eq!(kitty.best_blitter(), Blitter::Kitty);

    let sixel = Capabilities {
        sixel: true,
        ..Default::default()
    };
    assert_eq!(sixel.best_blitter(), Blitter::Sixel);

    let iterm2 = Capabilities {
        iterm2: true,
        ..Default::default()
    };
    assert_eq!(iterm2.best_blitter(), Blitter::Iterm2);

    // iTerm2 sits below Sixel: a host advertising both prefers Sixel.
    let sixel_and_iterm2 = Capabilities {
        sixel: true,
        iterm2: true,
        ..Default::default()
    };
    assert_eq!(sixel_and_iterm2.best_blitter(), Blitter::Sixel);

    let sextant = Capabilities {
        blitters: BlitterSupport {
            sextant: true,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(sextant.best_blitter(), Blitter::Sextant);

    assert_eq!(Capabilities::default().best_blitter(), Blitter::HalfBlock);
}

#[test]
fn best_blitter_precedence_kitty_over_everything() {
    let all = Capabilities {
        kitty_graphics: true,
        sixel: true,
        blitters: BlitterSupport {
            sextant: true,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(all.best_blitter(), Blitter::Kitty);

    let sixel_and_sextant = Capabilities {
        sixel: true,
        blitters: BlitterSupport {
            sextant: true,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(sixel_and_sextant.best_blitter(), Blitter::Sixel);
}

#[test]
fn best_blitter_never_picks_unsupported_protocol() {
    // Exhaustive sweep over field combinations: the resolver must never
    // return Kitty without kitty_graphics, nor Sixel without sixel, etc.
    for kitty in [false, true] {
        for sixel in [false, true] {
            for iterm2 in [false, true] {
                for sextant in [false, true] {
                    let caps = Capabilities {
                        kitty_graphics: kitty,
                        sixel,
                        iterm2,
                        blitters: BlitterSupport {
                            sextant,
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    match caps.best_blitter() {
                        Blitter::Kitty => assert!(kitty),
                        Blitter::Sixel => assert!(sixel && !kitty),
                        Blitter::Iterm2 => assert!(iterm2 && !sixel && !kitty),
                        Blitter::Sextant => {
                            assert!(sextant && !iterm2 && !sixel && !kitty)
                        }
                        Blitter::HalfBlock => {
                            assert!(!kitty && !sixel && !iterm2 && !sextant)
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn terminal_query_guard_rejects_unsafe_hosts() {
    assert!(terminal_query_allowed(
        true,
        true,
        "xterm-256color",
        None,
        false,
        false
    ));
    assert!(!terminal_query_allowed(
        true,
        false,
        "xterm-256color",
        None,
        false,
        false
    ));
    assert!(!terminal_query_allowed(
        false,
        true,
        "xterm-256color",
        None,
        false,
        false
    ));
    assert!(!terminal_query_allowed(
        true, true, "dumb", None, false, false
    ));
    assert!(!terminal_query_allowed(
        true,
        true,
        "screen-256color",
        Some(MultiplexerKind::Screen),
        false,
        false
    ));
    assert!(!terminal_query_allowed(true, true, "", None, false, false));
}

#[test]
fn terminal_query_guard_honors_force_and_disable_precedence() {
    assert!(terminal_query_allowed(
        true,
        true,
        "dumb",
        Some(MultiplexerKind::Tmux),
        true,
        false
    ));
    assert!(!terminal_query_allowed(
        true,
        true,
        "xterm-kitty",
        None,
        true,
        true
    ));
}

#[test]
fn automatic_query_hosts_require_a_real_terminal_identity() {
    assert!(!terminal_query_host_is_identified_env(
        "xterm-256color",
        false,
        false
    ));
    assert!(terminal_query_host_is_identified_env(
        "xterm-256color",
        true,
        false
    ));
    assert!(terminal_query_host_is_identified_env(
        "xterm-kitty",
        false,
        false
    ));
    assert!(terminal_query_host_is_identified_env("foot", false, false));
    assert!(!terminal_query_host_is_identified_env(
        "xterm-256color",
        true,
        true
    ));
}

#[test]
fn remote_env_does_not_treat_inherited_outer_identity_as_endpoint_proof() {
    assert!(terminal_is_remote_env(true, false, false));
    assert!(terminal_is_remote_env(false, true, false));
    assert!(terminal_is_remote_env(false, false, true));
    assert!(!terminal_is_remote_env(false, false, false));

    assert!(!term_is_kitty_graphics_host_env(
        "xterm-256color",
        "wezterm",
        None,
        true,
        false
    ));
    assert!(term_is_kitty_graphics_host_env(
        "xterm-kitty",
        "wezterm",
        None,
        true,
        false
    ));
    assert!(!term_is_sixel_host_env(
        "xterm-256color",
        "ghostty",
        None,
        true,
        false
    ));
    assert!(!term_is_iterm_host_env("iterm.app", None, true, false));
    assert!(term_is_iterm_host_env("iterm.app", None, true, true));
}

#[test]
fn terminal_multiplexer_detection_is_conservative() {
    assert_eq!(
        terminal_multiplexer_env("tmux-256color", false, false, false, false),
        Some(MultiplexerKind::Tmux)
    );
    assert_eq!(
        terminal_multiplexer_env("screen-256color", false, false, false, false),
        Some(MultiplexerKind::Screen)
    );
    assert_eq!(
        terminal_multiplexer_env("xterm-256color", false, false, true, false),
        Some(MultiplexerKind::Zellij)
    );
    assert_eq!(
        terminal_multiplexer_env("xterm-256color", false, false, false, true),
        Some(MultiplexerKind::Zellij)
    );
    assert_eq!(
        terminal_multiplexer_env("xterm-kitty", false, false, false, false),
        None
    );
    assert_eq!(
        terminal_multiplexer_env("tmux-256color", true, false, true, true),
        Some(MultiplexerKind::Tmux),
        "the innermost TERM identity wins over inherited variables"
    );
}

#[test]
fn multiplexer_protocol_policy_is_explicit_and_disable_wins_force() {
    for multiplexer in [MultiplexerKind::Tmux, MultiplexerKind::Screen] {
        for protocol in [
            TerminalProtocol::Queries,
            TerminalProtocol::SynchronizedOutput,
            TerminalProtocol::KittyGraphics,
            TerminalProtocol::Sixel,
            TerminalProtocol::Iterm2,
            TerminalProtocol::KittyKeyboard,
        ] {
            assert!(!terminal_protocol_allowed(
                Some(multiplexer),
                protocol,
                false,
                false
            ));
            assert!(terminal_protocol_allowed(
                Some(multiplexer),
                protocol,
                true,
                false
            ));
            assert!(!terminal_protocol_allowed(
                Some(multiplexer),
                protocol,
                true,
                true
            ));
        }
    }

    let zellij = Some(MultiplexerKind::Zellij);
    assert!(terminal_protocol_allowed(
        zellij,
        TerminalProtocol::Sixel,
        false,
        false
    ));
    assert!(terminal_protocol_allowed(
        zellij,
        TerminalProtocol::KittyKeyboard,
        false,
        false
    ));
    for protocol in [
        TerminalProtocol::Queries,
        TerminalProtocol::SynchronizedOutput,
        TerminalProtocol::KittyGraphics,
        TerminalProtocol::Iterm2,
    ] {
        assert!(!terminal_protocol_allowed(zellij, protocol, false, false));
    }
}

#[test]
fn kitty_env_fallback_is_blocked_inside_multiplexer_unless_forced() {
    assert!(term_is_kitty_graphics_host_env(
        "xterm-kitty",
        "",
        None,
        false,
        false
    ));
    assert!(term_is_kitty_graphics_host_env(
        "xterm-256color",
        "wezterm",
        None,
        false,
        false
    ));
    assert!(!term_is_kitty_graphics_host_env(
        "xterm-kitty",
        "wezterm",
        Some(MultiplexerKind::Tmux),
        false,
        false
    ));
    assert!(term_is_kitty_graphics_host_env(
        "xterm-256color",
        "",
        Some(MultiplexerKind::Tmux),
        false,
        true
    ));
}

#[test]
fn iterm_env_fallback_is_blocked_inside_multiplexer_unless_forced() {
    assert!(term_is_iterm_host_env("iterm.app", None, false, false));
    assert!(term_is_iterm_host_env("wezterm", None, false, false));
    assert!(!term_is_iterm_host_env(
        "wezterm",
        Some(MultiplexerKind::Tmux),
        false,
        false
    ));
    assert!(term_is_iterm_host_env(
        "xterm",
        Some(MultiplexerKind::Tmux),
        false,
        true
    ));
}

#[test]
fn zellij_uses_protocol_specific_graphics_policy() {
    let zellij = Some(MultiplexerKind::Zellij);
    assert!(term_is_sixel_host_env(
        "xterm-256color",
        "wezterm",
        zellij,
        false,
        false
    ));
    assert!(!term_is_kitty_graphics_host_env(
        "xterm-kitty",
        "wezterm",
        zellij,
        false,
        false
    ));
    assert!(!term_is_iterm_host_env("iterm.app", zellij, false, false));
    assert!(kitty_keyboard_allowed_env(zellij, false, false));
    assert!(!synchronized_update_allowed(zellij, false, false, false));
    assert!(synchronized_update_allowed(zellij, true, false, false));

    for multiplexer in [MultiplexerKind::Tmux, MultiplexerKind::Screen] {
        assert!(!kitty_keyboard_allowed_env(Some(multiplexer), false, false));
        assert!(kitty_keyboard_allowed_env(Some(multiplexer), true, false));
        assert!(!synchronized_update_allowed(
            Some(multiplexer),
            false,
            false,
            false
        ));
    }
}

#[test]
fn graphics_support_blocks_kitty_without_ack_or_force() {
    let support = GraphicsEmissionSupport {
        real_terminal: true,
        capabilities: Capabilities::default(),
        force_kitty: false,
        force_sixel: false,
        force_iterm: false,
    };
    assert!(!support.should_emit_kitty());

    let acked = GraphicsEmissionSupport {
        capabilities: Capabilities {
            kitty_graphics: true,
            ..Default::default()
        },
        ..support
    };
    assert!(acked.should_emit_kitty());

    let forced = GraphicsEmissionSupport {
        force_kitty: true,
        ..support
    };
    assert!(forced.should_emit_kitty());

    let captured = GraphicsEmissionSupport {
        real_terminal: false,
        force_kitty: true,
        ..support
    };
    assert!(!captured.should_emit_kitty());
}

#[test]
fn graphics_support_blocks_sprixels_without_ack_or_force() {
    let support = GraphicsEmissionSupport {
        real_terminal: true,
        capabilities: Capabilities::default(),
        force_kitty: false,
        force_sixel: false,
        force_iterm: false,
    };
    assert!(!support.should_emit_sprixel(SprixelProtocol::Sixel));
    assert!(!support.should_emit_sprixel(SprixelProtocol::Iterm2));
    assert!(!support.should_emit_sprixel(SprixelProtocol::Unknown));

    let sixel_acked = GraphicsEmissionSupport {
        capabilities: Capabilities {
            sixel: true,
            ..Default::default()
        },
        ..support
    };
    assert!(sixel_acked.should_emit_sprixel(SprixelProtocol::Sixel));

    let iterm_forced = GraphicsEmissionSupport {
        force_iterm: true,
        ..support
    };
    assert!(iterm_forced.should_emit_sprixel(SprixelProtocol::Iterm2));
}

#[test]
fn sprixel_protocol_detects_sixel_and_iterm() {
    assert_eq!(
        sprixel_protocol("\x1bPqpayload\x1b\\"),
        SprixelProtocol::Sixel
    );
    assert_eq!(
        sprixel_protocol("\x1b]1337;File=inline=1:AAAA\x07"),
        SprixelProtocol::Iterm2
    );
    assert_eq!(sprixel_protocol("plain"), SprixelProtocol::Unknown);
}
