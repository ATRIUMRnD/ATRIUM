# End-to-end test through the Tailscale Funnel URL (TLS, /mcp path mapping, token in path).
# Same URL claude.ai uses. Never prints the token; restores the clipboard.
$ErrorActionPreference = 'Stop'
$exe = "$PSScriptRoot\target\release\xing-pc-mcp.exe"
$saved = Get-Clipboard -Raw
& $exe url | Out-Null
$full = (Get-Clipboard -Raw).Trim()
if ($saved) { Set-Clipboard -Value $saved } else { Set-Clipboard -Value ' ' }
$base = $full.Substring(0, $full.IndexOf('/mcp/'))
$script:n = 0
function Check($label, $ok) { "{0,-40} {1}" -f $label, $(if ($ok) { 'PASS' } else { 'FAIL' }) }
function Rpc($method, $params) {
    $script:n++
    $body = @{ jsonrpc = '2.0'; id = $script:n; method = $method; params = $params } | ConvertTo-Json -Depth 8 -Compress
    Invoke-RestMethod -Uri $full -Method Post -ContentType 'application/json' -Body $body -TimeoutSec 20
}
"target: $base/mcp/... (token hidden)"
$init = Rpc 'initialize' @{ protocolVersion = '2025-06-18'; capabilities = @{}; clientInfo = @{ name = 'funnel-e2e'; version = '0' } }
Check 'TLS + initialize via Funnel' ($init.result.serverInfo.name -eq 'xing-pc-mcp')
$tools = (Rpc 'tools/list' @{}).result.tools.name -join ','
Check "tools/list ($tools)" ($tools -eq 'status,list_dir,read_file,run_preset')
$st = (Rpc 'tools/call' @{ name = 'status'; arguments = @{} }).result
Check 'tools/call status' ((-not $st.isError) -and $st.content[0].text -match 'read-only')
$ls = (Rpc 'tools/call' @{ name = 'list_dir'; arguments = @{ path = 'D:\ATRIUM\xing-pc-mcp' } }).result
Check 'tools/call list_dir' ((-not $ls.isError) -and $ls.content[0].text -match 'Cargo.toml')
$g = (Rpc 'tools/call' @{ name = 'run_preset'; arguments = @{ name = 'nvidia_smi' } }).result
Check 'tools/call run_preset nvidia_smi' ((-not $g.isError) -and $g.content[0].text -match 'NVIDIA')
$bad = (Rpc 'tools/call' @{ name = 'read_file'; arguments = @{ path = 'C:\Windows\win.ini' } }).result
Check 'refuses outside roots via Funnel' ($bad.isError)
try { Invoke-WebRequest -Uri "$base/mcp" -Method Post -ContentType 'application/json' -Body '{}' -UseBasicParsing -TimeoutSec 20 | Out-Null; Check 'no token via Funnel -> 401' $false }
catch { Check 'no token via Funnel -> 401' ($_.Exception.Response.StatusCode.value__ -eq 401) }
try { Invoke-WebRequest -Uri "$base/" -UseBasicParsing -TimeoutSec 20 | Out-Null; Check 'nothing else exposed on :10000' $false }
catch { Check 'nothing else exposed on :10000' ($_.Exception.Response.StatusCode.value__ -eq 404) }
& $exe verify-log
