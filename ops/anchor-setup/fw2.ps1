Get-NetFirewallRule -Direction Inbound -ErrorAction SilentlyContinue | ForEach-Object { $f = $_ | Get-NetFirewallApplicationFilter; if ($f.Program -match 'anchor-peer|ductei-sync') { "$($_.DisplayName) | $($_.Enabled) | $($_.Action) | $($_.Profile) | $($f.Program)" } }
Get-NetConnectionProfile | ForEach-Object { "$($_.InterfaceAlias) = $($_.NetworkCategory)" }
Get-NetTCPConnection -LocalPort 7421 -State Listen | Select-Object LocalAddress,OwningProcess
(Get-NetFirewallProfile | ForEach-Object { "$($_.Name): enabled=$($_.Enabled) inbound=$($_.DefaultInboundAction) allowLocal=$($_.AllowLocalFirewallRules)" })
