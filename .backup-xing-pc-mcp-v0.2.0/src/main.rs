mod audit;
mod config;
mod rpc;
mod tools;

use std::{
    io::Write,
    process::{Command, Stdio},
    sync::Arc,
};

fn main() -> anyhow::Result<()> {
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
            let mut clip = Command::new("clip").stdin(Stdio::piped()).spawn()?;
            clip.stdin.take().unwrap().write_all(url.as_bytes())?;
            clip.wait()?;
            println!("connector URL copied to clipboard ({}/mcp/...)", cfg.public_base);
            return Ok(());
        }
        _ => {}
    }
    let cfg = Arc::new(config::Config::load_or_init()?);
    audit::record(serde_json::json!({"event": "start", "bind": cfg.bind}));
    rpc::serve(cfg)
}
