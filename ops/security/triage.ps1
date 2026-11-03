# ATRIUM triage: READ-ONLY. Changes nothing. Run in an elevated PowerShell:
#   powershell -ExecutionPolicy Bypass -File D:\ATRIUM\ops\security\triage.ps1
# Writes D:\ATRIUM\ops\security\triage-report.txt so Claude can read it back.
$ErrorActionPreference = 'Continue'
$out = 'D:\ATRIUM\ops\security\triage-report.txt'
function Sec($t) { "`n===== $t =====" }
function P($id) { try { (Get-Process -Id $id -ErrorAction Stop).Path } catch { "pid $id" } }

& {
    "triage $(Get-Date -Format s) on $env:COMPUTERNAME"
    Sec 'LISTENING TCP'
    Get-NetTCPConnection -State Listen | Sort-Object LocalPort |
        ForEach-Object { '{0}:{1}  {2}' -f $_.LocalAddress, $_.LocalPort, (P $_.OwningProcess) }
    Sec 'LISTENING UDP'
    Get-NetUDPEndpoint | Where-Object { $_.LocalAddress -notin '127.0.0.1','::1' } | Sort-Object LocalPort |
        ForEach-Object { '{0}:{1}  {2}' -f $_.LocalAddress, $_.LocalPort, (P $_.OwningProcess) }
    Sec 'ESTABLISHED TCP'
    Get-NetTCPConnection -State Established | Where-Object { $_.RemoteAddress -notin '127.0.0.1','::1' } |
        Sort-Object OwningProcess |
        ForEach-Object { '{0}:{1}  {2}' -f $_.RemoteAddress, $_.RemotePort, (P $_.OwningProcess) }
    Sec 'NON-MICROSOFT SCHEDULED TASKS'
    Get-ScheduledTask | Where-Object { $_.State -ne 'Disabled' -and $_.TaskPath -notlike '\Microsoft\*' } |
        ForEach-Object { '{0}{1}  ->  {2} {3}' -f $_.TaskPath, $_.TaskName, $_.Actions.Execute, $_.Actions.Arguments }
    Sec 'STARTUP COMMANDS'
    Get-CimInstance Win32_StartupCommand | ForEach-Object { '{0} | {1} | {2}' -f $_.Name, $_.Command, $_.Location }
    Sec 'RUNNING NON-MICROSOFT SERVICES'
    Get-CimInstance Win32_Service | Where-Object { $_.State -eq 'Running' -and $_.PathName -notmatch '\\Windows\\' } |
        ForEach-Object { '{0} | {1}' -f $_.Name, $_.PathName }
    Sec 'LOCAL ADMINISTRATORS'
    Get-LocalGroupMember Administrators | ForEach-Object { '{0} ({1})' -f $_.Name, $_.PrincipalSource }
    Sec 'LOCAL USERS'
    Get-LocalUser | ForEach-Object { '{0} enabled={1} lastlogon={2}' -f $_.Name, $_.Enabled, $_.LastLogon }
    Sec 'REMOTE ACCESS'
    'RDP fDenyTSConnections (1 = off): ' + (Get-ItemProperty 'HKLM:\System\CurrentControlSet\Control\Terminal Server').fDenyTSConnections
    'RemoteRegistry: ' + (Get-Service RemoteRegistry -ErrorAction SilentlyContinue).Status
    'WinRM: ' + (Get-Service WinRM -ErrorAction SilentlyContinue).Status
    'sshd: ' + (Get-Service sshd -ErrorAction SilentlyContinue).Status
    'SMB1: ' + (Get-WindowsOptionalFeature -Online -FeatureName SMB1Protocol).State
    Sec 'TAILSCALE'
    if (Get-Command tailscale -ErrorAction SilentlyContinue) {
        tailscale status 2>&1; tailscale funnel status 2>&1; tailscale serve status 2>&1; tailscale lock status 2>&1
    } else { 'tailscale not on PATH' }
    Sec 'FIREWALL PROFILES'
    Get-NetFirewallProfile | ForEach-Object { '{0} enabled={1} in={2} out={3}' -f $_.Name, $_.Enabled, $_.DefaultInboundAction, $_.DefaultOutboundAction }
    Sec 'DEFENDER'
    Get-MpComputerStatus | Select-Object AMServiceEnabled, RealTimeProtectionEnabled, IsTamperProtected, AntivirusSignatureLastUpdated, QuickScanAge, FullScanAge | Format-List | Out-String
    'Recent threats:'; Get-MpThreatDetection | Select-Object -First 20 | Format-List | Out-String
    'Exclusions (attackers add these):'; $p = Get-MpPreference; $p.ExclusionPath; $p.ExclusionProcess; $p.ExclusionExtension
    Sec 'BOOT / TPM / BITLOCKER'
    'SecureBoot: ' + (Confirm-SecureBootUEFI)
    Get-Tpm | Select-Object TpmPresent, TpmReady, ManufacturerIdTxt, ManufacturerVersion | Format-List | Out-String
    Get-BitLockerVolume | ForEach-Object { '{0} {1} {2}%' -f $_.MountPoint, $_.ProtectionStatus, $_.EncryptionPercentage }
    Sec 'HOSTS FILE (non-comment lines)'
    Get-Content "$env:windir\System32\drivers\etc\hosts" | Where-Object { $_ -notmatch '^\s*(#|$)' }
    Sec 'PROXY'
    netsh winhttp show proxy
} *>&1 | Out-File -FilePath $out -Encoding utf8
Write-Host "Done. Report: $out"

