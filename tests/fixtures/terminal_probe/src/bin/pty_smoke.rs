//! Session-level smoke test of `slt-terminal-probe` through a real
//! pseudo-terminal (issue #450): ConPTY on Windows, `openpty` on Unix.
//!
//! The driver feeds everything the pseudo-console emits into a `vt100`
//! screen model and asserts on that screen, not on raw bytes, because ConPTY
//! re-renders the console buffer rather than forwarding the app's escape
//! sequences verbatim. It checks, in order:
//!
//! 1. the alternate screen is entered: the probe's rows are visible and the
//!    `SLT-MAIN-SCREEN-MARKER` line it printed first is hidden;
//! 2. rendered text: title, bordered box and CJK row;
//! 3. key input: `t` flips every variant cell (`x` -> `y`);
//! 4. resize: the probe reports the new pseudo-console size;
//! 5. exit: `q` ends the process with status 0, after which the marker is
//!    back and no probe row remains (the alternate screen was left).
//!
//! Usage: `slt-pty-smoke PROBE_BINARY [RAW_LOG_PATH]`. Every wait has a
//! deadline; on failure the screen and the tail of the raw stream are dumped.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

const MAIN_SCREEN_MARKER: &str = "SLT-MAIN-SCREEN-MARKER";
const START_COLS: u16 = 80;
const START_ROWS: u16 = 24;
const RESIZED_COLS: u16 = 100;
const RESIZED_ROWS: u16 = 30;
/// Debug builds on a cold Windows runner can take a while to paint.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const STEP_TIMEOUT: Duration = Duration::from_secs(15);
/// Cursor Position Report request. ConPTY sends one at startup (portable-pty
/// creates it with `PSEUDOCONSOLE_INHERIT_CURSOR`) and blocks until answered.
const CPR_REQUEST: &[u8] = b"\x1b[6n";

struct Session {
    parser: vt100::Parser,
    raw: Vec<u8>,
    scanned: usize,
    output: Receiver<Vec<u8>>,
    eof: bool,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    saw_alt_enter: bool,
    saw_alt_leave: bool,
}

impl Session {
    fn spawn(probe: &str) -> Result<Self, String> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: START_ROWS,
                cols: START_COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("openpty failed: {e}"))?;
        // portable-pty resolves bare names through PATH; give it an absolute
        // path (`absolute`, unlike `canonicalize`, avoids `\\?\` on Windows).
        let program =
            std::path::absolute(probe).map_err(|e| format!("resolving {probe} failed: {e}"))?;
        let mut command = CommandBuilder::new(program);
        command.env("SLT_DISABLE_TERMINAL_QUERIES", "1");
        if !cfg!(windows) {
            command.env("TERM", "xterm-256color");
        }
        for key in ["TMUX", "STY", "ZELLIJ", "TERM_PROGRAM"] {
            command.env_remove(key);
        }
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| format!("spawning {probe} failed: {e}"))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("cloning the pty reader failed: {e}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("taking the pty writer failed: {e}"))?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut chunk = [0u8; 16 * 1024];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => {
                        if tx.send(chunk[..n].to_vec()).is_err() {
                            return;
                        }
                    }
                }
            }
        });

        Ok(Self {
            parser: vt100::Parser::new(START_ROWS, START_COLS, 0),
            raw: Vec::new(),
            scanned: 0,
            output: rx,
            eof: false,
            writer,
            master: pair.master,
            child,
            saw_alt_enter: false,
            saw_alt_leave: false,
        })
    }

    /// Read whatever arrives within `budget`, feeding the screen model and
    /// answering cursor-position requests.
    fn pump(&mut self, budget: Duration) -> Result<(), String> {
        let deadline = Instant::now() + budget;
        loop {
            let now = Instant::now();
            if now >= deadline || self.eof {
                return Ok(());
            }
            match self.output.recv_timeout(deadline - now) {
                Ok(chunk) => {
                    self.parser.process(&chunk);
                    self.raw.extend_from_slice(&chunk);
                    self.scan_raw()?;
                }
                Err(RecvTimeoutError::Timeout) => return Ok(()),
                Err(RecvTimeoutError::Disconnected) => {
                    self.eof = true;
                    return Ok(());
                }
            }
        }
    }

    fn scan_raw(&mut self) -> Result<(), String> {
        // Re-scan a small overlap so sequences split across reads are seen.
        let start = self.scanned.saturating_sub(16);
        let window = &self.raw[start..];
        let requests = count(window, CPR_REQUEST);
        let already = count(&self.raw[start..self.scanned], CPR_REQUEST);
        self.saw_alt_enter |= count(window, b"\x1b[?1049h") > 0;
        self.saw_alt_leave |= count(window, b"\x1b[?1049l") > 0;
        self.scanned = self.raw.len();
        for _ in already..requests {
            let (row, col) = self.parser.screen().cursor_position();
            let reply = format!("\x1b[{};{}R", row + 1, col + 1);
            self.send(reply.as_bytes())?;
        }
        Ok(())
    }

    fn send(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.writer
            .write_all(bytes)
            .and_then(|()| self.writer.flush())
            .map_err(|e| format!("writing to the pty failed: {e}"))
    }

    fn screen_text(&self) -> String {
        self.parser.screen().contents()
    }

    fn wait_until(
        &mut self,
        what: &str,
        timeout: Duration,
        mut done: impl FnMut(&str) -> bool,
    ) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        loop {
            if done(&self.screen_text()) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!("timed out after {timeout:?} waiting for {what}"));
            }
            self.pump(Duration::from_millis(100))?;
        }
    }

    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), String> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("resizing the pty failed: {e}"))?;
        self.parser.screen_mut().set_size(rows, cols);
        Ok(())
    }

    fn wait_exit(&mut self, timeout: Duration) -> Result<portable_pty::ExitStatus, String> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => {}
                Err(e) => return Err(format!("waiting for the probe failed: {e}")),
            }
            if Instant::now() >= deadline {
                return Err(format!("probe did not exit within {timeout:?}"));
            }
            self.pump(Duration::from_millis(100))?;
        }
    }

    fn dump(&self) -> String {
        let tail_start = self.raw.len().saturating_sub(2048);
        format!(
            "--- screen ({}x{}) ---\n{}\n--- raw tail ---\n{}",
            self.parser.screen().size().1,
            self.parser.screen().size().0,
            self.screen_text(),
            self.raw[tail_start..]
                .escape_ascii()
                .to_string()
                .replace("\\x1b", "\n\\x1b")
        )
    }
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

fn pass(message: &str) {
    println!("PASS  {message}");
}

fn run(session: &mut Session) -> Result<(), String> {
    let title = format!("SLT-PROBE {START_COLS}x{START_ROWS} variant=x");
    session.wait_until("the first frame", STARTUP_TIMEOUT, |s| {
        s.contains(&title) && s.contains("keys:")
    })?;
    let screen = session.screen_text();
    if screen.contains(MAIN_SCREEN_MARKER) {
        return Err("main-screen marker visible while the probe runs".into());
    }
    pass("alternate screen entered (probe visible, main-screen marker hidden)");

    for expected in [
        "\u{250C}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2510}",
        "\u{2502}BOX-x",
        "CJK  |\u{D55C}\u{AE00}\u{6F22}\u{5B57}Bx|",
        "HIT  [ ] TARGET",
    ] {
        if !screen.contains(expected) {
            return Err(format!("rendered screen lacks {expected:?}"));
        }
    }
    pass("rendered title, bordered box, CJK row and widget row");

    session.send(b"t")?;
    session.wait_until("variant=y after key t", STEP_TIMEOUT, |s| {
        s.contains("variant=y") && s.contains("\u{2502}BOX-y") && s.contains("By|")
    })?;
    pass("key input re-rendered every variant cell");

    session.resize(RESIZED_COLS, RESIZED_ROWS)?;
    let resized = format!("SLT-PROBE {RESIZED_COLS}x{RESIZED_ROWS} variant=y");
    session.wait_until("the resized frame", STEP_TIMEOUT, |s| s.contains(&resized))?;
    pass("resize reached the app and re-rendered");

    session.send(b"q")?;
    let status = session.wait_exit(STEP_TIMEOUT)?;
    if !status.success() {
        return Err(format!("probe exited unsuccessfully: {status:?}"));
    }
    pass("probe exited with status 0 after q");

    // The pseudo-console repaints the restored main screen asynchronously.
    session.wait_until("the main screen after exit", STEP_TIMEOUT, |s| {
        s.contains(MAIN_SCREEN_MARKER) && !s.contains("SLT-PROBE")
    })?;
    if session.saw_alt_enter && !session.saw_alt_leave {
        return Err("?1049h was forwarded but ?1049l never arrived".into());
    }
    let forwarded = if session.saw_alt_enter {
        "?1049h/?1049l forwarded"
    } else {
        "pseudo-console repainted the main buffer"
    };
    pass(&format!(
        "alternate screen left, main screen restored ({forwarded})"
    ));
    Ok(())
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(probe) = args.next() else {
        eprintln!("usage: slt-pty-smoke PROBE_BINARY [RAW_LOG_PATH]");
        return ExitCode::from(2);
    };
    let log_path = args.next().map(PathBuf::from);
    let backend = if cfg!(windows) { "ConPTY" } else { "openpty" };
    println!("INFO  driving {probe} through {backend}");

    let mut session = match Session::spawn(&probe) {
        Ok(session) => session,
        Err(error) => {
            println!("FAIL  {error}");
            return ExitCode::FAILURE;
        }
    };
    let result = run(&mut session);
    if result.is_err() {
        let _ = session.child.kill();
    }
    if let Some(path) = log_path
        && let Err(error) = std::fs::write(&path, &session.raw)
    {
        eprintln!("warning: could not write {}: {error}", path.display());
    }
    match result {
        Ok(()) => {
            println!("DONE  {backend} smoke passed");
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("FAIL  {error}\n{}", session.dump());
            ExitCode::FAILURE
        }
    }
}
