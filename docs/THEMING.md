# Theming & Colors

SLT's theming system flows a `Theme` through all widgets automatically. Set a theme once and every widget picks up the colors without extra wiring.

## Preset Themes

SLT ships with 7 built-in themes:

| Theme | Constructor | Style | Primary color |
|-------|-------------|-------|---------------|
| **Dark** (default) | `Theme::dark()` | Dark | Cyan |
| **Light** | `Theme::light()` | Light | Blue (RGB) |
| **Dracula** | `Theme::dracula()` | Dark | Purple |
| **Catppuccin** (Mocha) | `Theme::catppuccin()` | Dark | Lavender |
| **Nord** | `Theme::nord()` | Dark | Frost blue |
| **Solarized Dark** | `Theme::solarized_dark()` | Dark | Blue |
| **Tokyo Night** | `Theme::tokyo_night()` | Dark | Blue |

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

### All 16 theme fields

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

| Field | Purpose |
|-------|---------|
| `fg` | Foreground color override |
| `bg` | Background color override |
| `border` | Border color override |
| `accent` | Accent/highlight color override |

All fields are `Option<Color>` -- unset fields fall back to the active theme.

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
let gray = white.blend(black, 0.5);  // ~Rgb(128, 128, 128)

// Lighten toward white (0.0 = unchanged, 1.0 = white)
let light_blue = blue.lighten(0.3);

// Darken toward black (0.0 = unchanged, 1.0 = black)
let dark_blue = blue.darken(0.3);
```

### Luminance and contrast

```rust
use slt::Color;

let bg = Color::Rgb(30, 30, 46);

// Perceived brightness (0.0 = darkest, 1.0 = brightest)
let lum = bg.luminance(); // ~0.03

// Automatic readable foreground for a background color
// Returns white for dark backgrounds, black for light ones
let fg = Color::contrast_fg(bg); // Rgb(255, 255, 255)
```

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

### ContainerStyle fields

`border`, `border_sides`, `border_style`, `bg`, `text_color`, `dark_bg`, `dark_border_style`, `padding`, `margin`, `gap`, `row_gap`, `col_gap`, `grow`, `align`, `align_self`, `justify`, `w`, `h`, `min_w`, `max_w`, `min_h`, `max_h`, `w_pct`, `h_pct`

All fields are `Option` -- unset fields leave the builder's current value unchanged.

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
