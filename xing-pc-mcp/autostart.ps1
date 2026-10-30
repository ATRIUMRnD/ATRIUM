# xing-pc-mcp autostart: registers a Windows scheduled task that starts the server at logon,
# hidden, and restarts it up to 5 times if it crashes. Run once in PowerShell:
#   D:\ATRIUM\xing-pc-mcp\autostart.ps1            (install)
#   D:\ATRIUM\xing-pc-mcp\autostart.ps1 -Remove    (uninstall)
param([switch]$Remove)
$ErrorActionPreference = 'Stop'
$name = 'xing-pc-mcp'
$exe  = 'D:\ATRIUM\xing-pc-mcp\target\release\xing-pc-mcp.exe'

if ($Remove) {
    Unregister-ScheduledTask -TaskName $name -Confirm:$false -ErrorAction SilentlyContinue
    Write-Host "autostart removed"
    return
}
if (-not (Test-Path $exe)) { throw "server not built: $exe" }

$action   = New-ScheduledTaskAction -Execute $exe -WorkingDirectory 'D:\ATRIUM\xing-pc-mcp'
$trigger  = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
            -RestartCount 5 -RestartInterval (New-TimeSpan -Minutes 1) `
            -ExecutionTimeLimit ([TimeSpan]::Zero) -MultipleInstances IgnoreNew -Hidden
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
Register-ScheduledTask -TaskName $name -Action $action -Trigger $trigger -Settings $settings `
    -Principal $principal -Force | Out-Null
Write-Host "autostart installed: $name starts at logon (limited rights, restarts on crash)"
