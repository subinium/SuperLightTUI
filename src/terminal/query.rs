//! Terminal query I/O: bounded reply reads, DA1 / DA2 / DECRPM / XTGETTCAP
//! parsing and synchronized-output resolution.

use super::*;

#[cfg(all(feature = "crossterm", unix))]
static REPLY_READ_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(all(feature = "crossterm", unix))]
struct NonblockingFdGuard<'a, Fd: rustix::fd::AsFd> {
    fd: &'a Fd,
    original: rustix::fs::OFlags,
    changed: bool,
}

#[cfg(all(feature = "crossterm", unix))]
impl<'a, Fd: rustix::fd::AsFd> NonblockingFdGuard<'a, Fd> {
    fn new(fd: &'a Fd) -> rustix::io::Result<Self> {
        let original = rustix::fs::fcntl_getfl(fd)?;
        let changed = !original.contains(rustix::fs::OFlags::NONBLOCK);
        if changed {
            rustix::fs::fcntl_setfl(fd, original | rustix::fs::OFlags::NONBLOCK)?;
        }
        Ok(Self {
            fd,
            original,
            changed,
        })
    }
}

#[cfg(all(feature = "crossterm", unix))]
impl<Fd: rustix::fd::AsFd> Drop for NonblockingFdGuard<'_, Fd> {
    fn drop(&mut self) {
        if self.changed {
            let _ = rustix::fs::fcntl_setfl(self.fd, self.original);
        }
    }
}

/// Read one terminal reply from raw stdin, hard-bounded by `timeout`, stopping
/// early once `is_complete` recognizes a full reply (or at the 4096-byte cap).
///
/// The read happens synchronously on the calling thread. Unix temporarily sets
/// `O_NONBLOCK` on stdin. Other platforms conservatively skip byte queries
/// instead of consuming native console records. No reader survives a deadline.
#[cfg(feature = "crossterm")]
pub(super) fn read_stdin_reply(
    request: &str,
    timeout: Duration,
    is_complete: impl FnMut(&[u8]) -> bool,
) -> Option<String> {
    crate::event::reply::query(request, timeout, is_complete)
}

#[cfg(unix)]
pub(crate) fn read_pending_input(
    timeout: Duration,
    mut complete: impl FnMut(&[u8]) -> bool,
) -> Vec<u8> {
    let _guard = REPLY_READ_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    read_reply_from_fd(&io::stdin(), Instant::now() + timeout, &mut complete)
}

#[cfg(all(feature = "crossterm", unix))]
pub(super) fn read_reply_from_fd<Fd: rustix::fd::AsFd>(
    fd: &Fd,
    deadline: Instant,
    is_complete: &mut dyn FnMut(&[u8]) -> bool,
) -> Vec<u8> {
    let Ok(_nonblocking) = NonblockingFdGuard::new(fd) else {
        return Vec::new();
    };
    let mut bytes = Vec::new();
    let mut byte = [0u8; 1];
    while Instant::now() < deadline && bytes.len() < 4096 {
        match rustix::io::read(fd, &mut byte) {
            Ok(0) => break,
            Ok(_) => {
                bytes.push(byte[0]);
                if is_complete(&bytes) {
                    break;
                }
            }
            Err(rustix::io::Errno::AGAIN) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                std::thread::sleep(remaining.min(Duration::from_millis(1)));
            }
            Err(rustix::io::Errno::INTR) => {}
            Err(_) => break,
        }
    }
    bytes
}

/// Test-only deadline collector for deterministic parser timing coverage.
#[cfg(all(feature = "crossterm", test))]
pub(super) fn collect_reply(
    rx: &std::sync::mpsc::Receiver<u8>,
    deadline: Instant,
    is_complete: &mut dyn FnMut(&[u8]) -> bool,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        match rx.recv_timeout(deadline - now) {
            Ok(byte) => {
                bytes.push(byte);
                if is_complete(&bytes) || bytes.len() >= 4096 {
                    break;
                }
            }
            // Timed out, or the pump thread is gone (stdin EOF / error).
            Err(_) => break,
        }
    }
    bytes
}

/// Completion predicate for OSC / DCS / CSI-`t` style replies, which terminate
/// with BEL (`\x07`) or ST (`ESC \`).
#[cfg(feature = "crossterm")]
pub(super) fn osc_reply_complete(bytes: &[u8]) -> bool {
    let len = bytes.len();
    bytes.last() == Some(&b'\x07')
        || (len >= 2 && bytes[len - 2] == 0x1B && bytes[len - 1] == b'\\')
}

/// Completion predicate builder for Device-Attributes replies: `c` is the
/// final byte of each DA reply, and a combined `CSI c CSI > c` query yields
/// two of them, so completion fires on the second `c`.
#[cfg(feature = "crossterm")]
pub(super) fn da_reply_complete() -> impl FnMut(&[u8]) -> bool {
    move |bytes: &[u8]| {
        let (replies, _) = crate::event::reply::split(bytes);
        replies.iter().filter(|&&byte| byte == b'c').count() >= 2
    }
}

/// Completion predicate for DECRPM replies (`CSI ? <mode> ; <Ps> $ y`).
#[cfg(feature = "crossterm")]
pub(super) fn decrpm_reply_complete(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(parse_decrpm_sync_output)
        .is_some()
}

/// Read a Device-Attributes reply, which (unlike OSC) terminates with the byte
/// `c` rather than BEL / ST. Drains up to two `c`-terminated CSI replies
/// (DA1 + DA2) within the timeout so a combined `CSI c CSI > c` query yields
/// both answers in one string.
///
/// The Kitty keyboard progressive-enhancement query (`CSI ? u`, issue #448)
/// rides in front of DA1 in the same write: a supporting terminal answers
/// `CSI ? <flags> u` before its DA1 reply, and a terminal that ignores the
/// query still ends the read through DA1/DA2, so the extra query never adds a
/// timeout.
#[cfg(feature = "crossterm")]
pub(super) fn read_da_response(timeout: Duration) -> Option<String> {
    read_stdin_reply("\x1b[?u\x1b[c\x1b[>c", timeout, da_reply_complete())
}

/// Parse the Kitty keyboard protocol reply (`CSI ? <flags> u`) out of a
/// combined probe response, returning the currently active flags.
///
/// Only a complete `ESC [ ? <digits> u` sequence counts; malformed input
/// (missing digits, non-digit bytes, wrong final byte) is rejected. The reply
/// may appear anywhere before the DA1 sentinel.
#[cfg(feature = "crossterm")]
pub(super) fn parse_kitty_keyboard_reply(response: &str) -> Option<u32> {
    let mut search = response;
    while let Some(pos) = search.find("\x1b[?") {
        let body = &search[pos + 3..];
        let digits = body.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 && body.as_bytes().get(digits) == Some(&b'u') {
            return body[..digits].parse().ok();
        }
        search = body;
    }
    None
}

/// Parse a cursor-position report (`CSI <row> ; <col> R`), returning the
/// one-based `(row, col)` of the last complete report in `response`.
#[cfg(feature = "crossterm")]
pub(super) fn parse_cursor_position_report(response: &str) -> Option<(u32, u32)> {
    let mut found = None;
    let mut search = response;
    while let Some(pos) = search.find("\x1b[") {
        let body = &search[pos + 2..];
        if let Some(end) = body.find(|c: char| !(c.is_ascii_digit() || c == ';'))
            && body.as_bytes()[end] == b'R'
            && let Some((row, col)) = body[..end].split_once(';')
            && let (Ok(row), Ok(col)) = (row.parse::<u32>(), col.parse::<u32>())
        {
            found = Some((row, col));
        }
        search = body;
    }
    found
}

/// Completion predicate for a cursor-position report.
#[cfg(feature = "crossterm")]
pub(super) fn cpr_reply_complete(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(parse_cursor_position_report)
        .is_some()
}

/// Parse a DA1 reply (`CSI ? <attrs> c`). Attribute `4` indicates Sixel
/// support. Only the DA1 segment is consulted; a trailing DA2 segment in the
/// same string is ignored here.
#[cfg(feature = "crossterm")]
pub(super) fn parse_da1(response: &str, caps: &mut Capabilities) {
    // DA1 reply: ESC [ ? <n> ; <n> ; ... c  (no `>` after `[`). Only a
    // parameter run made of digits and `;` that ends in `c` is a DA1 reply, so
    // a preceding Kitty keyboard reply (`ESC [ ? 4 u`) is never mistaken for
    // attribute `4`.
    let mut search = response;
    while let Some(pos) = search.find("\x1b[?") {
        let body = &search[pos + 3..];
        let end = body
            .find(|c: char| !(c.is_ascii_digit() || c == ';'))
            .unwrap_or(body.len());
        if body.as_bytes().get(end) == Some(&b'c') {
            for attr in body[..end].split(';') {
                if attr == "4" {
                    caps.sixel = true;
                }
            }
        }
        search = &body[end..];
    }
}

/// Parsed DA2 (secondary device attributes) terminal identity:
/// `(primary_id, firmware_version)` from `CSI > <id> ; <ver> ; <sub> c`.
///
/// Returns `None` if the string contains no DA2 reply. Kept separate from the
/// `Capabilities` mutation so it is independently testable and so callers that
/// want the raw identity (e.g. future per-terminal quirks) are not forced
/// through capability inference.
#[cfg(feature = "crossterm")]
pub(super) fn parse_da2(response: &str, _caps: &mut Capabilities) {
    // DA2 identifies terminal compatibility, not graphics support. In
    // particular, xterm and Kitty can both report primary identifier 41.
    let _identity = parse_da2_identity(response);
}

/// Extract `(primary_id, version)` from a DA2 reply, or `None` if absent.
#[cfg(feature = "crossterm")]
pub(super) fn parse_da2_identity(response: &str) -> Option<(u32, u32)> {
    let pos = response.find("\x1b[>")?;
    let body = &response[pos + 3..];
    let end = body.find('c')?;
    let mut parts = body[..end].split(';');
    let id = parts.next()?.trim().parse::<u32>().ok()?;
    let ver = parts.next().and_then(|s| s.trim().parse::<u32>().ok());
    Some((id, ver.unwrap_or(0)))
}

/// Parse a Kitty graphics protocol query ack (`APC G i=31;OK ST`). A terminal
/// that supports the protocol echoes the image id with an `OK` status; anything
/// else (silence, error status) leaves the flag untouched.
#[cfg(feature = "crossterm")]
pub(super) fn parse_kitty_graphics_ack(response: &str, caps: &mut Capabilities) {
    // Ack form: ESC _ G <key=val>;OK ESC \  — we sent i=31, so look for that id
    // paired with an OK status.
    if let Some(pos) = response.find("\x1b_G") {
        let body = &response[pos + 3..];
        let Some(end) = body.find("\x1b\\") else {
            return;
        };
        let payload = &body[..end];
        if let Some((parameters, "OK")) = payload.split_once(';')
            && parameters.split(',').any(|parameter| parameter == "i=31")
        {
            caps.kitty_graphics = true;
        }
    }
}

/// Parse an XTGETTCAP reply for the `Tc` (truecolor) capname. A valid reply is
/// `DCS 1 + r <hex(capname)>[=<hex(value)>] ST`; a leading `1` means the
/// capability is present.
#[cfg(feature = "crossterm")]
pub(super) fn parse_xtgettcap_truecolor(response: &str, caps: &mut Capabilities) {
    // Valid reply prefix: ESC P 1 + r  (DCS 1 + r ...). `Tc` -> hex 5463.
    if let Some(pos) = response.find("\x1bP1+r") {
        let body = &response[pos + 5..];
        if body
            .to_ascii_lowercase()
            .split([';', '\x1b'])
            .any(|seg| seg.starts_with("5463"))
        {
            caps.truecolor = true;
        }
    }
}

/// Tri-state outcome of the DECRQM ?2026 (synchronized output) probe.
///
/// The synchronized-output BSU/ESU emission is gated on this rather than on the
/// public [`Capabilities::sync_output`] bool alone, because the public flag is
/// only ever set on *positive* support evidence. Gating emission on that flag
/// directly would flip the historic always-emit behavior to never-emit on every
/// headless / non-answering host (a regression). This tri-state lets the gate
/// suppress BSU/ESU **only** when the terminal definitively reported the mode
/// unrecognized, and keep emitting in the `Unknown` (silent / headless) case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SyncOutputResolution {
    /// DECRQM confirmed mode ?2026 is recognized (set or reset).
    Supported,
    /// DECRQM explicitly reported mode ?2026 as not recognized (Ps = 0).
    Unsupported,
}

/// Process-global resolution of the synchronized-output probe, populated at most
/// once by [`probe_capabilities`]. Absent (`Unknown`) until the probe answers.
pub(super) static SYNC_OUTPUT_RESOLUTION: std::sync::OnceLock<SyncOutputResolution> =
    std::sync::OnceLock::new();

/// Whether the flush pipeline should wrap a frame in synchronized-output
/// BSU/ESU guards.
///
/// Returns `true` (emit) unless the DECRQM ?2026 probe *definitively* reported
/// the mode as unrecognized. A silent / headless / never-run probe leaves the
/// resolution `Unknown`, in which case this keeps emitting exactly as the
/// pre-gate code always did. This is the behavior-preserving half of the
/// capability gate: positive support and the unknown default both emit; only a
/// confirmed-unsupported terminal suppresses.
pub(super) fn should_emit_synchronized_update() -> bool {
    synchronized_update_allowed(
        terminal_multiplexer(),
        force_env_enabled("SLT_FORCE_SYNC_OUTPUT"),
        force_env_enabled("SLT_DISABLE_SYNC_OUTPUT"),
        matches!(
            SYNC_OUTPUT_RESOLUTION.get(),
            Some(SyncOutputResolution::Unsupported)
        ),
    )
}

pub(super) fn synchronized_update_allowed(
    multiplexer: Option<MultiplexerKind>,
    forced: bool,
    disabled: bool,
    probe_unsupported: bool,
) -> bool {
    !disabled
        && (forced
            || (!probe_unsupported
                && terminal_protocol_allowed(
                    multiplexer,
                    TerminalProtocol::SynchronizedOutput,
                    false,
                    false,
                )))
}

/// Read a DECRPM reply, which terminates with the byte `y` rather than BEL / ST
/// (used for the DECRQM ?2026 synchronized-output probe). Bounded by `timeout`
/// so a terminal that ignores the query cannot stall startup.
#[cfg(feature = "crossterm")]
pub(super) fn read_decrpm_response(timeout: Duration) -> Option<String> {
    read_stdin_reply("\x1b[?2026$p", timeout, decrpm_reply_complete)
}

/// Parse a DECRPM reply for synchronized output (mode `2026`):
/// `CSI ? 2026 ; <Ps> $ y`.
///
/// Returns:
///   * `Some(true)`  — mode recognized (`Ps` ∈ {1, 2, 3, 4}: set / reset /
///     permanently-set / permanently-reset all mean *supported*),
///   * `Some(false)` — mode not recognized (`Ps` = 0),
///   * `None`        — no DECRPM reply for mode 2026 in the string.
#[cfg(feature = "crossterm")]
pub(super) fn parse_decrpm_sync_output(response: &str) -> Option<bool> {
    // Reply body: ESC [ ? 2026 ; <Ps> $ y
    let pos = response.find("\x1b[?2026;")?;
    let body = &response[pos + "\x1b[?2026;".len()..];
    let end = body.find("$y")?;
    let ps = body[..end].trim().parse::<u32>().ok()?;
    // Ps = 0 → not recognized; any other reported state means the mode exists.
    Some(ps != 0)
}
