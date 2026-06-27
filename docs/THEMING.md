# Theming & Colors

SLT's theming system flows a `Theme` through all widgets automatically. Set a theme once and every widget picks up the colors without extra wiring.

## Preset Themes

SLT ships with 10 built-in themes:

| Theme | Constructor | Style | Primary color |
|-------|-------------|-------|---------------|
| **Dark** (default) | `Theme::dark()` | Dark | Cyan |
| **Light** | `Theme::light()` | Light | Blue (RGB) |
| **Dracula** | `Theme::dracula()` | Dark | Purple |
| **Catppuccin** (Mocha) | `Theme::catppuccin()` | Dark | Lavender |
| **Nord** | `Theme::nord()` | Dark | Frost blue |
| **Solarized Dark** | `Theme::solarized_dark()` | Dark | Blue |
| **Solarized Light** | `Theme::solarized_light()` | Light | Blue |
| **Tokyo Night** | `Theme::tokyo_night()` | Dark | Blue |
| **Gruvbox Dark** | `Theme::gruvbox_dark()` | Dark | Orange |
| **One Dark** | `Theme::one_dark()` | Dark | Blue |

### Usage with RunConfig

```rust
use slt::{RunConfig, Theme};

fn main() -> std::io::Result<()> {
    let config = RunConfig::default().theme(Theme::dracula());

    slt::run_with(config, |ui| {
        ui.text("Styled by Dracula");
    })
}
```

## ThemeBuilder

Build a custom theme by overriding specific fields. Unset fields fall back to `Theme::dark()` defaults.

```rust
use slt::{Color, Theme};

let theme = Theme::builder()
    .primary(Color::Rgb(255, 107, 107))
    .accent(Color::Cyan)
    .bg(Color::Rgb(20, 20, 30))
    .text(Color::Rgb(220, 220, 230))
    .is_dark(true)
    .build();
```

### Builder entry points

| Constructor | Pre-filled defaults | When to use |
|-------------|---------------------|-------------|
| `Theme::builder()` | `Theme::dark()` | Build a dark theme from scratch |
| `Theme::light_builder()` | `Theme::light()` | Build a light theme — keeps light bg/text/border defaults instead of dark ones (v0.19.2) |
| `Theme::builder_from(base)` | All fields of `base` | Derive a variant from any preset, override only the fields you want to change (v0.19.2) |

```rust
use slt::{Color, Theme};

// Nord with a custom primary — keeps Nord's frost/snow palette everywhere else.
let custom_nord = Theme::builder_from(Theme::nord())
    .primary(Color::Rgb(255, 0, 0))
    .build();
assert_eq!(custom_nord.bg, Theme::nord().bg);

// Light theme variant without re-specifying every light-mode field.
let my_light = Theme::light_builder()
    .primary(Color::Rgb(0, 100, 200))
    .build();
assert!(!my_light.is_dark);
```

### `const fn` ThemeBuilder (v0.19.2)

Every `ThemeBuilder` setter is `const fn`, including `builder()`, `builder_from()`, `light_builder()`, `build()`, and all field setters (`primary`, `secondary`, `accent`, `text`, `text_dim`, `border`, `bg`, `success`, `warning`, `error`, `selected_bg`, `selected_fg`, `surface`, `surface_hover`, `surface_text`, `is_dark`, `spacing`). You can define themes at compile time:

```rust
use slt::{Color, Theme};

const MY_THEME: Theme = Theme::builder()
    .primary(Color::Rgb(0, 0, 0))
    .accent(Color::Cyan)
    .build();
```

Compile-time themes incur no runtime construction cost and let you embed branded palettes as `static` data alongside other UI constants.

### All 17 theme fields

| Field | Purpose |
|-------|---------|
| `primary` | Focused borders, highlights |
| `secondary` | Less prominent highlights |
| `accent` | Decorative elements |
| `text` | Default foreground text |
| `text_dim` | Secondary labels, hints |
| `border` | Unfocused container borders |
| `bg` | Background (often `Color::Reset` to inherit terminal bg) |
| `success` | Success states (toasts, indicators) |
| `warning` | Warning states |
| `error` | Error states |
| `selected_bg` | Selected list/table row background |
| `selected_fg` | Selected list/table row foreground |
| `surface` | Card backgrounds, elevated containers |
| `surface_hover` | Hover/active surface, one step brighter than `surface` |
| `surface_text` | Text color readable on `surface` backgrounds |
| `is_dark` | Whether this theme is dark mode |
| `spacing` | `Spacing` struct for consistent padding/margin/gap scale |

> **Note (v0.17.0)**: `Theme` is now `#[non_exhaustive]`. Use `Theme::builder()` or preset constructors instead of struct literal syntax.

## Spacing Tokens

The `Spacing` struct provides a consistent spacing scale based on a configurable base unit (default: 1 cell).

```rust
let sp = ui.spacing();
ui.col_gap(sp.md(), |ui| {
    ui.container().p(sp.sm()).col(|ui| {
        ui.text("Consistent spacing");
    });
});
```

| Method | Value (base=1) |
|--------|----------------|
| `none()` | 0 |
| `xs()` | 1 |
| `sm()` | 2 |
| `md()` | 3 |
| `lg()` | 4 |
| `xl()` | 6 |
| `xxl()` | 8 |

Custom base: `Spacing::new(2)` doubles all values.

## ThemeColor (Semantic Tokens)

`ThemeColor` lets styles reference theme colors by name. Colors resolve automatically when the theme changes.

```rust
use slt::{ContainerStyle, ThemeColor, Border};

const CARD: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded)
    .p(1)
    .theme_bg(ThemeColor::Surface);        // adapts to any theme
    // .theme_border_fg(ThemeColor::Primary) // optional

// Resolve in code:
let primary = ui.color(ThemeColor::Primary);
let surface = ui.theme().resolve(ThemeColor::Surface);
```

| Variant | Resolves to |
|---------|-------------|
| `Primary`, `Secondary`, `Accent`, `Text`, `TextDim`, `Border`, `Bg` | Corresponding theme field |
| `Success`, `Warning`, `Error` | Feedback colors |
| `SelectedBg`, `SelectedFg` | Selection colors |
| `Surface`, `SurfaceHover`, `SurfaceText` | Surface colors |
| `Info`, `Link`, `FocusRing` | Aliases for `primary` (future-extensible) |
| `Custom(Color)` | Literal passthrough |

## Contrast Helpers

```rust
use slt::Color;

// WCAG 2.1 contrast ratio (>= 4.5 for AA normal text)
let ratio = Color::contrast_ratio_f64(fg, bg);
let ok = Color::meets_contrast_aa(fg, bg);

// Auto-select readable text color for any background
let fg = ui.theme().contrast_text_on(bg_color);

// Blend color against theme background
let overlay = ui.theme().overlay_f64(color, 0.5);
```

> **Note (v0.19.1)**: `Color::contrast_fg` and `Theme::contrast_text_on` use the WCAG 2.1 relative luminance threshold of `0.179`, not the previous `0.5`. Mid-tone backgrounds — Dracula purple (`Rgb(189, 147, 249)`, luminance ≈ 0.385), Solarized base1, Catppuccin lavender — now route to white text instead of black, matching WCAG AA contrast guidance. If you depended on the old midpoint behavior for stylistic reasons, override per-callsite with `WidgetColors` instead of relying on the default.

## Runtime Theme Switching

Change themes and dark mode on the fly inside your render closure:

```rust
use slt::{Color, Theme};

slt::run(|ui| {
    // Switch the entire theme
    if ui.key('t') {
        ui.set_theme(Theme::nord());
    }

    // Toggle dark mode
    if ui.key('d') {
        let dark = ui.is_dark_mode();
        ui.set_dark_mode(!dark);
    }

    // Pick a color based on current mode
    let accent = ui.light_dark(Color::Rgb(37, 99, 235), Color::Cyan);
    ui.text("Adaptive text").fg(accent);
})
```

| Method | Description |
|--------|-------------|
| `ui.set_theme(theme)` | Replace the active theme for all subsequent widgets |
| `ui.is_dark_mode()` | Returns `true` if dark mode is active |
| `ui.set_dark_mode(bool)` | Enable or disable dark mode |
| `ui.light_dark(light, dark)` | Returns `light` in light mode, `dark` in dark mode |

## WidgetColors

Override individual widget colors without changing the global theme. Many widgets have a `_colored` variant that accepts `WidgetColors`.

```rust
use slt::{Color, WidgetColors};

let custom = WidgetColors::new()
    .fg(Color::White)
    .bg(Color::Rgb(30, 30, 46))
    .border(Color::Cyan)
    .accent(Color::Yellow);

// Use the _colored variant
ui.button_colored("Save", &custom);
ui.list_colored(&mut list_state, &custom);
ui.table_colored(&mut table_state, &custom);
```

### WidgetColors fields

| Field | Type | Purpose |
|-------|------|---------|
| `fg` | `Option<Color>` | Foreground color override |
| `bg` | `Option<Color>` | Background color override |
| `border` | `Option<Color>` | Border color override |
| `accent` | `Option<Color>` | Accent/highlight color override |
| `theme_fg` | `Option<ThemeColor>` | Theme-aware foreground (takes precedence over `fg`) |
| `theme_bg` | `Option<ThemeColor>` | Theme-aware background (takes precedence over `bg`) |
| `theme_border` | `Option<ThemeColor>` | Theme-aware border (takes precedence over `border`) |
| `theme_accent` | `Option<ThemeColor>` | Theme-aware accent (takes precedence over `accent`) |

Resolution order: `theme_*` > literal field > theme default. Use `resolve_fg(&theme, fallback)` etc. for resolution.

## WidgetTheme (Global Widget Defaults)

Set default colors for all instances of a widget type via `RunConfig`:

```rust
use slt::{RunConfig, WidgetTheme, WidgetColors, Color};

let config = RunConfig::default()
    .widget_theme(
        WidgetTheme::new()
            .button(WidgetColors::new().accent(Color::Cyan))
            .table(WidgetColors::new().border(Color::Magenta))
    );

slt::run_with(config, |ui| {
    ui.button("All cyan");  // uses WidgetTheme.button defaults
})
```

Per-callsite `_colored()` overrides still take precedence over `WidgetTheme` defaults.

### Widgets with `_colored` variants

| Widget | Colored variant |
|--------|----------------|
| `button` | `button_colored(label, &colors)` |
| `list` | `list_colored(&mut state, &colors)` |
| `table` | `table_colored(&mut state, &colors)` |
| `tabs` | `tabs_colored(&mut state, &colors)` |
| `text_input` | `text_input_colored(&mut state, &colors)` |
| `select` | `select_colored(&mut state, &colors)` |
| `radio` | `radio_colored(&mut state, &colors)` |
| `checkbox` | `checkbox_colored(label, checked, &colors)` |
| `toggle` | `toggle_colored(label, enabled, &colors)` |
| `separator` | `separator_colored(color)` |
| `badge` | `badge_colored(label, color)` |
| `stat` | `stat_colored(label, value, color)` |
| `progress_bar` | `progress_bar_colored(ratio, width, color)` |
| `line_chart` | `line_chart_colored(data, w, h, color)` |
| `area_chart` | `area_chart_colored(data, w, h, color)` |

## Tailwind Palette

The `palette::tailwind` module provides all 22 Tailwind CSS color palettes as `const` values. Each palette has 11 shades from lightest (`c50`) to darkest (`c950`).

```rust
use slt::palette::tailwind::{BLUE, ROSE, SLATE};

// Access a specific shade
let primary = BLUE.c500;    // Rgb(59, 130, 246)
let danger  = ROSE.c600;    // Rgb(225, 29, 72)
let muted   = SLATE.c400;   // Rgb(148, 163, 184)
```

### Shade levels

| Shade | Meaning |
|-------|---------|
| `c50` | Lightest |
| `c100` | |
| `c200` | |
| `c300` | |
| `c400` | |
| `c500` | Mid / default |
| `c600` | |
| `c700` | |
| `c800` | |
| `c900` | |
| `c950` | Darkest |

### All 22 palettes

**Neutrals:** `SLATE`, `GRAY`, `ZINC`, `NEUTRAL`, `STONE`

**Colors:** `RED`, `ORANGE`, `AMBER`, `YELLOW`, `LIME`, `GREEN`, `EMERALD`, `TEAL`, `CYAN`, `SKY`, `BLUE`, `INDIGO`, `VIOLET`, `PURPLE`, `FUCHSIA`, `PINK`, `ROSE`

### Building a theme from Tailwind palettes

```rust
use slt::{Theme, palette::tailwind::*};

let theme = Theme::builder()
    .primary(INDIGO.c500)
    .secondary(TEAL.c500)
    .accent(PINK.c500)
    .text(SLATE.c50)
    .text_dim(SLATE.c400)
    .border(SLATE.c700)
    .bg(SLATE.c950)
    .success(EMERALD.c500)
    .warning(AMBER.c500)
    .error(RED.c500)
    .surface(SLATE.c800)
    .surface_hover(SLATE.c700)
    .surface_text(SLATE.c300)
    .is_dark(true)
    .build();
```

## Color Utilities

### Creating colors

```rust
use slt::Color;

// 24-bit true color
let coral = Color::Rgb(255, 127, 80);

// 256-color palette index
let gray = Color::Indexed(240);

// Named ANSI colors
let red = Color::Red;
let bright = Color::LightCyan;

// Reset to terminal default
let default = Color::Reset;
```

### Named colors

`Black`, `Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `White`, `DarkGray`, `LightRed`, `LightGreen`, `LightYellow`, `LightBlue`, `LightMagenta`, `LightCyan`, `LightWhite`, `Reset`

### Blending and adjustment

```rust
use slt::Color;

let white = Color::Rgb(255, 255, 255);
let black = Color::Rgb(0, 0, 0);
let blue = Color::Rgb(59, 130, 246);

// Alpha blending: blend(other, alpha)
// alpha=0.0 returns other, alpha=1.0 returns self
let gray = white.blend_f64(black, 0.5);  // ~Rgb(128, 128, 128)

// Lighten toward white (0.0 = unchanged, 1.0 = white)
let light_blue = blue.lighten_f64(0.3);

// Darken toward black (0.0 = unchanged, 1.0 = black)
let dark_blue = blue.darken_f64(0.3);
```

### Luminance and contrast

```rust
use slt::Color;

let bg = Color::Rgb(30, 30, 46);

// Perceived brightness (0.0 = darkest, 1.0 = brightest)
let lum = bg.luminance_f64(); // ~0.013 (sRGB-linearized, BT.709 weights)

// Automatic readable foreground for a background color
// Returns white if luminance > 0.179 (WCAG threshold), black otherwise
let fg = Color::contrast_fg(bg); // Rgb(255, 255, 255)
```

> **Note (v0.19.1)**: `Color::luminance` now applies the sRGB inverse transfer function (gamma decoding) to each channel before applying BT.709 weights `0.2126·R + 0.7152·G + 0.0722·B`. This matches the WCAG 2.1 relative luminance definition. Numerical results differ from pre-v0.19.1 — most notably, mid-tone colors land at lower luminance values than the old naive linear average produced. If your code compares `luminance()` against a hardcoded threshold, re-check it against the new scale.

### Downsampling for terminal compatibility

```rust
use slt::{Color, ColorDepth};

let color = Color::Rgb(59, 130, 246);

// TrueColor: returns unchanged
let true_c = color.downsampled(ColorDepth::TrueColor);

// EightBit: converts Rgb to nearest Indexed color
let eight = color.downsampled(ColorDepth::EightBit);

// Basic: converts to nearest named ANSI color
let basic = color.downsampled(ColorDepth::Basic);
```

## ColorDepth

Represents the terminal's color capability.

| Variant | Colors | Description |
|---------|--------|-------------|
| `TrueColor` | 16M | 24-bit RGB |
| `EightBit` | 256 | xterm-256color palette |
| `Basic` | 16 | Standard ANSI colors |

### Automatic detection

```rust
use slt::ColorDepth;

// Checks $COLORTERM for truecolor/24bit, then $TERM for 256color
let depth = ColorDepth::detect();
```

### Setting via RunConfig

```rust
use slt::{RunConfig, ColorDepth};

let config = RunConfig::default().color_depth(ColorDepth::EightBit);
```

## Style Type

`Style` sets foreground, background, and text modifiers for a terminal cell.

```rust
use slt::{Style, Color, Modifiers};

// Builder pattern
let style = Style::new()
    .fg(Color::Cyan)
    .bg(Color::Rgb(30, 30, 46))
    .bold()
    .italic();

// Modifiers can also be combined with |
let mods = Modifiers::BOLD | Modifiers::UNDERLINE;
```

### Available modifiers

| Modifier | Method | Constant |
|----------|--------|----------|
| Bold | `.bold()` | `Modifiers::BOLD` |
| Dim | `.dim()` | `Modifiers::DIM` |
| Italic | `.italic()` | `Modifiers::ITALIC` |
| Underline | `.underline()` | `Modifiers::UNDERLINE` |
| Reversed | `.reversed()` | `Modifiers::REVERSED` |
| Strikethrough | `.strikethrough()` | `Modifiers::STRIKETHROUGH` |

### Applying to text widgets

```rust
slt::run(|ui| {
    ui.text("bold cyan").bold().fg(Color::Cyan);
    ui.text("dimmed").dim();
    ui.text("warning").fg(Color::Yellow).italic();
})
```

## ContainerStyle

Define reusable style recipes as `const` values and apply them to containers.

```rust
use slt::{ContainerStyle, Border, Color, Align};

const CARD: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded)
    .p(1)
    .bg(Color::Indexed(236));

const DANGER: ContainerStyle = ContainerStyle::new()
    .bg(Color::Red);

slt::run(|ui| {
    // Apply a single style
    ui.container().apply(&CARD).col(|ui| {
        ui.text("Card content");
    });

    // Compose multiple styles (last write wins)
    ui.container().apply(&CARD).apply(&DANGER).col(|ui| {
        ui.text("Danger card");
    });
})
```

### Style inheritance with `extending()`

Define derived styles without duplicating fields:

```rust
use slt::{ContainerStyle, Border, ThemeColor};

const BUTTON: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded)
    .p(1);

const BUTTON_DANGER: ContainerStyle = ContainerStyle::extending(&BUTTON)
    .theme_bg(ThemeColor::Error);  // inherits border + padding from BUTTON
```

### ContainerStyle fields

`border`, `border_sides`, `border_style`, `bg`, `text_color`, `dark_bg`, `dark_border_style`, `padding`, `margin`, `gap`, `row_gap`, `col_gap`, `grow`, `align`, `align_self`, `justify`, `w`, `h`, `min_w`, `max_w`, `min_h`, `max_h`, `w_pct`, `h_pct`, `theme_bg`, `theme_text_color`, `theme_border_fg`, `extends`

All fields are `Option` -- unset fields leave the builder's current value unchanged. `theme_*` fields take precedence over their literal counterparts.

## Dark Mode Patterns

### Container-level dark mode overrides

Use `dark_bg()` and `dark_border_style()` on `ContainerBuilder` to set styles that apply only when dark mode is active:

```rust
use slt::{Border, Color, Style};

slt::run(|ui| {
    ui.bordered(Border::Rounded)
        .bg(Color::Rgb(248, 250, 252))           // light mode bg
        .dark_bg(Color::Rgb(30, 30, 46))          // dark mode bg
        .dark_border_style(Style::new().fg(Color::Rgb(88, 91, 112)))
        .col(|ui| {
            ui.text("Adapts to dark/light mode");
        });
})
```

### Using `light_dark()` for inline adaptation

```rust
use slt::Color;

slt::run(|ui| {
    let text_color = ui.light_dark(
        Color::Rgb(15, 23, 42),   // dark text for light mode
        Color::Rgb(205, 214, 244) // light text for dark mode
    );
    ui.text("Adaptive").fg(text_color);
})
```

### ContainerStyle with dark mode

```rust
use slt::{ContainerStyle, Border, Color, Style};

const PANEL: ContainerStyle = ContainerStyle::new()
    .border(Border::Rounded)
    .p(1)
    .bg(Color::Rgb(241, 245, 249))
    .dark_bg(Color::Rgb(49, 50, 68));
```

## Related Docs

- [QUICK_START.md](QUICK_START.md) -- basic setup and style chaining
- [WIDGETS.md](WIDGETS.md) -- categorized widget reference
- [PATTERNS.md](PATTERNS.md) -- common composition patterns
- [DESIGN_PRINCIPLES.md](DESIGN_PRINCIPLES.md) -- API philosophy
