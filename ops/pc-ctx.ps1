Select-String -Path D:\ATRIUM\AGENTS.md -Pattern '^## ' | ForEach-Object { $_.Line }
'----transport----'
$l = Get-Content D:\ATRIUM\DUCTEI\ductei-core\src\transport.rs
$i = 0
foreach ($x in $l) { $i++; if (($i -le 30) -or ($i -ge 60 -and $i -le 115)) { "{0,4}: {1}" -f $i, $x } }
'----roadmap-around-task1----'
Select-String -Path D:\ATRIUM\ROADMAP.md -Pattern '^## ' | ForEach-Object { "$($_.LineNumber): $($_.Line)" }
