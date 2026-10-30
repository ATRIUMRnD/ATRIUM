# atrium-online.ps1: run the ATRIUM loop with real IBM hardware enabled.
# Loads LIMEN_QPU_TOKEN / LIMEN_QPU_INSTANCE from the DPAPI-encrypted files in
# ~\.secrets into THIS process only, checks the key with IBM IAM, then runs
# atrium_up.py with the LIMEN venv. Secrets are never printed or written.
#
# While the loop runs it holds a Windows "system required" request, so xing-pc
# does not idle-sleep mid-night. The request ends when the loop stops, so the
# normal sleep setting comes back on its own (manual sleep still works).
#
#   Foreground (this window):
#     powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium-online.ps1
#   Background (no window, survives closing the terminal, logs to <root>.log):
#     powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium-online.ps1 -Background
#   Status / stop:
#     powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium-online.ps1 -Status
#     powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\atrium-online.ps1 -Stop
param(
    [int]$MaxOnlineJobs = 1,
    [string]$Root = "D:\atrium-online-$(Get-Date -f yyyyMMdd-HHmmss)",
    [switch]$Background,
    [switch]$Stop,
    [switch]$Status
)

$ErrorActionPreference = 'Stop'
$self = $MyInvocation.MyCommand.Path

function Get-LoopProcs {
    Get-CimInstance Win32_Process | Where-Object {
        $_.CommandLine -match 'atrium-online-\d{8}-\d{6}' -and $_.ProcessId -ne $PID
    }
}

if ($Status -or $Stop) {
    $procs = @(Get-LoopProcs)
    if ($procs.Count -eq 0) { Write-Host "[atrium-online] no loop running"; exit 0 }
    $roots = $procs | ForEach-Object { ([regex]'D:\\atrium-online-\d{8}-\d{6}').Match($_.CommandLine).Value } | Sort-Object -Unique
    Write-Host "[atrium-online] running: $($procs.Count) processes, root(s): $($roots -join ', ')"
    $port = @(Get-NetUDPEndpoint -LocalPort 9000 -ErrorAction SilentlyContinue).Count
    Write-Host "[atrium-online] VEYN OSC port 9000 listening: $([bool]$port)"
    if ($Stop) {
        $procs | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
        Write-Host "[atrium-online] stopped (normal sleep setting applies again)"
    }
    exit 0
}

if (@(Get-LoopProcs).Count -gt 0) {
    throw "a loop is already running (port 9000 is taken); run with -Status or -Stop first"
}

if ($Background) {
    $out = "$Root.log"; $err = "$Root.err.log"
    $p = Start-Process powershell -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $out -RedirectStandardError $err `
        -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $self,
                        '-MaxOnlineJobs', $MaxOnlineJobs, '-Root', $Root)
    Write-Host "[atrium-online] started in background (pid $($p.Id)); log: $out"
    Write-Host "[atrium-online] check: powershell -ExecutionPolicy Bypass -File $self -Status"
    exit 0
}

$secrets = "$env:USERPROFILE\.secrets"
$py = "$env:USERPROFILE\venvs\limen\Scripts\python.exe"
function Dec($f) { [System.Net.NetworkCredential]::new('', (Get-Content (Join-Path $secrets $f) | ConvertTo-SecureString)).Password.Trim() }

$env:LIMEN_QPU_TOKEN = Dec 'API_KEY.txt'
$env:LIMEN_QPU_INSTANCE = Dec 'CRM_API_TOKEN.txt'
if (-not $env:LIMEN_QPU_INSTANCE.StartsWith('crn:')) { throw "CRM_API_TOKEN.txt does not hold a CRN (should start with crn:)" }

try {
    Invoke-RestMethod -Method Post -Uri https://iam.cloud.ibm.com/identity/token `
        -ContentType 'application/x-www-form-urlencoded' `
        -Body @{ grant_type = 'urn:ibm:params:oauth:grant-type:apikey'; apikey = $env:LIMEN_QPU_TOKEN } | Out-Null
    Write-Host "[atrium-online] IBM key valid; root=$Root; hardware jobs allowed=$MaxOnlineJobs"
} catch {
    throw "IBM rejected the API key in $secrets\API_KEY.txt; save a new one and rerun"
}

Add-Type -Namespace Atrium -Name Power -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("kernel32.dll")]
public static extern uint SetThreadExecutionState(uint esFlags);
'@
# ES_CONTINUOUS | ES_SYSTEM_REQUIRED: no idle sleep while this thread lives.
[void][Atrium.Power]::SetThreadExecutionState([uint32]'0x80000001')
Write-Host "[atrium-online] idle sleep held off while the loop runs"

try {
    Set-Location D:\ATRIUM\DUCTEI
    & $py scripts\atrium_up.py --root $Root --limen D:\ATRIUM\LIMEN `
        --qallow-cli D:\ATRIUM\Qallow\target\debug\qallow.exe --veyn D:\VEYN `
        --veyn-daemon --online --max-online-jobs $MaxOnlineJobs
} finally {
    [void][Atrium.Power]::SetThreadExecutionState([uint32]'0x80000000')
    Write-Host "[atrium-online] loop ended; idle sleep allowed again"
}
