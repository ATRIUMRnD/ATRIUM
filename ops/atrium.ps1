# atrium.ps1: PowerShell front for D:\dev\bin\atrium.exe (DUCTEI\atrium-ctl).
#
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Up            # foreground
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Up -Background # hidden, survives the terminal
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Status
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Verify       # queued to the live supervisor; rooms stay up
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Check        # read-only chain check, any time
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Down
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium.ps1 -Build        # rebuild atrium.exe into D:\dev\bin
#
# Exit codes pass through from atrium.exe: 0 ok, 2 a room is down, 3 no live
# supervisor, 1 verify failed, 75 another supervisor owns the state dir.
param(
    [switch]$Up, [switch]$Status, [switch]$Verify, [switch]$Check, [switch]$Down, [switch]$Build,
    [switch]$Background,
    [string]$Config = "D:\ATRIUM\ops\atrium.toml",
    [string]$Exe = "D:\dev\bin\atrium.exe"
)
$ErrorActionPreference = 'Stop'

if ($Build) {
    Push-Location D:\ATRIUM\DUCTEI
    try {
        # sign: ML-DSA receipts. qfltl: encrypted anchor transport; atrium.toml sets
        # anchor.encrypt = true, so a build without it refuses to start (fail closed).
        cargo build -q --release -p atrium-ctl --features "sign,qfltl"
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        New-Item -ItemType Directory -Force (Split-Path $Exe) | Out-Null
        Copy-Item target\release\atrium.exe $Exe -Force
        Copy-Item target\release\atrium-keepalive.exe (Join-Path (Split-Path $Exe) 'atrium-keepalive.exe') -Force
        Write-Host "[atrium] built -> $Exe (features sign,qfltl)"
    } finally { Pop-Location }
    if (-not ($Up -or $Status -or $Verify -or $Check -or $Down)) { exit 0 }
}

if (-not (Test-Path $Exe)) { Write-Error "[atrium] $Exe missing; run with -Build"; exit 66 }

$n = @($Up, $Status, $Verify, $Check, $Down) | Where-Object { $_ } | Measure-Object | Select-Object -ExpandProperty Count
if ($n -ne 1) { Write-Error "[atrium] pick exactly one of -Up -Status -Verify -Check -Down (or -Build)"; exit 64 }

if ($Up -and $Background) {
    # Run through the atrium scheduled task so the supervisor belongs to no terminal
    # (a hidden child of this shell dies when Windows Terminal closes the window).
    Start-ScheduledTask -TaskName atrium
    Start-Sleep -Seconds 3
    & $Exe status $Config
    exit 0
}

$verb = if ($Up) { 'up' } elseif ($Status) { 'status' } elseif ($Verify) { 'verify' } elseif ($Check) { 'check' } else { 'down' }
& $Exe $verb $Config
exit $LASTEXITCODE
