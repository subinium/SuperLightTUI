# Pretext Reflow Demo — VHS recording tape (animated)
# Usage: vhs demo_pretext.tape
# Mirrors demo_fire.tape (animated GIF archetype).

Output assets/demo_pretext.gif

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
Type "cargo build --release --example demo_pretext 2>&1"
Enter
Sleep 60s
Type "clear"
Enter
Sleep 1s
Show

Type "./target/release/examples/demo_pretext"
Enter
Sleep 4s

Hide
Type "q"
Sleep 500ms
