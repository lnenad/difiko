# vhs demo recording script — produces demo.gif
# install vhs: https://github.com/charmbracelet/vhs
# then run:    vhs demo.tape

Output demo.gif

Set FontSize 14
Set Width 1200
Set Height 720
Set Theme "Catppuccin Mocha"
Set Padding 20

# adjust this path to a repo with interesting diffs
Type "difiko --repo ~/projects/some-repo --base main --compare HEAD"
Enter
Sleep 2s

# walk a few files
Down@200ms 4
Sleep 1s

# open fullscreen
Type "F"
Sleep 2s

# scroll inside the diff
Down@100ms 10
Sleep 1s

# next file
Type "J"
Sleep 1s

# toggle blame
Type "b"
Sleep 2s

# toggle split view
Type "u"
Sleep 2s

# back out
Escape
Sleep 1s

# help overlay
Type "?"
Sleep 2s
Escape

# quit
Type "q"
Sleep 500ms
