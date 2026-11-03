#!/bin/zsh
# atrium fleet forced command: the fleet key can do exactly these three things.
A=/Users/xingxerx/dev/atrium-anchor
case "$SSH_ORIGINAL_COMMAND" in
  status)
    a=$(launchctl list | awk '$3=="com.xingxerx.ductei-anchor"{print ($1=="-")?"DOWN":"up"}')
    w=$(launchctl list | awk '$3=="com.xingxerx.atrium-watch"{print ($1=="-")?"DOWN":"up"}')
    n=$(wc -l < $A/watch/alarms.jsonl 2>/dev/null | tr -d ' ')
    s=$(sed -E 's/.*"atrium":\{"seq":([0-9]+).*/\1/' $A/watch/watch-state.json 2>/dev/null)
    echo "anchor-peer ${a:-missing}, watcher ${w:-missing}, alarms ${n:-0}, last atrium seq ${s:-?}" ;;
  alarms)
    tail -n 10 $A/watch/alarms.jsonl 2>/dev/null ;;
  restart-anchor)
    launchctl kickstart -k gui/$(id -u)/com.xingxerx.ductei-anchor && launchctl kickstart -k gui/$(id -u)/com.xingxerx.atrium-watch && echo "anchor-peer and watcher restarted" ;;
  *)
    echo "refused: $SSH_ORIGINAL_COMMAND" >&2; exit 3 ;;
esac