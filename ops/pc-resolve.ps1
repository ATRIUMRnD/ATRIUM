$ErrorActionPreference = "Continue"
[Console]::OutputEncoding = [Text.Encoding]::UTF8
function Utf8NoBom($path, $text) { [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding $false)) }
function Ours($repo, $f)  { (git -C $repo show ":2:$f") -join "`n" }
function Theirs($repo, $f){ (git -C $repo show ":3:$f") -join "`n" }

# ---------- ATRIUM ----------
$A = 'D:\ATRIUM'
# AGENTS.md: upstream + section 7 appended
$ours = Ours $A 'AGENTS.md'; $th = Theirs $A 'AGENTS.md'
$idx = $th.IndexOf("## 7. Rooms outside the core mesh")
$sec7 = $th.Substring($idx)
Utf8NoBom "$A\AGENTS.md" ($ours.TrimEnd() + "`n`n" + $sec7.TrimEnd() + "`n")
# ROADMAP.md: upstream, with the two status-update sections inserted before Task 1
$ours = Ours $A 'ROADMAP.md'; $th = Theirs $A 'ROADMAP.md'
$s = $th.IndexOf("## Status update (October 24, 2026)")
$e = $th.IndexOf("## Task 1: CI harness", $s)
$block = $th.Substring($s, $e - $s).TrimEnd()
$t1 = $ours.IndexOf("## Task 1: CI harness")
Utf8NoBom "$A\ROADMAP.md" ($ours.Substring(0, $t1) + $block + "`n`n" + $ours.Substring($t1))
# 00-INDEX.md: upstream + Task 5 line
$ours = Ours $A 'harness-roadmap/00-INDEX.md'
$ours = $ours -replace '(?m)^(- \[04-qallow-kernel-invariants\.md\].*)$', "`$1`n- [05-heartbeat-loop.md](05-heartbeat-loop.md) - Task 5: Heartbeat loop (BUILT, scenarios run with scaled timings)"
Utf8NoBom "$A\harness-roadmap\00-INDEX.md" $ours
# 01/02/03: upstream wins
foreach ($f in 'harness-roadmap/01-ci-harness.md','harness-roadmap/02-spine-flow.md','harness-roadmap/03-self-improvement-loop.md') { git -C $A checkout --ours -- $f }
git -C $A add AGENTS.md ROADMAP.md harness-roadmap/00-INDEX.md harness-roadmap/01-ci-harness.md harness-roadmap/02-spine-flow.md harness-roadmap/03-self-improvement-loop.md
"ATRIUM unresolved: " + ((git -C $A diff --name-only --diff-filter=U) -join ',')

# ---------- LIMEN ----------
$L = 'D:\ATRIUM\LIMEN'
$txt = [IO.File]::ReadAllText("$L\docs\CHANGELOG.md")
$txt = $txt -replace '(?m)^<<<<<<< Updated upstream\r?\n', '' -replace '(?m)^=======\r?\n', '' -replace '(?m)^>>>>>>> Stashed changes\r?\n', ''
Utf8NoBom "$L\docs\CHANGELOG.md" $txt
git -C $L add docs/CHANGELOG.md
"LIMEN unresolved: " + ((git -C $L diff --name-only --diff-filter=U) -join ',')

# ---------- DUCTEI ----------
$D = 'D:\ATRIUM\DUCTEI'
$f = "$D\ductei-core\src\transport.rs"
$txt = [IO.File]::ReadAllText($f)
$h1old = "<<<<<<< Updated upstream`r?`nuse std::path::\{PathBuf, Path\};`r?`n=======`r?`nuse std::time::Duration;"
$txt = [regex]::Replace($txt, $h1old, "use std::path::{PathBuf, Path};`nuse std::time::Duration;")
$txt = [regex]::Replace($txt, "pub const ACK_TIMEOUT: Duration = Duration::from_secs\(5\);`r?`n>>>>>>> Stashed changes`r?`n", "pub const ACK_TIMEOUT: Duration = Duration::from_secs(5);`n")
$h2 = @"
        Self::connect_timeout(addr, CONNECT_TIMEOUT)
    }
    /// Attach a durable outbound intent/receipt log. When set, the client
    /// records intents before any network write and receipts before returning.
    pub fn with_outbox_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.outbox_path = path.into();
        self
    }

    /// Try each resolved address with a bounded wait; the stream also gets
    /// bounded read and write waits so a stuck peer cannot hang the sender.
    pub fn connect_timeout(addr: impl ToSocketAddrs, timeout: Duration) -> std::io::Result<Self> {
        let mut last = std::io::Error::new(std::io::ErrorKind::InvalidInput, "no address resolved");
        for a in addr.to_socket_addrs()? {
            match TcpStream::connect_timeout(&a, timeout) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(ACK_TIMEOUT))?;
                    stream.set_write_timeout(Some(ACK_TIMEOUT))?;
                    return Ok(Self { stream, outbox_path: default_outbox_path() });
                }
                Err(e) => last = e,
            }
        }
        Err(last)
"@
$txt = [regex]::Replace($txt, "(?s)<<<<<<< Updated upstream`r?`n        Ok\(Self \{ stream: TcpStream::connect\(addr\)\?, outbox_path: default_outbox_path\(\) \}\).*?>>>>>>> Stashed changes`r?`n", ($h2 -replace "`r`n","`n") + "`n")
Utf8NoBom $f $txt
"transport.rs markers left: " + ([regex]::Matches($txt, '(?m)^(<<<<<<<|=======|>>>>>>>)').Count)
git -C $D add ductei-core/src/transport.rs
"DUCTEI unresolved: " + ((git -C $D diff --name-only --diff-filter=U) -join ',')
