//! Visual styling primitives.
//!
//! Colors, themes, borders, padding, margin, constraints, alignment, and
//! text modifiers. Every widget inherits these through [`Theme`] automatically.

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

/// A color theme that flows through all widgets automatically.
///
/// Construct with [`Theme::dark()`] or [`Theme::light()`], or build a custom
/// theme by filling in the fields directly. Pass the theme via [`crate::RunConfig`]
/// and every widget will pick up the colors without any extra wiring.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Theme {
    /// Primary accent color, used for focused borders and highlights.
    pub primary: Color,
    /// Secondary accent color, used for less prominent highlights.
    pub secondary: Color,
    /// Accent color for decorative elements.
    pub accent: Color,
    /// Default foreground text color.
    pub text: Color,
    /// Dimmed text color for secondary labels and hints.
    pub text_dim: Color,
    /// Border color for unfocused containers.
    pub border: Color,
    /// Background color. Typically [`Color::Reset`] to inherit the terminal background.
    pub bg: Color,
    /// Color for success states (e.g., toast notifications).
    pub success: Color,
    /// Color for warning states.
    pub warning: Color,
    /// Color for error states.
    pub error: Color,
    /// Background color for selected list/table rows.
    pub selected_bg: Color,
    /// Foreground color for selected list/table rows.
    pub selected_fg: Color,
    /// Subtle surface color for card backgrounds and elevated containers.
    pub surface: Color,
    /// Hover/active surface color, one step brighter than `surface`.
    ///
    /// Used for interactive element hover states. Should be visually
    /// distinguishable from both `surface` and `border`.
    pub surface_hover: Color,
    /// Secondary text color guaranteed readable on `surface` backgrounds.
    ///
    /// Use this instead of `text_dim` when rendering on `surface`-colored
    /// containers. `text_dim` is tuned for the main `bg`; on `surface` it
    /// may lack contrast.
    pub surface_text: Color,
}

impl Theme {
    /// Create a dark theme with cyan primary and white text.
    pub fn dark() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            accent: Color::Magenta,
            text: Color::White,
            text_dim: Color::Indexed(245),
            border: Color::Indexed(240),
            bg: Color::Reset,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            selected_bg: Color::Cyan,
            selected_fg: Color::Black,
            surface: Color::Indexed(236),
            surface_hover: Color::Indexed(238),
            surface_text: Color::Indexed(250),
        }
    }

    /// Create a light theme with blue primary and black text.
    pub fn light() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::Cyan,
            accent: Color::Magenta,
            text: Color::Black,
            text_dim: Color::Indexed(240),
            border: Color::Indexed(245),
            bg: Color::Reset,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            selected_bg: Color::Blue,
            selected_fg: Color::White,
            surface: Color::Indexed(254),
            surface_hover: Color::Indexed(252),
            surface_text: Color::Indexed(238),
        }
    }

    /// Create a [`ThemeBuilder`] for configuring a custom theme.
    ///
    /// # Example
    ///
    /// ```
    /// use slt::{Color, Theme};
    ///
    /// let theme = Theme::builder()
    ///     .primary(Color::Rgb(255, 107, 107))
    ///     .accent(Color::Cyan)
    ///     .build();
    /// ```
    pub fn builder() -> ThemeBuilder {
        ThemeBuilder {
            primary: None,
            secondary: None,
            accent: None,
            text: None,
            text_dim: None,
            border: None,
            bg: None,
            success: None,
            warning: None,
            error: None,
            selected_bg: None,
            selected_fg: None,
            surface: None,
            surface_hover: None,
            surface_text: None,
        }
    }

    /// Dracula theme — purple primary on dark gray.
    pub fn dracula() -> Self {
        Self {
            primary: Color::Rgb(189, 147, 249),
            secondary: Color::Rgb(139, 233, 253),
            accent: Color::Rgb(255, 121, 198),
            text: Color::Rgb(248, 248, 242),
            text_dim: Color::Rgb(98, 114, 164),
            border: Color::Rgb(68, 71, 90),
            bg: Color::Rgb(40, 42, 54),
            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            error: Color::Rgb(255, 85, 85),
            selected_bg: Color::Rgb(189, 147, 249),
            selected_fg: Color::Rgb(40, 42, 54),
            surface: Color::Rgb(68, 71, 90),
            surface_hover: Color::Rgb(98, 100, 120),
            surface_text: Color::Rgb(191, 194, 210),
        }
    }

    /// Catppuccin Mocha theme — lavender primary on dark base.
    pub fn catppuccin() -> Self {
        Self {
            primary: Color::Rgb(180, 190, 254),
            secondary: Color::Rgb(137, 180, 250),
            accent: Color::Rgb(245, 194, 231),
            text: Color::Rgb(205, 214, 244),
            text_dim: Color::Rgb(127, 132, 156),
            border: Color::Rgb(88, 91, 112),
            bg: Color::Rgb(30, 30, 46),
            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            selected_bg: Color::Rgb(180, 190, 254),
            selected_fg: Color::Rgb(30, 30, 46),
            surface: Color::Rgb(49, 50, 68),
            surface_hover: Color::Rgb(69, 71, 90),
            surface_text: Color::Rgb(166, 173, 200),
        }
    }

    /// Nord theme — frost blue primary on polar night.
    pub fn nord() -> Self {
        Self {
            primary: Color::Rgb(136, 192, 208),
            secondary: Color::Rgb(129, 161, 193),
            accent: Color::Rgb(180, 142, 173),
            text: Color::Rgb(236, 239, 244),
            text_dim: Color::Rgb(76, 86, 106),
            border: Color::Rgb(76, 86, 106),
            bg: Color::Rgb(46, 52, 64),
            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            error: Color::Rgb(191, 97, 106),
            selected_bg: Color::Rgb(136, 192, 208),
            selected_fg: Color::Rgb(46, 52, 64),
            surface: Color::Rgb(59, 66, 82),
            surface_hover: Color::Rgb(67, 76, 94),
            surface_text: Color::Rgb(216, 222, 233),
        }
    }

    /// Solarized Dark theme — blue primary on dark base.
    pub fn solarized_dark() -> Self {
        Self {
            primary: Color::Rgb(38, 139, 210),
            secondary: Color::Rgb(42, 161, 152),
            accent: Color::Rgb(211, 54, 130),
            text: Color::Rgb(131, 148, 150),
            text_dim: Color::Rgb(88, 110, 117),
            border: Color::Rgb(88, 110, 117),
            bg: Color::Rgb(0, 43, 54),
            success: Color::Rgb(133, 153, 0),
            warning: Color::Rgb(181, 137, 0),
            error: Color::Rgb(220, 50, 47),
            selected_bg: Color::Rgb(38, 139, 210),
            selected_fg: Color::Rgb(253, 246, 227),
            surface: Color::Rgb(7, 54, 66),
            surface_hover: Color::Rgb(23, 72, 85),
            surface_text: Color::Rgb(147, 161, 161),
        }
    }

    /// Tokyo Night theme — blue primary on dark storm base.
    pub fn tokyo_night() -> Self {
        Self {
            primary: Color::Rgb(122, 162, 247),
            secondary: Color::Rgb(125, 207, 255),
            accent: Color::Rgb(187, 154, 247),
            text: Color::Rgb(169, 177, 214),
            text_dim: Color::Rgb(86, 95, 137),
            border: Color::Rgb(54, 58, 79),
            bg: Color::Rgb(26, 27, 38),
            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(224, 175, 104),
            error: Color::Rgb(247, 118, 142),
            selected_bg: Color::Rgb(122, 162, 247),
            selected_fg: Color::Rgb(26, 27, 38),
            surface: Color::Rgb(36, 40, 59),
            surface_hover: Color::Rgb(41, 46, 66),
            surface_text: Color::Rgb(192, 202, 245),
        }
    }
}

/// Builder for creating custom themes with defaults from `Theme::dark()`.
pub struct ThemeBuilder {
    primary: Option<Color>,
    secondary: Option<Color>,
    accent: Option<Color>,
    text: Option<Color>,
    text_dim: Option<Color>,
    border: Option<Color>,
    bg: Option<Color>,
    success: Option<Color>,
    warning: Option<Color>,
    error: Option<Color>,
    selected_bg: Option<Color>,
    selected_fg: Option<Color>,
    surface: Option<Color>,
    surface_hover: Option<Color>,
    surface_text: Option<Color>,
}

impl ThemeBuilder {
    pub fn primary(mut self, color: Color) -> Self {
        self.primary = Some(color);
        self
    }

    pub fn secondary(mut self, color: Color) -> Self {
        self.secondary = Some(color);
        self
    }

    pub fn accent(mut self, color: Color) -> Self {
        self.accent = Some(color);
        self
    }

    pub fn text(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }

    pub fn text_dim(mut self, color: Color) -> Self {
        self.text_dim = Some(color);
        self
    }

    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn success(mut self, color: Color) -> Self {
        self.success = Some(color);
        self
    }

    pub fn warning(mut self, color: Color) -> Self {
        self.warning = Some(color);
        self
    }

    pub fn error(mut self, color: Color) -> Self {
        self.error = Some(color);
        self
    }

    pub fn selected_bg(mut self, color: Color) -> Self {
        self.selected_bg = Some(color);
        self
    }

    pub fn selected_fg(mut self, color: Color) -> Self {
        self.selected_fg = Some(color);
        self
    }

    pub fn surface(mut self, color: Color) -> Self {
        self.surface = Some(color);
        self
    }

    pub fn surface_hover(mut self, color: Color) -> Self {
        self.surface_hover = Some(color);
        self
    }

    pub fn surface_text(mut self, color: Color) -> Self {
        self.surface_text = Some(color);
        self
    }

    pub fn build(self) -> Theme {
        let defaults = Theme::dark();
        Theme {
            primary: self.primary.unwrap_or(defaults.primary),
            secondary: self.secondary.unwrap_or(defaults.secondary),
            accent: self.accent.unwrap_or(defaults.accent),
            text: self.text.unwrap_or(defaults.text),
            text_dim: self.text_dim.unwrap_or(defaults.text_dim),
            border: self.border.unwrap_or(defaults.border),
            bg: self.bg.unwrap_or(defaults.bg),
            success: self.success.unwrap_or(defaults.success),
            warning: self.warning.unwrap_or(defaults.warning),
            error: self.error.unwrap_or(defaults.error),
            selected_bg: self.selected_bg.unwrap_or(defaults.selected_bg),
            selected_fg: self.selected_fg.unwrap_or(defaults.selected_fg),
            surface: self.surface.unwrap_or(defaults.surface),
            surface_hover: self.surface_hover.unwrap_or(defaults.surface_hover),
            surface_text: self.surface_text.unwrap_or(defaults.surface_text),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Terminal size breakpoint for responsive layouts.
///
/// Based on the current terminal width. Use [`Context::breakpoint`] to
/// get the active breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Breakpoint {
    /// Width < 40 columns (phone-sized)
    Xs,
    /// Width 40-79 columns (small terminal)
    Sm,
    /// Width 80-119 columns (standard terminal)
    Md,
    /// Width 120-159 columns (wide terminal)
    Lg,
    /// Width >= 160 columns (ultra-wide)
    Xl,
}

/// Border style for containers.
///
/// Pass to `Context::bordered()` to draw a box around a container.
/// Each variant uses a different set of Unicode box-drawing characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Border {
    /// Single-line box: `┌─┐│└─┘`
    Single,
    /// Double-line box: `╔═╗║╚═╝`
    Double,
    /// Rounded corners: `╭─╮│╰─╯`
    Rounded,
    /// Thick single-line box: `┏━┓┃┗━┛`
    Thick,
    /// Dashed border using light dash characters: ┄╌┄╌
    Dashed,
    /// Heavy dashed border: ┅╍┅╍
    DashedThick,
}

/// Character set for a specific border style.
///
/// Returned by [`Border::chars`]. Contains the six box-drawing characters
/// needed to render a complete border: four corners and two line segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BorderChars {
    /// Top-left corner character.
    pub tl: char,
    /// Top-right corner character.
    pub tr: char,
    /// Bottom-left corner character.
    pub bl: char,
    /// Bottom-right corner character.
    pub br: char,
    /// Horizontal line character.
    pub h: char,
    /// Vertical line character.
    pub v: char,
}

/// Controls which sides of a border are visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BorderSides {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

impl BorderSides {
    pub const fn all() -> Self {
        Self {
            top: true,
            right: true,
            bottom: true,
            left: true,
        }
    }

    pub const fn none() -> Self {
        Self {
            top: false,
            right: false,
            bottom: false,
            left: false,
        }
    }

    pub const fn horizontal() -> Self {
        Self {
            top: true,
            right: false,
            bottom: true,
            left: false,
        }
    }

    pub const fn vertical() -> Self {
        Self {
            top: false,
            right: true,
            bottom: false,
            left: true,
        }
    }

    pub fn has_horizontal(&self) -> bool {
        self.top || self.bottom
    }

    pub fn has_vertical(&self) -> bool {
        self.left || self.right
    }
}

impl Default for BorderSides {
    fn default() -> Self {
        Self::all()
    }
}

impl Border {
    /// Return the [`BorderChars`] for this border style.
    pub const fn chars(self) -> BorderChars {
        match self {
            Self::Single => BorderChars {
                tl: '┌',
                tr: '┐',
                bl: '└',
                br: '┘',
                h: '─',
                v: '│',
            },
            Self::Double => BorderChars {
                tl: '╔',
                tr: '╗',
                bl: '╚',
                br: '╝',
                h: '═',
                v: '║',
            },
            Self::Rounded => BorderChars {
                tl: '╭',
                tr: '╮',
                bl: '╰',
                br: '╯',
                h: '─',
                v: '│',
            },
            Self::Thick => BorderChars {
                tl: '┏',
                tr: '┓',
                bl: '┗',
                br: '┛',
                h: '━',
                v: '┃',
            },
            Self::Dashed => BorderChars {
                tl: '┌',
                tr: '┐',
                bl: '└',
                br: '┘',
                h: '┄',
                v: '┆',
            },
            Self::DashedThick => BorderChars {
                tl: '┏',
                tr: '┓',
                bl: '┗',
                br: '┛',
                h: '┅',
                v: '┇',
            },
        }
    }
}

/// Padding inside a container border.
///
/// Shrinks the content area inward from each edge. All values are in terminal
/// columns/rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Padding {
    /// Padding on the top edge.
    pub top: u32,
    /// Padding on the right edge.
    pub right: u32,
    /// Padding on the bottom edge.
    pub bottom: u32,
    /// Padding on the left edge.
    pub left: u32,
}

impl Padding {
    /// Create uniform padding on all four sides.
    pub const fn all(v: u32) -> Self {
        Self::new(v, v, v, v)
    }

    /// Create padding with `x` on left/right and `y` on top/bottom.
    pub const fn xy(x: u32, y: u32) -> Self {
        Self::new(y, x, y, x)
    }

    /// Create padding with explicit values for each side.
    pub const fn new(top: u32, right: u32, bottom: u32, left: u32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Total horizontal padding (`left + right`).
    pub const fn horizontal(self) -> u32 {
        self.left + self.right
    }

    /// Total vertical padding (`top + bottom`).
    pub const fn vertical(self) -> u32 {
        self.top + self.bottom
    }
}

/// Margin outside a container.
///
/// Adds space around the outside of a container's border. All values are in
/// terminal columns/rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Margin {
    /// Margin on the top edge.
    pub top: u32,
    /// Margin on the right edge.
    pub right: u32,
    /// Margin on the bottom edge.
    pub bottom: u32,
    /// Margin on the left edge.
    pub left: u32,
}

impl Margin {
    /// Create uniform margin on all four sides.
    pub const fn all(v: u32) -> Self {
        Self::new(v, v, v, v)
    }

    /// Create margin with `x` on left/right and `y` on top/bottom.
    pub const fn xy(x: u32, y: u32) -> Self {
        Self::new(y, x, y, x)
    }

    /// Create margin with explicit values for each side.
    pub const fn new(top: u32, right: u32, bottom: u32, left: u32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Total horizontal margin (`left + right`).
    pub const fn horizontal(self) -> u32 {
        self.left + self.right
    }

    /// Total vertical margin (`top + bottom`).
    pub const fn vertical(self) -> u32 {
        self.top + self.bottom
    }
}

/// Size constraints for layout computation.
///
/// All fields are optional. Unset constraints are unconstrained. Use the
/// builder methods to set individual bounds in a fluent style.
///
/// # Example
///
/// ```
/// use slt::Constraints;
///
/// let c = Constraints::default().min_w(10).max_w(40);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use = "configure constraints using the returned value"]
pub struct Constraints {
    /// Minimum width in terminal columns, if any.
    pub min_width: Option<u32>,
    /// Maximum width in terminal columns, if any.
    pub max_width: Option<u32>,
    /// Minimum height in terminal rows, if any.
    pub min_height: Option<u32>,
    /// Maximum height in terminal rows, if any.
    pub max_height: Option<u32>,
    /// Width as a percentage (1-100) of the parent container.
    pub width_pct: Option<u8>,
    /// Height as a percentage (1-100) of the parent container.
    pub height_pct: Option<u8>,
}

impl Constraints {
    /// Set the minimum width constraint.
    pub const fn min_w(mut self, min_width: u32) -> Self {
        self.min_width = Some(min_width);
        self
    }

    /// Set the maximum width constraint.
    pub const fn max_w(mut self, max_width: u32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// Set the minimum height constraint.
    pub const fn min_h(mut self, min_height: u32) -> Self {
        self.min_height = Some(min_height);
        self
    }

    /// Set the maximum height constraint.
    pub const fn max_h(mut self, max_height: u32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// Set width as a percentage (1-100) of the parent container.
    pub const fn w_pct(mut self, pct: u8) -> Self {
        self.width_pct = Some(pct);
        self
    }

    /// Set height as a percentage (1-100) of the parent container.
    pub const fn h_pct(mut self, pct: u8) -> Self {
        self.height_pct = Some(pct);
        self
    }
}

/// Cross-axis alignment within a container.
///
/// Controls how children are positioned along the axis perpendicular to the
/// container's main axis. For a `row()`, this is vertical alignment; for a
/// `col()`, this is horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Align {
    /// Align children to the start of the cross axis (default).
    #[default]
    Start,
    /// Center children on the cross axis.
    Center,
    /// Align children to the end of the cross axis.
    End,
}

/// Main-axis content distribution within a container.
///
/// Controls how children are distributed along the main axis. For a `row()`,
/// this is horizontal distribution; for a `col()`, this is vertical.
///
/// When children have `grow > 0`, they consume remaining space before justify
/// distribution applies. Justify modes only affect the leftover space after
/// flex-grow allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Justify {
    /// Pack children at the start (default). Uses `gap` for spacing.
    #[default]
    Start,
    /// Center children along the main axis with `gap` spacing.
    Center,
    /// Pack children at the end with `gap` spacing.
    End,
    /// First child at start, last at end, equal space between.
    SpaceBetween,
    /// Equal space around each child (half-size space at edges).
    SpaceAround,
    /// Equal space between all children and at both edges.
    SpaceEvenly,
}

/// Text modifier bitflags stored as a `u8`.
///
/// Combine modifiers with `|` or [`Modifiers::insert`]. Check membership with
/// [`Modifiers::contains`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Modifiers(pub u8);

impl Modifiers {
    /// No modifiers set.
    pub const NONE: Self = Self(0);
    /// Bold text.
    pub const BOLD: Self = Self(1 << 0);
    /// Dimmed/faint text.
    pub const DIM: Self = Self(1 << 1);
    /// Italic text.
    pub const ITALIC: Self = Self(1 << 2);
    /// Underlined text.
    pub const UNDERLINE: Self = Self(1 << 3);
    /// Reversed foreground/background colors.
    pub const REVERSED: Self = Self(1 << 4);
    /// Strikethrough text.
    pub const STRIKETHROUGH: Self = Self(1 << 5);

    /// Returns `true` if all bits in `other` are set in `self`.
    #[inline]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Set all bits from `other` into `self`.
    #[inline]
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Returns `true` if no modifiers are set.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Modifiers {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Visual style for a terminal cell (foreground, background, modifiers).
///
/// Styles are applied to text via the builder methods on `Context` widget
/// calls (e.g., `.bold()`, `.fg(Color::Cyan)`). All fields are optional;
/// `None` means "inherit from the terminal default."
///
/// # Example
///
/// ```
/// use slt::{Style, Color};
///
/// let style = Style::new().fg(Color::Cyan).bold();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[must_use = "build and pass the returned Style value"]
pub struct Style {
    /// Foreground color, or `None` to use the terminal default.
    pub fg: Option<Color>,
    /// Background color, or `None` to use the terminal default.
    pub bg: Option<Color>,
    /// Text modifiers (bold, italic, underline, etc.).
    pub modifiers: Modifiers,
}

impl Style {
    /// Create a new style with no color or modifiers set.
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            modifiers: Modifiers::NONE,
        }
    }

    /// Set the foreground color.
    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// Set the background color.
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Add the bold modifier.
    pub fn bold(mut self) -> Self {
        self.modifiers |= Modifiers::BOLD;
        self
    }

    /// Add the dim modifier.
    pub fn dim(mut self) -> Self {
        self.modifiers |= Modifiers::DIM;
        self
    }

    /// Add the italic modifier.
    pub fn italic(mut self) -> Self {
        self.modifiers |= Modifiers::ITALIC;
        self
    }

    /// Add the underline modifier.
    pub fn underline(mut self) -> Self {
        self.modifiers |= Modifiers::UNDERLINE;
        self
    }

    /// Add the reversed (inverted colors) modifier.
    pub fn reversed(mut self) -> Self {
        self.modifiers |= Modifiers::REVERSED;
        self
    }

    /// Add the strikethrough modifier.
    pub fn strikethrough(mut self) -> Self {
        self.modifiers |= Modifiers::STRIKETHROUGH;
        self
    }
}
