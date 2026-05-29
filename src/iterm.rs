//! iTerm2 OSC 1337 `File=` inline-image protocol encoder (issue #265).
//!
//! Unlike Sixel (which streams quantized raster rows) or Kitty (which uploads
//! raw RGBA), the iTerm2 protocol hands the terminal an **encoded image file**
//! (PNG/JPEG/GIF bytes) and lets the terminal decode and scale it. This is the
//! only pixel-accurate path on Tabby, on older iTerm2 builds that predate Kitty
//! graphics, and on WezTerm's iTerm2-compat mode.
//!
//! The wire form is:
//!
//! ```text
//! ESC ] 1337 ; File = inline=1 ; size=<bytes> ; width=<cols> ; height=<rows>
//!     ; preserveAspectRatio=<0|1> : <base64 payload> BEL
//! ```

/// Maximum encoded image-file payload accepted by [`encode_iterm_osc1337`], in
/// bytes. Mirrors the spirit of [`crate::buffer::MAX_IMAGE_PIXELS`]: a hostile
/// or accidental multi-megabyte payload should fall back to a placeholder
/// rather than flooding the terminal. 16 MiB comfortably covers any sane
/// inline screenshot while bounding the worst-case flush cost.
pub(crate) const MAX_ITERM_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;

/// Encode an iTerm2 OSC 1337 `File=` inline-image sequence.
///
/// `data` is **encoded image-file bytes** (PNG/JPEG/GIF), not raw RGBA. `cols`
/// and `rows` size the image in terminal cells. When `preserve_aspect` is true
/// the terminal keeps the source aspect ratio within the cell box
/// (`preserveAspectRatio=1`); otherwise it stretches to fill (`=0`).
///
/// Returns an empty string for empty input or a payload above
/// [`MAX_ITERM_PAYLOAD_BYTES`], so callers can treat an empty result as a
/// no-op and draw a placeholder.
pub(crate) fn encode_iterm_osc1337(
    data: &[u8],
    cols: u32,
    rows: u32,
    preserve_aspect: bool,
) -> String {
    if data.is_empty() || data.len() > MAX_ITERM_PAYLOAD_BYTES {
        return String::new();
    }

    let b64 = crate::terminal::base64_encode(data);
    let preserve = if preserve_aspect { 1 } else { 0 };
    let size = data.len();

    // `height=auto` is the documented value for "derive height from width +
    // aspect"; the caller signals it by passing `rows == 0`.
    let height = if rows == 0 {
        "auto".to_string()
    } else {
        rows.to_string()
    };

    format!(
        "\x1b]1337;File=inline=1;size={size};width={cols};height={height};preserveAspectRatio={preserve}:{b64}\x07"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc1337_well_formed_header() {
        let seq = encode_iterm_osc1337(b"PNGDATA", 20, 4, false);
        assert!(seq.starts_with("\x1b]1337;File="));
        assert!(seq.contains("inline=1"));
        assert!(seq.contains("size=7"));
        assert!(seq.contains("width=20"));
        assert!(seq.contains("height=4"));
        assert!(seq.contains("preserveAspectRatio=0"));
        assert!(seq.ends_with('\x07'));
    }

    #[test]
    fn osc1337_payload_is_base64() {
        let seq = encode_iterm_osc1337(b"Hello", 1, 1, false);
        // `base64_encode(b"Hello") == "SGVsbG8="` (validated in terminal tests).
        assert!(seq.contains(":SGVsbG8=\x07"));
    }

    #[test]
    fn osc1337_preserve_aspect_and_auto_height() {
        let seq = encode_iterm_osc1337(b"x", 10, 0, true);
        assert!(seq.contains("preserveAspectRatio=1"));
        assert!(seq.contains("height=auto"));
        assert!(!seq.contains("height=0"));
    }

    #[test]
    fn osc1337_empty_input_returns_empty() {
        assert!(encode_iterm_osc1337(&[], 4, 4, false).is_empty());
    }

    #[test]
    fn osc1337_oversized_payload_returns_empty() {
        let huge = vec![0u8; MAX_ITERM_PAYLOAD_BYTES + 1];
        assert!(encode_iterm_osc1337(&huge, 4, 4, false).is_empty());
    }

    #[test]
    fn osc1337_size_matches_byte_length() {
        let data = vec![1u8; 300];
        let seq = encode_iterm_osc1337(&data, 5, 2, false);
        assert!(seq.contains("size=300"));
    }

    #[test]
    fn iterm_image_on_test_backend_does_not_panic() {
        // Minimal fake PNG header bytes — content is irrelevant to the encoder.
        let png = [0x89u8, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut tb = crate::TestBackend::new(20, 4);
        tb.render(|ui| {
            let _ = ui.iterm_image(&png, 20, 4);
        });
    }

    #[test]
    fn iterm_image_on_test_backend_renders_fallback() {
        // A headless backend has no real TTY, so detection fails and the path
        // must degrade to the placeholder rather than emitting OSC bytes.
        let png = [0x89u8, b'P', b'N', b'G'];
        let mut tb = crate::TestBackend::new(20, 2);
        tb.render(|ui| {
            let _ = ui.iterm_image(&png, 20, 2);
        });
        tb.assert_contains("[iterm2 unsupported]");
    }

    #[test]
    fn iterm_image_fit_on_test_backend_renders_fallback() {
        let png = [0x89u8, b'P', b'N', b'G'];
        let mut tb = crate::TestBackend::new(20, 10);
        tb.render(|ui| {
            let _ = ui.iterm_image_fit(&png, 20);
        });
        tb.assert_contains("[iterm2 unsupported]");
    }
}
