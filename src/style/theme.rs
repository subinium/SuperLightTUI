use super::*;

/// Spacing scale for consistent padding, margin, and gap values.
///
/// All values are in terminal cells. The scale is relative to [`Spacing::base`],
/// which defaults to 1 cell. Use [`Theme::spacing`] to access the active scale,
/// or [`crate::Context::spacing`] for convenience.
///
/// # Example
///
/// ```
/// use slt::Spacing;
///
/// let sp = Spacing::new(1);
/// assert_eq!(sp.sm(), 2);
/// assert_eq!(sp.md(), 3);
/// assert_eq!(sp.xl(), 6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Spacing {
    /// Base unit in terminal cells. Defaults to 1.
    pub base: u32,
}

impl Spacing {
    /// Create a spacing scale with the given base unit.
    pub const fn new(base: u32) -> Self {
        Self { base }
    }

    /// Zero spacing.
    pub const fn none(&self) -> u32 {
        0
    }

    /// Extra-small spacing (1× base).
    pub const fn xs(&self) -> u32 {
        self.base
    }

    /// Small spacing (2× base).
    pub const fn sm(&self) -> u32 {
        self.base * 2
    }

    /// Medium spacing (3× base).
    pub const fn md(&self) -> u32 {
        self.base * 3
    }

    /// Large spacing (4× base).
    pub const fn lg(&self) -> u32 {
        self.base * 4
    }

    /// Extra-large spacing (6× base).
    pub const fn xl(&self) -> u32 {
        self.base * 6
    }

    /// Double extra-large spacing (8× base).
    pub const fn xxl(&self) -> u32 {
        self.base * 8
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self { base: 1 }
    }
}

/// Semantic color token that resolves against the active [`Theme`].
///
/// Use this when you want colors to automatically follow theme changes
/// without hardcoding specific [`Color`] values. Resolve with
/// [`Theme::resolve`] or [`crate::Context::color`].
///
/// # Example
///
/// ```
/// use slt::{Theme, ThemeColor};
///
/// let theme = Theme::dark();
/// let color = theme.resolve(ThemeColor::Primary);
/// assert_eq!(color, theme.primary);
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ThemeColor {
    /// Primary accent color.
    Primary,
    /// Secondary accent color.
    Secondary,
    /// Decorative accent color.
    Accent,
    /// Default text color.
    Text,
    /// Dimmed text color.
    TextDim,
    /// Border color for unfocused containers.
    Border,
    /// Background color.
    Bg,
    /// Success indicator color.
    Success,
    /// Warning indicator color.
    Warning,
    /// Error indicator color.
    Error,
    /// Selected item background.
    SelectedBg,
    /// Selected item foreground.
    SelectedFg,
    /// Surface background for elevated containers.
    Surface,
    /// Surface hover state.
    SurfaceHover,
    /// Text color readable on surface backgrounds.
    SurfaceText,
    /// Informational indicator (resolves to primary).
    Info,
    /// Hyperlink color (resolves to primary).
    Link,
    /// Focus ring outline color (resolves to primary).
    FocusRing,
    /// A literal color value, not resolved from the theme.
    Custom(Color),
}

/// A color theme that flows through all widgets automatically.
///
/// Construct with [`Theme::dark()`] or [`Theme::light()`], or use
/// [`Theme::builder()`] for custom themes. Pass via [`crate::RunConfig`]
/// and every widget picks up the colors without any extra wiring.
///
/// # Example
///
/// ```
/// use slt::{Color, Theme};
///
/// let theme = Theme::builder()
///     .primary(Color::Rgb(255, 107, 107))
///     .build();
/// ```
#[non_exhaustive]
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
    /// Spacing scale for consistent padding, margin, and gap values.
    pub spacing: Spacing,
}

impl Theme {
    /// Resolve a [`ThemeColor`] token to a concrete [`Color`].
    pub fn resolve(&self, token: ThemeColor) -> Color {
        match token {
            ThemeColor::Primary => self.primary,
            ThemeColor::Secondary => self.secondary,
            ThemeColor::Accent => self.accent,
            ThemeColor::Text => self.text,
            ThemeColor::TextDim => self.text_dim,
            ThemeColor::Border => self.border,
            ThemeColor::Bg => self.bg,
            ThemeColor::Success => self.success,
            ThemeColor::Warning => self.warning,
            ThemeColor::Error => self.error,
            ThemeColor::SelectedBg => self.selected_bg,
            ThemeColor::SelectedFg => self.selected_fg,
            ThemeColor::Surface => self.surface,
            ThemeColor::SurfaceHover => self.surface_hover,
            ThemeColor::SurfaceText => self.surface_text,
            ThemeColor::Info | ThemeColor::Link | ThemeColor::FocusRing => self.primary,
            ThemeColor::Custom(c) => c,
        }
    }

    /// Return a text color with guaranteed contrast against the given background.
    ///
    /// Delegates to [`Color::contrast_fg`].
    pub fn contrast_text_on(&self, bg: Color) -> Color {
        Color::contrast_fg(bg)
    }

    /// Blend a color with the theme's background at the given alpha.
    ///
    /// `alpha = 0.0` returns `self.bg`, `alpha = 1.0` returns `color` unchanged.
    pub fn overlay(&self, color: Color, alpha: f32) -> Color {
        color.blend(self.bg, alpha)
    }

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
            spacing: Spacing::new(1),
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
            spacing: Spacing::new(1),
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
            spacing: None,
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
            spacing: Spacing::new(1),
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
            spacing: Spacing::new(1),
        }
    }

    /// Nord theme — frost blue primary on polar night.
    pub fn nord() -> Self {
        Self {
            primary: Color::Rgb(136, 192, 208),
            secondary: Color::Rgb(129, 161, 193),
            accent: Color::Rgb(180, 142, 173),
            text: Color::Rgb(236, 239, 244),
            text_dim: Color::Rgb(216, 222, 233),
            border: Color::Rgb(59, 66, 82),
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
            spacing: Spacing::new(1),
        }
    }

    /// Solarized Dark theme — blue primary on dark base.
    pub fn solarized_dark() -> Self {
        Self {
            primary: Color::Rgb(38, 139, 210),
            secondary: Color::Rgb(42, 161, 152),
            accent: Color::Rgb(211, 54, 130),
            text: Color::Rgb(131, 148, 150),
            text_dim: Color::Rgb(101, 123, 131),
            border: Color::Rgb(7, 54, 66),
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
            spacing: Spacing::new(1),
        }
    }

    /// Solarized Light theme — warm ochre primary on light base.
    pub fn solarized_light() -> Self {
        Self {
            primary: Color::Rgb(38, 139, 210),
            secondary: Color::Rgb(42, 161, 152),
            accent: Color::Rgb(211, 54, 130),
            text: Color::Rgb(101, 123, 131),
            text_dim: Color::Rgb(88, 110, 117),
            border: Color::Rgb(238, 232, 213),
            bg: Color::Rgb(253, 246, 227),
            success: Color::Rgb(133, 153, 0),
            warning: Color::Rgb(181, 137, 0),
            error: Color::Rgb(220, 50, 47),
            selected_bg: Color::Rgb(38, 139, 210),
            selected_fg: Color::Rgb(253, 246, 227),
            surface: Color::Rgb(238, 232, 213),
            surface_hover: Color::Rgb(227, 221, 201),
            surface_text: Color::Rgb(88, 110, 117),
            is_dark: false,
            spacing: Spacing::new(1),
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
            spacing: Spacing::new(1),
        }
    }

    /// Gruvbox Dark theme — warm, retro tones on dark background.
    pub fn gruvbox_dark() -> Self {
        Self {
            primary: Color::Rgb(215, 153, 33),
            secondary: Color::Rgb(69, 133, 136),
            accent: Color::Rgb(177, 98, 134),
            text: Color::Rgb(235, 219, 178),
            text_dim: Color::Rgb(146, 131, 116),
            border: Color::Rgb(80, 73, 69),
            bg: Color::Rgb(40, 40, 40),
            success: Color::Rgb(152, 151, 26),
            warning: Color::Rgb(250, 189, 47),
            error: Color::Rgb(204, 36, 29),
            selected_bg: Color::Rgb(215, 153, 33),
            selected_fg: Color::Rgb(40, 40, 40),
            surface: Color::Rgb(60, 56, 54),
            surface_hover: Color::Rgb(80, 73, 69),
            surface_text: Color::Rgb(189, 174, 147),
            is_dark: true,
            spacing: Spacing::new(1),
        }
    }

    /// One Dark theme (Atom) — cool blues and purples on dark gray.
    pub fn one_dark() -> Self {
        Self {
            primary: Color::Rgb(97, 175, 239),
            secondary: Color::Rgb(86, 182, 194),
            accent: Color::Rgb(198, 120, 221),
            text: Color::Rgb(171, 178, 191),
            text_dim: Color::Rgb(92, 99, 112),
            border: Color::Rgb(62, 68, 81),
            bg: Color::Rgb(40, 44, 52),
            success: Color::Rgb(152, 195, 121),
            warning: Color::Rgb(229, 192, 123),
            error: Color::Rgb(224, 108, 117),
            selected_bg: Color::Rgb(97, 175, 239),
            selected_fg: Color::Rgb(40, 44, 52),
            surface: Color::Rgb(50, 55, 65),
            surface_hover: Color::Rgb(62, 68, 81),
            surface_text: Color::Rgb(152, 159, 172),
            is_dark: true,
            spacing: Spacing::new(1),
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
    spacing: Option<Spacing>,
}

impl ThemeBuilder {
    /// Set the primary color.
    pub fn primary(mut self, color: Color) -> Self {
        self.primary = Some(color);
        self
    }

    /// Set the secondary color.
    pub fn secondary(mut self, color: Color) -> Self {
        self.secondary = Some(color);
        self
    }

    /// Set the accent color.
    pub fn accent(mut self, color: Color) -> Self {
        self.accent = Some(color);
        self
    }

    /// Set the main text color.
    pub fn text(mut self, color: Color) -> Self {
        self.text = Some(color);
        self
    }

    /// Set the dimmed text color.
    pub fn text_dim(mut self, color: Color) -> Self {
        self.text_dim = Some(color);
        self
    }

    /// Set the border color.
    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    /// Set the background color.
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Set the success indicator color.
    pub fn success(mut self, color: Color) -> Self {
        self.success = Some(color);
        self
    }

    /// Set the warning indicator color.
    pub fn warning(mut self, color: Color) -> Self {
        self.warning = Some(color);
        self
    }

    /// Set the error indicator color.
    pub fn error(mut self, color: Color) -> Self {
        self.error = Some(color);
        self
    }

    /// Set the selected item background color.
    pub fn selected_bg(mut self, color: Color) -> Self {
        self.selected_bg = Some(color);
        self
    }

    /// Set the selected item foreground color.
    pub fn selected_fg(mut self, color: Color) -> Self {
        self.selected_fg = Some(color);
        self
    }

    /// Set the surface background color.
    pub fn surface(mut self, color: Color) -> Self {
        self.surface = Some(color);
        self
    }

    /// Set the surface hover color.
    pub fn surface_hover(mut self, color: Color) -> Self {
        self.surface_hover = Some(color);
        self
    }

    /// Set the surface text color.
    pub fn surface_text(mut self, color: Color) -> Self {
        self.surface_text = Some(color);
        self
    }

    /// Set the dark mode flag.
    pub fn is_dark(mut self, is_dark: bool) -> Self {
        self.is_dark = Some(is_dark);
        self
    }

    /// Set the spacing scale.
    pub fn spacing(mut self, spacing: Spacing) -> Self {
        self.spacing = Some(spacing);
        self
    }

    /// Build the theme. Unfilled fields use [`Theme::dark()`] defaults.
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
            spacing: self.spacing.unwrap_or(defaults.spacing),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_dark_preset_builds() {
        let t = Theme::dark();
        assert_eq!(t.primary, Color::Cyan);
        assert!(t.is_dark);
    }

    #[test]
    fn theme_light_preset_builds() {
        let t = Theme::light();
        assert_eq!(t.selected_fg, Color::White);
        assert!(!t.is_dark);
    }

    #[test]
    fn theme_dracula_preset_builds() {
        let t = Theme::dracula();
        assert_eq!(t.bg, Color::Rgb(40, 42, 54));
        assert!(t.is_dark);
    }

    #[test]
    fn theme_catppuccin_preset_builds() {
        let t = Theme::catppuccin();
        assert_eq!(t.bg, Color::Rgb(30, 30, 46));
        assert!(t.is_dark);
    }

    #[test]
    fn theme_nord_preset_builds() {
        let t = Theme::nord();
        assert_eq!(t.bg, Color::Rgb(46, 52, 64));
        assert!(t.is_dark);
    }

    #[test]
    fn theme_solarized_dark_preset_builds() {
        let t = Theme::solarized_dark();
        assert_eq!(t.bg, Color::Rgb(0, 43, 54));
        assert!(t.is_dark);
    }

    #[test]
    fn theme_tokyo_night_preset_builds() {
        let t = Theme::tokyo_night();
        assert_eq!(t.bg, Color::Rgb(26, 27, 38));
        assert!(t.is_dark);
    }

    #[test]
    fn theme_builder_sets_primary_and_accent() {
        let theme = Theme::builder()
            .primary(Color::Red)
            .accent(Color::Yellow)
            .build();

        assert_eq!(theme.primary, Color::Red);
        assert_eq!(theme.accent, Color::Yellow);
    }

    #[test]
    fn theme_builder_defaults_to_dark_for_unset_fields() {
        let defaults = Theme::dark();
        let theme = Theme::builder().primary(Color::Green).build();

        assert_eq!(theme.primary, Color::Green);
        assert_eq!(theme.secondary, defaults.secondary);
        assert_eq!(theme.text, defaults.text);
        assert_eq!(theme.text_dim, defaults.text_dim);
        assert_eq!(theme.border, defaults.border);
        assert_eq!(theme.surface_hover, defaults.surface_hover);
        assert_eq!(theme.is_dark, defaults.is_dark);
    }

    #[test]
    fn theme_builder_can_override_is_dark() {
        let theme = Theme::builder().is_dark(false).build();
        assert!(!theme.is_dark);
    }

    #[test]
    fn theme_default_matches_dark() {
        let default_theme = Theme::default();
        let dark = Theme::dark();
        assert_eq!(default_theme.primary, dark.primary);
        assert_eq!(default_theme.bg, dark.bg);
        assert_eq!(default_theme.is_dark, dark.is_dark);
    }

    #[test]
    fn theme_solarized_light_preset_builds() {
        let t = Theme::solarized_light();
        assert_eq!(t.bg, Color::Rgb(253, 246, 227));
        assert!(!t.is_dark);
    }

    #[test]
    fn theme_gruvbox_dark_preset_builds() {
        let t = Theme::gruvbox_dark();
        assert_eq!(t.bg, Color::Rgb(40, 40, 40));
        assert!(t.is_dark);
    }

    #[test]
    fn theme_one_dark_preset_builds() {
        let t = Theme::one_dark();
        assert_eq!(t.bg, Color::Rgb(40, 44, 52));
        assert!(t.is_dark);
    }

    // --- regression: issue #106 Nord/Solarized text_dim collides with border ---

    #[test]
    fn theme_text_dim_ne_border() {
        for theme in [
            Theme::nord(),
            Theme::solarized_dark(),
            Theme::solarized_light(),
        ] {
            assert_ne!(theme.text_dim, theme.border, "text_dim == border in theme");
        }
    }

    #[test]
    fn spacing_scale_values() {
        let sp = Spacing::new(1);
        assert_eq!(sp.none(), 0);
        assert_eq!(sp.xs(), 1);
        assert_eq!(sp.sm(), 2);
        assert_eq!(sp.md(), 3);
        assert_eq!(sp.lg(), 4);
        assert_eq!(sp.xl(), 6);
        assert_eq!(sp.xxl(), 8);
    }

    #[test]
    fn spacing_custom_base() {
        let sp = Spacing::new(2);
        assert_eq!(sp.xs(), 2);
        assert_eq!(sp.sm(), 4);
        assert_eq!(sp.md(), 6);
    }

    #[test]
    fn theme_color_resolve_maps_correctly() {
        let t = Theme::dark();
        assert_eq!(t.resolve(ThemeColor::Primary), t.primary);
        assert_eq!(t.resolve(ThemeColor::Secondary), t.secondary);
        assert_eq!(t.resolve(ThemeColor::Accent), t.accent);
        assert_eq!(t.resolve(ThemeColor::Text), t.text);
        assert_eq!(t.resolve(ThemeColor::TextDim), t.text_dim);
        assert_eq!(t.resolve(ThemeColor::Border), t.border);
        assert_eq!(t.resolve(ThemeColor::Bg), t.bg);
        assert_eq!(t.resolve(ThemeColor::Success), t.success);
        assert_eq!(t.resolve(ThemeColor::Warning), t.warning);
        assert_eq!(t.resolve(ThemeColor::Error), t.error);
        assert_eq!(t.resolve(ThemeColor::SelectedBg), t.selected_bg);
        assert_eq!(t.resolve(ThemeColor::SelectedFg), t.selected_fg);
        assert_eq!(t.resolve(ThemeColor::Surface), t.surface);
        assert_eq!(t.resolve(ThemeColor::SurfaceHover), t.surface_hover);
        assert_eq!(t.resolve(ThemeColor::SurfaceText), t.surface_text);
    }

    #[test]
    fn theme_color_aliases_resolve_to_primary() {
        let t = Theme::dark();
        assert_eq!(t.resolve(ThemeColor::Info), t.primary);
        assert_eq!(t.resolve(ThemeColor::Link), t.primary);
        assert_eq!(t.resolve(ThemeColor::FocusRing), t.primary);
    }

    #[test]
    fn theme_color_custom_passes_through() {
        let t = Theme::dark();
        let custom = Color::Rgb(42, 42, 42);
        assert_eq!(t.resolve(ThemeColor::Custom(custom)), custom);
    }

    #[test]
    fn theme_builder_spacing() {
        let sp = Spacing::new(3);
        let theme = Theme::builder().spacing(sp).build();
        assert_eq!(theme.spacing, sp);
    }

    #[test]
    fn theme_contrast_text_on_dark_bg() {
        let t = Theme::dark();
        let fg = t.contrast_text_on(Color::Rgb(0, 0, 0));
        assert_eq!(fg, Color::Rgb(255, 255, 255));
    }

    #[test]
    fn theme_contrast_text_on_light_bg() {
        let t = Theme::dark();
        let fg = t.contrast_text_on(Color::Rgb(255, 255, 255));
        assert_eq!(fg, Color::Rgb(0, 0, 0));
    }
}
