# QHR -> QPU lane -> limend live round trip (nightly step F).
#   0. fresh spool + fresh QHR state under D:\ATRIUM\ops\logs\smoke\work-qhr
#   1. limend (Windows venv, offline, CPU Aer) watches the spool
#   2. qhr submit qpu.run with a 4-var star QUBO: must be done with a cert and exact_energy -3;
#      the same request again under a new id must be a cache hit that never reaches limend
#   3. qhr submit a 65-var QUBO: must be refused in pre-flight (exit 3), never spooled
#   4. qhr verify on the state dir: receipt chain must check out
# Prints "QHR_QPU_PROBE PASS" and exits 0 only if all of that holds. Never commits.
param([string]$Work = 'D:\ATRIUM\ops\logs\smoke\work-qhr', [string]$LimendPoll = '0.2')

$ErrorActionPreference = 'Stop'
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$py = 'D:\LIMEN\.venv\Scripts\python.exe'
$manifest = 'D:\ATRIUM\DUCTEI\Cargo.toml'
$stamp = Get-Date -Format 'yyyyMMdd_HHmmss'

if (Test-Path $Work) { Remove-Item $Work -Recurse -Force }
$spool = Join-Path $Work 'spool'
$state = Join-Path $Work 'state'
foreach ($d in 'pending','certs','failed','done') { New-Item -ItemType Directory -Force (Join-Path $spool $d) | Out-Null }
New-Item -ItemType Directory -Force $state | Out-Null

& cargo build -q --release --manifest-path $manifest -p ductei-qhr --bin qhr
if ($LASTEXITCODE -ne 0) { Write-Output 'QHR_QPU_PROBE FAIL build'; exit 1 }
$qhr = 'D:\ATRIUM\DUCTEI\target\release\qhr.exe'
if ($env:CARGO_TARGET_DIR) { $qhr = Join-Path $env:CARGO_TARGET_DIR 'release\qhr.exe' }

function Job($id, $qubo, $budget) {
    $req = @{ qubo = $qubo; fidelity_target = 0.9; credit_budget = 0.0; offline = $true } | ConvertTo-Json -Depth 8 -Compress
    $job = @{
        task = @{ id = $id; intent = 'qhr qpu probe'; kind = 'code'; private = $true; effects = @() }
        op = 'qpu.run'; payload = $req; budget_ms = $budget; assurance = 'verified'
    } | ConvertTo-Json -Depth 8
    $p = Join-Path $Work "$id.json"
    [IO.File]::WriteAllText($p, $job)
    return $p
}

$star = @(
    ,@(@('hub','leaf0'), 2.0); ,@(@('leaf0','leaf0'), -1.0)
    ,@(@('hub','leaf1'), 2.0); ,@(@('leaf1','leaf1'), -1.0)
    ,@(@('hub','leaf2'), 2.0); ,@(@('leaf2','leaf2'), -1.0)
    ,@(@('hub','hub'), -1.0)
)
$big = @(0..64 | ForEach-Object { ,@(@("x$_","x$_"), -1.0) })

$fail = @()
$limend = Start-Process -FilePath $py -ArgumentList @('-m','limen.limend', $spool, '--poll-interval',$LimendPoll) `
    -WorkingDirectory 'D:\LIMEN' -PassThru -WindowStyle Hidden `
    -RedirectStandardOutput (Join-Path $Work 'limend.out') -RedirectStandardError (Join-Path $Work 'limend.err')
try {
    # 2. live round trip
    $t0 = Get-Date
    $out = & $qhr submit $state --job (Job "probe-ok-$stamp" $star 120000) --spool $spool 2>&1 | Out-String
    $code = $LASTEXITCODE
    $ms = [int]((Get-Date) - $t0).TotalMilliseconds
    Write-Output "submit ok-case exit=$code wall=${ms}ms"; Write-Output $out
    # Gate on the lane contract (done + a cert for this job id from tier 0), not on
    # QAOA quality: success_probability is mass on one bitstring and p=1 QAOA misses
    # a 0.9 target honestly. target_met is logged, never gated.
    $okDone = $out -match '"status":\s*"done"'
    $okCert = $out -match [regex]::Escape("\""job_id\"":\""probe-ok-$stamp\""") -and $out -match 'success_probability'
    if ($code -ne 0 -or -not $okDone -or -not $okCert) { $fail += "live round trip (exit $code, done=$okDone, cert=$okCert)" }
    # Exact check rides along: the star's optimum is hub=0, leaves=1, energy -3.
    if ($out -notmatch 'exact_energy\\?":\s*-3(\.0)?[,}]') { $fail += 'exact_energy missing or not -3' }

    # 2b. same request again, new id: must be a cache hit, never spooled
    $doneBefore = (Get-ChildItem (Join-Path $spool 'done') -File).Count
    $t1 = Get-Date
    $out3 = & $qhr submit $state --job (Job "probe-hit-$stamp" $star 120000) --spool $spool 2>&1 | Out-String
    $code3 = $LASTEXITCODE
    $ms3 = [int]((Get-Date) - $t1).TotalMilliseconds
    $e3 = [regex]::Match($out3, '"elapsed_us":\s*(\d+)').Groups[1].Value
    $doneAfter = (Get-ChildItem (Join-Path $spool 'done') -File).Count
    Write-Output "submit hit-case exit=$code3 wall=${ms3}ms lane_elapsed_us=$e3 spooled_delta=$($doneAfter - $doneBefore)"
    if ($code3 -ne 0 -or $out3 -notmatch 'cached\\?":\s*true' -or $doneAfter -ne $doneBefore) { $fail += "cache hit (exit $code3)" }

    # 3. pre-flight refusal: never reaches the spool
    $before = (Get-ChildItem (Join-Path $spool 'pending'), (Join-Path $spool 'done') -File).Count
    $out2 = & $qhr submit $state --job (Job "probe-big-$stamp" $big 120000) --spool $spool 2>&1 | Out-String
    $code2 = $LASTEXITCODE
    $after = (Get-ChildItem (Join-Path $spool 'pending'), (Join-Path $spool 'done') -File).Count
    Write-Output "submit big-case exit=$code2 spooled_delta=$($after - $before)"; Write-Output $out2
    if ($code2 -ne 3 -or $out2 -notmatch 'exceeds the lane ceiling' -or $after -ne $before) { $fail += "65-var refusal (exit $code2)" }
} finally {
    if (-not $limend.HasExited) { Stop-Process -Id $limend.Id -Force }
}

# 4. receipt chain
$v = & $qhr verify $state --allow-unpinned 2>&1 | Out-String
$vcode = $LASTEXITCODE
Write-Output "verify exit=$vcode"; Write-Output $v
if ($vcode -ne 0) { $fail += "verify (exit $vcode)" }

if ($fail.Count -eq 0) { Write-Output 'QHR_QPU_PROBE PASS'; exit 0 }
Write-Output ("QHR_QPU_PROBE FAIL: " + ($fail -join '; '))
Get-Content (Join-Path $Work 'limend.err') -Tail 20 -ErrorAction SilentlyContinue
exit 1
