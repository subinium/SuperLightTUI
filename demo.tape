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
Type "./target/release/examples/demo"
Enter
Sleep 2s
Show

Sleep 1s

Hide
Type "q"
Sleep 500ms
