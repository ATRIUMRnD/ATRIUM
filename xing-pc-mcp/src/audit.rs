//! Tamper-evident audit log, the same hash-chain pattern as QFLTL ward's timeline
//! (blake3 over each entry, every entry carrying the previous hash, GENESIS at the start).
//! Editing, deleting or reordering any line breaks `xing-pc-mcp verify-log`.
//!
//! Several processes append (the server, plus `kill` / `unkill` from the CLI), so the head is
//! never cached: each append opens the file exclusively (share mode 0 on Windows), reads the
//! last entry, and writes the next one while still holding the lock.
//!
//! `record_to` / `verify_at` are the same chain on any path; memory.rs reuses them for the
//! unified memory store, with an optional signature over each entry's hash (`sig`, which is
//! excluded from the hash like `hash` itself).

use crate::config;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const GENESIS: &str = "GENESIS";
const TAIL_BYTES: u64 = 16 * 1024;

static LOCAL: Mutex<()> = Mutex::new(());
/// Entries lost because the file stayed locked; the next entry that lands records the gap.
static DROPPED: AtomicU64 = AtomicU64::new(0);

/// Signs an entry hash; returns the signature text to store under `sig`.
pub type Sign<'a> = &'a dyn Fn(&str) -> String;
/// Checks one parsed entry (hash already verified). Err = the first problem found.
pub type Check<'a> = &'a dyn Fn(&Value) -> Result<(), String>;

pub fn file() -> PathBuf {
    config::dir().join("audit.chain.jsonl")
}

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

/// Hash of an entry is blake3 over its JSON with `hash` and `sig` removed.
/// serde_json's default map is sorted, so the serialization is deterministic.
pub fn hash_of(entry: &Value) -> String {
    let mut body = entry.clone();
    if let Some(m) = body.as_object_mut() {
        m.remove("hash");
        m.remove("sig");
    }
    blake3::hash(body.to_string().as_bytes()).to_hex().to_string()
}

fn open_exclusive(path: &Path) -> Option<File> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut o = OpenOptions::new();
        o.read(true).write(true).create(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            o.share_mode(0);
        }
        match o.open(path) {
            Ok(f) => return Some(f),
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(15)),
            Err(e) => {
                eprintln!("xing-pc-mcp: chain file locked too long ({}): {e}", path.display());
                return None;
            }
        }
    }
}

/// (seq, hash) of the last entry, read from the tail of the open file.
/// The tail window doubles until it holds one complete line, so a long entry can never
/// reset the chain to GENESIS. Only a truly empty file starts at GENESIS.
fn head(f: &mut File) -> (u64, String) {
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    if len == 0 {
        return (0, GENESIS.to_string());
    }
    let mut window = TAIL_BYTES;
    loop {
        let start = len.saturating_sub(window);
        let mut buf = Vec::new();
        if f.seek(SeekFrom::Start(start)).is_ok() {
            let _ = f.read_to_end(&mut buf);
        }
        let tail = String::from_utf8_lossy(&buf);
        let mut lines: Vec<&str> = tail.lines().filter(|l| !l.trim().is_empty()).collect();
        if start > 0 && !lines.is_empty() {
            lines.remove(0); // first line may be cut mid-entry
        }
        if let Some(v) = lines.iter().rev().find_map(|l| serde_json::from_str::<Value>(l).ok()) {
            return (v["seq"].as_u64().unwrap_or(0), v["hash"].as_str().unwrap_or(GENESIS).to_string());
        }
        if start == 0 {
            return (0, GENESIS.to_string());
        }
        window = window.saturating_mul(2);
    }
}

/// Append to the server's own audit chain.
pub fn record(fields: Value) {
    let _ = record_to(&file(), fields);
}

/// Append one chained entry to `path` (unsigned).
pub fn record_to(path: &Path, fields: Value) -> Option<Value> {
    record_to_with(path, fields, None)
}

/// Append one chained entry to `path`, signing its hash when `sign` is given. Returns the
/// entry as written (seq, ts, prev, hash, sig plus `fields`), None if the file stayed locked.
pub fn record_to_with(path: &Path, fields: Value, sign: Option<Sign>) -> Option<Value> {
    let _g = LOCAL.lock().unwrap_or_else(|p| p.into_inner());
    let Some(mut f) = open_exclusive(path) else {
        DROPPED.fetch_add(1, Ordering::Relaxed);
        return None;
    };
    let (seq, prev) = head(&mut f);
    let mut entry = json!({"seq": seq + 1, "ts": now_ms(), "prev": prev});
    let lost = DROPPED.swap(0, Ordering::Relaxed);
    if lost > 0 {
        entry["dropped_before"] = json!(lost);
    }
    if let (Some(e), Some(extra)) = (entry.as_object_mut(), fields.as_object()) {
        for (k, v) in extra {
            e.insert(k.clone(), v.clone());
        }
    }
    let h = hash_of(&entry);
    if let Some(s) = sign {
        entry["sig"] = Value::String(s(&h));
    }
    entry["hash"] = Value::String(h);
    if f.seek(SeekFrom::End(0)).is_ok() && writeln!(f, "{entry}").is_ok() {
        let _ = f.flush();
        return Some(entry);
    }
    None
}

/// Walk the server's audit chain. Ok(n) = n intact entries; Err names the first broken line.
pub fn verify() -> Result<u64, String> {
    verify_at(&file())
}

/// Walk any chain file. A missing file is an intact chain of zero entries.
pub fn verify_at(path: &Path) -> Result<u64, String> {
    verify_at_with(path, None)
}

/// Walk any chain file, running `check` on every hash-valid entry (signatures, schema).
pub fn verify_at_with(path: &Path, check: Option<Check>) -> Result<u64, String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    let mut prev = GENESIS.to_string();
    let mut n = 0u64;
    for (i, line) in text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
        let v: Value = serde_json::from_str(line).map_err(|e| format!("line {}: not JSON ({e})", i + 1))?;
        if v["prev"].as_str() != Some(prev.as_str()) {
            return Err(format!("line {}: chain broken (prev does not match the line before)", i + 1));
        }
        if v["seq"].as_u64() != Some(n + 1) {
            return Err(format!("line {}: sequence gap", i + 1));
        }
        let stated = v["hash"].as_str().unwrap_or("");
        if stated != hash_of(&v) {
            return Err(format!("line {}: content altered (hash mismatch)", i + 1));
        }
        if let Some(c) = check {
            c(&v).map_err(|e| format!("line {}: {e}", i + 1))?;
        }
        prev = stated.to_string();
        n += 1;
    }
    Ok(n)
}
