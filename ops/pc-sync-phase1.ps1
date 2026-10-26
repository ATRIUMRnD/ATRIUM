$ErrorActionPreference = "Continue"
$root = 'D:\ATRIUM'
function Room($r) {
  $p = if ($r -eq '.') { $root } else { Join-Path $root $r }
  $name = if ($r -eq '.') { 'ATRIUM' } else { $r }
  "===== $name"
  $dirty = (git -C $p status --porcelain | Measure-Object).Count
  if ($dirty -gt 0) {
    git -C $p stash push -u -m "pc WIP 2026-10-25 before sync" | Out-Null
    "stashed $dirty paths"
  }
  git -C $p pull --ff-only --quiet origin 2>&1 | Out-String
  "HEAD now " + (git -C $p rev-parse --short HEAD) + " " + (git -C $p rev-parse --abbrev-ref HEAD)
  if ($dirty -gt 0) {
    $out = git -C $p stash pop 2>&1 | Out-String
    $conf = git -C $p diff --name-only --diff-filter=U
    if ($conf) { "CONFLICTS:"; $conf; "stash kept:"; git -C $p stash list | Select-Object -First 1 }
    else { "pop clean"; git -C $p status --porcelain }
  }
  ""
}
Room 'Qallow'
"===== VEYN"
if (-not (Test-Path 'D:\ATRIUM\VEYN\.git')) {
  git clone --quiet https://github.com/ATRIUMRnD/VEYN.git D:\ATRIUM\VEYN 2>&1 | Out-String
  git -C D:\ATRIUM\VEYN checkout --quiet fix/clippy-double-must-use 2>&1 | Out-String
  "VEYN " + (git -C D:\ATRIUM\VEYN rev-parse --abbrev-ref HEAD) + " " + (git -C D:\ATRIUM\VEYN rev-parse --short HEAD)
}
""
Room '.'
Room 'LIMEN'
Room 'DUCTEI'
