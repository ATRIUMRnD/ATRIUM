# Sandbox test for v0.3.0: own config under D:, port 8799, never touches the live server.
$ErrorActionPreference = 'Continue'
$exe  = 'D:\ATRIUM\xing-pc-mcp\target-next\release\xing-pc-mcp.exe'
$home2 = 'D:\ATRIUM\xing-pc-mcp\.testhome'
Remove-Item $home2 -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory "$home2\repo" -Force | Out-Null
git -C "$home2\repo" init -q; git -C "$home2\repo" config user.name xingxerx; git -C "$home2\repo" config user.email test@local
$env:USERPROFILE = $home2
& $exe enable-write | Out-Null; $wurl = Get-Clipboard
$cfgp = "$home2\.xing-pc-mcp\config.json"
$c = Get-Content $cfgp -Raw | ConvertFrom-Json
$c.bind = '127.0.0.1:8799'; $c.roots = @("$home2\repo")
$c | ConvertTo-Json -Depth 5 | Set-Content $cfgp -Encoding UTF8
& $exe url | Out-Null; $rurl = Get-Clipboard
$base = 'http://127.0.0.1:8799'
$rtok = $rurl.Split('/')[-1]; $wtok = $wurl.Split('/')[-1]
$p = Start-Process $exe -PassThru -WindowStyle Hidden; Start-Sleep 2
function call($tok, $name, $a) {
  $b = @{jsonrpc='2.0'; id=1; method='tools/call'; params=@{name=$name; arguments=$a}} | ConvertTo-Json -Depth 6 -Compress
  try { $r = Invoke-RestMethod "$base/mcp/$tok" -Method Post -Body $b -ContentType 'application/json'; ($r.result.content[0].text) + $(if($r.result.isError){'  [isError]'}) } catch { "HTTP $($_.Exception.Response.StatusCode.value__)" }
}
"1 read token, write_file  -> " + (call $rtok 'write_file' @{path="$home2\repo\a.txt"; content='hi'})
"2 write token, write_file -> " + (call $wtok 'write_file' @{path="$home2\repo\a.txt"; content='hi'})
"3 write off D:            -> " + (call $wtok 'write_file' @{path='E:\x.txt'; content='no'})
"4 write into .git         -> " + (call $wtok 'write_file' @{path="$home2\repo\.git\hooks\x"; content='no'})
"5 escape with ..          -> " + (call $wtok 'write_file' @{path="$home2\repo\..\evil.txt"; content='no'})
"6 commit w/ attribution   -> " + (call $wtok 'prepare_commit' @{repo="$home2\repo"; message="x`nCo-Authored-By: Claude"; paths=@('a.txt')})
"7 commit w/ @mention      -> " + (call $wtok 'prepare_commit' @{repo="$home2\repo"; message='hi @someone'; paths=@('a.txt')})
"8 good prepare_commit     -> " + (call $wtok 'prepare_commit' @{repo="$home2\repo"; message='test commit'; paths=@('a.txt')})
"9 bad token               -> " + (call 'deadbeef' 'status' @{})
"10 empty preset/bad tool  -> " + (call $rtok 'nope' @{})
"11 approve 'no' keeps it  -> " + ('no' | & $exe approve | Select-String 'left pending')
"12 approve 'yes' commits  -> " + (('yes' | & $exe approve | Out-String) -replace "`r?`n",' ')
"13 git log                -> " + (git -C "$home2\repo" log --oneline -1)
"14 replay approve         -> " + ('yes' | & $exe approve)
& $exe kill | Out-Null
"15 kill switch blocks     -> " + (call $wtok 'status' @{})
& $exe unkill | Out-Null
& $exe disable-write | Out-Null; Stop-Process $p.Id -Force; $p = Start-Process $exe -PassThru -WindowStyle Hidden; Start-Sleep 2
"16 disabled write token   -> " + (call $wtok 'status' @{})
"17 read still works       -> " + (call $rtok 'status' @{})
"18 audit chain            -> " + (& $exe verify-log)
Stop-Process $p.Id -Force
