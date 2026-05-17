Output assets/demo.gif

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 760
Set Theme "Catppuccin Mocha"
Set PlaybackSpeed 1.0
Set TypingSpeed 60ms

Hide
Type "bash scripts/demo-seed.sh > /dev/null" Enter
Sleep 500ms
Type "export HOME=$PWD/.demo-home RONDO_FX=1" Enter
Type "clear" Enter
Show

Type "./target/release/rondo-tui"
Enter
Sleep 1800ms

# 1) Browse list, drill into detail sections
Down Sleep 350ms
Down Sleep 350ms
Down Sleep 350ms
Down Sleep 600ms
Type "l"
Sleep 500ms
Tab Sleep 600ms
Tab Sleep 600ms
Tab Sleep 600ms
Type "h"
Sleep 400ms

# 2) Quick-add a task with parser tokens
Type "a"
Sleep 500ms
Type "Wire KvGet host-fn #plugins !p3 due:tmrw"
Sleep 700ms
Enter
Sleep 1200ms

# 3) Filter to URG
Type "fu"
Sleep 1200ms
Type "fA"
Sleep 800ms

# 4) Fuzzy search
Type "/"
Sleep 300ms
Type "pomo"
Sleep 1200ms
Escape
Sleep 500ms

# 5) Journal page
Type "2"
Sleep 1400ms
Type "J" Sleep 500ms
Type "J" Sleep 500ms
Type "K" Sleep 500ms

# 6) Command palette -> analytics dashboard
Type ":"
Sleep 300ms
Type "analytics"
Sleep 500ms
Enter
Sleep 2200ms
Escape
Sleep 400ms

# 7) Pomodoro overlay
Type "1"
Sleep 800ms
Type "p"
Sleep 2500ms
Escape
Sleep 400ms

Type "q"
Sleep 400ms
