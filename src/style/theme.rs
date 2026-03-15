use super::*;

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
    /// Whether this theme is a dark theme. Used to initialize dark mode in Context.
    pub is_dark: bool,
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
            is_dark: true,
        }
    }

    /// Create a light theme with high-contrast dark text on light backgrounds.
    pub fn light() -> Self {
        Self {
            primary: Color::Rgb(37, 99, 235),
            secondary: Color::Rgb(14, 116, 144),
            accent: Color::Rgb(147, 51, 234),
            text: Color::Rgb(15, 23, 42),
            text_dim: Color::Rgb(100, 116, 139),
            border: Color::Rgb(203, 213, 225),
            bg: Color::Rgb(248, 250, 252),
            success: Color::Rgb(22, 163, 74),
            warning: Color::Rgb(202, 138, 4),
            error: Color::Rgb(220, 38, 38),
            selected_bg: Color::Rgb(37, 99, 235),
            selected_fg: Color::White,
            surface: Color::Rgb(241, 245, 249),
            surface_hover: Color::Rgb(226, 232, 240),
            surface_text: Color::Rgb(51, 65, 85),
            is_dark: false,
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
            is_dark: None,
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
            is_dark: true,
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
            is_dark: true,
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
            is_dark: true,
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
            is_dark: true,
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
            is_dark: true,
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
    is_dark: Option<bool>,
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

    pub fn is_dark(mut self, is_dark: bool) -> Self {
        self.is_dark = Some(is_dark);
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
            is_dark: self.is_dark.unwrap_or(defaults.is_dark),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
