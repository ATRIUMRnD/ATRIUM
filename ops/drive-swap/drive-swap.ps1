# drive-swap.ps1: move xing-pc's working drive off the USB hard disk onto the new NVMe,
# keeping every D:\ path valid. The new NVMe becomes D:, the USB HDD becomes E:, and
# every cold top-level folder left on the HDD gets a junction at D:\<name> -> E:\<name>.
#
# Phases (run in order; each is safe to rerun; nothing destructive without its own flag):
#   -Plan                      read-only (default): find the new disk, size the folders, print the plan
#   -Prepare -DiskNumber N     ADMIN. Initialize + format ONLY the new, empty NVMe disk N as N:
#                              (refuses any disk holding C: or D:, any non-NVMe, any non-empty disk,
#                              and asks you to type the disk serial)
#   -Copy                      bulk copy of the hot set D:\ -> N:\ while everything keeps running
#   -Cutover                   ADMIN. Stop ATRIUM and D:-tasks, final delta, verify (tree diff 0 and
#                              key files hash-identical), swap letters, junction the cold set, move
#                              Ollama models, restart, atrium check. Deletes nothing anywhere.
#   -Rollback                  ADMIN. Put the letters back (HDD = D:) and restart the old way
#
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\drive-swap\drive-swap.ps1 -Plan
param(
    [switch]$Plan, [switch]$Prepare, [switch]$Copy, [switch]$Cutover, [switch]$Rollback,
    [int]$DiskNumber = -1,
    [switch]$NoSizes,
    # Folders that move to the NVMe (code, builds, chains, keys, VMs, models). Everything
    # else stays on the HDD behind a junction, so its D:\ path keeps working.
    [string[]]$Hot = @('ATRIUM','dev','VEYN','Qallow','LIMEN','QFLTL','oneiro','self-define','brain',
                       'jspace-lab','CGX_MCPs','tools','keys','me','xing','atrium-live','atrium-online',
                       'llama.cpp','LM_Studio','CUDA_Agents','WSL','WSL_Distros','Grand_Cross','SYNOID'),
    [int]$BudgetPct = 75     # keep at least 25% of the NVMe free (NVMe paging and SSD wear)
)
$ErrorActionPreference = 'Stop'
$stamp     = Get-Date -Format 'yyyyMMdd'
$rel       = 'ATRIUM\ops\drive-swap'
$log       = "D:\$rel\swap-$stamp.log"
$state     = "D:\$rel\swap-state.json"
$tmpL      = 'N'
$skip      = @('$RECYCLE.BIN','System Volume Information')
$ownFiles  = @('swap-*.log','swap-state.json')      # written during the run; never part of the diff
$ollamaOld = Join-Path $env:USERPROFILE '.ollama\models'
$ollamaNew = 'D:\models\ollama'

function Log($m) { $l = "$(Get-Date -Format s) $m"; Write-Host $l; Add-Content -Path $script:log -Value $l -Encoding utf8 }
function Die($m) { Log "STOPPED: $m"; exit 1 }
function NeedAdmin {
    $p = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    if (-not $p.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { Die 'this phase needs an ADMIN PowerShell' }
}
function DiskOf($letter) { $p = Get-Partition -DriveLetter $letter -ErrorAction SilentlyContinue; if ($p) { Get-Disk -Number $p.DiskNumber } }
function Candidates {
    $sys = (DiskOf 'C').Number
    Get-Disk | Where-Object { $_.Number -ne $sys -and $_.BusType -eq 'NVMe' }
}
function SizeGB($path) {
    # robocopy /L counts bytes without reading file contents; far faster than Get-ChildItem on an HDD.
    $o = robocopy $path "$env:TEMP\__nul" /L /S /NJH /BYTES /NFL /NDL /NC /XJ /R:0 /W:0 2>$null
    $b = ($o | Select-String '^\s*Bytes\s*:\s*(\d+)').Matches | Select-Object -First 1
    if ($b) { [math]::Round([double]$b.Groups[1].Value / 1GB, 1) } else { 0 }
}
function Rc($src, $dst) {
    $a = @($src, $dst, '/MIR', '/COPY:DATS', '/DCOPY:DAT', '/XJ', '/R:1', '/W:1', '/MT:16', '/NFL', '/NDL', '/NP', '/XF') + $ownFiles + @("/LOG+:$script:log")
    & robocopy @a | Out-Null
    if ($LASTEXITCODE -ge 8) { Die "robocopy $src -> $dst failed (code $LASTEXITCODE)" }
}
function Diff($src, $dst) {
    $a = @($src, $dst, '/L', '/MIR', '/XJ', '/NJH', '/NJS', '/NDL', '/NP', '/FP', '/R:0', '/W:0', '/XF') + $ownFiles
    @(& robocopy @a 2>$null | Where-Object { $_.Trim() }).Count
}
function HotPresent { $Hot | Where-Object { Test-Path "D:\$_" } }
function Save($o) { $o | ConvertTo-Json -Depth 5 | Set-Content $script:state -Encoding utf8 }
function Load { if (Test-Path $script:state) { Get-Content $script:state -Raw | ConvertFrom-Json } else { [pscustomobject]@{} } }

if (-not ($Prepare -or $Copy -or $Cutover -or $Rollback)) { $Plan = $true }

# ------------------------------------------------------------------ plan (read-only)
if ($Plan) {
    $d = DiskOf 'D'
    Log "D: is disk $($d.Number) $($d.FriendlyName) ($($d.BusType))"
    if ($d.BusType -eq 'NVMe') { Log 'D: is already NVMe; nothing to swap.'; exit 0 }
    $c = @(Candidates)
    if ($c.Count -eq 0) { Log 'new NVMe not detected yet (install it, then rerun -Plan)' }
    foreach ($x in $c) {
        $h = Get-PhysicalDisk | Where-Object DeviceId -eq "$($x.Number)" | Get-StorageReliabilityCounter -ErrorAction SilentlyContinue
        Log ("candidate disk {0}: {1} {2} GB style={3} partitions={4} wear={5}% temp={6}C" -f $x.Number, $x.FriendlyName,
            [math]::Round($x.Size/1GB), $x.PartitionStyle, $x.NumberOfPartitions, $h.Wear, $h.Temperature)
        if ($x.PartitionStyle -ne 'RAW') { Log "  disk $($x.Number) is not empty (RAW); -Prepare will refuse it" }
    }
    if (-not $NoSizes) {
        $total = 0
        foreach ($n in HotPresent) { $g = SizeGB "D:\$n"; $total += $g; Log ("  hot  {0,-16} {1,8} GB" -f $n, $g) }
        $ol = if (Test-Path $ollamaOld) { SizeGB $ollamaOld } else { 0 }
        Log ("  ollama models (C:) {0} GB" -f $ol)
        $total += $ol
        $cap = if ($c.Count) { [math]::Round($c[0].Size/1GB * $BudgetPct / 100) } else { 1397 }
        Log ("hot set {0} GB vs budget {1} GB ({2}% of the NVMe); InferSSD wants ~470 GB more" -f $total, $cap, $BudgetPct)
        if ($total -gt $cap) { Log 'OVER BUDGET: drop folders from -Hot before -Copy' }
    }
    $cold = Get-ChildItem D:\ -Directory -Force | Where-Object { $_.Name -notin $Hot -and $_.Name -notin $skip -and -not ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) } | Select-Object -ExpandProperty Name
    Log "cold set (stays on the HDD, junctioned): $($cold -join ', ')"
    exit 0
}

# ------------------------------------------------------------------ prepare (destructive on the NEW, EMPTY disk only)
if ($Prepare) {
    NeedAdmin
    if ($DiskNumber -lt 0) { Die 'pass -DiskNumber from -Plan' }
    $x = Get-Disk -Number $DiskNumber
    if ($DiskNumber -in @((DiskOf 'C').Number, (DiskOf 'D').Number)) { Die "disk $DiskNumber holds C: or D:; refusing" }
    if ($x.BusType -ne 'NVMe') { Die "disk $DiskNumber is $($x.BusType), not NVMe" }
    if ($x.PartitionStyle -ne 'RAW') { Die "disk $DiskNumber is not empty (style $($x.PartitionStyle)); refusing to wipe it" }
    $ok = Read-Host "Type the disk serial to format disk $DiskNumber ($($x.FriendlyName), serial $($x.SerialNumber))"
    if ($ok.Trim() -ne "$($x.SerialNumber)".Trim()) { Die 'serial did not match; nothing changed' }
    Initialize-Disk -Number $DiskNumber -PartitionStyle GPT
    New-Partition -DiskNumber $DiskNumber -UseMaximumSize -DriveLetter $tmpL | Out-Null
    Format-Volume -DriveLetter $tmpL -FileSystem NTFS -NewFileSystemLabel 'ATRIUM-NVMe' -AllocationUnitSize 4096 -Confirm:$false | Out-Null
    Save ([pscustomobject]@{ disk = $DiskNumber; serial = "$($x.SerialNumber)".Trim(); prepared = (Get-Date -Format s) })
    Log "disk $DiskNumber prepared as ${tmpL}: (GPT, NTFS, label ATRIUM-NVMe)"
    exit 0
}

# ------------------------------------------------------------------ copy (online, repeatable)
if ($Copy) {
    if (-not (Test-Path "${tmpL}:\")) { Die "${tmpL}: not found; run -Prepare first" }
    foreach ($n in HotPresent) { Log "copy D:\$n"; Rc "D:\$n" "${tmpL}:\$n" }
    if (Test-Path $ollamaOld) { Log 'copy Ollama models'; Rc $ollamaOld "${tmpL}:\models\ollama" }
    Log 'bulk copy done; files that changed meanwhile are caught by -Cutover'
    exit 0
}

# ------------------------------------------------------------------ cutover
if ($Cutover) {
    NeedAdmin
    if (-not (Test-Path "${tmpL}:\")) { Die "${tmpL}: not found; run -Prepare and -Copy first" }
    if ((DiskOf 'D').BusType -eq 'NVMe') { Die 'D: is already the NVMe' }
    $st = Load

    Log '1. stop ATRIUM (its writer lock is released when it exits)'
    & D:\dev\bin\atrium.exe down D:\ATRIUM\ops\atrium.toml | Out-Null
    Stop-ScheduledTask -TaskName atrium -ErrorAction SilentlyContinue

    Log '2. disable every enabled scheduled task that runs from D:\ (remembered for re-enable)'
    $tasks = @(Get-ScheduledTask | Where-Object { $_.State -ne 'Disabled' -and ($_.Actions | Where-Object { "$($_.Execute) $($_.Arguments) $($_.WorkingDirectory)" -match '(?i)\bD:[\\/]' }) })
    $st | Add-Member -Force tasks @($tasks | ForEach-Object { @{ path = $_.TaskPath; name = $_.TaskName } })
    Save $st
    foreach ($t in $tasks) { Stop-ScheduledTask -TaskPath $t.TaskPath -TaskName $t.TaskName -ErrorAction SilentlyContinue; Disable-ScheduledTask -TaskPath $t.TaskPath -TaskName $t.TaskName | Out-Null }
    Log "   disabled $($tasks.Count) tasks"

    Log '3. stop processes running from D:\, WSL and Ollama'
    wsl --shutdown 2>$null
    Get-Process ollama*, 'ollama app' -ErrorAction SilentlyContinue | Stop-Process -Force
    Get-Process | Where-Object { $_.Path -match '^(?i)D:\\' } | ForEach-Object { Log "   stop $($_.Name) ($($_.Id))"; Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue }
    Start-Sleep 3

    Log '4. final delta copy and verify (no differences allowed)'
    foreach ($n in HotPresent) { Rc "D:\$n" "${tmpL}:\$n" }
    if (Test-Path $ollamaOld) { Rc $ollamaOld "${tmpL}:\models\ollama" }
    foreach ($n in HotPresent) { $k = Diff "D:\$n" "${tmpL}:\$n"; if ($k -ne 0) { Die "D:\$n differs from the copy ($k entries); nothing swapped" } }
    $crit = @('ATRIUM\ops\atrium-state\receipts.jsonl', 'ATRIUM\ops\atrium-state\gate-ml-dsa-65.sk.dpapi', 'ATRIUM\ops\atrium-state\gate-ml-dsa-65.pk')
    foreach ($f in $crit) {
        if ((Get-FileHash "D:\$f").Hash -ne (Get-FileHash "${tmpL}:\$f").Hash) { Die "hash mismatch on $f; nothing swapped" }
    }
    Log '   copies verified (tree diff 0, chain and signing key hash-identical)'

    Log '5. swap letters: HDD D: -> E:, NVMe N: -> D:'
    $hdd = Get-Partition -DriveLetter D
    $nv  = Get-Partition -DriveLetter $tmpL
    $st | Add-Member -Force hdd @{ disk = $hdd.DiskNumber; part = $hdd.PartitionNumber }
    $st | Add-Member -Force nvme @{ disk = $nv.DiskNumber; part = $nv.PartitionNumber }
    Save $st
    $hdd | Set-Partition -NewDriveLetter E
    $nv  | Set-Partition -NewDriveLetter D
    # The log and state lived on the HDD until now; carry them over, then keep writing on the NVMe.
    Copy-Item "E:\$rel\swap-$stamp.log" "D:\$rel\" -Force
    Save $st

    Log '6. junction the cold set D:\<name> -> E:\<name>'
    Get-ChildItem E:\ -Directory -Force | Where-Object { $_.Name -notin $Hot -and $_.Name -notin $skip } | ForEach-Object {
        if (-not (Test-Path "D:\$($_.Name)")) { New-Item -ItemType Junction -Path "D:\$($_.Name)" -Target $_.FullName | Out-Null; Log "   D:\$($_.Name) -> $($_.FullName)" }
    }

    Log '7. SMB share D re-pointed (encrypted, same account) and D: snapshots'
    if (Get-SmbShare -Name D -ErrorAction SilentlyContinue) {
        $acc = Get-SmbShareAccess -Name D | Where-Object AccessControlType -eq 'Allow' | Select-Object -First 1 -ExpandProperty AccountName
        Remove-SmbShare -Name D -Force
        New-SmbShare -Name D -Path 'D:\' -FullAccess $acc -EncryptData $true | Out-Null
    }
    vssadmin resize shadowstorage /for=D: /on=D: /maxsize=10% 2>$null | Out-Null

    Log '8. Ollama models on the NVMe'
    if (Test-Path $ollamaNew) { [Environment]::SetEnvironmentVariable('OLLAMA_MODELS', $ollamaNew, 'User'); Log "   OLLAMA_MODELS=$ollamaNew (originals left in $ollamaOld until you approve deleting them)" }

    Log '9. re-enable tasks, start ATRIUM, check the chain'
    foreach ($t in $st.tasks) { Enable-ScheduledTask -TaskPath $t.path -TaskName $t.name | Out-Null }
    Start-ScheduledTask -TaskName atrium
    Start-Sleep 15
    & D:\dev\bin\atrium.exe check D:\ATRIUM\ops\atrium.toml 2>&1 | ForEach-Object { Log "   $_" }
    & D:\dev\bin\atrium.exe status D:\ATRIUM\ops\atrium.toml 2>&1 | ForEach-Object { Log "   $_" }
    Log 'cutover done. The HDD is E: and still holds the old copies; nothing was deleted.'
    exit 0
}

# ------------------------------------------------------------------ rollback
if ($Rollback) {
    NeedAdmin
    $st = Load
    if (-not $st.hdd) { Die 'no swap recorded; nothing to roll back' }
    & D:\dev\bin\atrium.exe down D:\ATRIUM\ops\atrium.toml 2>$null | Out-Null
    foreach ($t in $st.tasks) { Disable-ScheduledTask -TaskPath $t.path -TaskName $t.name -ErrorAction SilentlyContinue | Out-Null }
    Get-Process | Where-Object { $_.Path -match '^(?i)D:\\' } | Stop-Process -Force -ErrorAction SilentlyContinue
    Get-Partition -DiskNumber $st.nvme.disk -PartitionNumber $st.nvme.part | Set-Partition -NewDriveLetter $tmpL
    Get-Partition -DiskNumber $st.hdd.disk -PartitionNumber $st.hdd.part | Set-Partition -NewDriveLetter D
    [Environment]::SetEnvironmentVariable('OLLAMA_MODELS', $null, 'User')
    foreach ($t in $st.tasks) { Enable-ScheduledTask -TaskPath $t.path -TaskName $t.name | Out-Null }
    Start-ScheduledTask -TaskName atrium
    Log 'rolled back: HDD is D: again, NVMe is N:. Anything written on the NVMe after cutover is NOT on the HDD.'
    exit 0
}
