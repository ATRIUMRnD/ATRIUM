$ErrorActionPreference = "Continue"
foreach ($pair in @(@('D:\ATRIUM','ATRIUM'),@('D:\ATRIUM\LIMEN','LIMEN'),@('D:\ATRIUM\DUCTEI','DUCTEI'))) {
  $p=$pair[0]; "######## $($pair[1])"
  foreach ($f in (git -C $p diff --name-only --diff-filter=U)) {
    $full = Join-Path $p $f
    $txt = Get-Content $full -Raw
    $n = ([regex]::Matches($txt,'(?m)^<<<<<<< ')).Count
    "---- $f  hunks=$n  lines=$((Get-Content $full).Count)"
    $lines = Get-Content $full
    $in=$false; $i=0
    foreach ($l in $lines) { $i++
      if ($l -like '<<<<<<<*') { $in=$true }
      if ($in) { "{0,5}: {1}" -f $i, $l }
      if ($l -like '>>>>>>>*') { $in=$false }
    }
  }
}
