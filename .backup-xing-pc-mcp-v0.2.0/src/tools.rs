use crate::config::Config;
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const MAX_ENTRIES: usize = 500;
const MAX_CMD_OUT: usize = 64 * 1024;
const CMD_TIMEOUT: Duration = Duration::from_secs(60);

pub fn list() -> Value {
    json!([
        {
            "name": "status",
            "description": "Show what this xing-pc server allows: roots, command presets, limits.",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "list_dir",
            "description": "List a folder on xing-pc. Only folders under the allowed roots.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string", "description": "Absolute Windows path, e.g. D:\\dev"}
            }, "required": ["path"]},
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "read_file",
            "description": "Read a text file on xing-pc (capped size). Only files under the allowed roots.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string"},
                "offset": {"type": "integer", "description": "Byte offset to start at, default 0"}
            }, "required": ["path"]},
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "run_preset",
            "description": "Run a named, read-only command preset (see status) in a folder under the allowed roots.",
            "inputSchema": {"type": "object", "properties": {
                "name": {"type": "string"},
                "cwd": {"type": "string", "description": "Working folder, default the first root"}
            }, "required": ["name"]},
            "annotations": {"readOnlyHint": true}
        }
    ])
}

pub fn call(cfg: &Config, name: &str, args: &Value) -> Result<String, String> {
    match name {
        "status" => Ok(status(cfg)),
        "list_dir" => list_dir(cfg, args),
        "read_file" => read_file(cfg, args),
        "run_preset" => run_preset(cfg, args),
        _ => Err(format!("unknown tool: {name}")),
    }
}

fn arg<'a>(args: &'a Value, k: &str) -> Result<&'a str, String> {
    args.get(k).and_then(|v| v.as_str()).ok_or_else(|| format!("missing `{k}`"))
}

fn denied(cfg: &Config, p: &str) -> bool {
    let low = p.to_lowercase();
    cfg.deny.iter().any(|d| low.contains(&d.to_lowercase()))
}

/// Canonicalize (resolves .., symlinks, junctions) and require the result to sit under a root.
fn resolve(cfg: &Config, raw: &str) -> Result<PathBuf, String> {
    let p = fs::canonicalize(raw).map_err(|e| format!("{raw}: {e}"))?;
    if denied(cfg, &p.to_string_lossy()) {
        return Err("path is on the deny list".into());
    }
    for r in &cfg.roots {
        if let Ok(rc) = fs::canonicalize(r) {
            if p.starts_with(&rc) {
                return Ok(p);
            }
        }
    }
    Err(format!("{raw} is outside the allowed roots"))
}

fn status(cfg: &Config) -> String {
    let presets: Vec<_> = cfg.presets.keys().collect();
    json!({
        "server": "xing-pc-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "mode": "read-only",
        "roots": cfg.roots,
        "presets": presets,
        "max_read_bytes": cfg.max_read_bytes
    })
    .to_string()
}

fn list_dir(cfg: &Config, args: &Value) -> Result<String, String> {
    let p = resolve(cfg, arg(args, "path")?)?;
    let mut out = Vec::new();
    let rd = fs::read_dir(&p).map_err(|e| e.to_string())?;
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if denied(cfg, &name) {
            continue;
        }
        let md = e.metadata().ok();
        let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = md.as_ref().filter(|m| m.is_file()).map(|m| m.len());
        out.push(json!({"name": name, "dir": is_dir, "size": size}));
        if out.len() >= MAX_ENTRIES {
            out.push(json!({"truncated": true}));
            break;
        }
    }
    Ok(json!({"path": p.display().to_string(), "entries": out}).to_string())
}

fn read_file(cfg: &Config, args: &Value) -> Result<String, String> {
    let p = resolve(cfg, arg(args, "path")?)?;
    let offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
    let mut f = fs::File::open(&p).map_err(|e| e.to_string())?;
    let md = f.metadata().map_err(|e| e.to_string())?;
    if !md.is_file() {
        return Err("not a file".into());
    }
    let len = md.len();
    f.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    f.take(cfg.max_read_bytes).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    if buf.iter().take(8192).any(|b| *b == 0) {
        return Err("looks binary, not returned".into());
    }
    let end = offset + buf.len() as u64;
    let mut text = String::from_utf8_lossy(&buf).to_string();
    if end < len {
        text.push_str(&format!("\n\n[truncated: read {offset}..{end} of {len} bytes; call again with offset {end}]"));
    }
    Ok(text)
}

fn run_preset(cfg: &Config, args: &Value) -> Result<String, String> {
    let name = arg(args, "name")?;
    let argv = cfg.presets.get(name).ok_or_else(|| format!("no preset `{name}`"))?;
    let cwd = match args.get("cwd").and_then(|v| v.as_str()) {
        Some(c) => resolve(cfg, c)?,
        None => resolve(cfg, &cfg.roots.first().ok_or("no roots configured")?.to_string_lossy())?,
    };
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn {}: {e}", argv[0]))?;
    let mut so = child.stdout.take().unwrap();
    let mut se = child.stderr.take().unwrap();
    let t_out = thread::spawn(move || { let mut v = Vec::new(); let _ = so.read_to_end(&mut v); v });
    let t_err = thread::spawn(move || { let mut v = Vec::new(); let _ = se.read_to_end(&mut v); v });
    let start = Instant::now();
    let code = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s.code(),
            Ok(None) if start.elapsed() > CMD_TIMEOUT => {
                let _ = child.kill();
                return Err(format!("`{name}` timed out after {}s", CMD_TIMEOUT.as_secs()));
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(e.to_string()),
        }
    };
    let mut out = t_out.join().unwrap_or_default();
    out.extend(t_err.join().unwrap_or_default());
    out.truncate(MAX_CMD_OUT);
    Ok(format!("[{name} in {} exit {:?}]\n{}", cwd.display(), code, String::from_utf8_lossy(&out)))
}
