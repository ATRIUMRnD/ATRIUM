# Fix: let tailnet (100.64.0.0/10) reach Sunshine; still block the public internet. Run elevated.
$r = 'ATRIUM Sunshine LAN+tailnet only'
Set-NetFirewallRule -DisplayName $r -RemoteAddress '1.0.0.0-9.255.255.255','11.0.0.0-100.63.255.255','100.128.0.0-172.15.255.255','172.32.0.0-192.167.255.255','192.169.0.0-223.255.255.255'
Get-NetFirewallRule -DisplayName $r | Get-NetFirewallAddressFilter | Select-Object RemoteAddress | Format-List
