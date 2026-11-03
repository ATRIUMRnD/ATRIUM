#!/bin/zsh
set -e
W=~/dev/atrium-anchor/watch
cat > $W/watch.toml <<EOF
# atrium-watch on the Mac mini: follows the anchor peer's inbound log live.
inbound = "/Users/xingxerx/dev/atrium-anchor/state/inbound/accepted.jsonl"
dir = "/Users/xingxerx/dev/atrium-anchor/watch"
poll_ms = 250

[[stream]]
name = "atrium"
pubkey = "/Users/xingxerx/dev/atrium-anchor/watch/keys/atrium.pk"
silence_secs = 90

[[stream]]
name = "video-watch"
pubkey = "/Users/xingxerx/dev/atrium-anchor/watch/keys/video-watch.pk"
silence_secs = 0

[alert]
cmd = ["/Users/xingxerx/dev/atrium-anchor/watch/notify.sh"]
telegram_env = "/Users/xingxerx/dev/atrium-anchor/watch/telegram.env"
cooldown_secs = 60
EOF
cat > $W/notify.sh <<'EOF'
#!/bin/zsh
/usr/bin/osascript -e "display notification \"${1//\"/'}\" with title \"ATRIUM ALARM\" sound name \"Sosumi\""
EOF
chmod 700 $W/notify.sh
P=~/Library/LaunchAgents/com.xingxerx.atrium-watch.plist
cat > $P <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>com.xingxerx.atrium-watch</string>
  <key>ProgramArguments</key><array>
    <string>/Users/xingxerx/dev/atrium-anchor/DUCTEI/target/release/atrium-watch</string>
    <string>/Users/xingxerx/dev/atrium-anchor/watch/watch.toml</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ThrottleInterval</key><integer>5</integer>
  <key>StandardOutPath</key><string>/Users/xingxerx/dev/atrium-anchor/logs/watch.log</string>
  <key>StandardErrorPath</key><string>/Users/xingxerx/dev/atrium-anchor/logs/watch.log</string>
</dict></plist>
EOF
plutil -lint $P
launchctl bootout gui/$(id -u) $P 2>/dev/null || true
launchctl bootstrap gui/$(id -u) $P
sleep 2
tail -n 3 ~/dev/atrium-anchor/logs/watch.log