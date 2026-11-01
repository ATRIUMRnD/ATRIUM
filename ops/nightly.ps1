# ATRIUM nightly: one task replacing limen-ductei-nightly-smoke and limen-nightly-smoke.
#   A  DUCTEI e2e: four limend -> ductei-limen-relay scenarios (Windows venv, CPU Aer)
#   B  LIMEN tier 0: 24-var QUBO on aer_gpu via WSL (proves the GPU path)
#   C  ductei-router: cargo test + a gated decide (C-drive write must be denied)
#   D  LIMEN Rust IR: cargo test in D:\LIMEN\rust + pytest tests/test_ir_bridge.py
#   E  atrium-spec: generated Rust/C/Python match spec/atrium.toml, and generated Rust matches ductei-limen
# Logs: D:\ATRIUM\ops\logs\smoke\<stamp>.log and latest.txt. Exit 0 iff all pass. Never commits.

$ErrorActionPreference = 'Continue'
$stamp  = Get-Date -Format 'yyyy-MM-dd_HHmm'
$logDir = 'D:\ATRIUM\ops\logs\smoke'
$log    = Join-Path $logDir "$stamp.log"
$latest = Join-Path $logDir 'latest.txt'
New-Item -ItemType Directory -Force $logDir | Out-Null
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$py = 'D:\LIMEN\.venv\Scripts\python.exe'

function Log($msg) { ("[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $msg) | Tee-Object -FilePath $log -Append }
function Run($name, [scriptblock]$body) {
    Log "${name}: start"
    $out = & $body 2>&1; $code = $LASTEXITCODE
    $out | ForEach-Object { Log "   $_" }
    return @{ out = ($out -join "`n"); code = $code }
}

$r = [ordered]@{}

# A: DUCTEI e2e
$a = Run 'A ductei_e2e' { Push-Location D:\ATRIUM\DUCTEI; $env:LIMEN_LOCAL_GPU = $null; & $py scripts\smoke_e2e.py --limen D:\LIMEN --workdir (Join-Path $logDir 'work-ductei'); Pop-Location }
$r['ductei_e2e'] = if ($a.code -eq 0 -and $a.out -match 'all four scenarios passed') { 'PASS' } else { "FAIL ($($a.code))" }

# B: LIMEN tier 0 on GPU (WSL)
$b = Run 'B limen_tier0_gpu' { & wsl -d Ubuntu-24.04 -- bash -c 'cd /mnt/d/LIMEN && LIMEN_LOCAL_GPU=1 ~/limen-venv/bin/python /mnt/d/ATRIUM/ops/tier0_probe.py 24 2>&1' }
$r['limen_tier0_gpu'] = if ($b.code -eq 0 -and $b.out -match 'report backend: aer_gpu tier 0') { 'PASS' } else { "FAIL ($($b.code))" }

# C: router tests + gate check
$c1 = Run 'C router_tests' { & cargo test -q --manifest-path D:\ATRIUM\DUCTEI\Cargo.toml -p ductei-router }
$deny = Join-Path $logDir 'deny.json'
'{"id":"nightly-deny","intent":"nightly gate probe","kind":"code","effects":[{"type":"write","target":"C:\\probe.txt"}]}' | Set-Content $deny
$c2 = Run 'C router_gate' { & cargo run -q --manifest-path D:\ATRIUM\DUCTEI\Cargo.toml -p ductei-router --bin ductei-route -- decide --task $deny --state (Join-Path $logDir 'router-state') }
$r['router'] = if ($c1.code -eq 0 -and $c2.code -eq 3) { 'PASS' } else { "FAIL (tests $($c1.code), gate exit $($c2.code) expected 3)" }

# D: Rust IR + Python bridge
$d1 = Run 'D ir_cargo' { & cargo test -q --manifest-path D:\LIMEN\rust\Cargo.toml }
$d2 = Run 'D ir_bridge' { Push-Location D:\LIMEN; & $py -m pytest -q tests\test_ir_bridge.py; Pop-Location }
$r['limen_ir'] = if ($d1.code -eq 0 -and $d2.code -eq 0) { 'PASS' } else { "FAIL (cargo $($d1.code), pytest $($d2.code))" }

# E: contract compiler drift check (spec -> generated files, generated Rust -> ductei-limen)
$e1 = Run 'E spec_check' { & cargo run -q --manifest-path D:\ATRIUM\DUCTEI\Cargo.toml -p atrium-spec -- check D:\ATRIUM\DUCTEI\atrium-spec\spec\atrium.toml D:\ATRIUM\DUCTEI\atrium-spec\generated }
$e2 = Run 'E spec_tests' { & cargo test -q --manifest-path D:\ATRIUM\DUCTEI\Cargo.toml -p atrium-spec }
$r['atrium_spec'] = if ($e1.code -eq 0 -and $e2.code -eq 0) { 'PASS' } else { "FAIL (check $($e1.code), tests $($e2.code))" }

$allPass = -not ($r.Values | Where-Object { $_ -notmatch '^PASS' })
$summary = (Get-Date -Format 's') + ' | ' + (($r.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ' | ') + " | log=$log"
Set-Content $latest $summary
Log "RESULT: $(if ($allPass) {'ALL PASS'} else {'FAILURES'})"
Get-ChildItem $logDir -Filter '*.log' | Where-Object LastWriteTime -lt (Get-Date).AddDays(-30) | Remove-Item -ErrorAction SilentlyContinue
exit $(if ($allPass) { 0 } else { 1 })

