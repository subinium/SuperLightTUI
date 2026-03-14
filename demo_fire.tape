# DOOM Fire Effect — VHS recording tape
# Usage: vhs demo_fire.tape
# Requires: cargo, vhs (brew install charmbracelet/tap/vhs)

Output assets/demo_fire.gif

Set Shell "bash"
Set Theme "Dracula"
Set FontSize 14
Set Width 1200
Set Height 700
Set Padding 0
Set Margin 0
Set Framerate 60
Set PlaybackSpeed 0.5
Set LoopOffset 25%

Require cargo

Hide
Type "cargo build --release --example demo_fire 2>&1 | tail -1"
Enter
Sleep 30s
Type "clear"
Enter
Sleep 1s
Show

Type "cargo run --release --example demo_fire"
Enter
Sleep 8s

Hide
Type "q"
Sleep 500ms
