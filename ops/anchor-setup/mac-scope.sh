#!/bin/zsh
set -e
cd ~/dev/atrium-anchor
chmod 600 state/qfltl/identity.seed
./DUCTEI/target/release/ductei-sync key-pin state --role client --peer xing-pc-atrium --file state/pc-atrium-client.pub 2>&1 | head -2
P=~/Library/LaunchAgents/com.xingxerx.ductei-anchor.plist
cp $P ~/dev/atrium-anchor/ductei-anchor.plist.pre-atrium.bak
grep -q 'mcp:atrium:anchor' $P || sed -i '' 's#<string>--scope</string><string>mcp:video-watch:anchor</string>#<string>--scope</string><string>mcp:video-watch:anchor</string>\
    <string>--scope</string><string>mcp:atrium:anchor</string>#' $P
plutil -lint $P
launchctl bootout gui/$(id -u) $P 2>/dev/null || true
launchctl bootstrap gui/$(id -u) $P
sleep 3
tail -n 2 logs/anchor.log