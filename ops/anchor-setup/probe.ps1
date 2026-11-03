Get-CimInstance Win32_Process -Filter 'ProcessId=21936' | Select-Object ProcessId,CommandLine,CreationDate | Format-List
Get-ScheduledTask | Where-Object { ($_.Actions.Execute + $_.Actions.Arguments) -match 'ductei|anchor' } | ForEach-Object { $_.TaskName + ' :: ' + $_.Actions.Execute + ' ' + $_.Actions.Arguments }
