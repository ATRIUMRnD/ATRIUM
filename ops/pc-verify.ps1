$ErrorActionPreference = "Continue"
'===== DUCTEI cargo check (workspace, all features)'
Set-Location D:\ATRIUM\DUCTEI
cargo check --workspace --all-features 2>&1 | Select-Object -Last 15
"exit=$LASTEXITCODE"
'===== DUCTEI transport tests (net feature)'
cargo test -p ductei-core --features net transport 2>&1 | Select-Object -Last 12
"exit=$LASTEXITCODE"
'===== LIMEN pytest (router + ecc term)'
Set-Location D:\ATRIUM\LIMEN
$py = if (Test-Path .\.venv\Scripts\python.exe) { '.\.venv\Scripts\python.exe' } elseif (Test-Path .\venv\Scripts\python.exe) { '.\venv\Scripts\python.exe' } else { 'python' }
"python = $py"
& $py -m pytest tests/test_router.py tests/test_ecc_term.py -q 2>&1 | Select-Object -Last 15
"exit=$LASTEXITCODE"
