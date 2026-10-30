use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::PathBuf};

/// On-disk policy, same pattern as video-watch's policy.json: edit the file, restart the server.
/// The token never sits on disk in plain text: it is wrapped with QFLTL's DPAPI keystore,
/// so a copied config.json is useless off this Windows user on this machine.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    /// Local bind address. Keep it on 127.0.0.1; the tunnel is the only way in.
    pub bind: String,
    /// Public base URL (the Tailscale Funnel address), used only by `xing-pc-mcp url`.
    #[serde(default = "default_public_base")]
    pub public_base: String,
    /// Legacy plain-text token. Migrated into `token_dpapi` and removed on first load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    /// Token wrapped by qfltl_crypto::keystore (DPAPI, user scope), hex encoded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    token_dpapi: Option<String>,
    /// The only folders the tools can see.
    pub roots: Vec<PathBuf>,
    /// Case-insensitive substrings; any path containing one is refused.
    pub deny: Vec<String>,
    pub max_read_bytes: u64,
    /// Named commands. The caller picks a name, never the argv.
    pub presets: BTreeMap<String, Vec<String>>,
    /// Unwrapped token, memory only.
    #[serde(skip)]
    pub secret: String,
}

fn default_public_base() -> String {
    "https://xing-pc.tail8b5cd4.ts.net:10000".into()
}

pub fn dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".xing-pc-mcp")
}

fn new_token() -> anyhow::Result<String> {
    let mut b = [0u8; 32];
    getrandom::getrandom(&mut b).map_err(|e| anyhow::anyhow!("rng: {e}"))?;
    Ok(hex(&b))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(s: &str) -> anyhow::Result<Vec<u8>> {
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

fn wrap(token: &str) -> anyhow::Result<String> {
    Ok(hex(&qfltl_crypto::keystore::protect(token.as_bytes())?))
}

impl Config {
    fn defaults() -> anyhow::Result<Self> {
        let mut presets = BTreeMap::new();
        presets.insert("git_status".into(), strs(&["git", "status", "--short", "--branch"]));
        presets.insert("git_log".into(), strs(&["git", "log", "--oneline", "-15"]));
        presets.insert("git_diff_stat".into(), strs(&["git", "diff", "--stat"]));
        presets.insert("nvidia_smi".into(), strs(&["nvidia-smi"]));
        Ok(Config {
            bind: "127.0.0.1:8787".into(),
            public_base: default_public_base(),
            token: None,
            token_dpapi: Some(wrap(&new_token()?)?),
            roots: vec![
                PathBuf::from(r"D:\dev"),
                PathBuf::from(r"D:\ATRIUM"),
                PathBuf::from(r"C:\Users\there\Code"),
            ],
            deny: strs(&[
                ".ssh", ".secrets", ".env", "id_ed25519", "id_rsa", ".pem", ".key", ".pfx",
                "credentials", ".xing-pc-mcp", r"\.git\config",
            ]),
            max_read_bytes: 256 * 1024,
            presets,
            secret: String::new(),
        })
    }

    pub fn load_or_init() -> anyhow::Result<Self> {
        let d = dir();
        fs::create_dir_all(&d)?;
        let p = d.join("config.json");
        let mut cfg: Config = if p.exists() {
            serde_json::from_str(&fs::read_to_string(&p)?)?
        } else {
            let c = Self::defaults()?;
            fs::write(&p, serde_json::to_string_pretty(&c)?)?;
            c
        };
        // Migrate a legacy plain-text token into the QFLTL keystore and scrub it from disk.
        if let Some(plain) = cfg.token.take() {
            cfg.token_dpapi = Some(wrap(&plain)?);
            fs::write(&p, serde_json::to_string_pretty(&cfg)?)?;
            eprintln!("xing-pc-mcp: token moved into the QFLTL DPAPI keystore");
        }
        let wrapped = cfg
            .token_dpapi
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("config has no token_dpapi"))?;
        let raw = qfltl_crypto::keystore::unprotect(&unhex(wrapped)?)?;
        cfg.secret = String::from_utf8(raw.to_vec())?;
        Ok(cfg)
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
