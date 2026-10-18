<div align="center">

# SuperLightTUI

**Construye interfaces de terminal como páginas web**: layout flexbox, encadenamiento estilo Tailwind, hooks y clic/hover/scroll, todo en un solo closure de Rust.

**Rápido de escribir. Árbol de dependencias ligero** (4 dependencias directas requeridas, 26 resueltas con las características por defecto, frente a 67 de ratatui 0.30.2 + crossterm, medido en 2026-10).

[![Crate Badge]][Crate]
[![Docs Badge]][Docs]
[![CI Badge]][CI]
[![MSRV Badge]][Crate]
[![Downloads Badge]][Crate]
[![License Badge]][License]

[Índice de Docs] · [Inicio Rápido] · [Guía de Widgets] · [Guía de Patrones] · [Ejemplos] · [Guía de Modos de Ejecución] · [Arquitectura]

[English](../README.md) · [中文](README.zh-CN.md) · **Español** · [日本語](README.ja.md) · [한국어](README.ko.md)

</div>

> Este documento es un resumen traducido del [README en inglés](../README.md). Si hay diferencias, el README y las guías en inglés son la referencia.

SuperLightTUI (SLT) es una biblioteca TUI de modo inmediato para Rust con una gramática pública deliberadamente pequeña.
Escribes un closure, SLT lo llama en cada frame y la biblioteca se encarga del layout, el foco, el diff y el renderizado.
El estado son variables normales de Rust: no hay trait de aplicación, enum de mensajes ni árbol de widgets retenido que configurar.

## Inicio rápido

```sh
cargo add superlighttui
```

<!-- slt-check: {"name": "readme_es_hello", "features": ["crossterm"], "context": "module"} -->
```rust
fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut slt::Context| {
        ui.text("hello, world");
    })
}
```

5 líneas. Sin trait `App`. Sin `Model`/`Update`/`View`. Sin bucle de eventos manual.
Ctrl+C sale limpiamente por defecto (desactívalo con `RunConfig::handle_ctrl_c(false)`).

Con el repositorio clonado, prueba `cargo run --example hello`. La lista completa está en [Ejemplos].

## Características principales

- **Ratón activado por defecto**: clics, rueda y arrastre funcionan sin configuración (por SSH, solo clics y rueda); hover con `MouseMode::Hover`, y `MouseMode::Off` lo desactiva por completo.
- **Layout flexbox**: la mayoría de las pantallas se construyen con `row()`, `col()` y `container()`.
- **Encadenamiento estilo Tailwind**: estilos con métodos encadenados como `.bold().fg(Color::Cyan).p(1).gap(1)`.
- **Hooks**: estado local de componentes con hooks; el estado de la aplicación vive en variables normales de Rust.
- **Más de 50 widgets**: entradas, tablas, listas, pestañas, modales, toasts, gráficos, markdown, paleta de comandos y más.
- **Testing**: `TestBackend` permite verificar renderizado e interacción sin terminal.
- **Varios runtimes**: el mismo closure funciona a pantalla completa, inline, static + inline y en el navegador (`slt-wasm`).
- **Rendimiento**: en el arnés de comparación equivalente con ratatui, SLT renderiza cada frame más rápido; consulta la [Guía de Rendimiento].

## Aprende la biblioteca

Las guías siguientes están **en inglés**. Lee la ruta de aprendizaje en orden; cada guía asume las anteriores.
El [Índice de Docs] enumera todas las páginas.

| Paso | Guía | Qué cubre |
|------|------|-----------|
| 1 | [Inicio Rápido] | Instalación, primera app, modelo mental del closure, layout, estado de widgets |
| 2 | [Guía de Estado](guide/02-state.md) | Estado de la app vs hooks vs estado de widgets, y la regla del frame anterior |
| 3 | [Guía de Patrones] | Componentes, formularios, pantallas, modales, mensajes async, estructura de apps grandes |
| 4 | [Guía de Interacción](guide/04-interaction.md) | Ratón, teclado, foco, atajos, hover, arrastre, rueda y scroll |
| 5 | [Guía de Overlays](guide/05-overlays.md) | `Anchor` y posicionamiento de overlays comparado con CSS/Flutter/React Native |
| 6 | [Guía de Temas](guide/06-theming.md) | Struct Theme, presets, ThemeBuilder, temas personalizados |
| 7 | [Guía de Animación](guide/07-animation.md) | Tween, spring, keyframes, sequence, stagger |
| 8 | [Guía de Testing](guide/08-testing.md) | `TestBackend`, `EventBuilder`, tests de varios frames, snapshots |
| 9 | [Guía de Modos de Ejecución] | `run` vs `run_inline` vs `run_static`, `RunConfig`, controlar `frame()` directamente |
| 10 | [Guía del Navegador](guide/10-browser.md) | El runtime de navegador `slt-wasm` |
| 11 | [Compatibilidad de Terminales](guide/11-terminal-compatibility.md) | Matriz de soporte de terminales/multiplexores/SSH, variables de entorno, alternativas |

Si vienes de otro stack: [Guía para Desarrolladores Web](coming-from/web.md) (HTML/CSS/React → SLT) y [Guía de Ratatui](coming-from/ratatui.md) (widgets, layout y el puente incremental `ratatui`).

| Referencia | Qué cubre |
|------------|-----------|
| [Guía de Widgets] | Catálogo por categorías de widgets, métodos de runtime y tipos de estado |
| [Guía de API de Estado](reference/state.md) | Campos públicos, accesores y métodos de mutación del estado de widgets |
| [Cookbook](reference/cookbook.md) | Recetas para copiar y pegar de apps comunes, cada una respaldada por un ejemplo ejecutable |
| [Ejemplos] | Ejemplos ejecutables agrupados por tipo de producto y área de funcionalidad |
| [Guía de Features](reference/features.md) | Feature flags, dependencias opcionales, combinaciones recomendadas |
| [Guía de Migración](reference/migration.md) | Cambios incompatibles versión a versión y rutas de actualización |
| [Guía de Depuración](reference/debugging.md) | Overlay F12, clipping, sorpresas de foco |
| [Guía para IA](reference/ai-guide.md) | El camino más rápido para quienes construyen con IA y agentes de código |

Para contribuidores: [Principios de Diseño](internals/design-principles.md) (reglas de API y nombres), [Arquitectura] (mapa de módulos, ciclo de vida del frame) y [Guía de Rendimiento] (presupuestos y benchmarks).

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
[License]: ../LICENSE
[Índice de Docs]: README.md
[Inicio Rápido]: guide/01-quick-start.md
[Guía de Patrones]: guide/03-patterns.md
[Guía de Modos de Ejecución]: guide/09-runtime-modes.md
[Guía de Widgets]: reference/widgets.md
[Ejemplos]: reference/examples.md
[Arquitectura]: internals/architecture.md
[Guía de Rendimiento]: internals/performance.md
