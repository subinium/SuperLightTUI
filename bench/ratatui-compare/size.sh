#!/usr/bin/env bash
# Release binary size: SLT vs ratatui 0.30.2 (issue #491).
#
# Two programs, each written for both libraries and built as its own crate:
#
# - hello:     `slt::run` + `ui.text` + `ui.key('q')` / `ratatui::run` +
#              `render_widget(&str)` + `event::read`.
# - dashboard: header, separator, a 200-row list with selection, a 100x3
#              table with selection, a gauge that follows the list, Tab to
#              move focus, Up/Down/j/k to move the selection, `q` to quit.
#              The ratatui version handles the same keys with its own
#              `ListState` / `TableState` and a focus enum (SLT's widgets do
#              that themselves; SLT also handles mouse clicks on rows).
#
# Two profiles, applied to both libraries equally:
#
# - release: the default `[profile.release]` (no LTO, opt-level 3,
#            codegen-units 16, unwind), then `strip`.
# - size:    `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`,
#            `panic = "abort"`, `strip = true` (the recipe in
#            docs/internals/performance.md "Binary size").
#
#   bash bench/ratatui-compare/size.sh          # print the markdown tables
#   SLT_HELLO_MAX_BYTES=1200000 bash ...        # also exit 1 when SLT hello
#                                               # (default features, release,
#                                               # stripped) exceeds the limit
#
# Record the tables in docs/internals/performance.md ("Binary size") on every
# release. Linker and toolchain changes move the numbers by a few KB, so gate
# only on large deltas. Scratch crates live in this crate's ignored target/
# directory.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
work="$here/target/size"
mkdir -p "$work"
export CARGO_TARGET_DIR="$work/target"
export CARGO_INCREMENTAL=0

slt_dep="superlighttui = { path = \"$repo\" }"
slt_ct_dep="superlighttui = { path = \"$repo\", default-features = false, features = [\"crossterm\"] }"
rat_dep='ratatui = "=0.30.2"'

new_crate() { # name dependency-line source-file
    mkdir -p "$work/$1/src"
    cat >"$work/$1/Cargo.toml" <<EOF
[package]
name = "$1"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
$2

[profile.size]
inherits = "release"
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
EOF
    cp "$3" "$work/$1/src/main.rs"
}

src="$work/sources"
mkdir -p "$src"

cat >"$src/slt-hello.rs" <<'EOF'
fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut slt::Context| {
        ui.text("Hello, world! (q to quit)");
        if ui.key('q') {
            ui.quit();
        }
    })
}
EOF

cat >"$src/ratatui-hello.rs" <<'EOF'
use ratatui::crossterm::event::{self, Event, KeyCode};
fn main() -> std::io::Result<()> {
    ratatui::run(|terminal| loop {
        terminal.draw(|f| f.render_widget("Hello, world! (q to quit)", f.area()))?;
        if let Event::Key(k) = event::read()? {
            if k.code == KeyCode::Char('q') {
                break Ok(());
            }
        }
    })
}
EOF

cat >"$src/slt-dashboard.rs" <<'EOF'
use slt::{Border, Context, ListState, TableState};

const ITEMS: usize = 200;
const ROWS: usize = 100;

fn main() -> std::io::Result<()> {
    let mut list = ListState::new((0..ITEMS).map(|i| format!("Item {i}")).collect());
    let rows: Vec<Vec<String>> = (0..ROWS)
        .map(|i| vec![i.to_string(), format!("name-{i}"), (i * 7 % 100).to_string()])
        .collect();
    let mut table = TableState::new(vec!["ID", "Name", "Score"], rows);
    slt::run(move |ui: &mut Context| {
        if ui.key('q') {
            ui.quit();
        }
        ui.text("Dashboard  (Tab: focus, Up/Down: move, q: quit)").bold();
        let _ = ui.separator();
        let _ = ui.container().grow(1).row(|ui| {
            let _ = ui.bordered(Border::Rounded).title("List").grow(1).col(|ui| {
                let _ = ui.list(&mut list);
            });
            let _ = ui.bordered(Border::Rounded).title("Table").grow(2).col(|ui| {
                let _ = ui.table(&mut table);
            });
        });
        let _ = ui.progress(list.selected_view_index() as f64 / (ITEMS - 1) as f64);
    })
}
EOF

cat >"$src/ratatui-dashboard.rs" <<'EOF'
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, List, ListState, Paragraph, Row, Table, TableState};

const ITEMS: usize = 200;
const ROWS: usize = 100;

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    List,
    Table,
}

fn main() -> std::io::Result<()> {
    let items: Vec<String> = (0..ITEMS).map(|i| format!("Item {i}")).collect();
    let rows: Vec<[String; 3]> = (0..ROWS)
        .map(|i| [i.to_string(), format!("name-{i}"), (i * 7 % 100).to_string()])
        .collect();
    let mut list_state = ListState::default().with_selected(Some(0));
    let mut table_state = TableState::default().with_selected(Some(0));
    let mut focus = Focus::List;
    ratatui::run(|terminal| loop {
        terminal.draw(|f| {
            use Constraint::{Fill, Length};
            let [header, sep, body, gauge] =
                Layout::vertical([Length(1), Length(1), Fill(1), Length(1)]).areas(f.area());
            let [left, right] = Layout::horizontal([Fill(1), Fill(2)]).areas(body);
            f.render_widget(
                Paragraph::new("Dashboard  (Tab: focus, Up/Down: move, q: quit)").bold(),
                header,
            );
            f.render_widget(Block::new().borders(Borders::TOP), sep);
            let selected = Style::new().bold().reversed();
            let list = List::new(items.iter().map(String::as_str))
                .block(Block::bordered().border_type(BorderType::Rounded).title("List"))
                .highlight_style(selected)
                .highlight_symbol("▸ ");
            f.render_stateful_widget(list, left, &mut list_state);
            let table = Table::new(
                rows.iter().map(|r| Row::new(r.iter().map(String::as_str))),
                [Length(4), Fill(1), Length(5)],
            )
            .header(Row::new(["ID", "Name", "Score"]).bold())
            .block(Block::bordered().border_type(BorderType::Rounded).title("Table"))
            .row_highlight_style(selected);
            f.render_stateful_widget(table, right, &mut table_state);
            let ratio = list_state.selected().unwrap_or(0) as f64 / (ITEMS - 1) as f64;
            f.render_widget(Gauge::default().ratio(ratio), gauge);
        })?;
        if let Event::Key(k) = event::read()? {
            if k.kind != KeyEventKind::Press {
                continue;
            }
            match (k.code, focus) {
                (KeyCode::Char('q'), _) => break Ok(()),
                (KeyCode::Tab | KeyCode::BackTab, Focus::List) => focus = Focus::Table,
                (KeyCode::Tab | KeyCode::BackTab, Focus::Table) => focus = Focus::List,
                (KeyCode::Up | KeyCode::Char('k'), Focus::List) => list_state.select_previous(),
                (KeyCode::Down | KeyCode::Char('j'), Focus::List) => list_state.select_next(),
                (KeyCode::Up | KeyCode::Char('k'), Focus::Table) => table_state.select_previous(),
                (KeyCode::Down | KeyCode::Char('j'), Focus::Table) => table_state.select_next(),
                _ => {}
            }
        }
    })
}
EOF

# name | dependency | source
crates=(
    "slt-hello|$slt_dep|slt-hello"
    "slt-hello-crossterm|$slt_ct_dep|slt-hello"
    "ratatui-hello|$rat_dep|ratatui-hello"
    "slt-dashboard|$slt_dep|slt-dashboard"
    "slt-dashboard-crossterm|$slt_ct_dep|slt-dashboard"
    "ratatui-dashboard|$rat_dep|ratatui-dashboard"
)

bytes() { wc -c <"$1" | tr -d ' '; }
sz() { bytes "$work/$1"; } # <name>.<profile> -> stripped bytes

for entry in "${crates[@]}"; do
    IFS='|' read -r name dep source <<<"$entry"
    new_crate "$name" "$dep" "$src/$source.rs"
    for profile in release size; do
        cargo build --quiet --profile "$profile" --manifest-path "$work/$name/Cargo.toml"
        out="$work/$name.$profile"
        cp "$CARGO_TARGET_DIR/$profile/$name" "$out"
        strip "$out"
    done
done

ratio() { # numerator denominator
    local r=$(( $1 * 100 / $2 ))
    printf '%d.%02dx' $(( r / 100 )) $(( r % 100 ))
}

echo "- host: $(uname -m)-$(uname -s)"
echo "- rustc: $(rustc --version)"
echo "- superlighttui: $(grep -m1 '^version' "$repo/Cargo.toml") ($(git -C "$repo" rev-parse --short HEAD))"
echo "- ratatui: 0.30.2 (default features)"
echo
echo "| Program | Library | release (stripped) | size profile |"
echo "|---|---|---|---|"
for entry in "${crates[@]}"; do
    IFS='|' read -r name _ _ <<<"$entry"
    case "$name" in
        *-crossterm) lib='SLT `crossterm` only' ;;
        slt-*) lib='SLT default' ;;
        *) lib='ratatui default' ;;
    esac
    program="${name#slt-}"
    program="${program#ratatui-}"
    program="${program%-crossterm}"
    echo "| $program | $lib | $(sz $name.release) | $(sz $name.size) |"
done
echo
echo "| SLT default / ratatui | release | size profile |"
echo "|---|---|---|"
for program in hello dashboard; do
    echo "| $program | $(ratio "$(sz slt-$program.release)" "$(sz ratatui-$program.release)") | $(ratio "$(sz slt-$program.size)" "$(sz ratatui-$program.size)") |"
done

slt_hello="$(sz slt-hello.release)"
if [ -n "${SLT_HELLO_MAX_BYTES:-}" ] && [ "$slt_hello" -gt "$SLT_HELLO_MAX_BYTES" ]; then
    echo "slt-hello stripped size $slt_hello exceeds SLT_HELLO_MAX_BYTES=$SLT_HELLO_MAX_BYTES" >&2
    exit 1
fi
