#![cfg(all(unix, feature = "crossterm"))]

use slt::{Event, KeyCode, KeyModifiers, RunConfig};
use std::io::{self, Write};
use std::time::{Duration, Instant};

fn stdio_flags() -> (rustix::fs::OFlags, rustix::fs::OFlags) {
    (
        rustix::fs::fcntl_getfl(io::stdin()).unwrap(),
        rustix::fs::fcntl_getfl(io::stdout()).unwrap(),
    )
}

fn marker(name: &str) {
    print!("\x1b]777;BURST_{name}\x07");
    io::stdout().flush().unwrap();
}

fn config() -> RunConfig {
    RunConfig::default()
        .tick_rate(Duration::from_millis(10))
        .no_fps_cap()
        .handle_ctrl_c(false)
        .handle_suspend(false)
        .kitty_keyboard(true)
        .report_all_keys(true)
}

fn paste_text() -> String {
    format!("paste:{}\n\x1b[?62;4c\u{754c}", "x".repeat(3072))
}

fn expected_functional() -> Vec<Event> {
    use slt::crossterm::event::{
        Event as NativeEvent, KeyCode as K, KeyEvent, KeyEventKind, KeyModifiers as M,
        MediaKeyCode as Media, ModifierKeyCode as Modifier,
    };
    let mut expected = Vec::new();
    for point in (57358..=57363).chain(57376..=57454) {
        let code = match point {
            57358 => K::CapsLock,
            57359 => K::ScrollLock,
            57360 => K::NumLock,
            57361 => K::PrintScreen,
            57362 => K::Pause,
            57363 => K::Menu,
            57376..=57398 => K::F((point - 57376 + 13) as u8),
            57399..=57408 => K::Char((b'0' + (point - 57399) as u8) as char),
            57409 => K::Char('.'),
            57410 => K::Char('/'),
            57411 => K::Char('*'),
            57412 => K::Char('-'),
            57413 => K::Char('+'),
            57414 => K::Enter,
            57415 => K::Char('='),
            57416 => K::Char(','),
            57417 => K::Left,
            57418 => K::Right,
            57419 => K::Up,
            57420 => K::Down,
            57421 => K::PageUp,
            57422 => K::PageDown,
            57423 => K::Home,
            57424 => K::End,
            57425 => K::Insert,
            57426 => K::Delete,
            57427 => K::KeypadBegin,
            57428..=57440 => K::Media(
                [
                    Media::Play,
                    Media::Pause,
                    Media::PlayPause,
                    Media::Reverse,
                    Media::Stop,
                    Media::FastForward,
                    Media::Rewind,
                    Media::TrackNext,
                    Media::TrackPrevious,
                    Media::Record,
                    Media::LowerVolume,
                    Media::RaiseVolume,
                    Media::MuteVolume,
                ][(point - 57428) as usize],
            ),
            57441..=57454 => K::Modifier(
                [
                    Modifier::LeftShift,
                    Modifier::LeftControl,
                    Modifier::LeftAlt,
                    Modifier::LeftSuper,
                    Modifier::LeftHyper,
                    Modifier::LeftMeta,
                    Modifier::RightShift,
                    Modifier::RightControl,
                    Modifier::RightAlt,
                    Modifier::RightSuper,
                    Modifier::RightHyper,
                    Modifier::RightMeta,
                    Modifier::IsoLevel3Shift,
                    Modifier::IsoLevel5Shift,
                ][(point - 57441) as usize],
            ),
            _ => unreachable!(),
        };
        for (mut modifiers, kind) in [
            (M::NONE, KeyEventKind::Press),
            (M::SHIFT | M::CONTROL, KeyEventKind::Repeat),
            (M::NONE, KeyEventKind::Release),
        ] {
            if let K::Modifier(modifier) = code {
                modifiers |= match modifier {
                    Modifier::LeftShift | Modifier::RightShift => M::SHIFT,
                    Modifier::LeftControl | Modifier::RightControl => M::CONTROL,
                    Modifier::LeftAlt | Modifier::RightAlt => M::ALT,
                    Modifier::LeftSuper | Modifier::RightSuper => M::SUPER,
                    Modifier::LeftHyper | Modifier::RightHyper => M::HYPER,
                    Modifier::LeftMeta | Modifier::RightMeta => M::META,
                    _ => M::NONE,
                };
            }
            expected.extend(slt::event::from_crossterm(NativeEvent::Key(
                KeyEvent::new_with_kind(code, modifiers, kind),
            )));
        }
    }
    expected.push(Event::key_char('A'));
    expected.push(Event::key_mod(KeyCode::BackTab, KeyModifiers::SHIFT));
    // The public conversion intentionally drops 13 Media keys x 3 kinds.
    assert_eq!(expected.len(), 257 - 39);
    expected
}

fn expected_events(case: &str) -> Vec<Event> {
    if let Some(count) = case.strip_prefix("ascii_") {
        return vec![Event::key_char('x'); count.parse().unwrap()];
    }
    match case {
        "functional" => expected_functional(),
        "paste" => vec![Event::paste(paste_text())],
        "split" => {
            let mut events = vec![Event::key_char('a'); 1023];
            events.extend([
                Event::key_char('\u{754c}'),
                Event::key(KeyCode::Left),
                Event::key(KeyCode::F(1)),
                Event::key_mod(KeyCode::Left, KeyModifiers::CONTROL),
                Event::paste("payload\x1b[?62;4c"),
            ]);
            events
        }
        "partial_idle" => vec![Event::key_char('x')],
        "resize" => vec![Event::resize(50, 18)],
        _ => panic!("unknown burst case {case}"),
    }
}

#[test]
fn public_burst_child() {
    let Ok(case) = std::env::var("SLT_BURST_CASE") else {
        return;
    };
    let mode = std::env::var("SLT_BURST_MODE").unwrap();
    #[cfg(feature = "async")]
    if case.starts_with("async_") {
        async_child(&case);
        return;
    }
    let expected = expected_events(&case);
    let inherited_flags = stdio_flags();
    let mut observed = Vec::new();
    let mut input_batches = Vec::new();
    let mut first_frame = true;
    let mut started = Instant::now();
    let mut settled = None;
    let mut frames = 0usize;
    let mut next_split = 1usize;
    let mut render = |ui: &mut slt::Context| {
        assert_eq!(
            stdio_flags(),
            inherited_flags,
            "input reader changed inherited stdio flags"
        );
        frames += 1;
        if first_frame {
            first_frame = false;
            started = Instant::now();
            // Session entry must install the listener before the first UI frame.
            marker("READY");
        }
        let incoming: Vec<Event> = ui
            .events()
            .filter(|event| matches!(event, Event::Key(_) | Event::Paste(_)) || case == "resize")
            .cloned()
            .collect();
        if !incoming.is_empty() {
            input_batches.push(incoming.len());
            observed.extend(incoming);
        }
        if case == "split" {
            while next_split <= 5 && observed.len() >= 1022 + next_split {
                marker(&format!("SPLIT_{next_split}"));
                next_split += 1;
            }
        }
        ui.text("BURST_TEST");
        if observed.len() >= expected.len() {
            let settled = settled.get_or_insert_with(Instant::now);
            if case != "partial_idle" || settled.elapsed() >= Duration::from_millis(150) {
                ui.quit();
            }
        }
        if started.elapsed() >= Duration::from_secs(2) {
            ui.quit();
        }
    };
    match mode.as_str() {
        "fullscreen" => slt::run_with(config(), &mut render).unwrap(),
        "inline" => slt::run_inline_with(3, config(), &mut render).unwrap(),
        _ => panic!("unknown mode {mode}"),
    }
    assert_eq!(
        stdio_flags(),
        inherited_flags,
        "stdio flags changed after run returned"
    );
    assert_eq!(
        observed, expected,
        "{mode}/{case}: event loss, reordering or duplicate delivery"
    );
    if case.starts_with("ascii_") {
        assert_eq!(
            input_batches.first(),
            Some(&256),
            "zero-timeout drain did not fill the first event batch"
        );
        assert!(input_batches.iter().all(|&count| count <= 256));
    }
    if case == "partial_idle" {
        assert!(frames >= 3, "partial input prevented timed frames");
    }
    println!(
        "SLT_BURST_RESULT events={} frames={frames} batches={input_batches:?}",
        observed.len()
    );
}

#[cfg(feature = "async")]
fn async_child(case: &str) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let partial = case == "async_partial";
    let inherited_flags = stdio_flags();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let mut ready_tx = Some(ready_tx);
        let (input_tx, input_rx) = tokio::sync::oneshot::channel();
        let mut input_tx = Some(input_tx);
        let frames = Arc::new(AtomicUsize::new(0));
        let delivered = Arc::new(AtomicUsize::new(0));
        let (frame_count, event_count) = (frames.clone(), delivered.clone());
        let run =
            slt::run_async_with::<()>(config().tick_rate(Duration::from_secs(1)), move |ui, _| {
                assert_eq!(
                    stdio_flags(),
                    inherited_flags,
                    "async input reader changed inherited stdio flags"
                );
                frame_count.fetch_add(1, Ordering::Relaxed);
                if let Some(ready) = ready_tx.take() {
                    marker("READY");
                    let _ = ready.send(());
                }
                for event in ui.events() {
                    assert_eq!(*event, Event::key_char('x'));
                    event_count.fetch_add(1, Ordering::Relaxed);
                    if let Some(input) = input_tx.take() {
                        let _ = input.send(());
                    }
                }
                ui.text("ASYNC_BURST_TEST");
            })
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), ready_rx)
            .await
            .unwrap()
            .unwrap();
        if partial {
            tokio::time::timeout(Duration::from_secs(2), input_rx)
                .await
                .unwrap()
                .unwrap();
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        let cancel_start = Instant::now();
        tokio::time::timeout(Duration::from_millis(500), run.cancel_and_join())
            .await
            .unwrap()
            .unwrap();
        assert!(cancel_start.elapsed() < Duration::from_millis(500));
        assert_eq!(
            stdio_flags(),
            inherited_flags,
            "stdio flags changed after async cancellation"
        );
        assert_eq!(delivered.load(Ordering::Relaxed), usize::from(partial));
        println!(
            "SLT_BURST_RESULT events={} frames={} cancelled=true",
            delivered.load(Ordering::Relaxed),
            frames.load(Ordering::Relaxed)
        );
    });
}

fn verify(mode: &str, case: &str) {
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/burst_pty.py"
        ))
        .arg(std::env::current_exe().unwrap())
        .arg(mode)
        .arg(case)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{mode}/{case}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn unsplit_functional_fixture_reaches_public_fullscreen_and_inline() {
    for mode in ["fullscreen", "inline"] {
        verify(mode, "functional");
    }
}

#[test]
fn ascii_read_boundaries_drain_without_extra_wake_input() {
    for mode in ["fullscreen", "inline"] {
        for size in [1023, 1024, 1025, 2048, 3250] {
            verify(mode, &format!("ascii_{size}"));
        }
    }
}

#[test]
fn bracketed_paste_and_split_sequences_preserve_public_events() {
    for mode in ["fullscreen", "inline"] {
        for case in ["paste", "split", "partial_idle"] {
            verify(mode, case);
        }
    }
}

#[test]
fn resize_signal_reaches_public_events_without_keyboard_input() {
    for mode in ["fullscreen", "inline"] {
        verify(mode, "resize");
    }
}

#[cfg(feature = "async")]
#[test]
fn async_idle_and_partial_input_do_not_delay_cancellation() {
    for case in ["async_idle", "async_partial"] {
        verify("fullscreen", case);
    }
}
