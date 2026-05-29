# Spreadsheet Demo — VHS screenshot
# Usage: vhs demo_spreadsheet.tape

Output assets/demo_spreadsheet.png

Set Shell "bash"
Set Theme "Dracula"
Set FontSize 16
Set Width 1000
Set Height 600
Set Padding 0
Set Margin 0

Require cargo

Hide
Type "./target/release/examples/demo_spreadsheet"
Enter
Sleep 2s
Show

Sleep 1s

Hide
Type "q"
Sleep 500ms
