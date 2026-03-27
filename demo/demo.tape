Output demo.webp

Set Shell bash
Set FontSize 22
Set Width 1200
Set Height 600
Set Padding 30
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms

Hide
Type "source ./setup.sh"
Enter
Sleep 500ms
Type "clear"
Enter
Sleep 500ms
Show

Sleep 500ms

Type "tp init"
Sleep 500ms
Enter
Sleep 1.5s

Type "tp get feature/billing"
Sleep 500ms
Enter
Sleep 2s

Type "echo 'pub fn billing() {}' > billing.rs"
Sleep 300ms
Enter
Sleep 500ms

Type "git add -A && git commit -qm 'feat: add billing endpoint'"
Sleep 300ms
Enter
Sleep 1s

Type "exit"
Sleep 300ms
Enter
Sleep 1.5s

Type "tp get fix/auth-timeout"
Sleep 500ms
Enter
Sleep 2s

Type "echo 'pub fn auth() {}' > auth.rs"
Sleep 300ms
Enter
Sleep 500ms

Type "git add -A && git commit -qm 'fix: increase auth token TTL'"
Sleep 300ms
Enter
Sleep 1s

Type "exit"
Sleep 300ms
Enter
Sleep 1.5s

Type "tp status"
Sleep 500ms
Enter
Sleep 3s

Sleep 2s
