$ErrorActionPreference = 'Stop'
Disable-ScheduledTask -TaskName DucteiAnchorNode | Out-Null
Stop-Process -Id 21936 -Force -ErrorAction SilentlyContinue
Get-CimInstance Win32_Process -Filter "Name='ductei-node.exe'" | Where-Object CommandLine -match '7421' | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
Start-Sleep 1
$a = New-ScheduledTaskAction -Execute 'conhost.exe' -Argument '--headless C:\Users\there\anchor-peer\anchor-serve.cmd' -WorkingDirectory 'C:\Users\there\anchor-peer'
$t = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$s = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1)
Register-ScheduledTask -TaskName atrium-anchor-peer -Action $a -Trigger $t -Settings $s -Force | Out-Null
Start-ScheduledTask -TaskName atrium-anchor-peer
Start-Sleep 3
Get-NetTCPConnection -LocalPort 7421 -State Listen | ForEach-Object { $p = Get-CimInstance Win32_Process -Filter "ProcessId=$($_.OwningProcess)"; "$($_.LocalAddress):$($_.LocalPort) <- $($p.CommandLine)" }
Get-Content C:\Users\there\anchor-peer\serve.log -Tail 5 -ErrorAction SilentlyContinue
