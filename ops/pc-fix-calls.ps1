$ErrorActionPreference = "Continue"
Set-Location D:\ATRIUM\DUCTEI
$R = 'D:\ATRIUM\DUCTEI\'
function Utf8NoBom($path, $text) { [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding $false)) }
function Patch($rel, $old, $new) {
  $t = [IO.File]::ReadAllText($R + $rel)
  if (-not $t.Contains($old)) { "NOT FOUND in $rel"; return }
  Utf8NoBom ($R + $rel) $t.Replace($old, $new); "patched $rel"
}
Patch 'ductei-core\src\bin\ductei-net-smoke.rs' 'send_local_first(addr, &env, &mut local)' 'send_local_first(addr, &env, &mut local, ductei_core::transport::default_outbox_path())'
Patch 'ductei-core\src\bin\ductei-node.rs' 'send_local_first(addr.as_str(), &env, &mut spool)' 'send_local_first(addr.as_str(), &env, &mut spool, ductei_core::transport::default_outbox_path())'
Patch 'ductei-core\tests\suite.rs' 'send_local_first(addr, &env("a", &["qallow.semantic.cert"], 1, 1), &mut local).unwrap()' 'send_local_first(addr, &env("a", &["qallow.semantic.cert"], 1, 1), &mut local, tmp("t5b-outbox.jsonl")).unwrap()'
'--- recheck'
cargo check --workspace --all-features 2>&1 | Select-Object -Last 4
"check exit=$LASTEXITCODE"
cargo test -p ductei-core --all-features 2>&1 | Select-String -Pattern 'test result|FAILED|panicked|^error' | ForEach-Object { $_.Line }
"test exit=$LASTEXITCODE"
