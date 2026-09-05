use super::*;

fn marker(text: &str) {
    println!("NATIVE_{text}");
    io::stdout().flush().unwrap();
}

#[test]
fn native_lifecycle_child() {
    let Ok(case) = std::env::var("SLT_NATIVE_LIFECYCLE_CASE") else {
        return;
    };
    crossterm::terminal::enable_raw_mode().unwrap();
    match case.as_str() {
        "late_cursor" => {
            assert_eq!(
                cursor_position().unwrap_err().kind(),
                io::ErrorKind::TimedOut
            );
            marker("TIMED_OUT");
            let ack = std::env::var("SLT_NATIVE_ACK").unwrap();
            let start = Instant::now();
            while !std::path::Path::new(&ack).exists() {
                assert!(start.elapsed() < Duration::from_secs(2));
                std::thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(
                cursor_position().unwrap_err().kind(),
                io::ErrorKind::BrokenPipe
            );
            assert_eq!(
                poll(Duration::ZERO).unwrap_err().kind(),
                io::ErrorKind::BrokenPipe
            );
            assert!(SOURCE.lock().unwrap().is_none());
        }
        "fatal_ticket" => {
            marker("READY");
            assert!(poll(Duration::from_secs(2)).unwrap());
            let generation = GENERATION.load(Ordering::Acquire);
            assert_eq!(
                cursor_position().unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
            assert!(GENERATION.load(Ordering::Acquire) > generation);
            assert_eq!(read().unwrap_err().kind(), io::ErrorKind::Interrupted);
            assert!(SOURCE.lock().unwrap().is_none());
            assert!(WAKE.lock().unwrap().is_none());
        }
        "release_wait" => {
            let generation = GENERATION.load(Ordering::Acquire);
            let (ready_tx, ready_rx) = std::sync::mpsc::channel();
            let reader = std::thread::spawn(move || {
                with_source(generation, |source| {
                    ready_tx.send(()).unwrap();
                    source.read()
                })
            });
            ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            release();
            assert_eq!(
                reader.join().unwrap().unwrap_err().kind(),
                io::ErrorKind::Interrupted
            );
            assert!(SOURCE.lock().unwrap().is_none());
            assert!(WAKE.lock().unwrap().is_none());
        }
        _ => panic!("unknown case"),
    }
    crossterm::terminal::disable_raw_mode().unwrap();
    marker("PASSED");
}

#[test]
fn cursor_timeout_fatal_reset_and_release_are_isolated() {
    for case in ["late_cursor", "fatal_ticket", "release_wait"] {
        let output = std::process::Command::new("python3")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/support/native_lifecycle_pty.py"
            ))
            .arg(std::env::current_exe().unwrap())
            .arg(case)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{case}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
