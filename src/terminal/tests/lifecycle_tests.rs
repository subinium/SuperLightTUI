//! Session, fullscreen and inline terminal lifecycle tests.

use super::*;

#[test]
fn review_resize_clears_old_owned_rows_but_preserves_unrelated_text() {
    fn apply(bytes: &[u8], cells: &mut [[u8; 40]; 24]) {
        let (mut x, mut y, mut index) = (0usize, 0usize, 0usize);
        while index < bytes.len() {
            if bytes[index..].starts_with(b"\x1b[") {
                let start = index + 2;
                let end = (start..bytes.len())
                    .find(|&i| (0x40..=0x7e).contains(&bytes[i]))
                    .unwrap();
                let parameters = std::str::from_utf8(&bytes[start..end]).unwrap();
                match bytes[end] {
                    b'H' => {
                        let mut values = parameters
                            .split(';')
                            .map(|value| value.parse::<usize>().unwrap());
                        y = values.next().unwrap() - 1;
                        x = values.next().unwrap() - 1;
                    }
                    b'K' => {
                        assert_eq!(parameters, "2");
                        cells[y].fill(b' ');
                    }
                    b'J' => {
                        assert!(parameters.is_empty() || parameters == "0");
                        cells[y][x..].fill(b' ');
                        for row in &mut cells[y + 1..] {
                            row.fill(b' ');
                        }
                    }
                    b'm' | b'h' | b'l' => {}
                    other => panic!("unexpected CSI final {other}"),
                }
                index = end + 1;
                continue;
            }
            match bytes[index] {
                b'\r' => x = 0,
                b'\n' => y += 1,
                byte => {
                    assert!(x < 40 && y < 24);
                    cells[y][x] = byte;
                    x += 1;
                }
            }
            index += 1;
        }
    }
    let mut cells = [[b' '; 40]; 24];
    cells[0][..10].copy_from_slice(b"STATIC_LOG");
    cells[12][..9].copy_from_slice(b"UNRELATED");
    let mut term = InlineTerminal::with_sink(40, 24, 4, 18);
    term.reserved = true;
    term.handle_resize_to(40, 10).unwrap();
    term.current
        .set_string(0, 0, "OLD_MARKER", Style::default());
    term.flush().unwrap();
    apply(&term.take_sink_bytes(), &mut cells);
    assert_eq!(&cells[6][..10], b"OLD_MARKER");
    term.handle_resize_to(40, 24).unwrap();
    term.current
        .set_string(0, 0, "NEW_MARKER", Style::default());
    term.flush().unwrap();
    apply(&term.take_sink_bytes(), &mut cells);
    assert!(
        cells[6..10]
            .iter()
            .all(|row| row.iter().all(|&byte| byte == b' '))
    );
    assert_eq!(&cells[18][..10], b"NEW_MARKER");
    assert_eq!(&cells[0][..10], b"STATIC_LOG");
    assert_eq!(&cells[12][..9], b"UNRELATED");
}

#[test]
fn review_abort_cleanup_includes_other_thread_sessions() {
    let id = register_active_session(SessionSnapshot {
        mode: TerminalSessionMode::Inline,
        mouse_mode: crate::MouseMode::Off,
        kitty_keyboard: false,
        report_all_keys: false,
        raw_mode_owned: false,
    });
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let id = register_active_session(SessionSnapshot {
            mode: TerminalSessionMode::Fullscreen,
            mouse_mode: crate::MouseMode::Hover,
            kitty_keyboard: true,
            report_all_keys: true,
            raw_mode_owned: true,
        });
        ready_tx.send(()).unwrap();
        done_rx.recv().unwrap();
        unregister_active_session(id);
    });
    ready_rx.recv().unwrap();
    let unwind = panic_cleanup_snapshots(false);
    assert_eq!(unwind.len(), 1);
    assert!(!unwind[0].raw_mode_owned);
    let abort = panic_cleanup_snapshots(true);
    assert!(abort.len() >= 2);
    assert!(
        abort
            .iter()
            .any(|snapshot| snapshot.raw_mode_owned && snapshot.report_all_keys)
    );
    done_tx.send(()).unwrap();
    worker.join().unwrap();
    unregister_active_session(id);
}

#[test]
fn review_failed_resize_metric_refresh_keeps_last_known_size() {
    let mut cache = CellPixelCache::default();
    let now = Instant::now();
    assert_eq!(cache.resolve(now, || Some((9, 18))), (9, 18));
    cache.refresh = true;
    cache.retry_after = None;
    assert_eq!(cache.resolve(now, || None), (9, 18));
    assert_eq!(
        cache.resolve(now + Duration::from_secs(1), || Some((10, 20))),
        (10, 20)
    );
}

#[test]
fn v024_failed_cell_size_queries_are_backed_off_and_resettable() {
    let mut cache = CellPixelCache::default();
    let now = Instant::now();
    let mut queries = 0;
    for _ in 0..100 {
        assert_eq!(
            cache.resolve(now, || {
                queries += 1;
                None
            }),
            (8, 16)
        );
    }
    assert_eq!(queries, 1);
    assert_eq!(
        cache.resolve(now + Duration::from_secs(1), || {
            queries += 1;
            Some((9, 18))
        }),
        (9, 18)
    );
    assert_eq!(queries, 2);
    assert_eq!(
        cache.resolve(now + Duration::from_secs(2), || panic!(
            "success must cache"
        )),
        (9, 18)
    );
    cache = CellPixelCache::default();
    assert_eq!(cache.resolve(now, || Some((10, 20))), (10, 20));
}

#[test]
#[ignore = "release-mode allocation and payload-sharing experiment"]
fn v024_terminal_preparation_benchmark() {
    use std::hint::black_box;
    let area = Rect::new(0, 0, 200, 60);
    let rounds = 1000;
    let before = Instant::now();
    for _ in 0..rounds {
        black_box(try_buffer_pair(area).unwrap());
    }
    let fresh = before.elapsed();
    let (mut current, mut previous) = try_buffer_pair(area).unwrap();
    let before = Instant::now();
    for _ in 0..rounds {
        reset_buffer_pair(&mut current, &mut previous, area).unwrap();
        black_box(&current);
    }
    let reuse = before.elapsed();
    let payload = "x".repeat(1024 * 1024);
    let shared: std::sync::Arc<str> = payload.clone().into();
    let before = Instant::now();
    for _ in 0..rounds {
        black_box(payload.clone());
    }
    let strings = before.elapsed();
    let before = Instant::now();
    for _ in 0..rounds {
        black_box(shared.clone());
    }
    let arcs = before.elapsed();
    std::println!(
        "rounds={rounds} area=200x60 fresh={fresh:?} reuse={reuse:?} string_1mib={strings:?} arc_1mib={arcs:?} retained_cell_bytes={}",
        (current.content.capacity() + previous.content.capacity())
            * std::mem::size_of::<crate::Cell>()
    );
}

#[test]
fn deferred_inline_mouse_keeps_physical_position_after_scroll_and_resize() {
    let mut term = InlineTerminal::with_sink(40, 24, 3, 10);
    term.reserved = true;
    let mut state = crate::FrameState::default();
    let mut events = vec![
        crate::Event::key(crate::KeyCode::Tab),
        crate::Event::mouse_click(2, 11),
    ];
    crate::localize_inline_events(&term, &mut state, &mut events);
    let delivered = crate::input_frame(&mut state, events);
    assert_eq!(delivered.len(), 1);
    assert_eq!(state.pending_input[0].as_mouse().unwrap().y, 1);
    term.write_scrollback(&["log".into()]).unwrap();
    crate::localize_inline_events(&term, &mut state, &mut Vec::new());
    assert_eq!(state.pending_input[0].as_mouse().unwrap().y, 0);
    term.handle_resize_to(40, 12).unwrap();
    crate::clear_frame_layout_cache(&mut state);
    crate::localize_inline_events(&term, &mut state, &mut Vec::new());
    assert_eq!(state.pending_input[0].as_mouse().unwrap().y, 2);
    term.handle_resize_to(1, 12).unwrap();
    crate::localize_inline_events(&term, &mut state, &mut Vec::new());
    assert!(state.pending_input.is_empty());
}

#[test]
fn v024_inline_mouse_coordinates_and_pixels_after_scrollback() {
    let mut term = InlineTerminal::with_sink(40, 24, 3, 10);
    term.reserved = true;
    let event = crate::Event::mouse_click(2, 10)
        .as_mouse()
        .cloned()
        .unwrap();
    assert_eq!(term.localize_mouse(event.clone()).unwrap().y, 0);
    let mut pixel_event = event.clone();
    pixel_event.pixel_y = Some(160);
    assert_eq!(term.localize_mouse(pixel_event).unwrap().pixel_y, Some(160));
    for y in [0, 9, 13, 23] {
        assert!(
            term.localize_mouse(crate::event::MouseEvent { y, ..event.clone() })
                .is_none()
        );
    }
    term.write_scrollback(&["log".into()]).unwrap();
    assert_eq!(term.origin_row(), 11);
    assert!(term.localize_mouse(event.clone()).is_none());
    term.handle_resize_to(40, 12).unwrap();
    assert_eq!(term.origin_row(), 9);
    assert_eq!(term.localize_mouse(event).unwrap().y, 1);
}

#[test]
fn v024_scrollback_wraps_physical_rows_without_empty_exact_width_row() {
    assert_eq!(scrollback_rows("abcdefgh", 4), ["abcd", "efgh"]);
    assert_eq!(scrollback_rows("abcde", 4), ["abcd", "e"]);
    assert_eq!(scrollback_rows("a\u{754c}b", 2), ["a", "\u{754c}", "b"]);
    assert_eq!(scrollback_rows("", 4), [""]);
    let mut term = InlineTerminal::with_sink(4, 24, 3, 10);
    term.reserved = true;
    term.write_scrollback(&["abcdefgh".into()]).unwrap();
    assert_eq!(term.origin_row(), 12);
    assert!(String::from_utf8_lossy(&term.take_sink_bytes()).contains("abcd\r\nefgh\r\n\r\n\r\n"));
}

#[test]
fn v024_same_size_redraw_reuses_buffer_pair_but_invalidates_graphics() {
    let mut term = InlineTerminal::with_sink(40, 24, 3, 10);
    let current = term.current.content.as_ptr();
    let previous = term.previous.content.as_ptr();
    term.current.set_string(0, 0, "old", Style::default());
    term.handle_resize_to(40, 24).unwrap();
    assert_eq!(current, term.current.content.as_ptr());
    assert_eq!(previous, term.previous.content.as_ptr());
    assert_eq!(term.current.get(0, 0).symbol, " ");
    assert!(String::from_utf8_lossy(&term.take_sink_bytes()).contains("\x1b[2K"));
}

#[test]
fn v024_session_registry_is_owner_thread_local() {
    let id = register_active_session(SessionSnapshot {
        mode: TerminalSessionMode::Inline,
        mouse_mode: crate::MouseMode::Off,
        kitty_keyboard: false,
        report_all_keys: false,
        raw_mode_owned: false,
    });
    assert!(active_session_snapshot().is_some());
    assert!(
        std::thread::spawn(|| active_session_snapshot().is_none())
            .join()
            .unwrap()
    );
    unregister_active_session(id);
}

#[test]
fn reset_current_buffer_applies_theme_background() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));

    reset_current_buffer(&mut buffer, Some(Color::Rgb(10, 20, 30)));
    assert_eq!(buffer.get(0, 0).style.bg, Some(Color::Rgb(10, 20, 30)));

    reset_current_buffer(&mut buffer, None);
    assert_eq!(buffer.get(0, 0).style.bg, None);
}

#[test]
fn inline_vertical_resize_clamps_and_restores_anchor_row() {
    let mut term = InlineTerminal::with_sink(80, 24, 4, 18);
    assert_eq!(term.start_row, 18);

    term.handle_resize_to(72, 10).unwrap();
    assert_eq!(term.size(), (72, 4));
    assert_eq!(term.start_row, 6);

    term.handle_resize_to(100, 30).unwrap();
    assert_eq!(term.size(), (100, 4));
    assert_eq!(term.start_row, 18);
}

#[test]
fn raw_mode_ownership_only_tracks_state_acquired_by_slt() {
    assert!(raw_mode_is_acquired(false));
    assert!(!raw_mode_is_acquired(true));

    let inherited = SessionSnapshot {
        mode: TerminalSessionMode::Fullscreen,
        mouse_mode: crate::MouseMode::Off,
        kitty_keyboard: false,
        report_all_keys: false,
        raw_mode_owned: false,
    };
    assert!(!inherited.raw_mode_owned);
}

#[test]
fn terminal_buffer_pair_rejects_invalid_geometry_before_allocation() {
    let oversized = Rect::new(0, 0, crate::buffer::MAX_BUFFER_CELLS as u32 + 1, 1);
    assert!(try_buffer_pair(oversized).is_err());

    let area = Rect::new(0, 0, 80, 24);
    let (current, previous) = try_buffer_pair(area).unwrap();
    assert_eq!(current.area, area);
    assert_eq!(previous.area, area);
}

#[test]
fn failed_inline_resize_preserves_both_buffers_and_viewport_state() {
    let mut term = InlineTerminal::with_sink(4, 24, 20, 4);
    term.current.set_string(0, 0, "safe", Style::new());
    let current_area = term.current.area;
    let previous_area = term.previous.area;
    let viewport_rows = term.viewport_rows;
    let start_row = term.start_row;

    assert!(term.handle_resize_to(u16::MAX, 3).is_err());
    assert_eq!(term.current.area, current_area);
    assert_eq!(term.previous.area, previous_area);
    assert_eq!(term.viewport_rows, viewport_rows);
    assert_eq!(term.start_row, start_row);
    assert_eq!(term.current.get(0, 0).symbol, "s");
}

#[test]
fn inline_scrollback_moves_region_and_invalidates_previous_frame() {
    let mut term = InlineTerminal::with_sink(40, 24, 3, 10);
    term.reserved = true;
    term.previous
        .set_string(0, 0, "stale dynamic frame", Style::default());

    term.write_scrollback(&["first".into(), "second\x1b[31m".into()])
        .unwrap();

    assert_eq!(term.start_row, 12);
    assert_eq!(term.previous.get(0, 0).symbol, " ");
    let bytes = String::from_utf8(term.take_sink_bytes()).unwrap();
    assert!(bytes.contains("first\r\n"));
    assert!(bytes.contains("second?[31m\r\n"));
    assert!(bytes.contains("\u{1b}[11;1H"), "moves to the owned row");
}

#[test]
fn fullscreen_session_enter_writes_alt_screen_sequence() {
    let session = TerminalSessionGuard {
        mode: TerminalSessionMode::Fullscreen,
        mouse_mode: crate::MouseMode::Off,
        kitty_keyboard: false,
        report_all_keys: false,
        raw_mode_owned: false,
        registry_id: None,
        restored: std::sync::atomic::AtomicBool::new(false),
        harness: true,
    };
    let mut out = Vec::new();
    write_session_enter(&mut out, &session).unwrap();
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("\u{1b}[?1049h"));
    assert!(output.contains("\u{1b}[?25l"));
    assert!(output.contains("\u{1b}[?2004h"));
}

#[test]
fn inline_session_enter_skips_alt_screen_sequence() {
    let session = TerminalSessionGuard {
        mode: TerminalSessionMode::Inline,
        mouse_mode: crate::MouseMode::Off,
        kitty_keyboard: false,
        report_all_keys: false,
        raw_mode_owned: false,
        registry_id: None,
        restored: std::sync::atomic::AtomicBool::new(false),
        harness: true,
    };
    let mut out = Vec::new();
    write_session_enter(&mut out, &session).unwrap();
    let output = String::from_utf8(out).unwrap();
    assert!(!output.contains("\u{1b}[?1049h"));
    assert!(output.contains("\u{1b}[?25l"));
    assert!(output.contains("\u{1b}[?2004h"));
}

#[test]
fn session_enter_writes_kitty_keyboard_flags_portably() {
    let session = TerminalSessionGuard {
        mode: TerminalSessionMode::Fullscreen,
        mouse_mode: crate::MouseMode::Off,
        kitty_keyboard: true,
        report_all_keys: true,
        raw_mode_owned: false,
        registry_id: None,
        restored: std::sync::atomic::AtomicBool::new(false),
        harness: true,
    };
    let mut out = Vec::new();
    write_session_enter(&mut out, &session).unwrap();
    let output = String::from_utf8(out).unwrap();
    let expected = format!("\u{1b}[>{}u", kitty_flags(true).bits());
    assert!(output.contains(&expected));
}

#[test]
fn fullscreen_session_cleanup_leaves_alt_screen() {
    let mut out = Vec::new();
    write_session_cleanup(&mut out, TerminalSessionMode::Fullscreen, false).unwrap();
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("\u{1b}[?1049l"));
    assert!(output.contains("\u{1b}[?25h"));
    assert!(output.contains("\u{1b}[?2004l"));
}

#[test]
fn inline_session_cleanup_keeps_normal_screen() {
    let mut out = Vec::new();
    write_session_cleanup(&mut out, TerminalSessionMode::Inline, false).unwrap();
    let output = String::from_utf8(out).unwrap();
    assert!(!output.contains("\u{1b}[?1049l"));
    assert!(output.ends_with('\n'));
    assert!(output.contains("\u{1b}[?25h"));
    assert!(output.contains("\u{1b}[?2004l"));
}

#[test]
fn session_exit_disables_focus_mouse_and_kitty_keyboard() {
    let mut out = Vec::new();
    write_session_exit(
        &mut out,
        TerminalSessionMode::Fullscreen,
        false,
        crate::MouseMode::Hover,
        true,
    )
    .unwrap();
    let output = String::from_utf8(out).unwrap();
    // Crossterm manages mouse/focus through the Windows console API rather
    // than writing those escape sequences to the supplied writer.
    #[cfg(not(windows))]
    assert!(output.contains("\u{1b}[?1004l"), "disables focus reporting");
    #[cfg(not(windows))]
    assert!(output.contains("\u{1b}[?1006l"), "disables SGR mouse mode");
    assert!(output.contains("\u{1b}[<1u"), "pops Kitty keyboard flags");
    assert!(output.contains("\u{1b}[?1049l"), "leaves alt screen");
}

#[test]
fn panic_cleanup_uses_full_session_exit_path() {
    let mut out = Vec::new();
    write_panic_cleanup(&mut out).unwrap();
    let output = String::from_utf8(out).unwrap();
    #[cfg(not(windows))]
    assert!(output.contains("\u{1b}[?1004l"), "disables focus reporting");
    assert!(output.contains("\u{1b}[<1u"), "pops Kitty keyboard flags");
    assert!(output.contains("\u{1b}[?1049l"), "leaves alt screen");
}

// ── Unix suspend/resume sequence tests (issue #263) ──────────────────

#[cfg(unix)]
#[test]
fn suspend_sequence_fullscreen_leaves_alt_screen() {
    let snapshot = test_snapshot(TerminalSessionMode::Fullscreen, false, false);
    let mut out = Vec::new();
    write_suspend_sequence(&mut out, &snapshot).unwrap();
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("\u{1b}[?1049l"), "leaves alt screen");
    assert!(output.contains("\u{1b}[?25h"), "shows cursor");
    assert!(output.contains("\u{1b}[?2004l"), "disables bracketed paste");
}

#[cfg(unix)]
#[test]
fn suspend_sequence_inline_keeps_normal_screen() {
    let snapshot = test_snapshot(TerminalSessionMode::Inline, false, false);
    let mut out = Vec::new();
    write_suspend_sequence(&mut out, &snapshot).unwrap();
    let output = String::from_utf8(out).unwrap();
    assert!(
        !output.contains("\u{1b}[?1049l"),
        "inline must not leave alt screen"
    );
    assert!(output.contains("\u{1b}[?25h"), "shows cursor");
    assert!(output.contains("\u{1b}[?2004l"), "disables bracketed paste");
}

#[cfg(unix)]
#[test]
fn suspend_sequence_disables_mouse_and_kitty_when_enabled() {
    let snapshot = test_snapshot(TerminalSessionMode::Fullscreen, true, true);
    let mut out = Vec::new();
    write_suspend_sequence(&mut out, &snapshot).unwrap();
    // DisableMouseCapture emits the SGR-mouse disable (?1006l) among others.
    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("\u{1b}[?1006l"), "disables SGR mouse mode");
}

#[cfg(unix)]
#[test]
fn resume_sequence_fullscreen_round_trips_enter_and_flags_redraw() {
    let snapshot = test_snapshot(TerminalSessionMode::Fullscreen, false, false);

    // The resume path re-enters the same byte state as the initial enter.
    let guard = TerminalSessionGuard {
        mode: snapshot.mode,
        mouse_mode: snapshot.mouse_mode,
        kitty_keyboard: snapshot.kitty_keyboard,
        report_all_keys: snapshot.report_all_keys,
        raw_mode_owned: snapshot.raw_mode_owned,
        registry_id: None,
        restored: std::sync::atomic::AtomicBool::new(false),
        harness: true,
    };
    let mut enter_bytes = Vec::new();
    write_session_enter(&mut enter_bytes, &guard).unwrap();
    let enter = String::from_utf8(enter_bytes).unwrap();
    assert!(enter.contains("\u{1b}[?1049h"));
    assert!(enter.contains("\u{1b}[?25l"));
    assert!(enter.contains("\u{1b}[?2004h"));

    // Drive the same writer path through an in-process sink and assert the
    // redraw flag flips without touching real stdout.
    NEEDS_FULL_REDRAW.store(false, std::sync::atomic::Ordering::SeqCst);
    let mut resume_bytes = Vec::new();
    resume_from_shell_with_writer(&mut resume_bytes, &snapshot).unwrap();
    assert_eq!(String::from_utf8(resume_bytes).unwrap(), enter);
    assert!(
        NEEDS_FULL_REDRAW.swap(false, std::sync::atomic::Ordering::SeqCst),
        "resume must request a full redraw exactly once"
    );
    assert!(
        !NEEDS_FULL_REDRAW.swap(false, std::sync::atomic::Ordering::SeqCst),
        "the redraw flag is consumed by the first swap (idempotent)"
    );
}

#[cfg(unix)]
#[test]
fn needs_full_redraw_swaps_true_once() {
    NEEDS_FULL_REDRAW.store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(NEEDS_FULL_REDRAW.swap(false, std::sync::atomic::Ordering::SeqCst));
    assert!(!NEEDS_FULL_REDRAW.swap(false, std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn kitty_flags_base_set_excludes_report_all_keys() {
    use crossterm::event::KeyboardEnhancementFlags;
    let flags = kitty_flags(false);
    assert!(flags.contains(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES));
    assert!(flags.contains(KeyboardEnhancementFlags::REPORT_EVENT_TYPES));
    assert!(!flags.contains(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES));
}

#[test]
fn kitty_flags_report_all_keys_sets_flag() {
    use crossterm::event::KeyboardEnhancementFlags;
    let flags = kitty_flags(true);
    assert!(flags.contains(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES));
    assert!(flags.contains(KeyboardEnhancementFlags::REPORT_EVENT_TYPES));
    assert!(flags.contains(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES));
}

// ── Mouse reporting modes (issue #444) ────────────────────────────────

/// Every DEC mouse mode SLT could emit, enabled (`h`) or disabled (`l`).
#[cfg(not(windows))]
fn mouse_sequences(output: &str, suffix: char) -> Vec<u16> {
    let mut found = Vec::new();
    let mut rest = output;
    while let Some(start) = rest.find("\u{1b}[?") {
        rest = &rest[start + 3..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(code) = digits.parse::<u16>()
            && matches!(code, 1000 | 1002 | 1003 | 1005 | 1006 | 1015)
            && rest[digits.len()..].starts_with(suffix)
        {
            found.push(code);
        }
    }
    found
}

#[cfg(not(windows))]
fn guard_with_mouse(mode: crate::MouseMode) -> TerminalSessionGuard {
    TerminalSessionGuard {
        mode: TerminalSessionMode::Fullscreen,
        mouse_mode: mode,
        kitty_keyboard: false,
        report_all_keys: false,
        raw_mode_owned: false,
        registry_id: None,
        restored: std::sync::atomic::AtomicBool::new(false),
        harness: true,
    }
}

#[cfg(not(windows))]
#[test]
fn each_mouse_mode_writes_exactly_its_dec_modes_on_enter_and_exit() {
    use crate::MouseMode;
    let cases: [(MouseMode, &[u16]); 4] = [
        (MouseMode::Off, &[]),
        (MouseMode::Click, &[1000, 1006]),
        (MouseMode::Drag, &[1000, 1002, 1006]),
        (MouseMode::Hover, &[1000, 1003, 1006]),
    ];
    for (mode, expected) in cases {
        let mut enter = Vec::new();
        write_session_enter(&mut enter, &guard_with_mouse(mode)).unwrap();
        let enter = String::from_utf8(enter).unwrap();
        assert_eq!(mouse_sequences(&enter, 'h'), expected, "{mode:?} enter");
        assert!(
            mouse_sequences(&enter, 'l').is_empty(),
            "{mode:?} enter disables nothing"
        );

        let reversed: Vec<u16> = expected.iter().rev().copied().collect();
        let mut exit = Vec::new();
        write_session_exit(
            &mut exit,
            TerminalSessionMode::Fullscreen,
            false,
            mode,
            false,
        )
        .unwrap();
        let exit = String::from_utf8(exit).unwrap();
        assert_eq!(mouse_sequences(&exit, 'l'), reversed, "{mode:?} exit");
        assert!(
            mouse_sequences(&exit, 'h').is_empty(),
            "{mode:?} exit enables nothing"
        );
    }
}

#[cfg(unix)]
#[test]
fn panic_and_suspend_restore_disable_the_active_mouse_mode_only() {
    use crate::MouseMode;
    for (mode, expected) in [
        (MouseMode::Off, &[][..]),
        (MouseMode::Click, &[1006, 1000][..]),
        (MouseMode::Drag, &[1006, 1002, 1000][..]),
        (MouseMode::Hover, &[1006, 1003, 1000][..]),
    ] {
        let mut snapshot = test_snapshot(TerminalSessionMode::Fullscreen, false, false);
        snapshot.mouse_mode = mode;
        // The panic hook and the SIGTSTP handler both restore through the
        // snapshot-driven exit writer.
        let mut out = Vec::new();
        write_suspend_sequence(&mut out, &snapshot).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert_eq!(mouse_sequences(&output, 'l'), expected, "{mode:?}");
        assert!(
            !output.contains("\u{1b}[?1015"),
            "urxvt encoding is never used"
        );
    }
    let mut out = Vec::new();
    write_panic_cleanup(&mut out).unwrap();
    let output = String::from_utf8(out).unwrap();
    assert_eq!(mouse_sequences(&output, 'l'), [1006, 1003, 1000]);
}
