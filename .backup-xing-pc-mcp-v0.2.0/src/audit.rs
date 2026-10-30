//! Tamper-evident audit log, the same hash-chain pattern as QFLTL ward's timeline
//! (blake3 over each entry, every entry carrying the previous hash, GENESIS at the start).
//! Editing, deleting or reordering any line breaks `xing-pc-mcp verify-log`.
//!
//! Several processes append (the server, plus `kill` / `unkill` from the CLI), so the head is
//! never cached: each append opens the file exclusively (share mode 0 on Windows), reads the
//! last entry, and writes the next one while still holding the lock.

use crate::config;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::Mutex,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const GENESIS: &str = "GENESIS";
const TAIL_BYTES: u64 = 16 * 1024;

static LOCAL: Mutex<()> = Mutex::new(());

pub fn file() -> PathBuf {
    config::dir().join("audit.chain.jsonl")
}

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

/// Hash of an entry is blake3 over its JSON with the `hash` field removed.
/// serde_json's default map is sorted, so the serialization is deterministic.
fn hash_of(entry: &Value) -> String {
    let mut body = entry.clone();
    if let Some(m) = body.as_object_mut() {
        m.remove("hash");
    }
    blake3::hash(body.to_string().as_bytes()).to_hex().to_string()
}

fn open_exclusive() -> Option<File> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let mut o = OpenOptions::new();
        o.read(true).write(true).create(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            o.share_mode(0);
        }
        match o.open(file()) {
            Ok(f) => return Some(f),
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(15)),
            Err(e) => {
                eprintln!("xing-pc-mcp: audit log locked too long: {e}");
                return None;
            }
        }
    }
}

/// (seq, hash) of the last entry, read from the tail of the open file.
fn head(f: &mut File) -> (u64, String) {
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(TAIL_BYTES);
    let mut tail = String::new();
    if f.seek(SeekFrom::Start(start)).is_ok() {
        let _ = f.read_to_string(&mut tail);
    }
    tail.lines()
        .rev()
        .filter(|l| !l.trim().is_empty())
        .find_map(|l| serde_json::from_str::<Value>(l).ok())
        .map(|v| (v["seq"].as_u64().unwrap_or(0), v["hash"].as_str().unwrap_or(GENESIS).to_string()))
        .unwrap_or((0, GENESIS.to_string()))
}

pub fn record(fields: Value) {
    let _g = LOCAL.lock().unwrap_or_else(|p| p.into_inner());
    let Some(mut f) = open_exclusive() else { return };
    let (seq, prev) = head(&mut f);
    let mut entry = json!({"seq": seq + 1, "ts": now_ms(), "prev": prev});
    if let (Some(e), Some(extra)) = (entry.as_object_mut(), fields.as_object()) {
        for (k, v) in extra {
            e.insert(k.clone(), v.clone());
        }
    }
    entry["hash"] = Value::String(hash_of(&entry));
    if f.seek(SeekFrom::End(0)).is_ok() {
        let _ = writeln!(f, "{entry}");
        let _ = f.flush();
    }
}

/// Walk the whole chain. Ok(n) = n intact entries; Err names the first broken line.
pub fn verify() -> Result<u64, String> {
    let text = match fs::read_to_string(file()) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("cannot read {}: {e}", file().display())),
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
        prev = stated.to_string();
        n += 1;
    }
    Ok(n)
}
