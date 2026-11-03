# Open xing-pc-mcp to Claude for a limited window, then close it automatically.
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\security\funnel-on.ps1            (2 hours)
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\security\funnel-on.ps1 -Hours 4
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\security\funnel-on.ps1 -Off
param([double]$Hours = 2, [switch]$Off)
$task = 'xing-pc-mcp-funnel-off'
$log  = 'D:\ATRIUM\ops\security\funnel.log'
function Log($m) { "$(Get-Date -Format s) $m" | Tee-Object -FilePath $log -Append }

if ($Off) {
    & tailscale funnel reset 2>&1 | Out-Null
    Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
    Log 'funnel OFF'; return
}

& tailscale funnel --bg 8787 2>&1 | Out-Null
$at = (Get-Date).AddHours($Hours)
$ps = (Get-Command powershell.exe).Source
$action = New-ScheduledTaskAction -Execute $ps -Argument "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Off"
Register-ScheduledTask -TaskName $task -Action $action -Trigger (New-ScheduledTaskTrigger -Once -At $at) `
    -Settings (New-ScheduledTaskSettingsSet -Hidden) -Force | Out-Null
Log "funnel ON until $($at.ToString('s'))"
