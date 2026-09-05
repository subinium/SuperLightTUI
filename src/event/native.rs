//! Owned Unix input: never return to a competing parser or lose read readiness.

use std::collections::VecDeque;
use std::io::{self, IsTerminal, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyboardEnhancementFlags};
use mio::{Events, Interest, Poll, Token, Waker, unix::SourceFd};
use rustix::fs::{Mode, OFlags};

mod parse;

#[cfg(test)]
mod lifecycle_tests;

const INPUT: Token = Token(0);
const RESIZE: Token = Token(1);
const CANCEL: Token = Token(2);
const READ_BYTES: usize = 1024;
const MAX_SEQUENCE_BYTES: usize = 4096;
const MAX_PASTE_BYTES: usize = 4 * 1024 * 1024;
const MAX_QUEUED_EVENTS: usize = 4096;
const MAX_QUEUED_PASTE_BYTES: usize = 8 * 1024 * 1024;
static SOURCE: Mutex<Option<InputSource>> = Mutex::new(None);
static GENERATION: AtomicU64 = AtomicU64::new(1);
static WAKE: Mutex<Option<WakeHandle>> = Mutex::new(None);
static FAILED: AtomicBool = AtomicBool::new(false);
static FAILURE: Mutex<Option<String>> = Mutex::new(None);

std::thread_local! {
    static POLL_GENERATION: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

#[derive(Clone)]
struct WakeHandle {
    generation: u64,
    cancelled: Arc<AtomicBool>,
    waker: Arc<Waker>,
}

fn cancelled() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "terminal input session ended")
}

pub(super) fn check_failure() -> io::Result<()> {
    if !FAILED.load(Ordering::Acquire) {
        return Ok(());
    }
    let failure = FAILURE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Err(io::Error::new(
        io::ErrorKind::BrokenPipe,
        format!(
            "native input cannot be resumed after {}; restart the process",
            failure.as_deref().unwrap_or("a terminal input failure")
        ),
    ))
}

fn fail_input(error: &io::Error, generation: u64) {
    *FAILURE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(error.to_string());
    FAILED.store(true, Ordering::Release);
    let _ = GENERATION.compare_exchange(
        generation,
        generation.wrapping_add(1),
        Ordering::AcqRel,
        Ordering::Acquire,
    );
}

#[derive(Debug, PartialEq, Eq)]
enum InternalEvent {
    Event(Event),
    CursorPosition(u16, u16),
    KeyboardEnhancementFlags(KeyboardEnhancementFlags),
    PrimaryDeviceAttributes,
}

#[derive(Default)]
struct Parser {
    bytes: Vec<u8>,
    events: VecDeque<Event>,
    paste_bytes: usize,
    cursor: Option<(u16, u16)>,
}

impl Parser {
    fn accept(&mut self, event: InternalEvent) -> io::Result<()> {
        match event {
            InternalEvent::Event(event) => {
                let bytes = match &event {
                    Event::Paste(text) => text.len(),
                    _ => 0,
                };
                if self.events.len() >= MAX_QUEUED_EVENTS
                    || bytes > MAX_QUEUED_PASTE_BYTES.saturating_sub(self.paste_bytes)
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "terminal input queue limit exceeded",
                    ));
                }
                self.paste_bytes += bytes;
                self.events.push_back(event);
            }
            InternalEvent::CursorPosition(x, y) => self.cursor = Some((x, y)),
            InternalEvent::KeyboardEnhancementFlags(_) | InternalEvent::PrimaryDeviceAttributes => {
            }
        }
        Ok(())
    }

    fn feed(&mut self, bytes: &[u8], more: bool) -> io::Result<()> {
        for (index, byte) in bytes.iter().enumerate() {
            let limit = if self.bytes.starts_with(b"\x1b[200~") {
                MAX_PASTE_BYTES
            } else {
                MAX_SEQUENCE_BYTES
            };
            if self.bytes.len() >= limit {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "terminal input sequence limit exceeded",
                ));
            }
            self.bytes.push(*byte);
            match parse::parse_event(&self.bytes, index + 1 < bytes.len() || more) {
                Ok(Some(event)) => {
                    self.accept(event)?;
                    self.bytes.clear();
                }
                Ok(None) => {}
                Err(_) => self.bytes.clear(),
            }
        }
        Ok(())
    }

    fn finish_escape(&mut self) -> io::Result<()> {
        if self.bytes == b"\x1b" {
            if let Some(event) = parse::parse_event(&self.bytes, false)? {
                self.accept(event)?;
            }
            self.bytes.clear();
        }
        Ok(())
    }

    fn pop(&mut self) -> Option<Event> {
        let event = self.events.pop_front()?;
        if let Event::Paste(text) = &event {
            self.paste_bytes -= text.len();
        }
        Some(event)
    }

    fn resize(&mut self, width: u16, height: u16) -> io::Result<()> {
        if let Some(Event::Resize(old_width, old_height)) = self.events.back_mut() {
            (*old_width, *old_height) = (width, height);
            return Ok(());
        }
        self.accept(InternalEvent::Event(Event::Resize(width, height)))
    }
}

struct InputSource {
    poller: Poll,
    ready_events: Events,
    input: OwnedFd,
    resize_rx: UnixStream,
    resize_handler: signal_hook::SigId,
    readable: bool,
    resize_readable: bool,
    parser: Parser,
    eof: bool,
    generation: Option<u64>,
    cancelled: Arc<AtomicBool>,
    waker: Arc<Waker>,
}

impl Drop for InputSource {
    fn drop(&mut self) {
        signal_hook::low_level::unregister(self.resize_handler);
        if let Some(generation) = self.generation {
            let mut wake = WAKE
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if wake
                .as_ref()
                .is_some_and(|entry| entry.generation == generation)
            {
                wake.take();
            }
        }
    }
}

impl InputSource {
    fn new(generation: u64) -> io::Result<Self> {
        // Reopen the terminal, not dup(stdin): dup would share O_NONBLOCK with
        // caller-owned stdin/stdout. ttyname also works without a controlling tty.
        let path = if io::stdin().is_terminal() {
            rustix::termios::ttyname(io::stdin(), Vec::new())?
        } else {
            c"/dev/tty".to_owned()
        };
        let input = rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC | OFlags::NOCTTY,
            Mode::empty(),
        )?;
        let mut source = Self::from_input(input)?;
        source.generation = Some(generation);
        *WAKE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(WakeHandle {
            generation,
            cancelled: Arc::clone(&source.cancelled),
            waker: Arc::clone(&source.waker),
        });
        Ok(source)
    }

    fn from_input(input: OwnedFd) -> io::Result<Self> {
        if !rustix::fs::fcntl_getfl(&input)?.contains(OFlags::NONBLOCK) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "input must be independently nonblocking",
            ));
        }
        let (resize_rx, resize_tx) = UnixStream::pair()?;
        resize_rx.set_nonblocking(true)?;
        resize_tx.set_nonblocking(true)?;
        let poller = Poll::new()?;
        poller
            .registry()
            .register(&mut SourceFd(&input.as_raw_fd()), INPUT, Interest::READABLE)?;
        poller.registry().register(
            &mut SourceFd(&resize_rx.as_raw_fd()),
            RESIZE,
            Interest::READABLE,
        )?;
        let waker = Arc::new(Waker::new(poller.registry(), CANCEL)?);
        let resize_handler =
            signal_hook::low_level::pipe::register(signal_hook::consts::SIGWINCH, resize_tx)?;
        Ok(Self {
            poller,
            ready_events: Events::with_capacity(4),
            input,
            resize_rx,
            resize_handler,
            readable: false,
            resize_readable: false,
            parser: Parser::default(),
            eof: false,
            generation: None,
            cancelled: Arc::new(AtomicBool::new(false)),
            waker,
        })
    }

    fn pump(&mut self, timeout: Option<Duration>, read_input: bool) -> io::Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(cancelled());
        }
        self.poller.poll(
            &mut self.ready_events,
            timeout.map(|wait| wait.min(Duration::from_secs(86400))),
        )?;
        if self.cancelled.load(Ordering::Acquire) {
            return Err(cancelled());
        }
        for event in self.ready_events.iter() {
            match event.token() {
                INPUT => self.readable = true,
                RESIZE => self.resize_readable = true,
                _ => {}
            }
        }
        let mut bytes = [0; READ_BYTES];
        if self.resize_readable {
            match rustix::io::read(&self.resize_rx, &mut bytes) {
                Ok(0) => self.resize_readable = false,
                Ok(_) => {
                    let (width, height) = crossterm::terminal::size()?;
                    self.parser.resize(width, height)?;
                }
                Err(rustix::io::Errno::AGAIN) => self.resize_readable = false,
                Err(rustix::io::Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
        if read_input && self.readable && !self.eof {
            match rustix::io::read(&self.input, &mut bytes) {
                Ok(0) => self.eof = true,
                Ok(count) => self.parser.feed(&bytes[..count], count == bytes.len())?,
                Err(rustix::io::Errno::AGAIN) => {
                    self.readable = false;
                    self.parser.finish_escape()?;
                }
                Err(rustix::io::Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
            // Keep readiness after every successful read, including a full
            // buffer. Only WouldBlock consumes the edge; the next call can
            // resume without either waiting for a new edge or blocking on read.
        }
        Ok(())
    }

    fn poll(&mut self, timeout: Option<Duration>) -> io::Result<bool> {
        let start = Instant::now();
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return Err(cancelled());
            }
            let cached = !self.parser.events.is_empty();
            let remaining = timeout.map(|wait| wait.saturating_sub(start.elapsed()));
            let wait = if cached || self.readable || self.resize_readable {
                Some(Duration::ZERO)
            } else {
                remaining
            };
            match self.pump(wait, !cached) {
                Err(error)
                    if error.kind() == io::ErrorKind::Interrupted
                        && !self.cancelled.load(Ordering::Acquire) => {}
                result => result?,
            }
            if !self.parser.events.is_empty() {
                return Ok(true);
            }
            if self.eof {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "terminal input closed",
                ));
            }
            if timeout.is_some_and(|wait| start.elapsed() >= wait) {
                return Ok(false);
            }
        }
    }

    fn read(&mut self) -> io::Result<Event> {
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return Err(cancelled());
            }
            if let Some(event) = self.parser.pop() {
                return Ok(event);
            }
            self.poll(None)?;
        }
    }

    fn cursor_position(&mut self) -> io::Result<(u16, u16)> {
        self.parser.cursor = None;
        let mut stdout = io::stdout();
        stdout.write_all(b"\x1b[6n")?;
        stdout.flush()?;
        let start = Instant::now();
        let timeout = Duration::from_secs(2);
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return Err(cancelled());
            }
            if let Some(position) = self.parser.cursor.take() {
                return Ok(position);
            }
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "cursor position query timed out",
                ));
            }
            let wait = if self.readable || self.resize_readable {
                Duration::ZERO
            } else {
                remaining
            };
            match self.pump(Some(wait), true) {
                Err(error)
                    if error.kind() == io::ErrorKind::Interrupted
                        && !self.cancelled.load(Ordering::Acquire) => {}
                result => result?,
            }
            if self.eof {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "terminal input closed",
                ));
            }
        }
    }
}

fn with_source<T>(
    generation: u64,
    action: impl FnOnce(&mut InputSource) -> io::Result<T>,
) -> io::Result<T> {
    if generation != GENERATION.load(Ordering::Acquire) {
        return Err(cancelled());
    }
    check_failure()?;
    let mut source = SOURCE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if generation != GENERATION.load(Ordering::Acquire) {
        return Err(cancelled());
    }
    check_failure()?;
    if source
        .as_ref()
        .is_some_and(|input| input.generation != Some(generation))
    {
        source.take();
    }
    if source.is_none() {
        *source = Some(InputSource::new(generation)?);
    }
    // A release can arrive while opening/registering the new source, before
    // its waker is published. Do not enter an I/O wait for that ended session.
    if generation != GENERATION.load(Ordering::Acquire) {
        source.take();
        return Err(cancelled());
    }
    let result = action(source.as_mut().expect("source initialized above"));
    if generation != GENERATION.load(Ordering::Acquire) {
        source.take();
        return Err(cancelled());
    }
    if let Err(error) = &result
        && !matches!(
            error.kind(),
            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
        )
    {
        // A timed-out CPR has no request identifier. Refuse stream reuse rather
        // than interpreting its late prefix/suffix as a new reply or user input.
        fail_input(error, generation);
        source.take();
    }
    result
}

pub(super) fn poll(timeout: Duration) -> io::Result<bool> {
    let generation = GENERATION.load(Ordering::Acquire);
    let result = with_source(generation, |source| source.poll(Some(timeout)));
    POLL_GENERATION.with(|previous| previous.set(matches!(result, Ok(true)).then_some(generation)));
    result
}

pub(super) fn read() -> io::Result<Event> {
    let generation = POLL_GENERATION
        .with(|previous| previous.get())
        .unwrap_or_else(|| GENERATION.load(Ordering::Acquire));
    with_source(generation, InputSource::read)
}

pub(super) fn cursor_position() -> io::Result<(u16, u16)> {
    with_source(
        GENERATION.load(Ordering::Acquire),
        InputSource::cursor_position,
    )
}

pub(super) fn prepare() -> io::Result<()> {
    with_source(GENERATION.load(Ordering::Acquire), |_| Ok(()))
}

pub(super) fn release() {
    let ended = GENERATION.fetch_add(1, Ordering::AcqRel);
    let wake = {
        let wake = WAKE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        wake.as_ref()
            .filter(|entry| entry.generation <= ended)
            .cloned()
    };
    if let Some(wake) = wake {
        wake.cancelled.store(true, Ordering::Release);
        let _ = wake.waker.wake();
    }
    // The waker cancels an in-flight read before waiting for its guard. No
    // Source operation acquires the terminal session registry lock.
    let mut source = SOURCE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if source.as_ref().is_some_and(|input| {
        input
            .generation
            .is_none_or(|generation| generation <= ended)
    }) {
        source.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn pair() -> (InputSource, UnixStream) {
        let (reader, writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        (InputSource::from_input(reader.into()).unwrap(), writer)
    }

    fn character(c: char) -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
    }

    #[test]
    fn unsplit_bursts_and_exact_buffer_boundaries_need_no_extra_edge() {
        for size in [1023, 1024, 1025, 2048, 3250] {
            let (mut source, mut writer) = pair();
            writer.write_all(&vec![b'x'; size]).unwrap();
            for index in 0..size {
                assert!(
                    source.poll(Some(Duration::ZERO)).unwrap(),
                    "byte {index}/{size}"
                );
                assert_eq!(source.read().unwrap(), character('x'));
            }
            assert!(!source.poll(Some(Duration::ZERO)).unwrap());
            assert!(source.parser.bytes.is_empty());
        }
    }

    #[test]
    fn functional_burst_preserves_every_crossterm_event() {
        let mut bytes = Vec::new();
        let mut expected = Vec::new();
        for point in (57358..57364).chain(57376..57455) {
            for (mask, kind) in [(1, 1), (6, 2), (193, 3)] {
                let sequence = format!("\x1b[{point};{mask}:{kind}u");
                let Some(InternalEvent::Event(event)) =
                    parse::parse_event(sequence.as_bytes(), false).unwrap()
                else {
                    panic!("functional key is an event");
                };
                expected.push(event);
                bytes.extend_from_slice(sequence.as_bytes());
            }
        }
        for sequence in [b"\x1b[97:65;2:1u".as_slice(), b"\x1b[9;2:1u"] {
            let Some(InternalEvent::Event(event)) = parse::parse_event(sequence, false).unwrap()
            else {
                panic!("functional key is an event");
            };
            expected.push(event);
            bytes.extend_from_slice(sequence);
        }
        assert_eq!((bytes.len(), expected.len()), (3250, 257));
        let (mut source, mut writer) = pair();
        writer.write_all(&bytes).unwrap();
        for event in expected {
            assert!(source.poll(Some(Duration::ZERO)).unwrap());
            assert_eq!(source.read().unwrap(), event);
        }
        assert!(!source.poll(Some(Duration::ZERO)).unwrap());
    }

    #[test]
    fn partial_sequences_obey_deadlines_and_resume_without_loss() {
        for (prefix, suffix, expected) in [
            (
                b"\xe7".as_slice(),
                b"\x95\x8c".as_slice(),
                character('\u{754c}'),
            ),
            (b"\x1b[", b"D", Event::Key(KeyCode::Left.into())),
            (b"\x1bO", b"P", Event::Key(KeyCode::F(1).into())),
            (
                b"\x1b[200~abc",
                b"def\x1b[201~",
                Event::Paste("abcdef".into()),
            ),
        ] {
            let (mut source, mut writer) = pair();
            writer.write_all(prefix).unwrap();
            assert!(!source.poll(Some(Duration::ZERO)).unwrap());
            let start = Instant::now();
            assert!(!source.poll(Some(Duration::from_millis(20))).unwrap());
            assert!(start.elapsed() < Duration::from_secs(1));
            assert_eq!(source.parser.bytes, prefix);
            writer.write_all(suffix).unwrap();
            assert!(source.poll(Some(Duration::from_secs(1))).unwrap());
            assert_eq!(source.read().unwrap(), expected);
        }
    }

    #[test]
    fn escape_at_full_read_boundary_settles_or_joins_the_next_chunk() {
        for suffix in [b"".as_slice(), b"[D"] {
            let (mut source, mut writer) = pair();
            let mut bytes = vec![b'x'; READ_BYTES - 1];
            bytes.push(27);
            bytes.extend_from_slice(suffix);
            writer.write_all(&bytes).unwrap();
            for _ in 0..READ_BYTES - 1 {
                assert!(source.poll(Some(Duration::ZERO)).unwrap());
                assert_eq!(source.read().unwrap(), character('x'));
            }
            assert!(source.poll(Some(Duration::ZERO)).unwrap());
            assert_eq!(
                source.read().unwrap(),
                Event::Key(
                    if suffix.is_empty() {
                        KeyCode::Esc
                    } else {
                        KeyCode::Left
                    }
                    .into()
                )
            );
        }
    }

    #[test]
    fn eof_delivers_queued_keys_then_returns_an_error() {
        let (mut source, mut writer) = pair();
        writer.write_all(b"ok").unwrap();
        drop(writer);
        assert!(source.poll(Some(Duration::from_secs(1))).unwrap());
        assert_eq!(source.read().unwrap(), character('o'));
        assert_eq!(source.read().unwrap(), character('k'));
        assert_eq!(
            source.poll(Some(Duration::ZERO)).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn blocking_descriptors_are_rejected_without_mutation() {
        let (reader, _writer) = UnixStream::pair().unwrap();
        let flags = rustix::fs::fcntl_getfl(&reader).unwrap();
        let clone = reader.try_clone().unwrap();
        assert!(InputSource::from_input(clone.into()).is_err());
        assert_eq!(rustix::fs::fcntl_getfl(&reader).unwrap(), flags);
    }

    #[test]
    fn cancellation_invalidates_a_previously_ready_event() {
        let (mut source, mut writer) = pair();
        writer.write_all(b"a").unwrap();
        assert!(source.poll(Some(Duration::ZERO)).unwrap());
        source.cancelled.store(true, Ordering::Release);
        assert_eq!(
            source.read().unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(
            source.poll(Some(Duration::ZERO)).unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
    }

    #[test]
    fn cancellation_wakes_an_indefinite_read_with_partial_input() {
        for prefix in [b"".as_slice(), b"\x1b["] {
            let (mut source, mut writer) = pair();
            writer.write_all(prefix).unwrap();
            assert!(!source.poll(Some(Duration::ZERO)).unwrap());
            let cancelled = Arc::clone(&source.cancelled);
            let waker = Arc::clone(&source.waker);
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            let reader = std::thread::spawn(move || {
                started_tx.send(()).unwrap();
                result_tx.send(source.read()).unwrap();
            });
            started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
            std::thread::sleep(Duration::from_millis(20));
            cancelled.store(true, Ordering::Release);
            waker.wake().unwrap();
            let result = result_rx.recv_timeout(Duration::from_secs(1));
            if result.is_err() {
                // Failure cleanup only; a rescue byte never counts as a pass.
                let _ = writer.write_all(b"!");
            }
            reader.join().unwrap();
            assert_eq!(
                result.unwrap().unwrap_err().kind(),
                io::ErrorKind::Interrupted
            );
        }
    }

    #[test]
    fn a_stale_poll_generation_cannot_reopen_input() {
        let stale = GENERATION.load(Ordering::Acquire).wrapping_sub(1);
        let result = with_source(stale, |_| -> io::Result<()> {
            panic!("stale source was reopened")
        });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Interrupted);
        POLL_GENERATION.with(|generation| generation.set(Some(stale)));
        assert_eq!(read().unwrap_err().kind(), io::ErrorKind::Interrupted);
        POLL_GENERATION.with(|generation| generation.set(None));
    }

    #[test]
    fn parser_limits_and_zero_coordinates_are_checked() {
        for invalid in [
            b"\x1b[0;0R".as_slice(),
            b"\x1b[<0;0;0M",
            b"\x1b[M\x20\x20\x20",
            b"\x1b[32;0;0M",
        ] {
            assert!(parse::parse_event(invalid, false).is_err());
        }
        let mut parser = Parser::default();
        parser.bytes.extend_from_slice(b"\x1b[");
        parser.bytes.resize(MAX_SEQUENCE_BYTES, b'1');
        assert_eq!(
            parser.feed(b"1", false).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        parser.bytes.clear();
        parser.bytes.extend_from_slice(b"\x1b[200~");
        parser.bytes.resize(MAX_PASTE_BYTES, b'x');
        assert_eq!(
            parser.feed(b"x", false).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        parser.bytes.clear();
        for _ in 0..MAX_QUEUED_EVENTS {
            parser.accept(InternalEvent::Event(character('x'))).unwrap();
        }
        assert!(parser.accept(InternalEvent::Event(character('x'))).is_err());
    }

    #[test]
    fn resize_coalesces_at_the_tail_without_reordering_or_starving_keys() {
        let mut parser = Parser::default();
        parser.feed(b"ab", false).unwrap();
        parser.resize(40, 20).unwrap();
        parser.resize(50, 30).unwrap();
        assert_eq!(parser.pop(), Some(character('a')));
        for width in 51..100 {
            parser.resize(width, 30).unwrap();
        }
        assert_eq!(parser.pop(), Some(character('b')));
        assert_eq!(parser.pop(), Some(Event::Resize(99, 30)));
        assert!(parser.pop().is_none());
    }

    proptest::proptest! {
        #[test]
        fn arbitrary_bytes_do_not_panic(bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..4096)) {
            let mut parser = Parser::default();
            let _ = parser.feed(&bytes, false);
        }
    }
}
