# atrium-nightly.ps1: the one nightly. Run by the limen-ductei-nightly-smoke task.
#   0. atrium down   (one writer per chain; nightly's limend runs never race the loop)
#   1. atrium verify (qhr chain, gate chain, full nightly A..G, ward, atrium chain),
#      every outcome a signed receipt on ops\atrium-state\receipts.jsonl
#   2. atrium up -Background, always, pass or fail
# Exit code is verify's: 0 all pass, 1 something failed.
$ErrorActionPreference = 'Continue'
$A = 'D:\ATRIUM\ops\atrium.ps1'
$log = 'D:\ATRIUM\ops\atrium-state\logs\nightly.log'
function Log($m) { ("[{0}] {1}" -f (Get-Date -Format 's'), $m) | Add-Content $log }

Log 'down'
& powershell -NoProfile -ExecutionPolicy Bypass -File $A -Down 2>&1 | ForEach-Object { Log "  $_" }
$code = 1
try {
    Log 'verify'
    & powershell -NoProfile -ExecutionPolicy Bypass -File $A -Verify 2>&1 | ForEach-Object { Log "  $_" }
    $code = $LASTEXITCODE
} finally {
    # Start through the `atrium` task, not a child process: a child would
    # inherit this script's stdout pipe and the caller would wait on it forever.
    Log "verify exit $code; up via task atrium"
    Start-ScheduledTask -TaskName 'atrium'
    Start-Sleep 5
    & D:\dev\bin\atrium.exe status D:\ATRIUM\ops\atrium.toml 2>&1 | ForEach-Object { Log "  $_" }
}
exit $code
