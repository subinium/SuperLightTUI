#!/usr/bin/env bash
# Hello-world release binary size: SLT (default and crossterm-only) vs
# ratatui 0.30.2 (issue #491). Each program is its own crate with the default
# `[profile.release]` (no LTO, opt-level 3, unwind), built with
# `cargo build --release` and then stripped.
#
#   bash bench/ratatui-compare/size.sh          # print the markdown table
#   SLT_HELLO_MAX_BYTES=1200000 bash ...        # also exit 1 when SLT default
#                                               # (stripped) exceeds the limit
#
# Record the table in docs/PERFORMANCE.md ("Binary size") on every release.
# Linker and toolchain changes move the numbers by a few KB, so gate only on
# large deltas. Scratch crates live in this crate's ignored target/ directory.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
work="$here/target/size"
mkdir -p "$work"
export CARGO_TARGET_DIR="$work/target"
export CARGO_INCREMENTAL=0

new_crate() { # name dependency-line
    mkdir -p "$work/$1/src"
    printf '[package]\nname = "%s"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n[workspace]\n[dependencies]\n%s\n' \
        "$1" "$2" >"$work/$1/Cargo.toml"
}

new_crate slt-default "superlighttui = { path = \"$repo\" }"
new_crate slt-crossterm \
    "superlighttui = { path = \"$repo\", default-features = false, features = [\"crossterm\"] }"
new_crate ratatui-default 'ratatui = "=0.30.2"'

cat >"$work/slt-default/src/main.rs" <<'EOF'
fn main() -> std::io::Result<()> {
    slt::run(|ui: &mut slt::Context| {
        ui.text("Hello, world! (q to quit)");
        if ui.key('q') {
            ui.quit();
        }
    })
}
EOF
cp "$work/slt-default/src/main.rs" "$work/slt-crossterm/src/main.rs"

cat >"$work/ratatui-default/src/main.rs" <<'EOF'
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

bytes() { wc -c <"$1" | tr -d ' '; }

echo "- host: $(uname -m)-$(uname -s)"
echo "- rustc: $(rustc --version)"
echo "- superlighttui: $(grep -m1 '^version' "$repo/Cargo.toml") ($repo)"
echo
echo "| Program | Unstripped bytes | Stripped bytes |"
echo "|---|---|---|"
slt_default=0
ratatui_default=0
for name in slt-default slt-crossterm ratatui-default; do
    cargo build --quiet --release --manifest-path "$work/$name/Cargo.toml"
    bin="$CARGO_TARGET_DIR/release/$name"
    cp "$bin" "$work/$name.stripped"
    strip "$work/$name.stripped"
    stripped="$(bytes "$work/$name.stripped")"
    echo "| $name | $(bytes "$bin") | $stripped |"
    if [ "$name" = slt-default ]; then slt_default="$stripped"; fi
    if [ "$name" = ratatui-default ]; then ratatui_default="$stripped"; fi
done
ratio=$(( slt_default * 100 / ratatui_default ))
printf '\nslt-default / ratatui-default (stripped): %d.%02dx\n' $(( ratio / 100 )) $(( ratio % 100 ))

if [ -n "${SLT_HELLO_MAX_BYTES:-}" ] && [ "$slt_default" -gt "$SLT_HELLO_MAX_BYTES" ]; then
    echo "slt-default stripped size $slt_default exceeds SLT_HELLO_MAX_BYTES=$SLT_HELLO_MAX_BYTES" >&2
    exit 1
fi
