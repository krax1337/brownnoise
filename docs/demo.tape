# Regenerate with: cargo build --release && vhs docs/demo.tape
# (records the local build; plays quietly through your speakers)
Output docs/demo.gif

Set Shell "bash"
Set FontSize 16
Set Width 1320
Set Height 150
Set Padding 24
Set Theme "Catppuccin Mocha"
Set TypingSpeed 55ms

Hide
Type "unset NO_COLOR; export XDG_CONFIG_HOME=$(mktemp -d) PATH=$PWD/target/release:$PATH PS1='$ ' && clear"
Enter
Show

Type "brownnoise -v 10"
Sleep 400ms
Enter
Sleep 2.5s

# volume up
Up@450ms 2
Sleep 800ms
# tone brighter, then deeper
Right@350ms 3
Sleep 600ms
Left@350ms 6
Sleep 800ms
# reset tone
Type "r"
Sleep 1s
# colors
Type "p"
Sleep 1.2s
Type "w"
Sleep 1.2s
Type "b"
Sleep 1s
# pause / resume
Space
Sleep 1.2s
Space
Sleep 1s
# quit with fade
Type "q"
Sleep 2.2s
