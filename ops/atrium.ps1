# atrium.ps1: PowerShell front for D:\dev\bin\atrium.exe (DUCTEI\atrium-ctl).
#
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Up            # foreground
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Up -Background # hidden, survives the terminal
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Status
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Verify
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Down
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Build        # rebuild atrium.exe into D:\dev\bin
#
# Exit codes pass through from atrium.exe: 0 ok, 2 a room is down, 3 no live
# supervisor, 1 verify failed, 75 another supervisor owns the state dir.
param(
    [switch]$Up, [switch]$Status, [switch]$Verify, [switch]$Down, [switch]$Build,
    [switch]$Background,
    [string]$Config = "D:\ATRIUM\ops\atrium.toml",
    [string]$Exe = "D:\dev\bin\atrium.exe"
)
$ErrorActionPreference = 'Stop'

if ($Build) {
    Push-Location D:\ATRIUM\DUCTEI
    try {
        cargo build -q --release -p atrium-ctl --features sign
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        New-Item -ItemType Directory -Force (Split-Path $Exe) | Out-Null
        Copy-Item target\release\atrium.exe $Exe -Force
        Write-Host "[atrium] built -> $Exe"
    } finally { Pop-Location }
    if (-not ($Up -or $Status -or $Verify -or $Down)) { exit 0 }
}

if (-not (Test-Path $Exe)) { Write-Error "[atrium] $Exe missing; run with -Build"; exit 66 }

$n = @($Up, $Status, $Verify, $Down) | Where-Object { $_ } | Measure-Object | Select-Object -ExpandProperty Count
if ($n -ne 1) { Write-Error "[atrium] pick exactly one of -Up -Status -Verify -Down (or -Build)"; exit 64 }

if ($Up -and $Background) {
    $state = (Select-String -Path $Config -Pattern '^\s*state\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
    New-Item -ItemType Directory -Force (Join-Path $state 'logs') | Out-Null
    $log = Join-Path $state 'logs\atrium-stdout.log'
    $p = Start-Process -FilePath $Exe -ArgumentList @('up', $Config) -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError (Join-Path $state 'logs\atrium-stderr.log')
    Write-Host "[atrium] supervisor started in background, pid $($p.Id); log $log"
    exit 0
}

$verb = if ($Up) { 'up' } elseif ($Status) { 'status' } elseif ($Verify) { 'verify' } else { 'down' }
& $Exe $verb $Config
exit $LASTEXITCODE
