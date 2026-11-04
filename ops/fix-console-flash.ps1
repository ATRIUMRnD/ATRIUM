# Stops the hourly cargo console pop-up: xing-pc-mcp spawns presets with CREATE_NO_WINDOW.
# Patches the working tree only (no commit), rebuilds, restarts the xing-pc-mcp task.
$ErrorActionPreference = 'Stop'
$repo = 'D:\ATRIUM\xing-pc-mcp'
$f = Join-Path $repo 'src\tools.rs'
$s = [IO.File]::ReadAllText($f)
if ($s -notmatch 'fn hide\(') {
  Copy-Item $f "$f.pre-nowindow.bak" -Force
  $s = $s.Replace('let mut child = Command::new(&argv[0])', "let mut cmd = Command::new(&argv[0]);`r`n    let mut child = hide(&mut cmd)")
  $s = $s.Replace('let _ = Command::new("taskkill")', 'let _ = hide(&mut Command::new("taskkill"))')
  $s += @"

/// No console window for preset children (the detached run-preset CLI has no console,
/// so cargo would otherwise get a fresh visible one).
fn hide(c: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}
"@
  [IO.File]::WriteAllText($f, $s)
  Write-Host 'patched tools.rs'
}
$exe = (Get-ScheduledTask -TaskName 'xing-pc-mcp').Actions[0].Execute
Write-Host "task exe: $exe"
Stop-ScheduledTask -TaskName 'xing-pc-mcp'
Get-Process xing-pc-mcp -ErrorAction SilentlyContinue | Stop-Process -Force
Push-Location $repo
cargo build --release
Pop-Location
$built = Join-Path $repo 'target\release\xing-pc-mcp.exe'
if ((Resolve-Path $exe).Path -ne (Resolve-Path $built).Path) { Copy-Item $built $exe -Force }
Start-ScheduledTask -TaskName 'xing-pc-mcp'
Write-Host 'done: xing-pc-mcp rebuilt and restarted'
