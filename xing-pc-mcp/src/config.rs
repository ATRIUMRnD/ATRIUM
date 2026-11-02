use crate::gpu::GpuConfig;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf};

/// On-disk policy, same pattern as video-watch's policy.json: edit the file, restart the server.
/// The token never sits on disk in plain text:
/// - Windows: wrapped with QFLTL's DPAPI keystore, so a copied config.json is useless off this
///   Windows user on this machine.
/// - macOS: stored in the login Keychain; config.json only holds the Keychain item's account id.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    /// Local bind address. Keep it on 127.0.0.1; the tunnel is the only way in.
    pub bind: String,
    /// Public base URL (the Tailscale Funnel address), used only by `xing-pc-mcp url`.
    #[serde(default = "default_public_base")]
    pub public_base: String,
    /// Legacy plain-text token. Migrated into the keystore and removed on first load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    /// Protected token, hex encoded (DPAPI blob on Windows, Keychain account id on macOS).
    /// Field name kept for compatibility with existing Windows configs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token_dpapi: Option<String>,
    /// Separate protected write token. None = writes are off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    write_token_dpapi: Option<String>,
    /// The only folders the tools can see.
    pub roots: Vec<PathBuf>,
    /// Case-insensitive substrings; any path containing one is refused.
    pub deny: Vec<String>,
    pub max_read_bytes: u64,
    /// Named commands. The caller picks a name, never the argv.
    pub presets: BTreeMap<String, Vec<String>>,
    /// Folder holding the unified memory chain (memory.jsonl). Git-ignored by default.
    #[serde(default = "default_memory_dir")]
    pub memory_dir: PathBuf,
    /// Unified GPU gateway (gpu.rs). All fields default, so an older config.json keeps working;
    /// set `"gpu": {"bind": ""}` to turn the gateway off.
    #[serde(default)]
    pub gpu: GpuConfig,
    /// Unwrapped token, memory only.
    #[serde(skip)]
    pub secret: String,
    /// Unwrapped write token, memory only. None = read-only server.
    #[serde(skip)]
    pub write_secret: Option<String>,
}

fn default_public_base() -> String {
    if cfg!(windows) {
        "https://xing-pc.tail8b5cd4.ts.net".into()
    } else {
        "https://xings-mac-mini.tail8b5cd4.ts.net".into()
    }
}

fn default_memory_dir() -> PathBuf {
    if cfg!(windows) {
        // Under ATRIUM's git-ignored /private/ folder, on D:.
        PathBuf::from(r"D:\ATRIUM\private\memory")
    } else {
        home().join("dev").join(".memory")
    }
}

fn home() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn dir() -> PathBuf {
    home().join(".xing-pc-mcp")
}

fn new_token() -> anyhow::Result<String> {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).map_err(|e| anyhow::anyhow!("rng: {e}"))?;
    Ok(hex(&b))
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn unhex(s: &str) -> anyhow::Result<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        anyhow::bail!("odd hex length");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| anyhow::anyhow!("hex: {e}")))
        .collect()
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Token at rest, Windows: QFLTL DPAPI keystore (user scope).
#[cfg(windows)]
mod keystore {
    pub fn protect(secret: &[u8]) -> anyhow::Result<Vec<u8>> {
        Ok(qfltl_crypto::keystore::protect(secret)?)
    }
    pub fn unprotect(blob: &[u8]) -> anyhow::Result<Vec<u8>> {
        Ok(qfltl_crypto::keystore::unprotect(blob)?.to_vec())
    }
}

/// Token at rest, macOS: login Keychain via the system `security` tool. The blob kept in
/// config.json is only a random account id; the secret itself lives in the Keychain.
/// Note: `security add-generic-password -w` takes the secret as an argument, so it is visible
/// to this same user's process list for the instant the command runs. Same trust boundary as
/// DPAPI user scope (anyone running as this user can read it either way).
#[cfg(not(windows))]
mod keystore {
    use std::process::{Command, Stdio};
    const SERVICE: &str = "xing-pc-mcp";

    pub fn protect(secret: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut id = [0u8; 8];
        getrandom::getrandom(&mut id).map_err(|e| anyhow::anyhow!("rng: {e}"))?;
        let account = super::hex(&id);
        let secret = std::str::from_utf8(secret)?;
        let st = Command::new("security")
            .args(["add-generic-password", "-U", "-s", SERVICE, "-a", &account, "-w", secret])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()?;
        if !st.success() {
            anyhow::bail!("Keychain refused to store the token (security exit {st})");
        }
        Ok(account.into_bytes())
    }

    pub fn unprotect(blob: &[u8]) -> anyhow::Result<Vec<u8>> {
        let account = std::str::from_utf8(blob)?;
        let out = Command::new("security")
            .args(["find-generic-password", "-s", SERVICE, "-a", account, "-w"])
            .stdin(Stdio::null())
            .output()?;
        if !out.status.success() {
            anyhow::bail!("token not found in the Keychain (account {account}); unlock the login keychain or rerun enable-write");
        }
        let mut v = out.stdout;
        while v.last().is_some_and(|b| *b == b'\n' || *b == b'\r') {
            v.pop();
        }
        Ok(v)
    }
}

/// Raw keystore access for other key material (sign.rs). Windows: DPAPI bytes in, blob out.
pub fn keystore_protect(secret: &[u8]) -> anyhow::Result<Vec<u8>> {
    keystore::protect(secret)
}

pub fn keystore_unprotect(blob: &[u8]) -> anyhow::Result<Vec<u8>> {
    keystore::unprotect(blob)
}

fn wrap(token: &str) -> anyhow::Result<String> {
    Ok(hex(&keystore::protect(token.as_bytes())?))
}

fn unwrap_token(wrapped: &str) -> anyhow::Result<String> {
    Ok(String::from_utf8(keystore::unprotect(&unhex(wrapped)?)?)?)
}

/// Built-in presets. Merged into any existing config.json at load (memory only, never
/// overwriting a preset the user defined), so new presets ship without editing the file.
fn default_presets() -> BTreeMap<String, Vec<String>> {
    let mut p = BTreeMap::new();
    p.insert("git_status".into(), strs(&["git", "status", "--short", "--branch"]));
    p.insert("git_log".into(), strs(&["git", "log", "--oneline", "-15"]));
    p.insert("git_diff_stat".into(), strs(&["git", "diff", "--stat"]));
    if cfg!(windows) {
        p.insert("nvidia_smi".into(), strs(&["nvidia-smi"]));
    } else {
        p.insert("sw_vers".into(), strs(&["sw_vers"]));
    }
    // Cargo diagnostics. They only write build output under the repo's target folder.
    p.insert(
        "cargo_check".into(),
        strs(&["cargo", "check", "--all-targets", "--message-format", "short", "--color", "never"]),
    );
    p.insert(
        "cargo_clippy".into(),
        strs(&["cargo", "clippy", "--all-targets", "--message-format", "short", "--color", "never"]),
    );
    p.insert(
        "cargo_test".into(),
        strs(&["cargo", "test", "--no-fail-fast", "--color", "never"]),
    );
    p
}

/// Unwrap with retries before treating the keystore as dead. At boot, a task can
/// start before the user profile (and its DPAPI master key) is loaded; a single
/// failure there must not rotate the tokens and kill the connector links.
fn unwrap_retry(wrapped: &str) -> anyhow::Result<String> {
    let mut last = None;
    for wait_ms in [0u64, 500, 1500, 4000, 10000] {
        if wait_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(wait_ms));
        }
        match unwrap_token(wrapped) {
            Ok(s) => return Ok(s),
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("unwrap failed")))
}

fn default_roots() -> Vec<PathBuf> {
    if cfg!(windows) {
        // D: only. The C drive stays out of the connector.
        // player-harness lives under D:\dev\player-harness, covered by D:\dev.
        vec![PathBuf::from(r"D:\dev"), PathBuf::from(r"D:\ATRIUM")]
    } else {
        vec![home().join("dev")]
    }
}

impl Config {
    fn defaults() -> anyhow::Result<Self> {
        Ok(Config {
            bind: "127.0.0.1:8787".into(),
            public_base: default_public_base(),
            token: None,
            token_dpapi: Some(wrap(&new_token()?)?),
            write_token_dpapi: None,
            roots: default_roots(),
            deny: strs(&[
                ".ssh", ".secrets", ".env", "id_ed25519", "id_rsa", ".pem", ".key", ".pfx",
                "credentials", ".xing-pc-mcp", r"\.git\config", "/.git/config", "keychains",
            ]),
            max_read_bytes: 256 * 1024,
            presets: default_presets(),
            memory_dir: default_memory_dir(),
            gpu: GpuConfig::default(),
            secret: String::new(),
            write_secret: None,
        })
    }

    pub fn load_or_init() -> anyhow::Result<Self> {
        let d = dir();
        fs::create_dir_all(&d)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&d, fs::Permissions::from_mode(0o700));
        }
        let p = d.join("config.json");
        let mut cfg: Config = if p.exists() {
            // Notepad / Windows PowerShell save UTF-8 with a BOM; accept it instead of failing.
            let text = fs::read_to_string(&p)?;
            serde_json::from_str(text.trim_start_matches('\u{feff}'))
                .map_err(|e| anyhow::anyhow!("config.json is not valid JSON ({e}): {}", p.display()))?
        } else {
            let c = Self::defaults()?;
            fs::write(&p, serde_json::to_string_pretty(&c)?)?;
            c
        };
        // Add any built-in preset the file is missing; user-defined ones win.
        for (k, v) in default_presets() {
            cfg.presets.entry(k).or_insert(v);
        }
        // Migrate a legacy plain-text token into the keystore and scrub it from disk.
        if let Some(plain) = cfg.token.take() {
            cfg.token_dpapi = Some(wrap(&plain)?);
            fs::write(&p, serde_json::to_string_pretty(&cfg)?)?;
            eprintln!("xing-pc-mcp: token moved into the keystore");
        }
        let wrapped = cfg
            .token_dpapi
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("config has no token_dpapi"))?;
        // Self-heal: a keystore reset (Windows DPAPI master key rotated by a password
        // reset or profile repair, macOS keychain wiped) leaves the wrapped tokens
        // unreadable and every restart would fail the same way. Instead of dying,
        // rotate exactly the tokens that failed, save, and leave a marker. The old
        // connector URLs stop working, which is the correct outcome: the secrets that
        // were protected by the dead key are gone, not leaked.
        let mut healed: Vec<String> = Vec::new();
        cfg.secret = match unwrap_retry(wrapped) {
            Ok(s) => s,
            Err(e) => {
                healed.push(format!("read token: {e}"));
                let t = new_token()?;
                cfg.token_dpapi = Some(wrap(&t)?);
                t
            }
        };
        if let Some(w) = cfg.write_token_dpapi.as_deref() {
            cfg.write_secret = Some(match unwrap_retry(w) {
                Ok(s) => s,
                Err(e) => {
                    healed.push(format!("write token: {e}"));
                    let t = new_token()?;
                    cfg.write_token_dpapi = Some(wrap(&t)?);
                    t
                }
            });
        }
        if !healed.is_empty() {
            cfg.save()?;
            let note = serde_json::json!({
                "at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
                "reason": healed,
                "next": "run `xing-pc-mcp url` and `xing-pc-mcp write-url`, paste the new links into claude.ai Connectors",
            });
            fs::write(d.join("healed.json"), serde_json::to_string_pretty(&note)?)?;
            eprintln!("xing-pc-mcp: keystore could not unwrap {} token(s); rotated them and wrote healed.json. Old connector links are dead; run `xing-pc-mcp url` / `write-url` for the new ones.", healed.len());
        }
        Ok(cfg)
    }

    /// True when the last load had to rotate tokens. Cleared by `heal_ack`.
    pub fn healed() -> Option<String> {
        fs::read_to_string(dir().join("healed.json")).ok()
    }

    pub fn heal_ack() -> anyhow::Result<()> {
        let f = dir().join("healed.json");
        if f.exists() {
            fs::remove_file(f)?;
        }
        Ok(())
    }

    fn save(&self) -> anyhow::Result<()> {
        let p = dir().join("config.json");
        let tmp = dir().join("config.json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        fs::rename(tmp, p)?;
        Ok(())
    }

    /// New write token every time (rotates the old one). Returns the plain token for the URL.
    pub fn enable_write(&mut self) -> anyhow::Result<String> {
        let t = new_token()?;
        self.write_token_dpapi = Some(wrap(&t)?);
        self.save()?;
        self.write_secret = Some(t.clone());
        Ok(t)
    }

    pub fn disable_write(&mut self) -> anyhow::Result<()> {
        self.write_token_dpapi = None;
        self.write_secret = None;
        self.save()
    }
}

pub fn killed() -> bool {
    dir().join("KILL").exists()
}

pub fn set_killed(on: bool) -> anyhow::Result<()> {
    let f = dir().join("KILL");
    fs::create_dir_all(dir())?;
    if on {
        fs::write(f, b"on")?;
    } else if f.exists() {
        fs::remove_file(f)?;
    }
    Ok(())
}
