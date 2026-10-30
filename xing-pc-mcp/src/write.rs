//! Write gate (v0.3.0). Only reachable with the separate write token.
//! Locks:
//! - writes land only on the D: drive (Windows) or under the home folder (macOS), under the
//!   allowed roots, never in a .git folder, never on the deny list
//! - commits are only *prepared* remotely; a person types `yes` at the machine (`xing-pc-mcp approve`)
//! - an approval id is consumed on use, so it can't be replayed
//! - commit messages with attribution / co-author lines or @-mentions are refused
//! - no push, ever
//! - the kill switch blocks all of it (checked in rpc before any tool runs)

use crate::{audit, config::{self, Config}, tools};
use serde_json::{json, Value};
use std::{
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const MAX_WRITE: usize = 768 * 1024;
const MAX_PATHS: usize = 200;

/// The volume writes are locked to: D: on Windows, the user's home on macOS.
#[cfg(windows)]
fn on_d(p: &Path) -> bool {
    let s = p.to_string_lossy().to_lowercase();
    s.starts_with("d:\\") || s.starts_with("\\\\?\\d:\\")
}

#[cfg(not(windows))]
fn on_d(p: &Path) -> bool {
    match std::env::var_os("HOME") {
        Some(h) if !h.is_empty() => {
            let home = PathBuf::from(h);
            let home = fs::canonicalize(&home).unwrap_or(home);
            p.starts_with(&home) || fs::canonicalize(p).map(|c| c.starts_with(&home)).unwrap_or(false)
        }
        _ => false,
    }
}

fn in_git_dir(p: &Path) -> bool {
    p.components().any(|c| c.as_os_str().eq_ignore_ascii_case(".git"))
}

/// Resolve a target that may not exist yet: canonicalize its parent, then re-check every lock.
fn resolve_target(cfg: &Config, raw: &str, create_dirs: bool) -> Result<PathBuf, String> {
    let raw_p = PathBuf::from(raw);
    let name = raw_p.file_name().ok_or("path has no file name")?.to_owned();
    let parent = raw_p.parent().ok_or("path has no parent folder")?;
    if create_dirs && !parent.exists() {
        // Refuse before creating anything if the requested parent is clearly off-limits.
        if !on_d(parent) || in_git_dir(parent) || tools::denied(cfg, &parent.to_string_lossy()) {
            return Err("refused: writes only go to the allowed volume, outside .git and the deny list".into());
        }
        let root_ok = cfg.roots.iter().any(|r| parent.starts_with(r));
        if !root_ok {
            return Err(format!("{raw} is outside the allowed roots"));
        }
        fs::create_dir_all(parent).map_err(|e| format!("create folders: {e}"))?;
    }
    let dir = tools::resolve(cfg, &parent.to_string_lossy())?;
    let full = dir.join(name);
    if !on_d(&full) {
        return Err("refused: writes are locked to the allowed volume".into());
    }
    if in_git_dir(&full) {
        return Err("refused: never write inside a .git folder".into());
    }
    if tools::denied(cfg, &full.to_string_lossy()) {
        return Err("path is on the deny list".into());
    }
    if full.is_dir() {
        return Err("target is a folder".into());
    }
    Ok(full)
}

pub fn write_file(cfg: &Config, args: &Value) -> Result<String, String> {
    let path = tools::arg(args, "path")?;
    let content = tools::arg(args, "content")?;
    let create_dirs = args.get("create_dirs").and_then(|v| v.as_bool()).unwrap_or(false);
    if content.len() > MAX_WRITE {
        return Err(format!("content too large ({} bytes, max {MAX_WRITE})", content.len()));
    }
    let full = resolve_target(cfg, path, create_dirs)?;
    let existed = full.exists();
    // Atomic: write a temp file next to it, then rename over the target.
    let tmp = full.with_extension(format!(
        "{}.xpm-tmp",
        full.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default()
    ));
    fs::write(&tmp, content.as_bytes()).map_err(|e| format!("write: {e}"))?;
    if let Err(e) = fs::rename(&tmp, &full) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("replace: {e}"));
    }
    let hash = blake3::hash(content.as_bytes()).to_hex().to_string();
    Ok(json!({
        "written": full.display().to_string(),
        "bytes": content.len(),
        "blake3": hash,
        "created": !existed
    })
    .to_string())
}

fn pending_dir() -> PathBuf {
    config::dir().join("pending")
}

fn bad_message(msg: &str) -> Option<&'static str> {
    let low = msg.to_lowercase();
    if msg.trim().is_empty() {
        return Some("empty commit message");
    }
    if low.contains("co-authored-by") || low.contains("generated with") || low.contains("claude-session") {
        return Some("attribution / co-author lines are not allowed");
    }
    if msg.contains('@') {
        return Some("@-mentions are not allowed in commit messages");
    }
    None
}

fn new_id() -> Result<String, String> {
    let mut b = [0u8; 6];
    getrandom::getrandom(&mut b).map_err(|e| format!("rng: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

pub fn prepare_commit(cfg: &Config, args: &Value) -> Result<String, String> {
    let repo = tools::resolve(cfg, tools::arg(args, "repo")?)?;
    if !on_d(&repo) {
        return Err("refused: commits are locked to repos on the allowed volume".into());
    }
    let message = tools::arg(args, "message")?;
    if let Some(why) = bad_message(message) {
        return Err(format!("refused: {why}"));
    }
    let paths: Vec<String> = args
        .get("paths")
        .and_then(|v| v.as_array())
        .ok_or("missing `paths` (list of files relative to the repo)")?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    if paths.is_empty() || paths.len() > MAX_PATHS {
        return Err(format!("`paths` must list 1..{MAX_PATHS} files"));
    }
    for p in &paths {
        if p.contains("..") || Path::new(p).is_absolute() || in_git_dir(Path::new(p)) {
            return Err(format!("bad path `{p}` (relative, no .., not in .git)"));
        }
        if tools::denied(cfg, p) {
            return Err(format!("`{p}` is on the deny list"));
        }
    }
    let id = new_id()?;
    let entry = json!({"id": id, "repo": repo.display().to_string(), "message": message, "paths": paths});
    fs::create_dir_all(pending_dir()).map_err(|e| e.to_string())?;
    fs::write(pending_dir().join(format!("{id}.json")), entry.to_string()).map_err(|e| e.to_string())?;
    audit::record(json!({"event": "commit_prepared", "id": id, "repo": entry["repo"], "files": entry["paths"].as_array().map(|a| a.len())}));
    Ok(json!({
        "pending": id,
        "next": "Xing approves at the machine: xing-pc-mcp approve",
        "pushed": false
    })
    .to_string())
}

pub fn list_pending() -> Vec<Value> {
    let Ok(rd) = fs::read_dir(pending_dir()) else { return Vec::new() };
    let mut v: Vec<Value> = rd
        .flatten()
        .filter_map(|e| fs::read_to_string(e.path()).ok())
        .filter_map(|t| serde_json::from_str(&t).ok())
        .collect();
    v.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    v
}

fn git(repo: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("git: {e}"))?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if out.status.success() { Ok(text) } else { Err(text) }
}

/// CLI only, run by a person at the machine. Shows each pending commit and asks yes/no.
/// "no" (or anything but yes) leaves it pending; "drop" deletes it.
pub fn approve_cli() -> anyhow::Result<()> {
    if config::killed() {
        println!("kill switch is on; nothing can be approved (xing-pc-mcp unkill)");
        return Ok(());
    }
    let pend = list_pending();
    if pend.is_empty() {
        println!("nothing pending");
        return Ok(());
    }
    let stdin = io::stdin();
    for p in pend {
        let id = p["id"].as_str().unwrap_or("").to_string();
        let repo = p["repo"].as_str().unwrap_or("").to_string();
        let msg = p["message"].as_str().unwrap_or("").to_string();
        let paths: Vec<String> = p["paths"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        println!("\n[{id}] {repo}\n  message: {msg}\n  files:");
        for f in &paths {
            println!("    {f}");
        }
        if let Ok(d) = git(&repo, &["diff", "--stat", "--"]) {
            if !d.trim().is_empty() { println!("  diff --stat:\n{d}"); }
        }
        print!("commit this? (yes / no / drop): ");
        let _ = io::stdout().flush();
        let mut ans = String::new();
        stdin.lock().read_line(&mut ans)?;
        let file = pending_dir().join(format!("{id}.json"));
        match ans.trim() {
            "yes" => {
                // Consume the id first so it can never run twice.
                if fs::remove_file(&file).is_err() {
                    println!("  already handled elsewhere, skipped");
                    continue;
                }
                if let Some(why) = bad_message(&msg) {
                    println!("  refused: {why}");
                    continue;
                }
                let mut add = vec!["add", "--"];
                add.extend(paths.iter().map(|s| s.as_str()));
                let res = git(&repo, &add).and_then(|_| {
                    let mut c = vec!["commit", "-m", msg.as_str(), "--"];
                    c.extend(paths.iter().map(|s| s.as_str()));
                    git(&repo, &c)
                });
                match res {
                    Ok(o) => {
                        audit::record(json!({"event": "commit_approved", "id": id, "repo": repo}));
                        println!("  committed (not pushed):\n{o}");
                    }
                    Err(e) => {
                        audit::record(json!({"event": "commit_failed", "id": id, "repo": repo}));
                        println!("  git refused, nothing committed:\n{e}");
                    }
                }
            }
            "drop" => {
                let _ = fs::remove_file(&file);
                audit::record(json!({"event": "commit_dropped", "id": id}));
                println!("  dropped");
            }
            _ => println!("  left pending"),
        }
    }
    Ok(())
}
