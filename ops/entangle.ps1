<#
.SYNOPSIS
  Entangle the ATRIUM rooms between this machine and a peer over Tailscale.

.DESCRIPTION
  Git over Tailscale SSH is the only transport. Nothing copies raw files.
  Two machines are "entangled" when every room's HEAD commit hash is identical
  on both sides. The script measures that state and writes a plain-English
  witness record to ops/witness/. It never force-pushes and never rewrites
  the peer: a divergence is reported, not resolved.

  Modes
    (default)   Verify: fetch from the peer and compare HEAD per room.
    -Fill       Clone any room the peer is missing (needs SSH to the peer).
    -Push       Fast-forward the peer to this machine where that is safe.
    -Pull       Fast-forward this machine to the peer where that is safe.

.EXAMPLE
  .\ops\entangle.ps1 -Peer xing-pc -PeerUser there -PeerRoot C:/Users/there/ATRIUM
#>
[CmdletBinding()]
param(
  [string]$Peer     = "xing-pc",
  [string]$PeerUser = "xing",
  [string]$PeerRoot = "D:/ATRIUM",
  [string[]]$Rooms  = @(".", "DUCTEI", "LIMEN", "Qallow", "VEYN"),
  [switch]$Fill,
  [switch]$Push,
  [switch]$Pull
)

# Native tools (ssh, git) write progress to stderr; under "Stop" PowerShell 5.1
# would turn that into a terminating error, so stay on "Continue" and check results explicitly.
$ErrorActionPreference = "Continue"
$sshOpts = @("-o", "BatchMode=yes", "-o", "ConnectTimeout=5", "-o", "StrictHostKeyChecking=accept-new")
# Git for Windows ships its own ssh, which does not see the key Windows OpenSSH holds.
# Pin git to the Windows OpenSSH client so git and the direct ssh calls below agree.
$winSsh = "C:/Windows/System32/OpenSSH/ssh.exe"
$env:GIT_SSH_COMMAND = "$winSsh " + ($sshOpts -join " ")
$here   = Split-Path -Parent $PSScriptRoot
$stamp  = Get-Date -Format "yyyyMMdd-HHmmss"
$report = Join-Path $PSScriptRoot "witness/entangle-$stamp.txt"
$lines  = @("entangle witness  $(Get-Date -Format s)  host=$env:COMPUTERNAME  peer=$Peer", "")

function Say($t) { Write-Host $t; $script:lines += $t }

# 1. Peer must be on the tailnet and answer SSH; otherwise stop and witness.
$ts = tailscale status 2>$null | Select-String -Pattern "\s$Peer\s"
if (-not $ts) { Say "STOP  $Peer is not visible on the tailnet (tailscale status)."; $lines | Set-Content $report; exit 2 }
$ping = ssh @sshOpts "$PeerUser@$Peer" "echo entangled" 2>$null
if ($ping -ne "entangled") {
  Say "STOP  SSH to $PeerUser@$Peer refused. Authorize this machine's public key on the peer"
  Say "      (see ops/ENTANGLE.md), then rerun."
  $lines | Set-Content $report; exit 2
}

$diverged = 0
foreach ($room in $Rooms) {
  $local = Join-Path $here $room
  $name  = if ($room -eq ".") { "ATRIUM" } else { $room }
  $remotePath = if ($room -eq ".") { $PeerRoot.TrimEnd("/") } else { $PeerRoot.TrimEnd("/") + "/" + $room }
  # scp-style URL: Windows sshd reads ssh://host/D:/x as "/D:/x" and git-receive-pack fails.
  $url = "${PeerUser}@${Peer}:$remotePath"

  if (-not (Test-Path (Join-Path $local ".git"))) { Say "$name  SKIP  not a git repo here"; continue }
  Push-Location $local
  try {
    $branch = git rev-parse --abbrev-ref HEAD
    $head   = git rev-parse HEAD
    $dirty  = (git status --porcelain | Measure-Object).Count

    # Peer missing the room? Fill it from origin so both sides share history roots.
    $peerHas = ssh @sshOpts "$PeerUser@$Peer" "git -C `"$remotePath`" rev-parse --is-inside-work-tree" 2>$null
    if ($peerHas -ne "true") {
      if ($Fill) {
        $origin = git remote get-url origin
        Say "$name  FILL  cloning $origin on $Peer at $remotePath"
        ssh @sshOpts "$PeerUser@$Peer" "git clone --quiet `"$origin`" `"$remotePath`""
      } else { Say "$name  MISSING on peer (rerun with -Fill)"; continue }
    }

    if (-not (git remote | Select-String -Quiet "^pc$")) { git remote add pc $url } else { git remote set-url pc $url }
    # A failed fetch must not fall back to stale pc/* refs from an earlier run.
    git fetch --quiet --prune pc 2>$null
    if ($LASTEXITCODE -ne 0) { Say "$name  FETCH FAILED from $url; not comparing against stale refs"; $diverged++; continue }
    $peerHead = git rev-parse "pc/$branch" 2>$null
    if (-not $peerHead) { Say "$name  peer has no branch '$branch'"; $diverged++; continue }

    $state = "DIVERGED"
    if ($head -eq $peerHead) { $state = "MATCH" }
    else {
      git merge-base --is-ancestor $peerHead $head 2>$null
      if ($LASTEXITCODE -eq 0) { $state = "AHEAD" }
      else {
        git merge-base --is-ancestor $head $peerHead 2>$null
        if ($LASTEXITCODE -eq 0) { $state = "BEHIND" }
      }
    }

    Say "$name  $state  local=$($head.Substring(0,7)) peer=$($peerHead.Substring(0,7)) branch=$branch dirty=$dirty"

    if ($state -eq "AHEAD"  -and $Push) { git push --quiet pc "${branch}:${branch}"; Say "$name  pushed -> peer now at $($head.Substring(0,7))" }
    if ($state -eq "BEHIND" -and $Pull -and $dirty -eq 0) { git merge --ff-only --quiet "pc/$branch"; Say "$name  pulled <- local now at $($peerHead.Substring(0,7))" }
    if ($state -eq "DIVERGED") { $diverged++; Say "$name  both sides moved. Reconcile by hand; nothing was overwritten." }
  } finally { Pop-Location }
}

$lines += ""
$lines += "result: $(if ($diverged -eq 0) { 'ENTANGLED' } else { "$diverged room(s) need a human" })"
$lines | Set-Content $report
Write-Host "`nwitness: $report"
if ($diverged -eq 0) { exit 0 } else { exit 1 }
