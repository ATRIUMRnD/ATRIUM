use crate::{config::Config, memory, write};
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
/// Cargo test/clippy on a cold target folder routinely runs past a minute.
const CMD_TIMEOUT: Duration = Duration::from_secs(300);

/// Tools that need the write connector URL.
const WRITE_TOOLS: [&str; 5] = ["write_file", "prepare_commit", "list_pending", "memory_add", "memory_forget"];

pub fn list(writable: bool) -> Value {
    let mut v = json!([
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
            "description": "Read a text file on xing-pc (capped size, UTF-8 or UTF-16). Only files under the allowed roots.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string"},
                "offset": {"type": "integer", "description": "Byte offset to start at, default 0"}
            }, "required": ["path"]},
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "run_preset",
            "description": "Run a named command preset (see status) in a folder under the allowed roots. Cargo presets only write under the repo's target folder.",
            "inputSchema": {"type": "object", "properties": {
                "name": {"type": "string"},
                "cwd": {"type": "string", "description": "Working folder, default the first root"}
            }, "required": ["name"]},
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "memory_status",
            "description": "Unified memory: entry counts and the list of topics in Xing's shared memory store (one store for every chat, Claude Code, the Mac and voice).",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "memory_recent",
            "description": "Unified memory: newest live entries first. Call this at the start of a conversation to load shared context.",
            "inputSchema": {"type": "object", "properties": {
                "topic": {"type": "string", "description": "Only this topic (case-insensitive), e.g. brain, limen, ductei"},
                "limit": {"type": "integer", "description": "Max entries, default 20, max 100"}
            }},
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "memory_search",
            "description": "Unified memory: case-insensitive substring search over text, topic and tags of live entries, newest first.",
            "inputSchema": {"type": "object", "properties": {
                "query": {"type": "string"},
                "topic": {"type": "string"},
                "limit": {"type": "integer"}
            }, "required": ["query"]},
            "annotations": {"readOnlyHint": true}
        },
        {
            "name": "memory_verify",
            "description": "Unified memory: walk the hash chain and report whether the store is intact.",
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": {"readOnlyHint": true}
        }
    ]);
    if writable {
        if let Some(a) = v.as_array_mut() {
            a.push(json!({
                "name": "write_file",
                "description": "Create or replace a text file on xing-pc. D: drive only, under the allowed roots, never in .git. Atomic replace.",
                "inputSchema": {"type": "object", "properties": {
                    "path": {"type": "string", "description": "Absolute path on D:, e.g. D:\\dev\\x\\src\\main.rs"},
                    "content": {"type": "string"},
                    "create_dirs": {"type": "boolean", "description": "Create missing parent folders, default false"}
                }, "required": ["path", "content"]},
                "annotations": {"destructiveHint": true}
            }));
            a.push(json!({
                "name": "prepare_commit",
                "description": "Queue a git commit for Xing to approve at xing-pc (xing-pc-mcp approve). Nothing is committed or pushed by this call.",
                "inputSchema": {"type": "object", "properties": {
                    "repo": {"type": "string", "description": "Repo folder on D:"},
                    "message": {"type": "string", "description": "Commit message. No attribution lines, no @-mentions."},
                    "paths": {"type": "array", "items": {"type": "string"}, "description": "Files relative to the repo"}
                }, "required": ["repo", "message", "paths"]}
            }));
            a.push(json!({
                "name": "list_pending",
                "description": "List commits waiting for Xing's approval.",
                "inputSchema": {"type": "object", "properties": {}},
                "annotations": {"readOnlyHint": true}
            }));
            a.push(json!({
                "name": "memory_add",
                "description": "Unified memory: append one entry (append-only, hash-chained). Only record what Xing stated or decided; one entry per fact.",
                "inputSchema": {"type": "object", "properties": {
                    "text": {"type": "string", "description": "The fact, decision, preference or status, in Xing's terms"},
                    "topic": {"type": "string", "description": "Short slug, e.g. brain, limen, ductei, video-watch, career"},
                    "kind": {"type": "string", "enum": ["fact", "decision", "preference", "status", "note"], "description": "Default note"},
                    "tags": {"type": "array", "items": {"type": "string"}},
                    "source": {"type": "string", "description": "Where it came from: chat, code, mac, voice"},
                    "supersedes": {"type": "integer", "description": "seq of the entry this replaces (the old one stops showing)"}
                }, "required": ["text", "topic"]}
            }));
            a.push(json!({
                "name": "memory_forget",
                "description": "Unified memory: retire an entry by seq. Appends a tombstone; the chain is never rewritten.",
                "inputSchema": {"type": "object", "properties": {
                    "seq": {"type": "integer"},
                    "reason": {"type": "string"},
                    "source": {"type": "string"}
                }, "required": ["seq"]},
                "annotations": {"destructiveHint": true}
            }));
        }
    }
    v
}

pub fn call(cfg: &Config, name: &str, args: &Value, writable: bool) -> Result<String, String> {
    if !writable && WRITE_TOOLS.contains(&name) {
        return Err(format!("`{name}` needs the write connector URL (xing-pc-mcp enable-write)"));
    }
    match name {
        "write_file" => write::write_file(cfg, args),
        "prepare_commit" => write::prepare_commit(cfg, args),
        "list_pending" => Ok(json!(write::list_pending()).to_string()),
        "memory_add" => memory::add(cfg, args),
        "memory_forget" => memory::forget(cfg, args),
        "memory_status" => memory::status(cfg),
        "memory_recent" => memory::recent(cfg, args),
        "memory_search" => memory::search(cfg, args),
        "memory_verify" => memory::verify(cfg),
        "status" => Ok(status(cfg, writable)),
        "list_dir" => list_dir(cfg, args),
        "read_file" => read_file(cfg, args),
        "run_preset" => run_preset(cfg, args),
        _ => Err(format!("unknown tool: {name}")),
    }
}

pub fn arg<'a>(args: &'a Value, k: &str) -> Result<&'a str, String> {
    args.get(k).and_then(|v| v.as_str()).ok_or_else(|| format!("missing `{k}`"))
}

pub fn denied(cfg: &Config, p: &str) -> bool {
    let low = p.to_lowercase();
    cfg.deny.iter().any(|d| low.contains(&d.to_lowercase()))
}

/// Strip the `\\?\` verbatim prefix canonicalize adds on Windows (same job as the dunce crate),
/// keeping real UNC paths intact.
fn clean(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy().into_owned();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return PathBuf::from(rest);
    }
    p
}

/// Canonicalize (resolves .., symlinks, junctions) and require the result to sit under a root.
pub fn resolve(cfg: &Config, raw: &str) -> Result<PathBuf, String> {
    let p = fs::canonicalize(raw).map_err(|e| format!("{raw}: {e}"))?;
    if denied(cfg, &p.to_string_lossy()) {
        return Err("path is on the deny list".into());
    }
    for r in &cfg.roots {
        if let Ok(rc) = fs::canonicalize(r) {
            if p.starts_with(&rc) {
                return Ok(clean(p));
            }
        }
    }
    Err(format!("{raw} is outside the allowed roots"))
}

fn status(cfg: &Config, writable: bool) -> String {
    let presets: Vec<_> = cfg.presets.keys().collect();
    json!({
        "server": "xing-pc-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "mode": if writable { "write (D: only, commits need approval at the PC)" } else { "read-only" },
        "kill_switch": crate::config::killed(),
        "roots": cfg.roots,
        "presets": presets,
        "max_read_bytes": cfg.max_read_bytes,
        "memory_dir": cfg.memory_dir
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

/// UTF-16 byte order from the file's BOM (Windows PowerShell 5 `>` writes UTF-16 LE).
/// Some(true) = little endian, Some(false) = big endian, None = not UTF-16.
fn utf16_order(f: &mut fs::File) -> Option<bool> {
    let mut bom = [0u8; 2];
    f.seek(SeekFrom::Start(0)).ok()?;
    f.read_exact(&mut bom).ok()?;
    match bom {
        [0xFF, 0xFE] => Some(true),
        [0xFE, 0xFF] => Some(false),
        _ => None,
    }
}

fn read_file(cfg: &Config, args: &Value) -> Result<String, String> {
    let p = resolve(cfg, arg(args, "path")?)?;
    let mut offset = args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
    let mut f = fs::File::open(&p).map_err(|e| e.to_string())?;
    let md = f.metadata().map_err(|e| e.to_string())?;
    if !md.is_file() {
        return Err("not a file".into());
    }
    let len = md.len();
    let utf16 = utf16_order(&mut f);
    if utf16.is_some() {
        // Skip the BOM and stay on a code-unit boundary.
        offset = offset.max(2);
        offset -= offset % 2;
    }
    f.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    f.take(cfg.max_read_bytes).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let mut text = match utf16 {
        Some(le) => {
            buf.truncate(buf.len() - buf.len() % 2);
            let units: Vec<u16> = buf
                .chunks_exact(2)
                .map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) })
                .collect();
            String::from_utf16_lossy(&units)
        }
        None => {
            if buf.iter().take(8192).any(|b| *b == 0) {
                return Err("looks binary, not returned".into());
            }
            String::from_utf8_lossy(&buf).to_string()
        }
    };
    let end = offset + buf.len() as u64;
    if end < len {
        text.push_str(&format!("\n\n[truncated: read {offset}..{end} of {len} bytes; call again with offset {end}]"));
    }
    Ok(text)
}

fn run_preset(cfg: &Config, args: &Value) -> Result<String, String> {
    let name = arg(args, "name")?;
    let argv = cfg.presets.get(name).ok_or_else(|| format!("no preset `{name}`"))?;
    if argv.is_empty() || argv[0].trim().is_empty() {
        return Err(format!("preset `{name}` is empty in config.json"));
    }
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
    let (Some(mut so), Some(mut se)) = (child.stdout.take(), child.stderr.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("could not capture command output".into());
    };
    let t_out = thread::spawn(move || { let mut v = Vec::new(); let _ = (&mut so).take(MAX_CMD_OUT as u64).read_to_end(&mut v); let _ = std::io::copy(&mut so, &mut std::io::sink()); v });
    let t_err = thread::spawn(move || { let mut v = Vec::new(); let _ = (&mut se).take(MAX_CMD_OUT as u64).read_to_end(&mut v); let _ = std::io::copy(&mut se, &mut std::io::sink()); v });
    let start = Instant::now();
    let code = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s.code(),
            Ok(None) if start.elapsed() > CMD_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Fresh folder under this repo's target dir (stays on D:, never the system temp on C:).
    fn sandbox(tag: &str) -> PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("test-sandbox").join(format!("{tag}-{n}"));
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Config with one root, the real deny list, and no keystore touch.
    fn cfg(root: &Path) -> Config {
        serde_json::from_value(json!({
            "bind": "127.0.0.1:0",
            "roots": [root],
            "deny": [".ssh", ".secrets", ".env", "id_ed25519", "id_rsa", ".pem", ".key", ".pfx",
                     "credentials", ".xing-pc-mcp", "\\.git\\config", "/.git/config", "keychains"],
            "max_read_bytes": 1024,
            "presets": {"echo_empty": []},
            "memory_dir": root.join("memory")
        }))
        .unwrap()
    }

    fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

    #[test]
    fn path_inside_root_resolves() {
        let d = sandbox("inside");
        let root = d.join("root");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("a.txt"), "hi").unwrap();
        assert!(resolve(&cfg(&root), &s(&root.join("sub").join("a.txt"))).is_ok());
    }

    #[test]
    fn sibling_with_root_as_prefix_is_refused() {
        // "root2" starts with the string "root" but is not under it.
        let d = sandbox("prefix");
        let root = d.join("root");
        let evil = d.join("root2");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&evil).unwrap();
        let e = resolve(&cfg(&root), &s(&evil)).unwrap_err();
        assert!(e.contains("outside the allowed roots"), "{e}");
    }

    #[test]
    fn dotdot_escape_is_refused() {
        let d = sandbox("dotdot");
        let root = d.join("root");
        fs::create_dir_all(&root).unwrap();
        fs::write(d.join("outside.txt"), "x").unwrap();
        let raw = format!("{}{}..{}outside.txt", s(&root), std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR);
        assert!(resolve(&cfg(&root), &raw).is_err());
    }

    #[test]
    fn deny_list_blocks_secrets_inside_root() {
        let d = sandbox("deny");
        let root = d.join("root");
        fs::create_dir_all(root.join(".ssh")).unwrap();
        fs::write(root.join(".env"), "TOKEN=x").unwrap();
        fs::write(root.join("server.KEY"), "k").unwrap();
        let c = cfg(&root);
        for p in [root.join(".env"), root.join(".ssh"), root.join("server.KEY")] {
            assert_eq!(resolve(&c, &s(&p)).unwrap_err(), "path is on the deny list", "{}", p.display());
        }
    }

    #[test]
    fn list_dir_hides_denied_names() {
        let d = sandbox("hide");
        let root = d.join("root");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(".env"), "x").unwrap();
        fs::write(root.join("ok.txt"), "x").unwrap();
        let out = list_dir(&cfg(&root), &json!({"path": s(&root)})).unwrap();
        assert!(out.contains("ok.txt"));
        assert!(!out.contains(".env"));
    }

    #[cfg(windows)]
    #[test]
    fn c_drive_is_unreachable_with_d_roots() {
        let d = sandbox("cdrive");
        let c = cfg(&d);
        for raw in [r"C:\Windows", r"C:\Users", r"C:\"] {
            assert!(resolve(&c, raw).is_err(), "{raw} must be refused");
        }
    }

    #[test]
    fn missing_path_is_an_error_not_a_panic() {
        let d = sandbox("missing");
        assert!(resolve(&cfg(&d), &s(&d.join("nope.txt"))).is_err());
    }

    #[test]
    fn read_file_decodes_utf16_le_from_powershell() {
        let d = sandbox("utf16");
        let mut bytes = vec![0xFF, 0xFE];
        for u in "error[E0425]".encode_utf16() { bytes.extend(u.to_le_bytes()); }
        fs::write(d.join("log.txt"), bytes).unwrap();
        assert_eq!(read_file(&cfg(&d), &json!({"path": s(&d.join("log.txt"))})).unwrap(), "error[E0425]");
    }

    #[test]
    fn read_file_refuses_binary_and_truncates_large() {
        let d = sandbox("bin");
        fs::write(d.join("b.bin"), [1u8, 0, 2, 3]).unwrap();
        assert!(read_file(&cfg(&d), &json!({"path": s(&d.join("b.bin"))})).is_err());
        fs::write(d.join("big.txt"), "a".repeat(3000)).unwrap();
        let t = read_file(&cfg(&d), &json!({"path": s(&d.join("big.txt"))})).unwrap();
        assert!(t.contains("call again with offset 1024"), "{t}");
    }

    #[test]
    fn write_tools_refused_on_read_only_connector() {
        let d = sandbox("ro");
        let c = cfg(&d);
        for name in WRITE_TOOLS {
            let e = call(&c, name, &json!({}), false).unwrap_err();
            assert!(e.contains("needs the write connector"), "{name}: {e}");
        }
        let names: Vec<String> = list(false).as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        assert!(!names.iter().any(|n| n == "write_file" || n == "memory_add"));
        assert!(names.iter().any(|n| n == "memory_recent"));
    }

    #[test]
    fn memory_read_tools_work_on_read_only_connector() {
        let d = sandbox("memro");
        let c = cfg(&d);
        assert!(call(&c, "memory_status", &json!({}), false).is_ok());
        assert!(call(&c, "memory_verify", &json!({}), false).is_ok());
        assert_eq!(call(&c, "memory_recent", &json!({}), false).unwrap(), "[]");
    }

    #[test]
    fn presets_by_name_only() {
        let d = sandbox("preset");
        let c = cfg(&d);
        assert!(run_preset(&c, &json!({"name": "rm -rf"})).unwrap_err().contains("no preset"));
        assert!(run_preset(&c, &json!({"name": "echo_empty"})).unwrap_err().contains("is empty"));
        let outside = sandbox("preset-out");
        assert!(run_preset(&c, &json!({"name": "echo_empty", "cwd": s(&outside)})).is_err());
    }

    #[test]
    fn unknown_tool_is_refused() {
        let d = sandbox("unk");
        assert!(call(&cfg(&d), "shell", &json!({}), true).unwrap_err().contains("unknown tool"));
    }
}
