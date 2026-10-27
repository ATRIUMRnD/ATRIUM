<#
.SYNOPSIS
  Runs on xing-pc. Waits for xing-laptop to come online on the tailnet, then
  brings its work over to the PC safely. Meant to fire from a scheduled task
  every 10 minutes; exits fast when the laptop is offline.

.DESCRIPTION
  Reuses ops/entangle.ps1 (git over Tailscale SSH, fast-forward only,
  witness files). Two things happen once the laptop answers:
    1. entangle.ps1 verify, then -Pull: fast-forward PC rooms to the laptop
       where that is safe (dirty PC trees and divergence are refused, not forced).
    2. Uncommitted laptop work is copied as read-only patch files into
       ops/incoming/<stamp>/<room>.patch (plus untracked file lists). Nothing on
       the laptop is changed and nothing is applied on the PC.
  Never commits, pushes, stashes or deletes. A run marker stops repeat syncs
  within $MinGapMinutes.
#>
[CmdletBinding()]
param(
  [string]$Peer          = "xing-laptop",
  [string]$PeerUser      = "there",
  [string]$PeerRoot      = "C:/Users/there/ATRIUM",
  [string[]]$Rooms       = @(".", "DUCTEI", "LIMEN", "Qallow", "VEYN"),
  [int]$MinGapMinutes    = 60
)
$ErrorActionPreference = "Continue"
$opsDir   = $PSScriptRoot
$marker   = Join-Path $opsDir "incoming/.last-sync"
$logFile  = Join-Path $opsDir "incoming/await-laptop.log"
New-Item -ItemType Directory -Force -Path (Join-Path $opsDir "incoming") | Out-Null
function Log($t) { "$(Get-Date -Format s)  $t" | Add-Content -Path $logFile; Write-Host $t }

# 1. Is the laptop online? "tailscale status" marks offline peers with "offline".
$row = tailscale status 2>$null | Select-String -Pattern "\s$Peer\s"
if (-not $row)                     { exit 0 }   # not on the tailnet at all
if ("$row" -match "offline")       { exit 0 }   # asleep or powered off
if (Test-Path $marker) {
  $age = (Get-Date) - (Get-Item $marker).LastWriteTime
  if ($age.TotalMinutes -lt $MinGapMinutes) { exit 0 }
}

# 2. SSH must answer (laptop needs OpenSSH Server and this PC's key in authorized_keys).
$sshOpts = @("-o","BatchMode=yes","-o","ConnectTimeout=8","-o","StrictHostKeyChecking=accept-new")
$winSsh  = "C:/Windows/System32/OpenSSH/ssh.exe"
$hello = & $winSsh @sshOpts "$PeerUser@$Peer" "echo ok" 2>$null
if ($hello -ne "ok") { Log "laptop online but SSH refused or no key; nothing done"; exit 2 }
Log "laptop online; syncing"

# 3. Committed work: reuse entangle.ps1 (verify, then fast-forward PC to laptop).
$ent = Join-Path $opsDir "entangle.ps1"
& $ent -Peer $Peer -PeerUser $PeerUser -PeerRoot $PeerRoot -Rooms $Rooms | Out-Null
Log "entangle verify exit=$LASTEXITCODE"
& $ent -Peer $Peer -PeerUser $PeerUser -PeerRoot $PeerRoot -Rooms $Rooms -Pull | Out-Null
Log "entangle -Pull exit=$LASTEXITCODE (see ops/witness for per-room result)"

# 4. Uncommitted work: read-only snapshot as patches.
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$dest  = Join-Path $opsDir "incoming/$stamp"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
foreach ($r in $Rooms) {
  $name = if ($r -eq ".") { "ATRIUM" } else { $r }
  $path = if ($r -eq ".") { $PeerRoot } else { "$PeerRoot/$r" }
  $g = "git --no-optional-locks -C `"$path`""
  $branch = & $winSsh @sshOpts "$PeerUser@$Peer" "$g rev-parse --abbrev-ref HEAD" 2>$null
  $head   = & $winSsh @sshOpts "$PeerUser@$Peer" "$g rev-parse --short HEAD" 2>$null
  $diff   = & $winSsh @sshOpts "$PeerUser@$Peer" "$g diff --binary HEAD" 2>$null
  $untr   = & $winSsh @sshOpts "$PeerUser@$Peer" "$g ls-files --others --exclude-standard" 2>$null
  if ($diff) { $diff | Set-Content -Encoding utf8 (Join-Path $dest "$name.patch") }
  if ($untr) { $untr | Set-Content -Encoding utf8 (Join-Path $dest "$name.untracked.txt") }
  Log ("{0,-7} branch={1} head={2} patch={3} untracked={4}" -f $name, $branch, $head, [bool]$diff, (@($untr) | Where-Object { $_ }).Count)
}
Set-Content -Path $marker -Value $stamp
Log "done; snapshot in ops/incoming/$stamp (nothing applied, nothing committed)"
exit 0
