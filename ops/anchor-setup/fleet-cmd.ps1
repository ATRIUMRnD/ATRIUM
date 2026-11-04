# atrium fleet forced command (Windows laptop): the fleet key can do exactly these three things.
# Installed at C:\Users\there\anchor-peer\fleet-cmd.ps1; wired by command= in administrators_authorized_keys.
$c = "$env:SSH_ORIGINAL_COMMAND".Trim()
function St($n) { $t = Get-ScheduledTask -TaskName $n -ErrorAction SilentlyContinue; if (-not $t) { 'missing' } elseif ($t.State -eq 'Running') { 'up' } else { 'DOWN' } }
switch ($c) {
  'status' {
    $acc = 'C:\Users\there\anchor-peer\inbound\accepted.jsonl'
    $n = if (Test-Path $acc) { (Get-Content $acc | Measure-Object -Line).Lines } else { 0 }
    "anchor-peer $(St 'atrium-anchor-peer'), node-watchdog $(St 'ductei-node-watchdog'), sync-serve $(St 'ductei-sync-serve'), anchors accepted $n"
  }
  'alarms' { Get-Content 'C:\Users\there\anchor-peer\inbound\rejected.jsonl' -Tail 10 -ErrorAction SilentlyContinue }
  'restart-anchor' {
    Stop-ScheduledTask -TaskName 'atrium-anchor-peer' -ErrorAction SilentlyContinue
    Start-ScheduledTask -TaskName 'atrium-anchor-peer'
    'anchor-peer restarted'
  }
  default { [Console]::Error.WriteLine("refused: $c"); exit 3 }
}
