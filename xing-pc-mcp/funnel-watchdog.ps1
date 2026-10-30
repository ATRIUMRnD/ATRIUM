# xing-pc-mcp Funnel watchdog. Checks that Tailscale Funnel still forwards the public
# address to the local server, and turns it back on if not. Safe to run any time.
#   D:\ATRIUM\xing-pc-mcp\funnel-watchdog.ps1              (check once, fix if needed)
#   D:\ATRIUM\xing-pc-mcp\funnel-watchdog.ps1 -Install     (run every 5 minutes, hidden)
#   D:\ATRIUM\xing-pc-mcp\funnel-watchdog.ps1 -Remove      (stop running it)
param([switch]$Install, [switch]$Remove)
$ErrorActionPreference = 'Continue'
$task   = 'xing-pc-mcp-funnel'
$port   = 8787
$public = 'https://xing-pc.tail8b5cd4.ts.net/mcp'
$log    = Join-Path $PSScriptRoot 'funnel-watchdog.log'

function Log($m) {
    if ((Test-Path $log) -and (Get-Item $log).Length -gt 256KB) { Move-Item $log "$log.old" -Force }
    "$(Get-Date -Format s) $m" | Add-Content -Path $log -Encoding utf8
}

if ($Remove) {
    Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
    Write-Host "funnel watchdog removed"
    return
}
if ($Install) {
    $ps = (Get-Command powershell.exe).Source
    $action   = New-ScheduledTaskAction -Execute $ps -WorkingDirectory $PSScriptRoot `
                -Argument "-NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    $trigger  = New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(1) -RepetitionInterval (New-TimeSpan -Minutes 5)
    $logon    = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
                -ExecutionTimeLimit (New-TimeSpan -Minutes 2) -MultipleInstances IgnoreNew -Hidden
    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
    Register-ScheduledTask -TaskName $task -Action $action -Trigger @($logon, $trigger) -Settings $settings `
        -Principal $principal -Force | Out-Null
    Write-Host "funnel watchdog installed: checks every 5 minutes (limited rights)"
    return
}

$ts = Get-Command tailscale -ErrorAction SilentlyContinue
if (-not $ts) { Log "tailscale not found on PATH"; exit 1 }

# 1. Local server must be up first; restarting it is the other task's job.
$local = $false
try { $null = Invoke-WebRequest "http://127.0.0.1:$port/mcp" -Method Post -Body '{}' -ContentType 'application/json' -UseBasicParsing -TimeoutSec 5; $local = $true }
catch { if ($_.Exception.Response) { $local = $true } }
if (-not $local) { Log "local server not answering on $port; leaving Funnel alone"; exit 0 }

# 2. Funnel config must point at the server.
$status = (& tailscale funnel status 2>&1) -join "`n"
$configured = $status -match "127\.0\.0\.1:$port" -and $status -match 'Funnel on'

# 3. The public address must answer (401 = auth working = alive).
$reachable = $false
if ($configured) {
    try { $null = Invoke-WebRequest $public -Method Post -Body '{}' -ContentType 'application/json' -UseBasicParsing -TimeoutSec 10; $reachable = $true }
    catch { if ($_.Exception.Response) { $reachable = $true } }
}

if ($configured -and $reachable) { exit 0 }

Log "funnel down (configured=$configured reachable=$reachable); re-enabling"
& tailscale funnel --bg $port 2>&1 | ForEach-Object { Log "  $_" }
Start-Sleep -Seconds 3
$after = (& tailscale funnel status 2>&1) -join "`n"
if ($after -match "127\.0\.0\.1:$port") { Log "funnel restored" } else { Log "funnel still down: $after"; exit 1 }
