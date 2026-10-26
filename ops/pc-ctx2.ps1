Set-Location D:\ATRIUM\DUCTEI
'--- signature'
$l = Get-Content ductei-core\src\transport.rs; $i=0
foreach ($x in $l) { $i++; if ($i -ge 200 -and $i -le 235) { "{0,4}: {1}" -f $i,$x } }
'--- call sites'
Get-ChildItem -Recurse -Include *.rs ductei-core\src\bin, ductei-core\tests | Select-String -Pattern 'send_local_first\(' | ForEach-Object { "$($_.Path.Replace('D:\ATRIUM\DUCTEI\','')):$($_.LineNumber): $($_.Line.Trim())" }
'--- how upstream tests call it'
git show origin/main:ductei-core/tests/suite.rs | Select-String -Pattern 'send_local_first\(' | ForEach-Object { $_.Line.Trim() }
'--- bin files'
Get-ChildItem ductei-core\src\bin | Select-Object -ExpandProperty Name
