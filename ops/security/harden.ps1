# ATRIUM hardening. Keeps xing-pc-mcp + Funnel running (Xing's call, Nov 2 2026).
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\security\harden.ps1
# Run in an elevated (admin) PowerShell. Each step asks first. Logs to harden.log next to this file.
$ErrorActionPreference = 'Continue'
$log = 'D:\ATRIUM\ops\security\harden.log'
function Log($m) { "$(Get-Date -Format s) $m" | Tee-Object -FilePath $log -Append }
function Ask($q) { (Read-Host "$q [y/N]") -eq 'y' }

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole('Administrators')) {
    Write-Host 'Run this in an elevated (admin) PowerShell.'; exit 1
}

# --- Admin-only checks the non-elevated triage could not see ---
"`n== Defender exclusions (should be empty or known) =="
$p = Get-MpPreference; $p.ExclusionPath; $p.ExclusionProcess; $p.ExclusionExtension
"`n== Secure Boot ==";  Confirm-SecureBootUEFI
"`n== TPM =="; Get-Tpm | Select-Object TpmPresent, TpmReady, ManufacturerIdTxt | Format-List
"`n== BitLocker =="; Get-BitLockerVolume | Format-Table MountPoint, ProtectionStatus, EncryptionPercentage
"`n== SMB1 =="; (Get-WindowsOptionalFeature -Online -FeatureName SMB1Protocol).State
Read-Host "`nReview the above, then press Enter to continue"

if (Ask 'OpenSSH server is running and listening on 0.0.0.0:22. Stop and disable it') {
    Stop-Service sshd -ErrorAction SilentlyContinue; Set-Service sshd -StartupType Disabled
    Log 'sshd stopped and disabled' }

if (Ask 'Firewall: set default inbound to Block on every profile (existing allow rules still apply)') {
    Set-NetFirewallProfile -Profile Domain,Private,Public -Enabled True -DefaultInboundAction Block -DefaultOutboundAction Allow
    Log 'firewall: inbound default block' }

if (Ask 'Block SMB (445) and NetBIOS (137-139) inbound from everything except the home LAN') {
    New-NetFirewallRule -DisplayName 'ATRIUM block SMB non-LAN' -Direction Inbound -Protocol TCP -LocalPort 139,445 `
        -RemoteAddress Internet -Action Block -Profile Any -ErrorAction SilentlyContinue | Out-Null
    Log 'firewall: SMB/NetBIOS blocked from Internet' }

if (Ask 'Block wildcard listeners that do not need the network: Voicemod (39998), LM Link (33593)') {
    New-NetFirewallRule -DisplayName 'ATRIUM block Voicemod inbound' -Direction Inbound `
        -Program 'C:\Users\xing\AppData\Local\VoicemodV3\app\last\Voicemod.exe' -Action Block -ErrorAction SilentlyContinue | Out-Null
    New-NetFirewallRule -DisplayName 'ATRIUM block LM Link inbound' -Direction Inbound `
        -Program 'C:\Users\xing\.lmstudio\extensions\frameworks\lmlink-connector-win-x86_64-avx2-0.1.1\lmlink-connector.exe' -Action Block -ErrorAction SilentlyContinue | Out-Null
    Log 'firewall: Voicemod + LM Link inbound blocked' }

if (Ask 'Sunshine (game streaming, ports 47984-48010) listens on all interfaces. Limit it to LAN + Tailscale') {
    New-NetFirewallRule -DisplayName 'ATRIUM Sunshine LAN+tailnet only' -Direction Inbound -Protocol TCP -LocalPort 47984,47989,47990,48010 `
        -RemoteAddress Internet -Action Block -ErrorAction SilentlyContinue | Out-Null
    Log 'firewall: Sunshine blocked from Internet' }

if (Ask 'Remove SMB1 and keep RDP / Remote Registry / WinRM off') {
    Disable-WindowsOptionalFeature -Online -FeatureName SMB1Protocol -NoRestart | Out-Null
    Set-ItemProperty 'HKLM:\System\CurrentControlSet\Control\Terminal Server' -Name fDenyTSConnections -Value 1
    foreach ($s in 'RemoteRegistry','WinRM') { Set-Service $s -StartupType Disabled -ErrorAction SilentlyContinue }
    Log 'SMB1 removed; RDP/RemoteRegistry/WinRM disabled' }

if (Ask 'Defender: update, PUA blocking, protect D:\ATRIUM, start a FULL scan (never run on this box)') {
    Update-MpSignature
    Set-MpPreference -PUAProtection Enabled -EnableControlledFolderAccess Enabled
    Add-MpPreference -ControlledFolderAccessProtectedFolders 'D:\ATRIUM'
    foreach ($app in 'C:\Users\xing\.cargo\bin\cargo.exe','C:\Program Files\Git\cmd\git.exe','D:\ATRIUM\xing-pc-mcp\target\release\xing-pc-mcp.exe') {
        if (Test-Path $app) { Add-MpPreference -ControlledFolderAccessAllowedApplications $app } }
    Start-MpScan -ScanType FullScan -AsJob | Out-Null
    Log 'defender: PUA on, CFA on D:\ATRIUM (cargo/git/mcp allowed), full scan started' }

$d = Get-BitLockerVolume -MountPoint 'D:' -ErrorAction SilentlyContinue
if ($d -and $d.ProtectionStatus -ne 'On' -and (Ask 'BitLocker: encrypt D: (XTS-AES 256). You MUST write down the recovery key shown')) {
    Enable-BitLocker -MountPoint 'D:' -EncryptionMethod XtsAes256 -RecoveryPasswordProtector | Out-Null
    $rp = (Get-BitLockerVolume -MountPoint 'D:').KeyProtector | Where-Object KeyProtectorType -eq 'RecoveryPassword'
    Write-Host "`nRECOVERY KEY (write/print it, it is NOT saved anywhere):`n$($rp.RecoveryPassword)`n" -ForegroundColor Yellow
    Read-Host 'Press Enter after you have written it down'
    Log 'bitlocker: D: encryption started (key not logged)' }

Log 'done. xing-pc-mcp and Funnel left running as requested. Reboot to finish SMB1 removal.'
