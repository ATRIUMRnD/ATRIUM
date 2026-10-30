# Local smoke test for xing-pc-mcp. Never prints the token.
$ErrorActionPreference = 'Stop'
$exe = "$PSScriptRoot\target\release\xing-pc-mcp.exe"
$saved = Get-Clipboard -Raw
& $exe url | Out-Null
$token = ((Get-Clipboard -Raw) -split '/mcp/')[-1].Trim()
if ($saved) { Set-Clipboard -Value $saved } else { Set-Clipboard -Value ' ' }
$raw = Get-Content "$env:USERPROFILE\.xing-pc-mcp\config.json" -Raw
"{0,-34} {1}" -f 'QFLTL: no plain token on disk', $(if ($raw -notmatch '"token"\s*:' -and $raw -match 'token_dpapi' -and -not $raw.Contains($token)) { 'PASS' } else { 'FAIL' })
$url = "http://127.0.0.1:8787/mcp"
$h = @{ Authorization = "Bearer $token" }
$script:n = 0
function Rpc($method, $params) {
    $script:n++
    $body = @{ jsonrpc = '2.0'; id = $script:n; method = $method; params = $params } | ConvertTo-Json -Depth 8 -Compress
    Invoke-RestMethod -Uri $url -Method Post -Headers $h -ContentType 'application/json' -Body $body
}
function Call($tool, $a) { (Rpc 'tools/call' @{ name = $tool; arguments = $a }).result }
function Check($label, $ok) { "{0,-34} {1}" -f $label, $(if ($ok) { 'PASS' } else { 'FAIL' }) }

$init = Rpc 'initialize' @{ protocolVersion = '2025-06-18'; capabilities = @{}; clientInfo = @{ name = 'smoke'; version = '0' } }
Check 'initialize' ($init.result.serverInfo.name -eq 'xing-pc-mcp')
Check 'tools/list has 4 tools' ((Rpc 'tools/list' @{}).result.tools.Count -eq 4)
Check 'status' (-not (Call 'status' @{}).isError)
Check 'list_dir D:\ATRIUM' (-not (Call 'list_dir' @{ path = 'D:\ATRIUM' }).isError)
$r = Call 'read_file' @{ path = 'D:\ATRIUM\xing-pc-mcp\Cargo.toml' }
Check 'read_file own Cargo.toml' ((-not $r.isError) -and $r.content[0].text -match 'xing-pc-mcp')
Check 'refuse outside root (C:\Windows)' ((Call 'read_file' @{ path = 'C:\Windows\win.ini' }).isError)
Check 'refuse traversal (..)' ((Call 'read_file' @{ path = 'D:\ATRIUM\..\Windows\win.ini' }).isError)
Check 'refuse own config dir' ((Call 'read_file' @{ path = "$env:USERPROFILE\.xing-pc-mcp\config.json" }).isError)
Check 'refuse .git\config' ((Call 'read_file' @{ path = 'D:\ATRIUM\.git\config' }).isError)
$g = Call 'run_preset' @{ name = 'git_status'; cwd = 'D:\ATRIUM' }
Check 'run_preset git_status' ((-not $g.isError) -and $g.content[0].text -match 'exit Some\(0\)')
Check 'refuse unknown preset' ((Call 'run_preset' @{ name = 'powershell' }).isError)

& $exe kill | Out-Null
Check 'kill switch blocks calls' ((Call 'status' @{}).isError)
& $exe unkill | Out-Null
Check 'unkill restores calls' (-not (Call 'status' @{}).isError)

try { Invoke-WebRequest -Uri $url -Method Post -ContentType 'application/json' -Body '{}' -UseBasicParsing | Out-Null; Check 'no token -> 401' $false }
catch { Check 'no token -> 401' ($_.Exception.Response.StatusCode.value__ -eq 401) }
try { Invoke-WebRequest -Uri "$url/wrongtoken" -Method Post -ContentType 'application/json' -Body '{}' -UseBasicParsing | Out-Null; Check 'wrong path token -> 401' $false }
catch { Check 'wrong path token -> 401' ($_.Exception.Response.StatusCode.value__ -eq 401) }
try { Invoke-WebRequest -Uri $url -Method Post -Headers ($h + @{ Origin = 'https://evil.example' }) -ContentType 'application/json' -Body '{}' -UseBasicParsing | Out-Null; Check 'foreign Origin -> 403' $false }
catch { Check 'foreign Origin -> 403' ($_.Exception.Response.StatusCode.value__ -eq 403) }
$note = '{"jsonrpc":"2.0","method":"notifications/initialized"}'
Check 'notification -> 202' ((Invoke-WebRequest -Uri $url -Method Post -Headers $h -ContentType 'application/json' -Body $note -UseBasicParsing).StatusCode -eq 202)
$chain = "$env:USERPROFILE\.xing-pc-mcp\audit.chain.jsonl"
& $exe verify-log | Out-Null
Check 'QFLTL: audit chain verifies' ($LASTEXITCODE -eq 0)
$orig = [IO.File]::ReadAllText($chain)
[IO.File]::WriteAllText($chain, $orig.Replace('"tool":"status"', '"tool":"statuz"'))
& $exe verify-log | Out-Null
Check 'QFLTL: tampered chain detected' ($LASTEXITCODE -eq 2)
[IO.File]::WriteAllText($chain, $orig)
& $exe verify-log
