use super::*;

#[test]
fn cross_query_paste_keeps_reply_looking_payload_at_every_split() {
    let bytes = b"\x1b[200~A\x1b[?62;4cB\x1b[201~";
    for first in 1..bytes.len() {
        for second in first..bytes.len() {
            let mut replay = Replay::default();
            assert!(replay.demultiplex(&bytes[..first]).is_empty());
            assert!(replay.demultiplex(&bytes[first..second]).is_empty());
            assert!(replay.demultiplex(&bytes[second..]).is_empty());
            assert_eq!(
                replay.events,
                VecDeque::from([Event::Paste("A\x1b[?62;4cB".into())]),
                "split {first}/{second}"
            );
            assert!(replay.bytes.is_empty());
        }
    }
}

#[cfg(unix)]
fn review_case(case: &str) {
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/terminal_reply_pty.py"
        ))
        .arg(std::env::current_exe().unwrap())
        .arg(case)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn native_buffered_order_and_partial_parser_ownership() {
    for case in ["order", "paste", "utf8", "csi", "ss3", "kitty"] {
        review_case(case);
    }
}

#[cfg(unix)]
#[test]
fn functional_key_replay_matches_native_crossterm() {
    review_case("functional");
}

#[cfg(unix)]
#[test]
fn separate_queries_preserve_partial_paste() {
    review_case("queries");
}

#[cfg(unix)]
#[test]
fn review_pty_child() {
    let Ok(case) = std::env::var("SLT_REPLY_REVIEW_CASE") else {
        return;
    };
    crossterm::terminal::enable_raw_mode().unwrap();
    fn marker(name: &str) {
        std::println!("REVIEW_{name}");
        io::stdout().flush().unwrap();
    }
    if case == "queries" {
        assert!(query("\x1b]777;QUERY1\x07", Duration::from_millis(50), |_| false).is_none());
        assert!(query("\x1b]777;QUERY2\x07", Duration::from_millis(50), |_| false).is_none());
        assert_eq!(
            query("\x1b]777;QUERY3\x07", Duration::from_secs(2), |bytes| bytes
                .ends_with(b"c"))
            .unwrap(),
            "\x1b[?62;4c"
        );
        assert_eq!(read().unwrap(), Event::Paste("A\x1b[?62;4cB".into()));
    } else if case == "functional" {
        let count = (6 + 79) * 3 + 2;
        query(
            "\x1b]777;FUNCTIONAL_QUERY\x07",
            Duration::from_secs(5),
            |bytes| bytes.ends_with(b"c"),
        )
        .unwrap();
        let replay: Vec<_> = (0..count).map(|_| read().unwrap()).collect();
        assert!(replay.iter().any(|event| matches!(
            event,
            Event::Key(KeyEvent {
                code: KeyCode::F(13),
                ..
            })
        )));
        assert!(replay.iter().any(|event| matches!(
            event,
            Event::Key(KeyEvent {
                code: KeyCode::Modifier(ModifierKeyCode::LeftControl),
                ..
            })
        )));
        // This checks mapping parity, not native burst liveness. Crossterm's
        // Linux mio source can stall with unread bytes after a 1024-byte read
        // (upstream PR #1057). Acknowledge complete batches before sending
        // more input; retain every event and the original timeout/assertion.
        let mut native = Vec::with_capacity(count);
        while native.len() < count {
            marker(&format!("FUNCTIONAL_NATIVE_{}", native.len() / 64));
            let end = (native.len() + 64).min(count);
            for index in native.len()..end {
                assert!(
                    crossterm::event::poll(Duration::from_secs(2)).unwrap(),
                    "native event {index}/{count} was not received"
                );
                native.push(crossterm::event::read().unwrap());
            }
        }
        assert_eq!(replay, native);
    } else {
        marker("PRIME");
        if case == "order" {
            assert!(poll(Duration::from_secs(2)).unwrap());
            for _ in 0..256 {
                assert_eq!(read().unwrap(), key(KeyCode::Char('x'), KeyModifiers::NONE));
            }
            // The remaining 'a' must actually be buffered by crossterm.
            assert!(
                crate::terminal::read_pending_input(Duration::from_millis(5), |_| true).is_empty()
            );
        } else {
            // A complete sentinel lets native poll return while the suffix
            // of the same read remains in its private partial parser.
            assert!(poll(Duration::from_secs(2)).unwrap());
            assert_eq!(read().unwrap(), key(KeyCode::Char('x'), KeyModifiers::NONE));
            assert!(
                crate::terminal::read_pending_input(Duration::from_millis(5), |_| true).is_empty()
            );
        }
        marker("PRIMED");
        assert!(query("\x1b]52;c;?\x07", Duration::from_secs(1), |_| true).is_none());
        assert_eq!(crate::terminal::cell_pixel_size(), (9, 18));
        let expected = match case.as_str() {
            "order" => key(KeyCode::Char('a'), KeyModifiers::NONE),
            "paste" => Event::Paste("AB".into()),
            "utf8" => key(KeyCode::Char('\u{754c}'), KeyModifiers::NONE),
            "csi" => key(KeyCode::Left, KeyModifiers::NONE),
            "ss3" => key(KeyCode::F(1), KeyModifiers::NONE),
            "kitty" => key(
                KeyCode::Modifier(ModifierKeyCode::LeftControl),
                KeyModifiers::CONTROL,
            ),
            _ => unreachable!(),
        };
        assert!(poll(Duration::from_secs(2)).unwrap());
        assert_eq!(read().unwrap(), expected);
        if case == "order" {
            assert_eq!(read().unwrap(), key(KeyCode::Char('b'), KeyModifiers::NONE));
        }
    }
    crossterm::terminal::disable_raw_mode().unwrap();
    marker("PASSED");
}
