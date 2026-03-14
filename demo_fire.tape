# DOOM Fire Effect — VHS recording tape
# Usage: vhs demo_fire.tape
# Requires: cargo, vhs (brew install charmbracelet/tap/vhs)

Output assets/demo_fire.gif

Set Shell "bash"
Set Theme "Dracula"
Set FontSize 16
Set Width 800
Set Height 500
Set Padding 0
Set Margin 0
Set Framerate 30
Set PlaybackSpeed 1.0
Set LoopOffset 20%

Require cargo

Hide
Type "cargo build --release --example demo_fire 2>&1"
Enter
Sleep 60s
Type "clear"
Enter
Sleep 1s
Show

Type "./target/release/examples/demo_fire"
Enter
Sleep 4s

Hide
Type "q"
Sleep 500ms
