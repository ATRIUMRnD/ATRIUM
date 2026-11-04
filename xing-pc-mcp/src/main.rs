mod audit;
mod config;
mod gpu;
mod memory;
mod rpc;
mod sign;
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

/// `run-preset <name> <cwd> <out-file>`: the agent harness's way in (ductei-agent's
/// PresetDispatcher). Same policy as the MCP tool: preset by name only, cwd under the roots,
/// kill switch honoured, every call on the audit chain. The result lands in <out-file>
/// (must sit under the roots) via tmp + rename, so a reader never sees half a file.
fn run_preset_cli() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().skip(2).collect();
    let [name, cwd, out] = a.as_slice() else {
        anyhow::bail!("usage: xing-pc-mcp run-preset <name> <cwd> <out-file>");
    };
    let cfg = config::Config::load_or_init()?;
    let out_path = std::path::Path::new(out);
    let parent = out_path.parent().ok_or_else(|| anyhow::anyhow!("out-file has no parent"))?;
    let parent = tools::resolve(&cfg, &parent.to_string_lossy()).map_err(|e| anyhow::anyhow!(e))?;
    let file = out_path.file_name().ok_or_else(|| anyhow::anyhow!("out-file has no name"))?;
    if tools::denied(&cfg, &file.to_string_lossy()) {
        anyhow::bail!("out-file name is on the deny list");
    }
    let dest = parent.join(file);
    let (ok, output) = if config::killed() {
        (false, "kill switch is on; preset refused".to_string())
    } else {
        match tools::run_preset_for(&cfg, &serde_json::json!({"name": name, "cwd": cwd}), tools::DETACHED_TIMEOUT) {
            Ok(s) => (true, s),
            Err(e) => (false, e),
        }
    };
    audit::record(serde_json::json!({"event": "cli_run_preset", "name": name, "cwd": cwd, "ok": ok}));
    let tmp = dest.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string(&serde_json::json!({"ok": ok, "output": output}))?)?;
    std::fs::rename(&tmp, &dest)?;
    Ok(())
}

fn run() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("run-preset") => return run_preset_cli(),
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
        Some("verify-memory") => {
            let cfg = config::Config::load_or_init()?;
            match memory::verify(&cfg) {
                Ok(s) => println!("memory chain intact: {s}"),
                Err(e) => {
                    println!("MEMORY CHAIN BROKEN: {e}");
                    std::process::exit(2);
                }
            }
            return Ok(());
        }
        Some("memory-pubkey") => {
            // Public by design: what another machine needs to verify this chain's signatures.
            match sign::signer() {
                Some(s) => println!("{{\"scheme\":\"{}\",\"kid\":\"{}\",\"public_key_hex\":\"{}\"}}", sign::SCHEME, s.kid, s.public_hex()),
                None => println!("memory signing is off on this machine"),
            }
            return Ok(());
        }
        Some("url") => {
            // Copy the connector URL to the clipboard without ever printing the token.
            let cfg = config::Config::load_or_init()?;
            let url = format!("{}/mcp/{}", cfg.public_base.trim_end_matches('/'), cfg.secret);
            copy_to_clipboard(&url)?;
            println!("read-only connector URL copied to clipboard ({}/mcp/...)", cfg.public_base);
            if config::Config::healed().is_some() {
                println!("note: tokens were rotated after a keystore reset (healed.json); paste this link into claude.ai Connectors, then run `xing-pc-mcp heal-ack`");
            }
            return Ok(());
        }
        Some("heal-status") => {
            match config::Config::healed() {
                Some(s) => println!("{s}"),
                None => println!("no self-heal pending"),
            }
            return Ok(());
        }
        Some("heal-ack") => {
            config::Config::heal_ack()?;
            println!("healed.json cleared");
            return Ok(());
        }
        Some("gpu-key") => {
            // The gateway's API key is the read token (or the write token for /v1/memory/add).
            // Clipboard only, never printed.
            let cfg = config::Config::load_or_init()?;
            copy_to_clipboard(&cfg.secret)?;
            println!("GPU gateway API key (read token) copied to clipboard. Base URL: {}:8444/v1", cfg.public_base.trim_end_matches('/'));
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
            anyhow::bail!("unknown command `{other}` (run-preset, kill, unkill, url, write-url, gpu-key, enable-write, disable-write, approve, pending, verify-log, verify-memory, memory-pubkey, config-path)");
        }
        _ => {}
    }
    let cfg = Arc::new(config::Config::load_or_init()?);
    audit::record(serde_json::json!({"event": "start", "bind": cfg.bind, "gpu_bind": cfg.gpu.bind}));
    // The GPU gateway runs beside the MCP server in the same process; an empty gpu.bind turns it off.
    if !cfg.gpu.bind.trim().is_empty() {
        let g = cfg.clone();
        std::thread::spawn(move || {
            if let Err(e) = gpu::serve(g) {
                eprintln!("xing-pc-mcp gpu gateway: {e:#}");
                audit::record(serde_json::json!({"event": "gpu_gateway_failed", "error": e.to_string()}));
            }
        });
    }
    rpc::serve(cfg)
}
