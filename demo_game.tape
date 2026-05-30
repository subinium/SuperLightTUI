# Games Demo — VHS recording tape (animated)
# Usage: vhs demo_game.tape
# Mirrors demo_fire.tape (animated GIF archetype).

Output assets/demo_game.gif

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
Show

Type "./target/release/examples/demo_game"
Enter
Sleep 4s

Hide
Type "q"
Sleep 500ms
