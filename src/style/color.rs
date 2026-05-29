/// Terminal color.
///
/// Covers the standard 16 named colors, 256-color palette indices, and
/// 24-bit RGB true color. Use [`Color::Reset`] to restore the terminal's
/// default foreground or background.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    /// Reset to the terminal's default color.
    Reset,
    /// Standard black (color index 0).
    Black,
    /// Standard red (color index 1).
    Red,
    /// Standard green (color index 2).
    Green,
    /// Standard yellow (color index 3).
    Yellow,
    /// Standard blue (color index 4).
    Blue,
    /// Standard magenta (color index 5).
    Magenta,
    /// Standard cyan (color index 6).
    Cyan,
    /// Standard white (color index 7).
    White,
    /// Bright black / dark gray (color index 8).
    DarkGray,
    /// Bright red (color index 9).
    LightRed,
    /// Bright green (color index 10).
    LightGreen,
    /// Bright yellow (color index 11).
    LightYellow,
    /// Bright blue (color index 12).
    LightBlue,
    /// Bright magenta (color index 13).
    LightMagenta,
    /// Bright cyan (color index 14).
    LightCyan,
    /// Bright white (color index 15).
    LightWhite,
    /// 24-bit true color.
    Rgb(u8, u8, u8),
    /// 256-color palette index.
    Indexed(u8),
}

#[inline]
fn to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

impl Color {
    /// Resolve to `(r, g, b)` for luminance and blending operations.
    ///
    /// Named colors map to their typical terminal palette values.
    /// [`Color::Reset`] maps to black; [`Color::Indexed`] maps to the xterm-256 palette.
    fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            Color::Rgb(r, g, b) => (r, g, b),
            Color::Black => (0, 0, 0),
            Color::Red => (205, 49, 49),
            Color::Green => (13, 188, 121),
            Color::Yellow => (229, 229, 16),
            Color::Blue => (36, 114, 200),
            Color::Magenta => (188, 63, 188),
            Color::Cyan => (17, 168, 205),
            Color::White => (229, 229, 229),
            Color::DarkGray => (128, 128, 128),
            Color::LightRed => (255, 0, 0),
            Color::LightGreen => (0, 255, 0),
            Color::LightYellow => (255, 255, 0),
            Color::LightBlue => (0, 0, 255),
            Color::LightMagenta => (255, 0, 255),
            Color::LightCyan => (0, 255, 255),
            Color::LightWhite => (255, 255, 255),
            Color::Reset => (0, 0, 0),
            Color::Indexed(idx) => xterm256_to_rgb(idx),
        }
    }

    /// Compute relative luminance using ITU-R BT.709 coefficients.
    ///
    /// Returns a value in `[0.0, 1.0]` where 0 is darkest and 1 is brightest.
    /// Use this to determine whether text on a given background should be
    /// light or dark.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::Color;
    ///
    /// let dark = Color::Rgb(30, 30, 46);
    /// assert!(dark.luminance() < 0.15);
    ///
    /// let light = Color::Rgb(205, 214, 244);
    /// assert!(light.luminance() > 0.6);
    /// ```
    pub fn luminance(self) -> f32 {
        let (r, g, b) = self.to_rgb();
        let rf = to_linear(r as f32 / 255.0);
        let gf = to_linear(g as f32 / 255.0);
        let bf = to_linear(b as f32 / 255.0);
        0.2126 * rf + 0.7152 * gf + 0.0722 * bf
    }

    /// Return a contrasting foreground color for the given background.
    ///
    /// Uses the WCAG 2.1 relative luminance threshold (0.179) to decide
    /// between white and black text. For theme-aware contrast, prefer using
    /// this over hardcoding `theme.bg` as the foreground.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::Color;
    ///
    /// let bg = Color::Rgb(189, 147, 249); // Dracula purple
    /// let fg = Color::contrast_fg(bg);
    /// // Dracula purple → white (WCAG luminance 0.385 < 0.179 threshold)
    /// ```
    pub fn contrast_fg(bg: Color) -> Color {
        if bg.luminance() > 0.179 {
            Color::Rgb(0, 0, 0)
        } else {
            Color::Rgb(255, 255, 255)
        }
    }

    /// Blend this color over another with the given alpha.
    ///
    /// `alpha` is in `[0.0, 1.0]` where 0.0 returns `other` unchanged and
    /// 1.0 returns `self` unchanged. Both colors are resolved to RGB.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::Color;
    ///
    /// let white = Color::Rgb(255, 255, 255);
    /// let black = Color::Rgb(0, 0, 0);
    /// let gray = white.blend(black, 0.5);
    /// // ≈ Rgb(128, 128, 128)
    /// ```
    pub fn blend(self, other: Color, alpha: f32) -> Color {
        let alpha = alpha.clamp(0.0, 1.0);
        let (r1, g1, b1) = self.to_rgb();
        let (r2, g2, b2) = other.to_rgb();
        let r = (r1 as f32 * alpha + r2 as f32 * (1.0 - alpha)).round() as u8;
        let g = (g1 as f32 * alpha + g2 as f32 * (1.0 - alpha)).round() as u8;
        let b = (b1 as f32 * alpha + b2 as f32 * (1.0 - alpha)).round() as u8;
        Color::Rgb(r, g, b)
    }

    /// Lighten this color by the given amount (0.0–1.0).
    ///
    /// Blends toward white. `amount = 0.0` returns the original color;
    /// `amount = 1.0` returns white.
    pub fn lighten(self, amount: f32) -> Color {
        Color::Rgb(255, 255, 255).blend(self, 1.0 - amount.clamp(0.0, 1.0))
    }

    /// Darken this color by the given amount (0.0–1.0).
    ///
    /// Blends toward black. `amount = 0.0` returns the original color;
    /// `amount = 1.0` returns black.
    pub fn darken(self, amount: f32) -> Color {
        Color::Rgb(0, 0, 0).blend(self, 1.0 - amount.clamp(0.0, 1.0))
    }

    /// Compute the WCAG 2.1 contrast ratio between two colors.
    ///
    /// Returns a value >= 1.0. A ratio >= 4.5 meets WCAG AA for normal text;
    /// >= 3.0 meets AA for large text.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::Color;
    ///
    /// let ratio = Color::contrast_ratio(Color::White, Color::Black);
    /// assert!(ratio > 15.0);
    /// ```
    pub fn contrast_ratio(a: Color, b: Color) -> f32 {
        let la = a.luminance() + 0.05;
        let lb = b.luminance() + 0.05;
        if la > lb {
            la / lb
        } else {
            lb / la
        }
    }

    /// Returns `true` if the contrast ratio between two colors meets WCAG AA
    /// for normal text (ratio >= 4.5).
    pub fn meets_contrast_aa(fg: Color, bg: Color) -> bool {
        Self::contrast_ratio(fg, bg) >= 4.5
    }

    /// Downsample this color to fit the given color depth.
    ///
    /// - `TrueColor`: returns self unchanged.
    /// - `EightBit`: converts `Rgb` to the nearest `Indexed` color.
    /// - `Basic`: converts `Rgb` and `Indexed` to the nearest named color.
    /// - `NoColor`: returns [`Color::Reset`] — emit no ANSI color at all.
    ///
    /// Named colors (`Red`, `Green`, etc.) and `Reset` pass through at
    /// depths other than `NoColor`.
    pub fn downsampled(self, depth: ColorDepth) -> Color {
        match depth {
            ColorDepth::TrueColor => self,
            ColorDepth::EightBit => match self {
                Color::Rgb(r, g, b) => Color::Indexed(rgb_to_ansi256(r, g, b)),
                other => other,
            },
            ColorDepth::Basic => match self {
                Color::Rgb(r, g, b) => rgb_to_ansi16(r, g, b),
                Color::Indexed(i) => {
                    let (r, g, b) = xterm256_to_rgb(i);
                    rgb_to_ansi16(r, g, b)
                }
                other => other,
            },
            ColorDepth::NoColor => Color::Reset,
        }
    }

    /// Parse a hex string (`#rgb` or `#rrggbb`) into [`Color::Rgb`].
    ///
    /// The leading `#` is required. Short form `#rgb` expands each nibble
    /// (`#abc` → `Rgb(0xaa, 0xbb, 0xcc)`). Returns `None` for any malformed
    /// input (wrong length, non-hex digits, missing `#`).
    ///
    /// # Example
    ///
    /// ```
    /// use slt::Color;
    ///
    /// assert_eq!(Color::from_hex("#ff6b6b"), Some(Color::Rgb(255, 107, 107)));
    /// assert_eq!(Color::from_hex("#abc"), Some(Color::Rgb(170, 187, 204)));
    /// assert_eq!(Color::from_hex("ff6b6b"), None); // missing '#'
    /// assert_eq!(Color::from_hex("#xyz"), None); // non-hex
    /// ```
    #[doc(alias = "parse")]
    pub fn from_hex(s: &str) -> Option<Color> {
        let hex = s.strip_prefix('#')?;
        match hex.len() {
            3 => {
                let mut it = hex.chars().map(|c| c.to_digit(16));
                let r = it.next()??;
                let g = it.next()??;
                let b = it.next()??;
                // Expand each nibble: 0xa -> 0xaa.
                Some(Color::Rgb((r * 17) as u8, (g * 17) as u8, (b * 17) as u8))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Color::Rgb(r, g, b))
            }
            _ => None,
        }
    }

    /// Format an `Rgb` color as a `#rrggbb` hex string.
    ///
    /// Non-`Rgb` variants are first resolved to their RGB equivalent via the
    /// internal palette, so the result is always a valid `#rrggbb` token.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::Color;
    ///
    /// assert_eq!(Color::Rgb(255, 107, 107).to_hex(), "#ff6b6b");
    /// ```
    pub fn to_hex(self) -> String {
        let (r, g, b) = self.to_rgb();
        format!("#{r:02x}{g:02x}{b:02x}")
    }
}

#[cfg(feature = "serde")]
impl Color {
    /// Serialized token for a named color, or `None` for non-named variants.
    fn named_token(self) -> Option<&'static str> {
        Some(match self {
            Color::Reset => "reset",
            Color::Black => "black",
            Color::Red => "red",
            Color::Green => "green",
            Color::Yellow => "yellow",
            Color::Blue => "blue",
            Color::Magenta => "magenta",
            Color::Cyan => "cyan",
            Color::White => "white",
            Color::DarkGray => "darkgray",
            Color::LightRed => "lightred",
            Color::LightGreen => "lightgreen",
            Color::LightYellow => "lightyellow",
            Color::LightBlue => "lightblue",
            Color::LightMagenta => "lightmagenta",
            Color::LightCyan => "lightcyan",
            Color::LightWhite => "lightwhite",
            Color::Rgb(..) | Color::Indexed(_) => return None,
        })
    }

    /// Parse a color from a human-friendly token used in theme files.
    ///
    /// Accepts `#rgb` / `#rrggbb` hex, named colors (case-insensitive, e.g.
    /// `"cyan"`, `"lightblue"`, `"darkgray"`, `"reset"`), and `indexed:N`
    /// palette indices (`0..=255`).
    fn from_token(s: &str) -> Option<Color> {
        if let Some(c) = Color::from_hex(s) {
            return Some(c);
        }
        let lower = s.trim().to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("indexed:") {
            return rest.trim().parse::<u8>().ok().map(Color::Indexed);
        }
        Some(match lower.as_str() {
            "reset" | "default" => Color::Reset,
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "white" => Color::White,
            "darkgray" | "darkgrey" | "gray" | "grey" => Color::DarkGray,
            "lightred" => Color::LightRed,
            "lightgreen" => Color::LightGreen,
            "lightyellow" => Color::LightYellow,
            "lightblue" => Color::LightBlue,
            "lightmagenta" => Color::LightMagenta,
            "lightcyan" => Color::LightCyan,
            "lightwhite" => Color::LightWhite,
            _ => return None,
        })
    }

    /// The canonical serialized token for this color.
    ///
    /// Named colors emit their lowercase name, `Rgb` emits `#rrggbb`,
    /// `Indexed(n)` emits `indexed:n`. This is the inverse of [`Color::from_token`].
    fn to_token(self) -> String {
        if let Some(name) = self.named_token() {
            return name.to_string();
        }
        match self {
            Color::Indexed(n) => format!("indexed:{n}"),
            // `Rgb` and any other true-color variant.
            other => other.to_hex(),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Color {
    /// Serialize as a human-friendly string token (`#rrggbb`, a named color,
    /// or `indexed:N`) so theme files stay hand-editable and round-trip.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_token())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Color {
    /// Deserialize from a token string: `#rgb`/`#rrggbb`, a named color
    /// (case-insensitive), or `indexed:N`.
    fn deserialize<D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ColorVisitor;

        impl serde::de::Visitor<'_> for ColorVisitor {
            type Value = Color;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a color token like \"#ff6b6b\", \"cyan\", or \"indexed:245\"")
            }

            fn visit_str<E>(self, value: &str) -> Result<Color, E>
            where
                E: serde::de::Error,
            {
                Color::from_token(value).ok_or_else(|| {
                    E::custom(format!(
                        "invalid color token {value:?}: expected #rgb/#rrggbb, a named color, or indexed:N"
                    ))
                })
            }
        }

        deserializer.deserialize_str(ColorVisitor)
    }
}

/// Terminal color depth capability.
///
/// Determines the maximum number of colors a terminal can display.
/// Use [`ColorDepth::detect`] for automatic detection via environment
/// variables, or specify explicitly in [`crate::RunConfig`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColorDepth {
    /// 24-bit true color (16 million colors).
    TrueColor,
    /// 256-color palette (xterm-256color).
    EightBit,
    /// 16 basic ANSI colors.
    Basic,
    /// No color output — every color is downsampled to [`Color::Reset`] and
    /// the terminal emits no SGR color codes. Selected automatically by
    /// [`ColorDepth::detect`] when the `NO_COLOR` environment variable is
    /// set to any non-empty value, per <https://no-color.org>.
    NoColor,
}

#[cfg(test)]
mod color_depth_tests {
    use super::{Color, ColorDepth};

    #[test]
    fn no_color_downsamples_everything_to_reset() {
        assert_eq!(Color::Red.downsampled(ColorDepth::NoColor), Color::Reset);
        assert_eq!(
            Color::Rgb(10, 20, 30).downsampled(ColorDepth::NoColor),
            Color::Reset
        );
        assert_eq!(
            Color::Indexed(44).downsampled(ColorDepth::NoColor),
            Color::Reset
        );
    }
}

impl ColorDepth {
    /// Detect the terminal's color depth from environment variables.
    ///
    /// Order of precedence:
    /// 1. `NO_COLOR` (any non-empty value) → [`ColorDepth::NoColor`]
    /// 2. `COLORTERM=truecolor|24bit` → [`ColorDepth::TrueColor`]
    /// 3. `TERM` contains `256color` → [`ColorDepth::EightBit`]
    /// 4. Fallback → [`ColorDepth::Basic`] (16 colors)
    pub fn detect() -> Self {
        // https://no-color.org — ANY non-empty value disables color.
        if std::env::var("NO_COLOR")
            .ok()
            .is_some_and(|v| !v.is_empty())
        {
            return Self::NoColor;
        }
        if let Ok(ct) = std::env::var("COLORTERM") {
            let ct = ct.to_lowercase();
            if ct == "truecolor" || ct == "24bit" {
                return Self::TrueColor;
            }
        }
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("256color") {
                return Self::EightBit;
            }
        }
        Self::Basic
    }
}

fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r >= 248 {
            return 231;
        }
        return 232 + (((r as u16 - 8) * 24 / 240) as u8);
    }

    let ri = if r < 48 {
        0
    } else {
        ((r as u16 - 35) / 40) as u8
    };
    let gi = if g < 48 {
        0
    } else {
        ((g as u16 - 35) / 40) as u8
    };
    let bi = if b < 48 {
        0
    } else {
        ((b as u16 - 35) / 40) as u8
    };
    16 + 36 * ri.min(5) + 6 * gi.min(5) + bi.min(5)
}

fn rgb_to_ansi16(r: u8, g: u8, b: u8) -> Color {
    let lum = 0.2126 * to_linear(r as f32 / 255.0)
        + 0.7152 * to_linear(g as f32 / 255.0)
        + 0.0722 * to_linear(b as f32 / 255.0);

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let saturation = if max == 0 {
        0.0
    } else {
        (max - min) as f32 / max as f32
    };

    if saturation < 0.2 {
        // Grayscale: classify purely by luminance.
        return match lum {
            l if l < 0.05 => Color::Black,
            l if l < 0.25 => Color::DarkGray,
            l if l < 0.7 => Color::White,
            _ => Color::White, // LightWhite is not available in Color enum
        };
    }

    // For chromatic colors the "bright" variant of each ANSI hue (e.g. xterm
    // LightRed = `(255, 85, 85)`) is distinguished by the minimum channel
    // being lifted off zero — it's a brighter, partially desaturated version
    // of the same hue. A perfectly saturated primary (`min == 0`) like pure
    // red `(255, 0, 0)` should map to the standard color. We pick the bright
    // variant only when both the value is high and the color is desaturated
    // enough to look "lifted".
    let bright = max >= 200 && min >= 64;

    let rf = r as f32;
    let gf = g as f32;
    let bf = b as f32;

    if rf >= gf && rf >= bf {
        if gf > bf * 1.5 {
            if bright {
                Color::LightYellow
            } else {
                Color::Yellow
            }
        } else if bf > gf * 1.5 {
            if bright {
                Color::LightMagenta
            } else {
                Color::Magenta
            }
        } else if bright {
            Color::LightRed
        } else {
            Color::Red
        }
    } else if gf >= rf && gf >= bf {
        if bf > rf * 1.5 {
            if bright {
                Color::LightCyan
            } else {
                Color::Cyan
            }
        } else if bright {
            Color::LightGreen
        } else {
            Color::Green
        }
    } else if rf > gf * 1.5 {
        if bright {
            Color::LightMagenta
        } else {
            Color::Magenta
        }
    } else if gf > rf * 1.5 {
        if bright {
            Color::LightCyan
        } else {
            Color::Cyan
        }
    } else if bright {
        Color::LightBlue
    } else {
        Color::Blue
    }
}

fn xterm256_to_rgb(idx: u8) -> (u8, u8, u8) {
    match idx {
        0 => (0, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        16..=231 => {
            let n = idx - 16;
            let b_idx = n % 6;
            let g_idx = (n / 6) % 6;
            let r_idx = n / 36;
            let to_val = |i: u8| if i == 0 { 0u8 } else { 55 + 40 * i };
            (to_val(r_idx), to_val(g_idx), to_val(b_idx))
        }
        232..=255 => {
            let v = 8 + 10 * (idx - 232);
            (v, v, v)
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn blend_halfway_rounds_to_128() {
        assert_eq!(
            Color::Rgb(255, 255, 255).blend(Color::Rgb(0, 0, 0), 0.5),
            Color::Rgb(128, 128, 128)
        );
    }

    #[test]
    fn contrast_ratio_white_on_black_is_high() {
        let ratio = Color::contrast_ratio(Color::White, Color::Black);
        assert!(ratio > 15.0);
    }

    #[test]
    fn contrast_ratio_same_color_is_one() {
        let ratio = Color::contrast_ratio(Color::Rgb(100, 100, 100), Color::Rgb(100, 100, 100));
        assert!((ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn meets_contrast_aa_white_on_black() {
        assert!(Color::meets_contrast_aa(Color::White, Color::Black));
    }

    #[test]
    fn meets_contrast_aa_low_contrast_fails() {
        assert!(!Color::meets_contrast_aa(
            Color::Rgb(180, 180, 180),
            Color::Rgb(200, 200, 200)
        ));
    }

    // --- regression: issue #104 rgb_to_ansi256 overflow at r=g=b=248 ---

    #[test]
    fn rgb_to_ansi256_no_overflow_full_range() {
        // 256^3 exhaustive — guarantees no panic in debug or release builds
        for r in 0u8..=255 {
            for g in 0u8..=255 {
                for b in 0u8..=255 {
                    let _ = Color::Rgb(r, g, b).downsampled(ColorDepth::EightBit);
                }
            }
        }
    }

    #[test]
    fn rgb_248_maps_to_231() {
        assert_eq!(
            Color::Rgb(248, 248, 248).downsampled(ColorDepth::EightBit),
            Color::Indexed(231)
        );
    }

    // --- regression: issue #105 WCAG luminance sRGB gamma ---

    #[test]
    fn luminance_dracula_purple_wcag() {
        let l = Color::Rgb(189, 147, 249).luminance();
        assert!((l - 0.385).abs() < 0.01, "expected ~0.385, got {l}");
    }

    #[test]
    fn contrast_aa_dracula_pair() {
        let p = Color::Rgb(189, 147, 249);
        let bg = Color::Rgb(40, 42, 54);
        assert!(Color::meets_contrast_aa(p, bg));
        let r = Color::contrast_ratio(p, bg);
        assert!((r - 5.90).abs() < 0.1, "expected ~5.90, got {r}");
    }

    #[test]
    fn contrast_white_on_black_is_21() {
        let r = Color::contrast_ratio(Color::Rgb(255, 255, 255), Color::Rgb(0, 0, 0));
        assert!((r - 21.0).abs() < 0.5, "expected ~21.0, got {r}");
    }

    // --- regression: issue #107 rgb_to_ansi16 includes bright (8-15) colors ---

    #[test]
    fn rgb_to_ansi16_bright_variants() {
        // bright red → LightRed
        assert_eq!(
            Color::Rgb(255, 80, 80).downsampled(ColorDepth::Basic),
            Color::LightRed
        );
        // dark red → Red
        assert_eq!(
            Color::Rgb(128, 20, 20).downsampled(ColorDepth::Basic),
            Color::Red
        );
        // bright gray → White
        assert_eq!(
            Color::Rgb(200, 200, 200).downsampled(ColorDepth::Basic),
            Color::White
        );
        // dark gray → DarkGray
        assert_eq!(
            Color::Rgb(80, 80, 80).downsampled(ColorDepth::Basic),
            Color::DarkGray
        );
    }
}
