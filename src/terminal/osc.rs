//! OSC helpers: color-scheme detection (OSC 11), clipboard (OSC 52) and base64.

use super::*;

/// Detected terminal color scheme from OSC 11.
#[non_exhaustive]
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// Dark background detected.
    Dark,
    /// Light background detected.
    Light,
    /// Could not determine the scheme.
    Unknown,
}

/// Read an OSC-style reply (BEL- or ST-terminated), hard-bounded by `timeout`.
#[cfg(feature = "crossterm")]
pub(super) fn read_osc_response(
    request: &str,
    timeout: Duration,
    prefixes: &[&[u8]],
) -> Option<String> {
    read_stdin_reply(request, timeout, |bytes| {
        prefixes.iter().any(|prefix| {
            bytes
                .windows(prefix.len())
                .position(|window| window == *prefix)
                .is_some_and(|start| osc_reply_complete(&bytes[start..]))
        })
    })
}

/// Query the terminal's background color via OSC 11 and return the detected scheme.
/// Returns `Unknown` after the native input reader has started; raw queries
/// cannot safely take over crossterm's private partial-input state.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn detect_color_scheme() -> ColorScheme {
    if !terminal_queries_allowed() {
        return ColorScheme::Unknown;
    }

    let Some(response) =
        read_osc_response("\x1b]11;?\x07", Duration::from_millis(100), &[b"\x1b]11;"])
    else {
        return ColorScheme::Unknown;
    };

    parse_osc11_response(&response)
}

#[cfg(feature = "crossterm")]
pub(crate) fn parse_osc11_response(response: &str) -> ColorScheme {
    let Some(rgb_pos) = response.find("rgb:") else {
        return ColorScheme::Unknown;
    };

    let payload = &response[rgb_pos + 4..];
    let end = payload
        .find(['\x07', '\x1b', '\r', '\n', ' ', '\t'])
        .unwrap_or(payload.len());
    let rgb = &payload[..end];

    let mut channels = rgb.split('/');
    let (Some(r), Some(g), Some(b), None) = (
        channels.next(),
        channels.next(),
        channels.next(),
        channels.next(),
    ) else {
        return ColorScheme::Unknown;
    };

    fn parse_channel(channel: &str) -> Option<f64> {
        if channel.is_empty() || channel.len() > 4 {
            return None;
        }
        let value = u16::from_str_radix(channel, 16).ok()? as f64;
        let max = ((1u32 << (channel.len() * 4)) - 1) as f64;
        if max <= 0.0 {
            return None;
        }
        Some((value / max).clamp(0.0, 1.0))
    }

    let (Some(r), Some(g), Some(b)) = (parse_channel(r), parse_channel(g), parse_channel(b)) else {
        return ColorScheme::Unknown;
    };

    let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
    if luminance < 0.5 {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    }
}

pub(crate) fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            CHARS[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[(triple & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Write a bare `OSC 52` clipboard sequence (no multiplexer wrapping, no
/// size cap) and flush.
#[cfg(test)]
pub(crate) fn copy_to_clipboard(w: &mut impl Write, text: &str) -> io::Result<()> {
    w.write_all(&osc52_sequence(text, None, false))?;
    w.flush()
}

/// Clipboard delivery options taken from [`crate::RunConfig`] (issue #446).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClipboardOptions {
    pub(crate) passthrough: bool,
    pub(crate) max_bytes: usize,
}

impl Default for ClipboardOptions {
    fn default() -> Self {
        Self {
            passthrough: false,
            max_bytes: crate::RunConfig::DEFAULT_CLIPBOARD_MAX_BYTES,
        }
    }
}

/// GNU screen truncates string sequences; chunk the passthrough payload.
pub(super) const SCREEN_DCS_CHUNK: usize = 76;

/// Encode `text` as an `OSC 52` clipboard write, wrapped for multiplexer
/// passthrough when `passthrough` is set and a tmux / screen session is
/// detected. zellij and direct terminals always receive the bare sequence.
pub(super) fn osc52_sequence(
    text: &str,
    multiplexer: Option<MultiplexerKind>,
    passthrough: bool,
) -> Vec<u8> {
    let encoded = base64_encode(text.as_bytes());
    match (passthrough, multiplexer) {
        (true, Some(MultiplexerKind::Tmux)) => {
            // `ESC P tmux; <inner, every ESC doubled> ESC \`
            let inner = format!("\x1b]52;c;{encoded}\x1b\\");
            let mut out = Vec::with_capacity(inner.len() + 12);
            out.extend_from_slice(b"\x1bPtmux;");
            for byte in inner.bytes() {
                if byte == 0x1b {
                    out.push(0x1b);
                }
                out.push(byte);
            }
            out.extend_from_slice(b"\x1b\\");
            out
        }
        (true, Some(MultiplexerKind::Screen)) => {
            // The inner OSC must end in BEL: an inner ST would close the DCS.
            let inner = format!("\x1b]52;c;{encoded}\x07");
            let mut out = Vec::with_capacity(inner.len() + inner.len() / 16 + 8);
            for chunk in inner.as_bytes().chunks(SCREEN_DCS_CHUNK) {
                out.extend_from_slice(b"\x1bP");
                out.extend_from_slice(chunk);
                out.extend_from_slice(b"\x1b\\");
            }
            out
        }
        _ => format!("\x1b]52;c;{encoded}\x1b\\").into_bytes(),
    }
}

/// Write one clipboard payload and report the outcome instead of failing the
/// frame (issue #446). Payloads over `options.max_bytes` are not written.
pub(super) fn write_clipboard(
    w: &mut impl Write,
    text: &str,
    options: ClipboardOptions,
    multiplexer: Option<MultiplexerKind>,
) -> crate::ClipboardStatus {
    if text.len() > options.max_bytes {
        return crate::ClipboardStatus::TooLarge {
            bytes: text.len(),
            max: options.max_bytes,
        };
    }
    let bytes = osc52_sequence(text, multiplexer, options.passthrough);
    match w.write_all(&bytes).and_then(|()| w.flush()) {
        Ok(()) => crate::ClipboardStatus::Sent,
        Err(error) => crate::ClipboardStatus::WriteFailed(error.kind()),
    }
}

#[cfg(feature = "crossterm")]
fn parse_osc52_response(response: &str) -> Option<String> {
    let osc_pos = response.find("]52;")?;
    let body = &response[osc_pos + 4..];
    let semicolon = body.find(';')?;
    let payload = &body[semicolon + 1..];

    let end = payload
        .find("\x1b\\")
        .or_else(|| payload.find('\x07'))
        .unwrap_or(payload.len());
    let encoded = payload[..end].trim();
    if encoded.is_empty() || encoded == "?" {
        return None;
    }

    base64_decode(encoded)
}

/// Read clipboard contents via an OSC 52 terminal query.
///
/// Writes the OSC 52 read request (`ESC ] 52 ; c ; ? BEL`) to stdout, then
/// blocks reading the terminal's reply from stdin for up to ~200 ms. Returns
/// the decoded clipboard text, or `None` if the terminal does not answer, the
/// reply is empty, or it cannot be decoded. Many terminals disable OSC 52 reads
/// by default for security, in which case this always returns `None`.
///
/// # Note
///
/// This standalone fallback is only valid **before any native event reader**
/// has touched stdin. SLT permanently disables raw queries once its native
/// event polling or cursor query starts, returning `None` without writing a
/// request. Draining ready events is insufficient: crossterm can still own a
/// partial UTF-8, key or paste sequence. Direct calls to external crossterm
/// input APIs cannot be tracked by SLT; callers must not invoke this function
/// after those calls or concurrently with another input reader.
///
/// Startup probes preserve interleaved input for SLT's event adapter. For
/// clipboard reads inside a running application, use an OS clipboard API.
/// Clipboard writing is unaffected because it never reads stdin.
#[cfg(feature = "crossterm")]
#[cfg_attr(docsrs, doc(cfg(feature = "crossterm")))]
pub fn read_clipboard() -> Option<String> {
    if !terminal_queries_allowed() {
        return None;
    }

    let response = read_osc_response(
        "\x1b]52;c;?\x07",
        Duration::from_millis(200),
        &[b"\x1b]52;"],
    )?;
    parse_osc52_response(&response)
}

#[cfg(feature = "crossterm")]
pub(super) fn base64_decode(input: &str) -> Option<String> {
    let mut filtered: Vec<u8> = input
        .bytes()
        .filter(|b| !matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
        .collect();

    match filtered.len() % 4 {
        0 => {}
        2 => filtered.extend_from_slice(b"=="),
        3 => filtered.push(b'='),
        _ => return None,
    }

    fn decode_val(b: u8) -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let mut out = Vec::with_capacity((filtered.len() / 4) * 3);
    for chunk in filtered.as_chunks::<4>().0 {
        let p2 = chunk[2] == b'=';
        let p3 = chunk[3] == b'=';
        if p2 && !p3 {
            return None;
        }

        let v0 = decode_val(chunk[0])? as u32;
        let v1 = decode_val(chunk[1])? as u32;
        let v2 = if p2 { 0 } else { decode_val(chunk[2])? as u32 };
        let v3 = if p3 { 0 } else { decode_val(chunk[3])? as u32 };

        let triple = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
        out.push(((triple >> 16) & 0xFF) as u8);
        if !p2 {
            out.push(((triple >> 8) & 0xFF) as u8);
        }
        if !p3 {
            out.push((triple & 0xFF) as u8);
        }
    }

    String::from_utf8(out).ok()
}
