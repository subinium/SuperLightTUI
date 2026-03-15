/// Terminal color.
///
/// Covers the standard 16 named colors, 256-color palette indices, and
/// 24-bit RGB true color. Use [`Color::Reset`] to restore the terminal's
/// default foreground or background.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// 24-bit true color.
    Rgb(u8, u8, u8),
    /// 256-color palette index.
    Indexed(u8),
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
        let rf = r as f32 / 255.0;
        let gf = g as f32 / 255.0;
        let bf = b as f32 / 255.0;
        0.2126 * rf + 0.7152 * gf + 0.0722 * bf
    }

    /// Return a contrasting foreground color for the given background.
    ///
    /// Uses the BT.709 luminance threshold (0.5) to decide between white
    /// and black text. For theme-aware contrast, prefer using this over
    /// hardcoding `theme.bg` as the foreground.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::Color;
    ///
    /// let bg = Color::Rgb(189, 147, 249); // Dracula purple
    /// let fg = Color::contrast_fg(bg);
    /// // Purple is mid-bright → returns black for readable text
    /// ```
    pub fn contrast_fg(bg: Color) -> Color {
        if bg.luminance() > 0.5 {
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
        let r = (r1 as f32 * alpha + r2 as f32 * (1.0 - alpha)) as u8;
        let g = (g1 as f32 * alpha + g2 as f32 * (1.0 - alpha)) as u8;
        let b = (b1 as f32 * alpha + b2 as f32 * (1.0 - alpha)) as u8;
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

    /// Downsample this color to fit the given color depth.
    ///
    /// - `TrueColor`: returns self unchanged.
    /// - `EightBit`: converts `Rgb` to the nearest `Indexed` color.
    /// - `Basic`: converts `Rgb` and `Indexed` to the nearest named color.
    ///
    /// Named colors (`Red`, `Green`, etc.) and `Reset` pass through all depths.
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
        }
    }
}

/// Terminal color depth capability.
///
/// Determines the maximum number of colors a terminal can display.
/// Use [`ColorDepth::detect`] for automatic detection via environment
/// variables, or specify explicitly in [`RunConfig`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColorDepth {
    /// 24-bit true color (16 million colors).
    TrueColor,
    /// 256-color palette (xterm-256color).
    EightBit,
    /// 16 basic ANSI colors.
    Basic,
}

impl ColorDepth {
    /// Detect the terminal's color depth from environment variables.
    ///
    /// Checks `$COLORTERM` for `truecolor`/`24bit`, then `$TERM` for
    /// `256color`. Falls back to `Basic` (16 colors) if neither is set.
    pub fn detect() -> Self {
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
        if r > 248 {
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
    let lum =
        0.2126 * (r as f32 / 255.0) + 0.7152 * (g as f32 / 255.0) + 0.0722 * (b as f32 / 255.0);

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let saturation = if max == 0 {
        0.0
    } else {
        (max - min) as f32 / max as f32
    };

    if saturation < 0.2 {
        return if lum < 0.15 {
            Color::Black
        } else {
            Color::White
        };
    }

    let rf = r as f32;
    let gf = g as f32;
    let bf = b as f32;

    if rf >= gf && rf >= bf {
        if gf > bf * 1.5 {
            Color::Yellow
        } else if bf > gf * 1.5 {
            Color::Magenta
        } else {
            Color::Red
        }
    } else if gf >= rf && gf >= bf {
        if bf > rf * 1.5 {
            Color::Cyan
        } else {
            Color::Green
        }
    } else if rf > gf * 1.5 {
        Color::Magenta
    } else if gf > rf * 1.5 {
        Color::Cyan
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
