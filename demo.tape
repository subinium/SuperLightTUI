# Widget Demo — VHS screenshot
# Usage: vhs demo.tape
# Mirrors demo_spreadsheet.tape (static screenshot archetype).

Output assets/demo.png

Set Shell "bash"
Set Theme "Dracula"
Set FontSize 16
Set Width 1200
Set Height 800
Set Padding 0
Set Margin 0

Require cargo

Hide
Type "cargo build --release --example demo 2>&1"
Enter
Sleep 60s
Type "clear"
Enter
Sleep 1s
Type "./target/release/examples/demo"
Enter
Sleep 2s
Show

Sleep 1s

Hide
Type "q"
Sleep 500ms
