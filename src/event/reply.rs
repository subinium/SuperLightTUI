//! Preserve native input consumed while a synchronous terminal probe owns stdin.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MediaKeyCode,
    ModifierKeyCode, MouseButton, MouseEvent, MouseEventKind,
};
use std::collections::VecDeque;
use std::io;
#[cfg(unix)]
use std::io::Write;
use std::sync::Mutex;
use std::time::Duration;

#[cfg(test)]
mod review_tests;

#[derive(Default)]
struct Replay {
    bytes: Vec<u8>,
    events: VecDeque<Event>,
    native_owned: bool,
}

static REPLAY: Mutex<Replay> = Mutex::new(Replay {
    bytes: Vec::new(),
    events: VecDeque::new(),
    native_owned: false,
});

/// Split only complete, recognizable probe replies. Paste is opaque input,
/// including escape sequences resembling replies inside its delimiters.
pub(crate) fn split(bytes: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut replies = Vec::new();
    let mut input = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let tail = &bytes[offset..];
        if tail.starts_with(b"\x1b[200~") {
            let end = find(&tail[6..], b"\x1b[201~").map_or(tail.len(), |end| end + 12);
            input.extend_from_slice(&tail[..end]);
            offset += end;
        } else if tail[0] == 0x1b {
            match sequence_len(tail) {
                Some(len) => {
                    let sequence = &tail[..len];
                    if is_reply(sequence) {
                        replies.extend_from_slice(sequence);
                    } else {
                        input.extend_from_slice(sequence);
                    }
                    offset += len;
                }
                None => {
                    input.extend_from_slice(tail);
                    break;
                }
            }
        } else {
            input.push(tail[0]);
            offset += 1;
        }
    }
    (replies, input)
}

fn find(bytes: &[u8], needle: &[u8]) -> Option<usize> {
    bytes
        .windows(needle.len())
        .position(|window| window == needle)
}

fn sequence_len(bytes: &[u8]) -> Option<usize> {
    match *bytes.get(1)? {
        b'[' => {
            if bytes.starts_with(b"\x1b[M") {
                return (bytes.len() >= 6).then_some(6);
            }
            bytes
                .iter()
                .enumerate()
                .skip(2)
                .find(|(_, byte)| (0x40..=0x7e).contains(*byte))
                .map(|(index, _)| index + 1)
        }
        b']' | b'P' | b'_' => bytes.iter().enumerate().skip(2).find_map(|(index, byte)| {
            if *byte == 7 || (*byte == b'\\' && bytes[index - 1] == 0x1b) {
                Some(index + 1)
            } else {
                None
            }
        }),
        b'O' => (bytes.len() >= 3).then_some(3),
        _ => Some(1),
    }
}

fn numeric_fields(body: &str) -> bool {
    !body.is_empty()
        && body
            .split(';')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

fn reply_prefix(bytes: &[u8]) -> bool {
    [
        b"\x1b[6;".as_slice(),
        b"\x1b[?",
        b"\x1b[>",
        b"\x1b]11;",
        b"\x1b]52;",
        b"\x1b_Gi=31",
        b"\x1bP1+r5463",
        b"\x1bP0+r5463",
    ]
    .iter()
    .any(|prefix| bytes.starts_with(prefix))
}

fn is_reply(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    if let Some(body) = text.strip_prefix("\x1b[") {
        if let Some(fields) = body
            .strip_suffix('c')
            .and_then(|body| body.strip_prefix(['?', '>']))
        {
            return numeric_fields(fields);
        }
        if let Some(fields) = body
            .strip_prefix("6;")
            .and_then(|body| body.strip_suffix('t'))
        {
            return fields.split(';').count() == 2 && numeric_fields(fields);
        }
        if let Some(state) = body
            .strip_prefix("?2026;")
            .and_then(|body| body.strip_suffix("$y"))
        {
            return numeric_fields(state);
        }
        return false;
    }
    let body = text
        .strip_suffix("\x1b\\")
        .or_else(|| text.strip_suffix('\x07'));
    body.is_some_and(|body| {
        body.starts_with("\x1b]11;rgb:")
            || body.starts_with("\x1b]52;")
            || body
                .strip_prefix("\x1b_G")
                .and_then(|body| body.split_once(';'))
                .is_some_and(|(parameters, _)| parameters.split(',').any(|part| part == "i=31"))
            || body
                .strip_prefix("\x1bP")
                .is_some_and(|body| body.starts_with("1+r5463") || body.starts_with("0+r5463"))
    })
}

/// Raw probing is startup-only. Once crossterm may hold an event or partial
/// sequence, ownership never returns to the raw reader during this process.
#[cfg(unix)]
pub(crate) fn query(
    request: &str,
    timeout: Duration,
    mut complete: impl FnMut(&[u8]) -> bool,
) -> Option<String> {
    let mut replay = REPLAY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if replay.native_owned {
        return None;
    }
    let mut out = io::stdout();
    out.write_all(request.as_bytes()).ok()?;
    out.flush().ok()?;
    let prefix = replay.bytes.clone();
    let bytes = crate::terminal::read_pending_input(timeout, |bytes| {
        let mut combined = prefix.clone();
        combined.extend_from_slice(bytes);
        let (replies, _) = split(&combined);
        complete(&replies)
    });
    let replies = replay.demultiplex(&bytes);
    (!replies.is_empty())
        .then(|| String::from_utf8(replies).ok())
        .flatten()
}

#[cfg(not(unix))]
pub(crate) fn query(
    _request: &str,
    _timeout: Duration,
    _complete: impl FnMut(&[u8]) -> bool,
) -> Option<String> {
    None
}

pub(crate) fn cursor_position() -> io::Result<(u16, u16)> {
    let mut replay = REPLAY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !replay.bytes.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "unfinished input before native cursor query",
        ));
    }
    replay.native_owned = true;
    drop(replay);
    #[cfg(unix)]
    return super::native::cursor_position();
    #[cfg(not(unix))]
    crossterm::cursor::position()
}

pub(crate) fn release_input() {
    #[cfg(unix)]
    super::native::release();
}

pub(crate) fn prepare_input() -> io::Result<()> {
    #[cfg(unix)]
    super::native::prepare()?;
    Ok(())
}

impl Replay {
    #[cfg(any(unix, test))]
    fn demultiplex(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.bytes.extend_from_slice(bytes);
        let (replies, input) = split(&self.bytes);
        self.bytes = input;
        self.decode(false);
        replies
    }
    fn decode(&mut self, expired: bool) {
        let mut offset = 0;
        while offset < self.bytes.len() {
            let bytes = &self.bytes[offset..];
            if bytes.starts_with(b"\x1b[200~") {
                if let Some(end) = find(&bytes[6..], b"\x1b[201~") {
                    self.events.push_back(Event::Paste(
                        String::from_utf8_lossy(&bytes[6..6 + end]).into_owned(),
                    ));
                    offset += end + 12;
                    continue;
                }
                // Preserve a split paste across query deadlines and event polls.
                if bytes.len() < 4 * 1024 * 1024 {
                    break;
                }
                self.events.push_back(Event::Paste(
                    String::from_utf8_lossy(&bytes[6..]).into_owned(),
                ));
                offset = self.bytes.len();
                break;
            }
            if bytes[0] == 0x1b
                && let Some(len) = sequence_len(bytes)
                && len > 1
            {
                if is_reply(&bytes[..len]) {
                    offset += len;
                    continue;
                }
                if let Some(event) = decode_sequence(&bytes[..len]) {
                    self.events.push_back(event);
                    offset += len;
                    continue;
                }
            }
            if bytes[0] == 0x1b
                && sequence_len(bytes).is_none()
                && (!expired || (reply_prefix(bytes) && bytes.len() < 4096))
            {
                break;
            }
            let (event, len) = match decode_key(bytes) {
                Some(key) => key,
                None => break,
            };
            self.events.push_back(event);
            offset += len;
        }
        self.bytes.drain(..offset);
    }
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

fn decode_key(bytes: &[u8]) -> Option<(Event, usize)> {
    let byte = bytes[0];
    let code = match byte {
        b'\t' => KeyCode::Tab,
        b'\r' | b'\n' => KeyCode::Enter,
        0x7f => KeyCode::Backspace,
        0x1b => {
            if let Some(next) = bytes.get(1)
                && !matches!(next, b'[' | b']' | b'P' | b'_' | b'O' | 0x1b)
                && let Some((Event::Key(mut event), len)) = decode_key(&bytes[1..])
            {
                event.modifiers.insert(KeyModifiers::ALT);
                return Some((Event::Key(event), len + 1));
            }
            KeyCode::Esc
        }
        0..=26 => {
            return Some((
                key(
                    KeyCode::Char(if byte == 0 {
                        ' '
                    } else {
                        (b'a' + byte - 1) as char
                    }),
                    KeyModifiers::CONTROL,
                ),
                1,
            ));
        }
        28..=31 => {
            return Some((
                key(
                    KeyCode::Char((b'\\' + byte - 28) as char),
                    KeyModifiers::CONTROL,
                ),
                1,
            ));
        }
        _ => {
            let len = match byte {
                0xc2..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf4 => 4,
                _ => 1,
            };
            if bytes.len() < len {
                return None;
            }
            let ch = bytes
                .get(..len)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .and_then(|text| text.chars().next());
            return Some((
                key(KeyCode::Char(ch.unwrap_or('\u{fffd}')), KeyModifiers::NONE),
                if ch.is_some() { len } else { 1 },
            ));
        }
    };
    Some((key(code, KeyModifiers::NONE), 1))
}

fn modifiers(value: u32) -> KeyModifiers {
    let value = value.saturating_sub(1);
    let mut result = KeyModifiers::NONE;
    for (bit, modifier) in [
        (1, KeyModifiers::SHIFT),
        (2, KeyModifiers::ALT),
        (4, KeyModifiers::CONTROL),
        (8, KeyModifiers::SUPER),
        (16, KeyModifiers::HYPER),
        (32, KeyModifiers::META),
    ] {
        if value & bit != 0 {
            result.insert(modifier);
        }
    }
    result
}

fn functional_key(value: u32) -> Option<(KeyCode, KeyEventState)> {
    use KeyCode as K;
    let code = match value {
        57358 => K::CapsLock,
        57359 => K::ScrollLock,
        57360 => K::NumLock,
        57361 => K::PrintScreen,
        57362 => K::Pause,
        57363 => K::Menu,
        57376..=57398 => K::F((value - 57376 + 13) as u8),
        57399..=57408 => K::Char((b'0' + (value - 57399) as u8) as char),
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
                MediaKeyCode::Play,
                MediaKeyCode::Pause,
                MediaKeyCode::PlayPause,
                MediaKeyCode::Reverse,
                MediaKeyCode::Stop,
                MediaKeyCode::FastForward,
                MediaKeyCode::Rewind,
                MediaKeyCode::TrackNext,
                MediaKeyCode::TrackPrevious,
                MediaKeyCode::Record,
                MediaKeyCode::LowerVolume,
                MediaKeyCode::RaiseVolume,
                MediaKeyCode::MuteVolume,
            ][(value - 57428) as usize],
        ),
        57441..=57454 => K::Modifier(
            [
                ModifierKeyCode::LeftShift,
                ModifierKeyCode::LeftControl,
                ModifierKeyCode::LeftAlt,
                ModifierKeyCode::LeftSuper,
                ModifierKeyCode::LeftHyper,
                ModifierKeyCode::LeftMeta,
                ModifierKeyCode::RightShift,
                ModifierKeyCode::RightControl,
                ModifierKeyCode::RightAlt,
                ModifierKeyCode::RightSuper,
                ModifierKeyCode::RightHyper,
                ModifierKeyCode::RightMeta,
                ModifierKeyCode::IsoLevel3Shift,
                ModifierKeyCode::IsoLevel5Shift,
            ][(value - 57441) as usize],
        ),
        _ => return None,
    };
    Some((
        code,
        if (57399..=57427).contains(&value) {
            KeyEventState::KEYPAD
        } else {
            KeyEventState::NONE
        },
    ))
}

fn decode_csi_u(body: &str) -> Option<Event> {
    let mut fields = body.split(';');
    let mut codepoints = fields.next()?.split(':');
    let point = codepoints.next()?.parse::<u32>().ok()?;
    let mut modifier_fields = fields.next().unwrap_or("1").split(':');
    let mask = modifier_fields.next()?.parse::<u8>().ok()?;
    let mut mods = modifiers(u32::from(mask));
    let kind = match modifier_fields.next() {
        Some("2") => KeyEventKind::Repeat,
        Some("3") => KeyEventKind::Release,
        _ => KeyEventKind::Press,
    };
    let (mut code, mut state) = if let Some(functional) = functional_key(point) {
        functional
    } else {
        let ch = char::from_u32(point)?;
        let code = match ch {
            '\x1b' => KeyCode::Esc,
            '\r' => KeyCode::Enter,
            '\n' if !crossterm::terminal::is_raw_mode_enabled().unwrap_or(false) => KeyCode::Enter,
            '\t' if mods.contains(KeyModifiers::SHIFT) => KeyCode::BackTab,
            '\t' => KeyCode::Tab,
            '\x7f' => KeyCode::Backspace,
            ch => KeyCode::Char(ch),
        };
        (code, KeyEventState::NONE)
    };
    if let KeyCode::Modifier(modifier) = code {
        let implied = match modifier {
            ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift => KeyModifiers::SHIFT,
            ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl => KeyModifiers::CONTROL,
            ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt => KeyModifiers::ALT,
            ModifierKeyCode::LeftSuper | ModifierKeyCode::RightSuper => KeyModifiers::SUPER,
            ModifierKeyCode::LeftHyper | ModifierKeyCode::RightHyper => KeyModifiers::HYPER,
            ModifierKeyCode::LeftMeta | ModifierKeyCode::RightMeta => KeyModifiers::META,
            _ => KeyModifiers::NONE,
        };
        mods.insert(implied);
    }
    if mods.contains(KeyModifiers::SHIFT)
        && let Some(shifted) = codepoints
            .next()
            .and_then(|point| point.parse::<u32>().ok())
            .and_then(char::from_u32)
    {
        code = KeyCode::Char(shifted);
        mods.remove(KeyModifiers::SHIFT);
    }
    if mask.saturating_sub(1) & 64 != 0 {
        state.insert(KeyEventState::CAPS_LOCK);
    }
    if mask.saturating_sub(1) & 128 != 0 {
        state.insert(KeyEventState::NUM_LOCK);
    }
    Some(Event::Key(KeyEvent::new_with_kind_and_state(
        code, mods, kind, state,
    )))
}

fn decode_sequence(bytes: &[u8]) -> Option<Event> {
    if bytes == b"\x1b[I" {
        return Some(Event::FocusGained);
    }
    if bytes == b"\x1b[O" {
        return Some(Event::FocusLost);
    }
    let text = std::str::from_utf8(bytes).ok()?;
    let body = text
        .strip_prefix("\x1b[")
        .or_else(|| text.strip_prefix("\x1bO"))?;
    let last = body.as_bytes().last().copied()?;
    if last == b'u' {
        return decode_csi_u(&body[..body.len() - 1]);
    }
    let fields: Vec<&str> = body[..body.len() - 1].split(';').collect();
    if body.starts_with('<') && matches!(last, b'M' | b'm') {
        let code = fields[0].strip_prefix('<')?.parse::<u32>().ok()?;
        let x = fields.get(1)?.parse::<u16>().ok()?.checked_sub(1)?;
        let y = fields.get(2)?.parse::<u16>().ok()?.checked_sub(1)?;
        let button = match code & 3 {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            _ => MouseButton::Right,
        };
        let kind = if code & 64 != 0 {
            match code & 3 {
                0 => MouseEventKind::ScrollUp,
                1 => MouseEventKind::ScrollDown,
                2 => MouseEventKind::ScrollLeft,
                _ => MouseEventKind::ScrollRight,
            }
        } else if last == b'm' {
            MouseEventKind::Up(button)
        } else if code & 32 != 0 {
            if code & 3 == 3 {
                MouseEventKind::Moved
            } else {
                MouseEventKind::Drag(button)
            }
        } else {
            MouseEventKind::Down(button)
        };
        let mut mods = KeyModifiers::NONE;
        if code & 4 != 0 {
            mods.insert(KeyModifiers::SHIFT);
        }
        if code & 8 != 0 {
            mods.insert(KeyModifiers::ALT);
        }
        if code & 16 != 0 {
            mods.insert(KeyModifiers::CONTROL);
        }
        return Some(Event::Mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: mods,
        }));
    }
    let modifier_fields: Vec<&str> = fields.get(1).copied().unwrap_or("1").split(':').collect();
    let mods = modifiers(modifier_fields[0].parse().ok()?);
    let first = fields[0].split(':').next().unwrap_or("");
    let code = match last {
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        b'H' => KeyCode::Home,
        b'F' => KeyCode::End,
        b'Z' => return Some(key(KeyCode::BackTab, KeyModifiers::SHIFT)),
        b'P' => KeyCode::F(1),
        b'Q' => KeyCode::F(2),
        b'R' => KeyCode::F(3),
        b'S' => KeyCode::F(4),
        b'~' => match first.parse::<u32>().ok()? {
            1 | 7 => KeyCode::Home,
            2 => KeyCode::Insert,
            3 => KeyCode::Delete,
            4 | 8 => KeyCode::End,
            5 => KeyCode::PageUp,
            6 => KeyCode::PageDown,
            11..=15 => KeyCode::F(first.parse::<u8>().ok()? - 10),
            17..=21 => KeyCode::F(first.parse::<u8>().ok()? - 11),
            23..=24 => KeyCode::F(first.parse::<u8>().ok()? - 12),
            _ => return None,
        },
        _ => return None,
    };
    let mut event = KeyEvent::new(code, mods);
    event.kind = match modifier_fields.get(1).copied() {
        Some("2") => KeyEventKind::Repeat,
        Some("3") => KeyEventKind::Release,
        _ => KeyEventKind::Press,
    };
    Some(Event::Key(event))
}

pub(crate) fn poll(timeout: Duration) -> io::Result<bool> {
    #[cfg(unix)]
    super::native::check_failure()?;
    let mut replay = REPLAY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !replay.events.is_empty() {
        return Ok(true);
    }
    if !replay.bytes.is_empty() {
        #[cfg(unix)]
        {
            let prefix = replay.bytes.clone();
            let incoming = crate::terminal::read_pending_input(
                timeout.min(Duration::from_millis(25)),
                |bytes| {
                    let mut probe = Replay {
                        bytes: prefix.clone(),
                        events: VecDeque::new(),
                        native_owned: false,
                    };
                    probe.bytes.extend_from_slice(bytes);
                    probe.decode(false);
                    !probe.events.is_empty()
                },
            );
            replay.bytes.extend_from_slice(&incoming);
        }
        replay.decode(!timeout.is_zero());
        if !replay.events.is_empty() {
            return Ok(true);
        }
        // An unfinished paste remains owned by the raw replay path.
        if !replay.bytes.is_empty() {
            return Ok(false);
        }
    }
    replay.native_owned = true;
    drop(replay);
    #[cfg(unix)]
    return super::native::poll(timeout);
    #[cfg(not(unix))]
    crossterm::event::poll(timeout)
}

pub(crate) fn read() -> io::Result<Event> {
    #[cfg(unix)]
    super::native::check_failure()?;
    loop {
        let mut replay = REPLAY
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(event) = replay.events.pop_front() {
            return Ok(event);
        }
        if replay.bytes.is_empty() {
            replay.native_owned = true;
            drop(replay);
            #[cfg(unix)]
            return super::native::read();
            #[cfg(not(unix))]
            return crossterm::event::read();
        }
        drop(replay);
        poll(Duration::from_millis(25))?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies_do_not_consume_keys_paste_or_focus() {
        let bytes = b"a\x1b[I\x1b[?62;4c\x1b[200~paste\x1b[>41;1;0c\x1b[201~\x1b[>41;1;0cb\x1b[O";
        let (replies, input) = split(bytes);
        assert_eq!(replies, b"\x1b[?62;4c\x1b[>41;1;0c");
        let mut replay = Replay {
            bytes: input,
            events: VecDeque::new(),
            native_owned: false,
        };
        replay.decode(false);
        assert_eq!(
            replay.events,
            VecDeque::from([
                key(KeyCode::Char('a'), KeyModifiers::NONE),
                Event::FocusGained,
                Event::Paste("paste\x1b[>41;1;0c".into()),
                key(KeyCode::Char('b'), KeyModifiers::NONE),
                Event::FocusLost,
            ])
        );
    }

    #[test]
    fn split_utf8_paste_and_arrows_remain_ordered() {
        let mut replay = Replay::default();
        for bytes in [
            b"\xe7".as_slice(),
            b"\x95\x8c\x1b[",
            b"1;5D\x1b[200~hello",
            b"\x1b[201~",
        ] {
            replay.bytes.extend_from_slice(bytes);
            replay.decode(false);
        }
        assert_eq!(
            replay.events,
            VecDeque::from([
                key(KeyCode::Char('\u{754c}'), KeyModifiers::NONE),
                key(KeyCode::Left, KeyModifiers::CONTROL),
                Event::Paste("hello".into()),
            ])
        );
        assert!(replay.bytes.is_empty());
    }

    #[test]
    fn ordinary_terminators_and_malformed_replies_are_input() {
        for bytes in [
            b"cty\x07".as_slice(),
            b"\x1b[6;x;2t",
            b"\x1b_Gi=310;OK\x1b\\",
            b"\x1b[?62;",
        ] {
            assert_eq!(split(bytes), (Vec::new(), bytes.to_vec()));
        }
    }

    #[test]
    fn late_split_reply_is_not_replayed_as_keys() {
        let mut replay = Replay {
            bytes: b"\x1b[6;16;".to_vec(),
            events: VecDeque::new(),
            native_owned: false,
        };
        replay.decode(true);
        assert!(replay.events.is_empty());
        replay.bytes.extend_from_slice(b"8tZ");
        replay.decode(false);
        assert_eq!(
            replay.events,
            VecDeque::from([key(KeyCode::Char('Z'), KeyModifiers::NONE)])
        );
    }
}
