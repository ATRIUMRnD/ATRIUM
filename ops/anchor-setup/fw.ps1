Get-NetFirewallRule -Enabled True -Direction Inbound -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -match 'ductei|7421|anchor' } | ForEach-Object { $f = $_ | Get-NetFirewallApplicationFilter; $pf = $_ | Get-NetFirewallPortFilter; "$($_.DisplayName) | $($_.Action) | $($f.Program) | $($pf.LocalPort)" }
try {
  New-NetFirewallRule -DisplayName 'atrium-anchor-peer 7421 (tailnet, xing-pc only)' -Direction Inbound -Action Allow -Protocol TCP -LocalPort 7421 -LocalAddress 100.121.3.55 -RemoteAddress 100.106.147.75 -Program 'C:\Users\there\anchor-peer\bin\ductei-sync.exe' -Profile Any -ErrorAction Stop | Out-Null
  'rule added'
} catch { "rule add failed: $($_.Exception.Message)" }
