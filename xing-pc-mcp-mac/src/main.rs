mod audit;
mod config;
mod rpc;
mod tools;
mod write;

use std::{
    io::Write,
    process::{Command, Stdio},
    sync::Arc,
};

fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    let tool = if cfg!(windows) { "clip" } else { "pbcopy" };
    let mut clip = Command::new(tool).stdin(Stdio::piped()).spawn()?;
    clip.stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("clipboard not available"))?
        .write_all(text.as_bytes())?;
    clip.wait()?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("xing-pc-mcp: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("kill") => {
            config::set_killed(true)?;
            audit::record(serde_json::json!({"event": "kill_on"}));
            println!("kill switch ON: every tool call is refused until `xing-pc-mcp unkill`");
            return Ok(());
        }
        Some("unkill") => {
            config::set_killed(false)?;
            audit::record(serde_json::json!({"event": "kill_off"}));
            println!("kill switch OFF");
            return Ok(());
        }
        Some("config-path") => {
            println!("{}", config::dir().join("config.json").display());
            return Ok(());
        }
        Some("verify-log") => {
            match audit::verify() {
                Ok(n) => println!("audit chain intact: {n} entries ({})", audit::file().display()),
                Err(e) => {
                    println!("AUDIT CHAIN BROKEN: {e}");
                    std::process::exit(2);
                }
            }
            return Ok(());
        }
        Some("url") => {
            // Copy the connector URL to the clipboard without ever printing the token.
            let cfg = config::Config::load_or_init()?;
            let url = format!("{}/mcp/{}", cfg.public_base.trim_end_matches('/'), cfg.secret);
            copy_to_clipboard(&url)?;
            println!("read-only connector URL copied to clipboard ({}/mcp/...)", cfg.public_base);
            return Ok(());
        }
        Some("enable-write") => {
            let mut cfg = config::Config::load_or_init()?;
            let t = cfg.enable_write()?;
            copy_to_clipboard(&format!("{}/mcp/{}", cfg.public_base.trim_end_matches('/'), t))?;
            audit::record(serde_json::json!({"event": "write_enabled"}));
            println!("write token created (old one, if any, is dead). Write URL copied to clipboard.");
            println!("restart the server, then add the URL as a separate connector.");
            return Ok(());
        }
        Some("write-url") => {
            // Copy the CURRENT write URL (no rotation), so a lost clipboard never needs a restart.
            let cfg = config::Config::load_or_init()?;
            let w = cfg
                .write_secret
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("writes are off; run `xing-pc-mcp enable-write` first"))?;
            copy_to_clipboard(&format!("{}/mcp/{}", cfg.public_base.trim_end_matches('/'), w))?;
            println!("current write URL copied to clipboard ({}/mcp/...). Paste it now.", cfg.public_base);
            return Ok(());
        }
        Some("disable-write") => {
            let mut cfg = config::Config::load_or_init()?;
            cfg.disable_write()?;
            audit::record(serde_json::json!({"event": "write_disabled"}));
            println!("writes OFF. Restart the server; the write URL now gets 401.");
            return Ok(());
        }
        Some("approve") => return write::approve_cli(),
        Some("pending") => {
            let p = write::list_pending();
            if p.is_empty() {
                println!("nothing pending");
            }
            for e in p {
                println!("[{}] {}  {}", e["id"].as_str().unwrap_or(""), e["repo"].as_str().unwrap_or(""), e["message"].as_str().unwrap_or(""));
            }
            return Ok(());
        }
        Some(other) => {
            anyhow::bail!("unknown command `{other}` (kill, unkill, url, write-url, enable-write, disable-write, approve, pending, verify-log, config-path)");
        }
        _ => {}
    }
    let cfg = Arc::new(config::Config::load_or_init()?);
    audit::record(serde_json::json!({"event": "start", "bind": cfg.bind}));
    rpc::serve(cfg)
}
