# ATRIUM apply-all: non-interactive version of harden.ps1 (approved by Xing, Nov 2 2026).
# Keeps xing-pc-mcp + Funnel. Must run elevated. Writes:
#   D:\ATRIUM\ops\security\apply-all.log          (what changed, no secrets)
#   D:\ATRIUM\private\SECRETS-WRITE-DOWN-THEN-DELETE.txt  (recovery secrets)
$ErrorActionPreference = 'Continue'
$log = 'D:\ATRIUM\ops\security\apply-all.log'
$sec = 'D:\ATRIUM\private\SECRETS-WRITE-DOWN-THEN-DELETE.txt'
function Log($m) { "$(Get-Date -Format s) $m" | Add-Content -Path $log -Encoding utf8 }
function Try-Step($name, [scriptblock]$b) { try { & $b; Log "OK   $name" } catch { Log "FAIL $name : $($_.Exception.Message)" } }

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole('Administrators')) { Log 'FAIL not elevated'; exit 1 }
Log '===== apply-all start ====='

# Admin-only audit first
Try-Step 'audit' {
    $p = Get-MpPreference
    Log "  defender exclusions path=[$($p.ExclusionPath -join ';')] proc=[$($p.ExclusionProcess -join ';')] ext=[$($p.ExclusionExtension -join ';')]"
    Log "  secureboot=$(Confirm-SecureBootUEFI)"
    $t = Get-Tpm; Log "  tpm present=$($t.TpmPresent) ready=$($t.TpmReady) mfr=$($t.ManufacturerIdTxt)"
    Get-BitLockerVolume | ForEach-Object { Log "  bitlocker $($_.MountPoint) $($_.ProtectionStatus) $($_.EncryptionPercentage)%" }
    Log "  smb1=$((Get-WindowsOptionalFeature -Online -FeatureName SMB1Protocol).State)"
}

# SSH: keep it, but only over Tailscale and only from Xing's own devices, key-only, user xing only
Try-Step 'ssh tailnet-only' {
    $mine = @('100.121.3.55','100.111.252.86','100.126.56.67')   # xing-laptop, xings-mac-mini, iphone-15
    Get-NetFirewallRule -ErrorAction SilentlyContinue | Where-Object { $_.Direction -eq 'Inbound' -and $_.Enabled -eq 'True' -and
        (($_ | Get-NetFirewallPortFilter).LocalPort -contains '22') -and $_.DisplayName -notlike 'ATRIUM*' } |
        ForEach-Object { Disable-NetFirewallRule -Name $_.Name; Log "  disabled old ssh rule: $($_.DisplayName)" }
    Get-NetFirewallRule -DisplayName 'ATRIUM ssh tailnet-only' -ErrorAction SilentlyContinue | Remove-NetFirewallRule
    New-NetFirewallRule -DisplayName 'ATRIUM ssh tailnet-only' -Direction Inbound -Action Allow -Protocol TCP -LocalPort 22 `
        -RemoteAddress $mine -InterfaceAlias 'Tailscale' -Profile Any | Out-Null
    Log "  ssh allowed only from $($mine -join ', ') on the Tailscale interface"

    $cfg = 'C:\ProgramData\ssh\sshd_config'
    if (Test-Path $cfg) {
        Copy-Item $cfg "$cfg.atrium.bak" -Force
        $c = Get-Content $cfg | Where-Object { $_ -notmatch '^\s*(PasswordAuthentication|PermitRootLogin|AllowUsers|PermitEmptyPasswords|MaxAuthTries|LoginGraceTime)\b' }
        $keys = @('C:\ProgramData\ssh\administrators_authorized_keys', 'C:\Users\xing\.ssh\authorized_keys') |
                Where-Object { (Test-Path $_) -and (Get-Item $_).Length -gt 0 }
        $hard = @('PermitRootLogin no','AllowUsers xing','PermitEmptyPasswords no','MaxAuthTries 3','LoginGraceTime 30')
        if ($keys) { $hard += 'PasswordAuthentication no'; Log '  key found: passwords disabled (key-only)' }
        else { Log '  NO authorized key found: passwords left ON to avoid lockout. Add your key, then set PasswordAuthentication no' }
        # global settings must sit above any Match block
        $i = ($c | Select-String -Pattern '^\s*Match\b' | Select-Object -First 1).LineNumber
        if ($i) { $c = $c[0..($i-2)] + $hard + $c[($i-1)..($c.Count-1)] } else { $c += $hard }
        Set-Content $cfg $c -Encoding ascii
        $test = (& 'C:\Windows\System32\OpenSSH\sshd.exe' -t 2>&1) -join ' '
        if ($LASTEXITCODE -ne 0) { Copy-Item "$cfg.atrium.bak" $cfg -Force; Log "  sshd_config invalid, restored backup: $test" }
        else { Restart-Service sshd; Log '  sshd_config hardened, sshd restarted' }
    } else { Log '  sshd_config not found' } }
Try-Step 'firewall default inbound block' { Set-NetFirewallProfile -Profile Domain,Private,Public -Enabled True -DefaultInboundAction Block -DefaultOutboundAction Allow }

function Rule($n, [hashtable]$a) {
    Get-NetFirewallRule -DisplayName $n -ErrorAction SilentlyContinue | Remove-NetFirewallRule
    New-NetFirewallRule -DisplayName $n -Direction Inbound -Action Block @a | Out-Null
}
Try-Step 'block SMB/NetBIOS from Internet' { Rule 'ATRIUM block SMB non-LAN' @{Protocol='TCP'; LocalPort=@(139,445); RemoteAddress='Internet'} }
Try-Step 'block Sunshine from Internet'    { Rule 'ATRIUM Sunshine LAN+tailnet only' @{Protocol='TCP'; LocalPort=@(47984,47989,47990,48010); RemoteAddress='Internet'} }
Try-Step 'block Voicemod inbound' { Rule 'ATRIUM block Voicemod inbound' @{Program='C:\Users\xing\AppData\Local\VoicemodV3\app\last\Voicemod.exe'} }
Try-Step 'block LM Link inbound'  { Rule 'ATRIUM block LM Link inbound' @{Program='C:\Users\xing\.lmstudio\extensions\frameworks\lmlink-connector-win-x86_64-avx2-0.1.1\lmlink-connector.exe'} }
# Ollama: keep laptop + mac mini, block iPhone and anything non-tailnet
Try-Step 'Ollama limited to laptop + mac mini' { Rule 'ATRIUM Ollama block iphone+internet' @{Protocol='TCP'; LocalPort=11434; RemoteAddress=@('100.126.56.67','Internet')} }

Try-Step 'SMB1 removed' { Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol -NoRestart | Out-Null }
Try-Step 'RDP/RemoteRegistry/WinRM off' {
    Set-ItemProperty 'HKLM:\System\CurrentControlSet\Control\Terminal Server' -Name fDenyTSConnections -Value 1
    foreach ($s in 'RemoteRegistry','WinRM') { Set-Service $s -StartupType Disabled -ErrorAction SilentlyContinue } }

Try-Step 'defender PUA + controlled folder access' {
    Update-MpSignature
    Set-MpPreference -PUAProtection Enabled -EnableControlledFolderAccess Enabled
    Add-MpPreference -ControlledFolderAccessProtectedFolders 'D:\ATRIUM'
    $apps = @('C:\Users\xing\.cargo\bin\cargo.exe','C:\Users\xing\.cargo\bin\rustc.exe','C:\Program Files\Git\cmd\git.exe',
              'C:\Program Files\Git\mingw64\bin\git.exe','D:\ATRIUM\xing-pc-mcp\target\release\xing-pc-mcp.exe',
              'D:\dev\bin\atrium.exe','C:\Program Files\ward\ward.exe','C:\Program Files\nodejs\node.exe',
              'C:\Users\xing\.local\bin\claude.exe')
    foreach ($a in $apps) { if (Test-Path $a) { Add-MpPreference -ControlledFolderAccessAllowedApplications $a } } }

# BitLocker D: only if C is protected, so D can auto-unlock at boot (MCP + atrium run from D)
Try-Step 'bitlocker D' {
    $c = Get-BitLockerVolume -MountPoint 'C:'; $d = Get-BitLockerVolume -MountPoint 'D:'
    if ($d.ProtectionStatus -eq 'On') { Log '  D already protected' }
    elseif ($c.ProtectionStatus -ne 'On') { Log '  SKIPPED: C is not BitLocker-protected, so D could not auto-unlock and MCP/atrium would fail at boot' }
    else {
        Enable-BitLocker -MountPoint 'D:' -EncryptionMethod XtsAes256 -RecoveryPasswordProtector -UsedSpaceOnly | Out-Null
        Enable-BitLockerAutoUnlock -MountPoint 'D:' | Out-Null
        $rp = ((Get-BitLockerVolume -MountPoint 'D:').KeyProtector | Where-Object KeyProtectorType -eq 'RecoveryPassword').RecoveryPassword
        "BitLocker D: recovery key: $rp" | Add-Content $sec
        Log '  D encryption started with auto-unlock (key in secrets file)' } }

Try-Step 'tailnet lock' {
    $st = (& tailscale lock status 2>&1) -join "`n"
    if ($st -match 'is enabled') { Log '  already enabled' }
    else {
        $out = (& tailscale lock init --gen-disablements 2 --confirm 2>&1) -join "`n"
        "`n--- tailnet lock init output (disablement secrets) ---`n$out" | Add-Content $sec
        Log "  init ran; status now: $(((& tailscale lock status 2>&1) | Select-Object -First 1))" } }

Try-Step 'full scan started' { Start-MpScan -ScanType FullScan -AsJob | Out-Null }
Log '===== apply-all done (reboot to finish SMB1 removal) ====='

