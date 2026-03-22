<div align="center">

# SuperLightTUI

**Rapidísimo de escribir. Ultraligero de ejecutar.**

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[Índice de Docs] · [Inicio Rápido] · [Guía de Widgets] · [Ejemplos] · [Guía de Patrones] · [Arquitectura] · [Contribuir]

[English](../README.md) · [中文](README.zh-CN.md) · **Español** · [日本語](README.ja.md) · [한국어](README.ko.md)

</div>

SuperLightTUI es una biblioteca TUI de modo inmediato para Rust.
Escribes un closure, SLT lo llama en cada frame, y la biblioteca se encarga del layout, diffing, foco y renderizado.

## Galería

<table>
  <tr>
    <td align="center"><img src="../assets/demo.png" alt="Widget Demo" /><br/><b>Widget Demo</b><br/><sub><code>cargo run --example demo</code></sub></td>
    <td align="center"><img src="../assets/demo_dashboard.png" alt="Dashboard" /><br/><b>Dashboard</b><br/><sub><code>cargo run --example demo_dashboard</code></sub></td>
    <td align="center"><img src="../assets/demo_website.png" alt="Website" /><br/><b>Website Layout</b><br/><sub><code>cargo run --example demo_website</code></sub></td>
  </tr>
  <tr>
    <td align="center"><img src="../assets/demo_spreadsheet.png" alt="Spreadsheet" /><br/><b>Spreadsheet</b><br/><sub><code>cargo run --example demo_spreadsheet</code></sub></td>
    <td align="center"><img src="../assets/demo_game.gif" alt="Games" /><br/><b>Games</b><br/><sub><code>cargo run --example demo_game</code></sub></td>
    <td align="center"><img src="../assets/demo_fire.gif" alt="DOOM Fire" /><br/><b>DOOM Fire Effect</b><br/><sub><code>cargo run --release --example demo_fire</code></sub></td>
  </tr>
</table>

## Primeros pasos

```sh
cargo add superlighttui
```

```rust
fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut slt::Context| {
        ui.text("hello, world");
    })
}
```

5 líneas. Sin struct `App`. Sin `Model`/`Update`/`View`. Sin bucle de eventos. Ctrl+C funciona sin configuración.

## Una aplicación real

```rust
use slt::{Border, Color, Context, KeyCode};

fn main() -> std::io::Result<()> {
    let mut count: i32 = 0;

    slt::run(|ui: &mut Context| {
        if ui.key('q') {
            ui.quit();
        }
        if ui.key('k') || ui.key_code(KeyCode::Up) {
            count += 1;
        }
        if ui.key('j') || ui.key_code(KeyCode::Down) {
            count -= 1;
        }

        ui.bordered(Border::Rounded).title("Counter").pad(1).gap(1).col(|ui| {
            ui.text("Counter").bold().fg(Color::Cyan);
            ui.row(|ui| {
                ui.text("Count:");
                let color = if count >= 0 { Color::Green } else { Color::Red };
                ui.text(format!("{count}")).bold().fg(color);
            });
            ui.text("k +1 / j -1 / q quit").dim();
        });
    })
}
```

El estado vive en tu closure. El layout usa `row()` y `col()`. El estilo se encadena. Eso es todo.

## Por qué SLT

- **Tu closure ES la aplicación** — sin estado de framework, sin implementaciones de traits, sin bucle de mensajes.
- **Layout como CSS, sintaxis como Tailwind** — `row()`, `col()`, `gap()`, `grow()`, `spacer()`, más abreviaciones como `.p()`, `.px()`, `.m()`, `.w()`, `.max_w()`.
- **Los widgets conectan todo automáticamente** — orden de foco, hover, clics, scroll y comportamiento de teclado vienen integrados.
- **Núcleo pequeño, extras opcionales** — las dependencias del núcleo son `unicode-width` y `compact_str`; el I/O de terminal viene del `crossterm` opcional; async, serde, image, qrcode y resaltado de sintaxis van tras feature flags.
- **La higiene de biblioteca importa** — cero `unsafe`, feature flags explícitos, documentación, ejemplos, tests y disciplina de releases con semver.

## Superficie de API común

```rust
// Texto y layout
ui.text("Hello").bold().fg(Color::Cyan);
ui.row(|ui| {
    ui.text("left");
    ui.spacer();
    ui.text("right");
});

// Entradas y acciones
ui.text_input(&mut name);
if ui.button("Save").clicked {}
ui.checkbox("Dark mode", &mut dark);

// Datos y navegación
ui.tabs(&mut tabs);
ui.list(&mut items);
ui.table(&mut data);
ui.command_palette(&mut palette);

// Overlays y salida enriquecida
ui.toast(&mut toasts);
ui.modal(|ui| {
    ui.text("Confirm?").bold();
});
ui.markdown("# Hello **world**");

// Visualización
ui.chart(|c| {
    c.line(&data);
    c.grid(true);
}, 50, 16);
ui.sparkline(&values, 16);
ui.canvas(40, 10, |cv| {
    cv.circle(20, 20, 15);
});
```

Para la lista categorizada de widgets, consulta la [Guía de Widgets].

## Aprende la biblioteca

| Documento | Qué cubre |
|-----------|-----------|
| [Índice de Docs] | Estructura de documentación y mapa de guías |
| [Inicio Rápido] | Instalación, primera app, contador, layout, input |
| [Guía de Widgets] | Catálogo de widgets y APIs principales |
| [Guía de Patrones] | Estado, formularios, overlays, async, widgets personalizados |
| [Ejemplos] | Índice de ejemplos y comandos de ejecución |
| [Arquitectura] | Mapa de módulos, ciclo de vida del frame |
| [Guía de Backends] | `Backend`, `AppState`, `frame()`, modo inline |
| [Guía de Testing] | `TestBackend`, `EventBuilder`, pruebas de interacción |
| [Guía de Depuración] | Overlay F12, clipping, retardo de un frame |
| [Guía AI] | Referencia rápida para agentes de código AI |
| [Guía de Animación] | Tween, Spring, Keyframes, Sequence, Stagger |
| [Guía de Temas] | Temas predefinidos, ThemeBuilder, temas personalizados |
| [Guía de Características] | Feature flags, dependencias opcionales |
| [`docs/DESIGN_PRINCIPLES.md`](DESIGN_PRINCIPLES.md) | Por qué la API tiene esta forma |

## Ejemplos destacados

| Ejemplo | Comando | Enfoque |
|---------|---------|---------|
| hello | `cargo run --example hello` | La app más pequeña posible |
| counter | `cargo run --example counter` | Estado + entrada de teclado |
| demo | `cargo run --example demo` | Tour amplio de widgets |
| demo_dashboard | `cargo run --example demo_dashboard` | Layout de dashboard |
| demo_cli | `cargo run --example demo_cli` | Layout de herramienta CLI |
| demo_infoviz | `cargo run --example demo_infoviz` | Gráficos y visualización de datos |
| demo_game | `cargo run --example demo_game` | Interacción en modo inmediato |
| async_demo | `cargo run --example async_demo --features async` | Mensajes en segundo plano |

El índice categorizado completo está en [Ejemplos].

## Widgets personalizados y backends

- Implementa `Widget` para construir widgets reutilizables con acceso completo a foco, layout, eventos y temas.
- Implementa `Backend` + controla `frame()` si quieres un renderer no terminal, harness de tests, o destino embebido.
- Usa `TestBackend` para renderizado headless y aserciones estilo snapshot.

Consulta [Guía de Patrones] y [Arquitectura] para los caminos más profundos.

## Contribuir

Lee [Contribuir], luego `docs/DESIGN_PRINCIPLES.md` y [Arquitectura].
El proceso de release y CI espera que el formato, check, clippy, tests y compilación de ejemplos estén en verde.

## Licencia

[MIT](../LICENSE)

<!-- Badge definitions -->
[Crate Badge]: https://img.shields.io/crates/v/superlighttui?style=flat-square&logo=rust&color=E05D44
[Docs Badge]: https://img.shields.io/docsrs/superlighttui?style=flat-square&logo=docs.rs
[CI Badge]: https://img.shields.io/github/actions/workflow/status/subinium/SuperLightTUI/ci.yml?branch=main&style=flat-square&label=CI
[MSRV Badge]: https://img.shields.io/crates/msrv/superlighttui?style=flat-square&label=MSRV
[Downloads Badge]: https://img.shields.io/crates/d/superlighttui?style=flat-square
[License Badge]: https://img.shields.io/crates/l/superlighttui?style=flat-square&color=1370D3

<!-- Link definitions -->
[CI]: https://github.com/subinium/SuperLightTUI/actions/workflows/ci.yml
[Crate]: https://crates.io/crates/superlighttui
[Docs]: https://docs.rs/superlighttui
[Índice de Docs]: README.md
[Inicio Rápido]: QUICK_START.md
[Guía de Widgets]: WIDGETS.md
[Ejemplos]: EXAMPLES.md
[Guía de Patrones]: PATTERNS.md
[Arquitectura]: ARCHITECTURE.md
[Guía de Backends]: BACKENDS.md
[Guía de Testing]: TESTING.md
[Guía de Depuración]: DEBUGGING.md
[Guía AI]: AI_GUIDE.md
[Guía de Animación]: ANIMATION.md
[Guía de Temas]: THEMING.md
[Guía de Características]: FEATURES.md
[Contribuir]: ../CONTRIBUTING.md
[License]: ../LICENSE
