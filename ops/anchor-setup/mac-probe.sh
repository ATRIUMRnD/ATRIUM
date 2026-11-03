#!/bin/zsh
cd ~/dev/atrium-anchor
ls
grep -rl "ductei-sync" ~/Library/LaunchAgents 2>/dev/null
for f in $(grep -rl "ductei-sync" ~/Library/LaunchAgents 2>/dev/null); do echo "== $f"; cat "$f"; done
./DUCTEI/target/release/ductei-sync key-show state 2>&1
ls state/qfltl state/qfltl/* 2>/dev/null
git -C DUCTEI log --oneline -1