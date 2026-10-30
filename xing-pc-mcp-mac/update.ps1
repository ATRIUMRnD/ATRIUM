# xing-pc-mcp safe update: build to target-next, swap, restart, health-check, roll back on failure.
#   D:\ATRIUM\xing-pc-mcp\update.ps1
$ErrorActionPreference = 'Stop'
$root = 'D:\ATRIUM\xing-pc-mcp'
$live = "$root\target\release\xing-pc-mcp.exe"
$new  = "$root\target-next\release\xing-pc-mcp.exe"
$bak  = "$root\target\release\xing-pc-mcp.prev.exe"
Set-Location $root
cargo build --release --target-dir "$root\target-next"
if ($LASTEXITCODE -ne 0) { throw "build failed, live server untouched" }
function Healthy { try { Invoke-WebRequest http://127.0.0.1:8787/mcp -Method Post -ContentType 'application/json' -Body '{"jsonrpc":"2.0","id":1,"method":"ping"}' -UseBasicParsing -TimeoutSec 5 | Out-Null; $true } catch { $_.Exception.Response -ne $null } }
Stop-ScheduledTask xing-pc-mcp -EA 0
Get-Process xing-pc-mcp -EA 0 | Stop-Process -Force
Start-Sleep 1
Copy-Item $live $bak -Force
Copy-Item $new $live -Force
Start-ScheduledTask xing-pc-mcp
Start-Sleep 4
if (Healthy) { Write-Host "update OK, server healthy"; return }
Write-Warning "new build unhealthy, rolling back"
Get-Process xing-pc-mcp -EA 0 | Stop-Process -Force
Copy-Item $bak $live -Force
Start-ScheduledTask xing-pc-mcp
Start-Sleep 4
if (Healthy) { Write-Host "rolled back, old server healthy" } else { throw "rollback also unhealthy, check last-start.err" }
