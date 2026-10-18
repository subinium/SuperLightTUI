//! Terminal query I/O, reply parsing and OSC helper tests.

use super::*;

#[test]
fn v024_cell_size_reply_completes_at_t_not_timeout() {
    let (bytes, elapsed) = collect_with_feed(
        b"\x1b[6;16;8t",
        Duration::ZERO,
        Duration::from_secs(2),
        &mut cell_size_reply_complete,
    );
    assert_eq!(parse_cell_pixel_size(&bytes), Some((8, 16)));
    assert!(elapsed < Duration::from_secs(1));
    assert!(!cell_size_reply_complete(b"\x1b[6;0;8t"));
    assert!(!cell_size_reply_complete(b"\x1b[6;16;8;9t"));
    assert!(!cell_size_reply_complete(b"\x1b[6;16;8"));
}

/// Feed `bytes` to a channel from a helper thread after `delay`, then run
/// [`collect_reply`] against it with the given budget and predicate.
fn collect_with_feed(
    bytes: &'static [u8],
    delay: Duration,
    budget: Duration,
    is_complete: &mut dyn FnMut(&[u8]) -> bool,
) -> (Vec<u8>, Duration) {
    let (tx, rx) = std::sync::mpsc::channel::<u8>();
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        for &b in bytes {
            if tx.send(b).is_err() {
                return;
            }
        }
        // Keep the sender alive past the collector's budget: the real
        // pump thread only drops its sender on stdin EOF, so dropping it
        // here right after the payload would disconnect the channel and
        // end the wait early, masking deadline behavior.
        std::thread::sleep(Duration::from_secs(3));
    });
    let start = Instant::now();
    let out = collect_reply(&rx, start + budget, is_complete);
    (out, start.elapsed())
}

#[cfg(unix)]
fn open_raw_pty() -> (rustix::fd::OwnedFd, rustix::fd::OwnedFd) {
    use rustix::fs::{Mode, OFlags};
    use rustix::pty::OpenptFlags;
    use rustix::termios::OptionalActions;

    let controller = rustix::pty::openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).unwrap();
    rustix::pty::grantpt(&controller).unwrap();
    rustix::pty::unlockpt(&controller).unwrap();
    let path = rustix::pty::ptsname(&controller, Vec::new()).unwrap();
    let user = rustix::fs::open(&path, OFlags::RDWR | OFlags::NOCTTY, Mode::empty()).unwrap();
    let mut attrs = rustix::termios::tcgetattr(&user).unwrap();
    attrs.make_raw();
    rustix::termios::tcsetattr(&user, OptionalActions::Now, &attrs).unwrap();
    (controller, user)
}

#[cfg(unix)]
fn pty_write_all(fd: &impl rustix::fd::AsFd, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let written = rustix::io::write(fd, bytes).unwrap();
        bytes = &bytes[written..];
    }
}

#[test]
fn collect_reply_osc_bel_terminator_completes_early() {
    let reply = b"\x1b]11;rgb:0000/0000/0000\x07";
    let (out, elapsed) = collect_with_feed(
        reply,
        Duration::ZERO,
        Duration::from_secs(2),
        &mut osc_reply_complete,
    );
    assert_eq!(out, reply);
    assert!(
        elapsed < Duration::from_secs(1),
        "should not wait out the budget"
    );
}

#[test]
fn collect_reply_osc_st_terminator_completes_early() {
    let reply = b"\x1bP>|tmux 3.5a\x1b\\";
    let (out, elapsed) = collect_with_feed(
        reply,
        Duration::ZERO,
        Duration::from_secs(2),
        &mut osc_reply_complete,
    );
    assert_eq!(out, reply);
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn collect_reply_silence_returns_empty_at_deadline() {
    // The silent-host case that used to deadlock startup: no bytes ever
    // arrive. The collector must give up at the deadline, not block.
    let budget = Duration::from_millis(150);
    let (out, elapsed) =
        collect_with_feed(b"", Duration::from_secs(5), budget, &mut osc_reply_complete);
    assert!(out.is_empty());
    assert!(elapsed >= budget);
    assert!(
        elapsed < Duration::from_secs(2),
        "must not block past the budget"
    );
}

#[test]
fn collect_reply_da_drains_two_replies() {
    let reply = b"\x1b[?62;4c\x1b[>1;10;0c";
    let (out, elapsed) = collect_with_feed(
        reply,
        Duration::ZERO,
        Duration::from_secs(2),
        &mut da_reply_complete(),
    );
    assert_eq!(out, reply);
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn collect_reply_da_lone_reply_returns_partial_at_deadline() {
    // A terminal that answers DA1 but ignores DA2: the collector waits out
    // the budget, then hands back the partial reply for best-effort parse
    // (pre-pump behavior, preserved).
    let budget = Duration::from_millis(150);
    let (out, elapsed) = collect_with_feed(
        b"\x1b[?62;4c",
        Duration::ZERO,
        budget,
        &mut da_reply_complete(),
    );
    assert_eq!(out, b"\x1b[?62;4c");
    assert!(elapsed >= budget);
}

#[test]
fn collect_reply_unterminated_caps_at_4096_bytes() {
    static BIG: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let big = BIG.get_or_init(|| vec![b'x'; 5000]).as_slice();
    let (tx, rx) = std::sync::mpsc::channel::<u8>();
    for &b in big {
        tx.send(b).unwrap();
    }
    let out = collect_reply(
        &rx,
        Instant::now() + Duration::from_secs(2),
        &mut osc_reply_complete,
    );
    assert_eq!(out.len(), 4096);
}

#[test]
fn decrpm_predicate_terminates_on_y() {
    let reply = b"\x1b[?2026;1$y";
    let (out, _) = collect_with_feed(
        reply,
        Duration::ZERO,
        Duration::from_secs(2),
        &mut decrpm_reply_complete,
    );
    assert_eq!(out, reply);
}

#[cfg(unix)]
#[test]
fn pty_probe_deadline_preserves_first_tab_printable_and_escape_bytes() {
    for payload in [b"\t".as_slice(), b"A".as_slice(), b"\x1b[D".as_slice()] {
        let (controller, user) = open_raw_pty();
        let silent = read_reply_from_fd(
            &user,
            Instant::now() + Duration::from_millis(15),
            &mut |_| false,
        );
        assert!(silent.is_empty());

        pty_write_all(&controller, payload);
        let received = read_reply_from_fd(
            &user,
            Instant::now() + Duration::from_millis(100),
            &mut |bytes| bytes.len() == payload.len(),
        );
        assert_eq!(received, payload, "first post-deadline input changed");
    }
}

#[cfg(unix)]
#[test]
fn pty_responsive_and_partial_replies_keep_existing_semantics() {
    let (controller, user) = open_raw_pty();
    let full = b"\x1b[?62;4c\x1b[>1;10;0c";
    pty_write_all(&controller, full);
    let start = Instant::now();
    let received = read_reply_from_fd(
        &user,
        start + Duration::from_secs(1),
        &mut da_reply_complete(),
    );
    assert_eq!(received, full);
    assert!(start.elapsed() < Duration::from_millis(250));

    let (controller, user) = open_raw_pty();
    let partial = b"\x1b[?62;4c";
    pty_write_all(&controller, partial);
    let received = read_reply_from_fd(
        &user,
        Instant::now() + Duration::from_millis(20),
        &mut da_reply_complete(),
    );
    assert_eq!(received, partial);
}

#[test]
fn base64_encode_empty() {
    assert_eq!(base64_encode(b""), "");
}

#[test]
fn base64_encode_hello() {
    assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
}

#[test]
fn base64_encode_padding() {
    assert_eq!(base64_encode(b"a"), "YQ==");
    assert_eq!(base64_encode(b"ab"), "YWI=");
    assert_eq!(base64_encode(b"abc"), "YWJj");
}

#[test]
fn base64_encode_unicode() {
    assert_eq!(base64_encode("한글".as_bytes()), "7ZWc6riA");
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_osc11_response_dark_and_light() {
    assert_eq!(
        parse_osc11_response("\x1b]11;rgb:0000/0000/0000\x1b\\"),
        ColorScheme::Dark
    );
    assert_eq!(
        parse_osc11_response("\x1b]11;rgb:ffff/ffff/ffff\x07"),
        ColorScheme::Light
    );
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_da1_attribute_4_sets_sixel() {
    let mut caps = Capabilities::default();
    parse_da1("\x1b[?62;4;6c", &mut caps);
    assert!(caps.sixel);
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_da1_without_4_leaves_sixel_false() {
    let mut caps = Capabilities::default();
    parse_da1("\x1b[?62;1;6c", &mut caps);
    assert!(!caps.sixel);
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_da1_ignores_da2_segment_in_same_string() {
    // DA1 (no `4`) followed by DA2 — DA2 must not be mistaken for DA1.
    let mut caps = Capabilities::default();
    parse_da1("\x1b[?62;1c\x1b[>0;276;0c", &mut caps);
    assert!(!caps.sixel);
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_da2_no_panic_on_garbage() {
    let mut caps = Capabilities::default();
    // Must not panic and must not set kitty_graphics on an unknown id.
    parse_da2("\x1b[>99;1;0c", &mut caps);
    assert!(!caps.kitty_graphics);
    parse_da2("not a da2 reply", &mut caps);
    assert!(!caps.kitty_graphics);
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_da2_shared_id_does_not_set_kitty_graphics() {
    let mut caps = Capabilities::default();
    // Kitty reports DA2 primary id 41.
    parse_da2("\x1b[>41;4000;0c", &mut caps);
    assert!(!caps.kitty_graphics);
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_da2_identity_extracts_id_and_version() {
    assert_eq!(parse_da2_identity("\x1b[>0;276;0c"), Some((0, 276)));
    assert_eq!(parse_da2_identity("\x1b[>41;4000;0c"), Some((41, 4000)));
    assert_eq!(parse_da2_identity("no reply here"), None);
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_kitty_graphics_ack_ok_sets_flag() {
    let mut caps = Capabilities::default();
    parse_kitty_graphics_ack("\x1b_Gi=31;OK\x1b\\", &mut caps);
    assert!(caps.kitty_graphics);
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_kitty_graphics_ack_error_or_wrong_id_leaves_flag() {
    let mut caps = Capabilities::default();
    // Error status must not flag support.
    parse_kitty_graphics_ack("\x1b_Gi=31;ENOENT:bad\x1b\\", &mut caps);
    assert!(!caps.kitty_graphics);
    // A different image id is not our query.
    parse_kitty_graphics_ack("\x1b_Gi=99;OK\x1b\\", &mut caps);
    assert!(!caps.kitty_graphics);
    // No APC at all.
    parse_kitty_graphics_ack("garbage", &mut caps);
    assert!(!caps.kitty_graphics);
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_decrpm_sync_output_recognized_states_are_supported() {
    // Ps = 1 (set), 2 (reset), 3 (perm set), 4 (perm reset) all mean the
    // mode is recognized → supported.
    assert_eq!(parse_decrpm_sync_output("\x1b[?2026;1$y"), Some(true));
    assert_eq!(parse_decrpm_sync_output("\x1b[?2026;2$y"), Some(true));
    assert_eq!(parse_decrpm_sync_output("\x1b[?2026;3$y"), Some(true));
    assert_eq!(parse_decrpm_sync_output("\x1b[?2026;4$y"), Some(true));
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_decrpm_sync_output_ps0_is_unsupported() {
    // Ps = 0 → mode not recognized.
    assert_eq!(parse_decrpm_sync_output("\x1b[?2026;0$y"), Some(false));
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_decrpm_sync_output_garbage_is_none() {
    // No DECRPM reply for mode 2026 in the string → inconclusive.
    assert_eq!(parse_decrpm_sync_output("not a decrpm reply"), None);
    // A reply for a *different* mode must not match.
    assert_eq!(parse_decrpm_sync_output("\x1b[?2004;1$y"), None);
    // Truncated reply (missing `$y` terminator) → None, not a panic.
    assert_eq!(parse_decrpm_sync_output("\x1b[?2026;1"), None);
    // Non-numeric Ps → None.
    assert_eq!(parse_decrpm_sync_output("\x1b[?2026;x$y"), None);
}

#[test]
fn sync_output_gate_defaults_to_emit() {
    // With the probe never having run (the unit-test process never enters a
    // real terminal session), the resolution stays `Unknown`, so the gate
    // must keep emitting BSU/ESU — preserving the historic always-emit
    // behavior on headless / non-answering hosts.
    assert!(should_emit_synchronized_update());
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_xtgettcap_tc_sets_truecolor() {
    let mut caps = Capabilities::default();
    // DCS 1 + r 5463 (=Tc) ST → truecolor present.
    parse_xtgettcap_truecolor("\x1bP1+r5463=\x1b\\", &mut caps);
    assert!(caps.truecolor);
}

#[cfg(feature = "crossterm")]
#[test]
fn parse_xtgettcap_invalid_leaves_truecolor_false() {
    let mut caps = Capabilities::default();
    // DCS 0 + r (capability NOT present) must not set the flag.
    parse_xtgettcap_truecolor("\x1bP0+r5463\x1b\\", &mut caps);
    assert!(!caps.truecolor);
    // Wrong capname hex must not match.
    parse_xtgettcap_truecolor("\x1bP1+r1234=\x1b\\", &mut caps);
    assert!(!caps.truecolor);
}

#[cfg(feature = "crossterm")]
#[test]
fn base64_decode_round_trip_hello() {
    let encoded = base64_encode("hello".as_bytes());
    assert_eq!(base64_decode(&encoded), Some("hello".to_string()));
}

#[cfg(feature = "crossterm")]
#[test]
fn color_scheme_equality() {
    assert_eq!(ColorScheme::Dark, ColorScheme::Dark);
    assert_ne!(ColorScheme::Dark, ColorScheme::Light);
    assert_eq!(ColorScheme::Unknown, ColorScheme::Unknown);
}

#[test]
fn copy_to_clipboard_writes_osc52() {
    let mut output: Vec<u8> = Vec::new();
    copy_to_clipboard(&mut output, "test").unwrap();
    let s = String::from_utf8(output).unwrap();
    assert!(s.starts_with("\x1b]52;c;"));
    assert!(s.ends_with("\x1b\\"));
    assert!(s.contains(&base64_encode(b"test")));
}
