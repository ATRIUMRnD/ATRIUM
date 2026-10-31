//! Unified memory (v0.5.0): one append-only, hash-chained store on xing-pc that every surface
//! (claude.ai chats, Claude Code, the Mac, voice) reads and writes through this server.
//!
//! Same chain as audit.rs (blake3 over each entry, prev hash, GENESIS at the start), so
//! `memory_verify` catches any edit, deletion or reorder. On Windows every entry is also
//! ML-DSA-65 signed (sign.rs), so a chain rebuilt from scratch by someone with the file but
//! not the DPAPI-wrapped key fails verification too. Nothing is ever rewritten:
//! `memory_forget` appends a tombstone and `supersedes` points a new entry at the old one.
//! Readers see only the live set (not tombstoned, not superseded).
//!
//! The file lives under cfg.memory_dir (default D:\ATRIUM\private\memory, git-ignored),
//! so it never leaves the machine except through the token-gated connector.

use crate::{audit, config::Config, sign, tools};
use serde_json::{json, Value};
use std::{collections::HashSet, fs, path::PathBuf};

const MAX_TEXT: usize = 8 * 1024;
const MAX_TAGS: usize = 16;
const MAX_LIMIT: usize = 100;
const KINDS: [&str; 5] = ["fact", "decision", "preference", "status", "note"];

pub fn file(cfg: &Config) -> PathBuf {
    cfg.memory_dir.join("memory.jsonl")
}

fn load(cfg: &Config) -> Result<Vec<Value>, String> {
    let text = match fs::read_to_string(file(cfg)) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("read memory: {e}")),
    };
    Ok(text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect())
}

/// Entries still in force: not a tombstone, not tombstoned, not superseded.
fn live(all: &[Value]) -> Vec<&Value> {
    let mut dead = HashSet::new();
    for e in all {
        if let Some(s) = e["tombstone"].as_u64() {
            dead.insert(s);
        }
        if let Some(s) = e["supersedes"].as_u64() {
            dead.insert(s);
        }
    }
    all.iter()
        .filter(|e| e["kind"].as_str() != Some("forget"))
        .filter(|e| !dead.contains(&e["seq"].as_u64().unwrap_or(0)))
        .collect()
}

fn limit(args: &Value) -> usize {
    args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(20).clamp(1, MAX_LIMIT)
}

fn topic_filter(args: &Value) -> Option<String> {
    args.get("topic").and_then(|v| v.as_str()).map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty())
}

fn topic_matches(e: &Value, want: &Option<String>) -> bool {
    match want {
        None => true,
        Some(t) => e["topic"].as_str().map(|x| x.to_lowercase() == *t).unwrap_or(false),
    }
}

/// What callers see: chain fields (prev/hash/sig) stay internal.
fn public(e: &Value) -> Value {
    let mut out = json!({
        "seq": e["seq"], "ts": e["ts"], "source": e["source"], "kind": e["kind"],
        "topic": e["topic"], "text": e["text"], "tags": e["tags"]
    });
    if let Some(s) = e.get("supersedes") {
        out["supersedes"] = s.clone();
    }
    if e.get("sig").is_some() {
        out["signed"] = json!(true);
    }
    out
}

/// Append `fields` to the chain, signed when this machine has a signer.
fn append(cfg: &Config, mut fields: Value) -> Result<Value, String> {
    fs::create_dir_all(&cfg.memory_dir).map_err(|e| format!("memory dir: {e}"))?;
    let path = file(cfg);
    let written = match sign::signer() {
        Some(s) => {
            fields["kid"] = json!(s.kid);
            fields["scheme"] = json!(sign::SCHEME);
            let f = |h: &str| s.sign(h);
            audit::record_to_with(&path, fields, Some(&f))
        }
        None => audit::record_to(&path, fields),
    };
    written.ok_or_else(|| "memory file is locked; try again".to_string())
}

pub fn status(cfg: &Config) -> Result<String, String> {
    let all = load(cfg)?;
    let live_n = live(&all).len();
    let mut topics: Vec<String> = live(&all)
        .iter()
        .filter_map(|e| e["topic"].as_str().map(String::from))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    topics.sort();
    let signing = match sign::signer() {
        Some(s) => json!({"scheme": sign::SCHEME, "kid": s.kid}),
        None => json!("off"),
    };
    Ok(json!({
        "file": file(cfg).display().to_string(),
        "entries": all.len(),
        "live": live_n,
        "topics": topics,
        "signing": signing
    })
    .to_string())
}

pub fn recent(cfg: &Config, args: &Value) -> Result<String, String> {
    let all = load(cfg)?;
    let want = topic_filter(args);
    let n = limit(args);
    let out: Vec<Value> = live(&all).into_iter().rev().filter(|e| topic_matches(e, &want)).take(n).map(public).collect();
    Ok(json!(out).to_string())
}

pub fn search(cfg: &Config, args: &Value) -> Result<String, String> {
    let q = tools::arg(args, "query")?.trim().to_lowercase();
    if q.is_empty() {
        return Err("empty query".into());
    }
    let all = load(cfg)?;
    let want = topic_filter(args);
    let n = limit(args);
    let hit = |e: &Value| -> bool {
        let text = e["text"].as_str().unwrap_or("").to_lowercase();
        let topic = e["topic"].as_str().unwrap_or("").to_lowercase();
        let tags = e["tags"].as_array().map(|a| a.iter().filter_map(|t| t.as_str()).any(|t| t.to_lowercase().contains(&q))).unwrap_or(false);
        text.contains(&q) || topic.contains(&q) || tags
    };
    let out: Vec<Value> = live(&all).into_iter().rev().filter(|e| topic_matches(e, &want) && hit(e)).take(n).map(public).collect();
    Ok(json!(out).to_string())
}

fn source(args: &Value) -> String {
    args.get("source").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or("unknown").chars().take(32).collect()
}

pub fn add(cfg: &Config, args: &Value) -> Result<String, String> {
    let text = tools::arg(args, "text")?.trim();
    if text.is_empty() {
        return Err("empty text".into());
    }
    if text.len() > MAX_TEXT {
        return Err(format!("text too long ({} bytes, max {MAX_TEXT})", text.len()));
    }
    let topic = tools::arg(args, "topic")?.trim().to_lowercase();
    if topic.is_empty() || topic.len() > 64 {
        return Err("topic must be 1..64 chars".into());
    }
    let kind = args.get("kind").and_then(|v| v.as_str()).unwrap_or("note");
    if !KINDS.contains(&kind) {
        return Err(format!("kind must be one of {}", KINDS.join(", ")));
    }
    let tags: Vec<String> = args
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|t| t.as_str()).map(|t| t.trim().to_lowercase()).filter(|t| !t.is_empty()).collect())
        .unwrap_or_default();
    if tags.len() > MAX_TAGS {
        return Err(format!("at most {MAX_TAGS} tags"));
    }
    let all = load(cfg)?;
    let supersedes = args.get("supersedes").and_then(|v| v.as_u64());
    if let Some(s) = supersedes {
        if !live(&all).iter().any(|e| e["seq"].as_u64() == Some(s)) {
            return Err(format!("supersedes {s}: no live entry with that seq"));
        }
    }
    let mut fields = json!({"source": source(args), "kind": kind, "topic": topic, "text": text, "tags": tags});
    if let Some(s) = supersedes {
        fields["supersedes"] = json!(s);
    }
    let entry = append(cfg, fields)?;
    Ok(public(&entry).to_string())
}

pub fn forget(cfg: &Config, args: &Value) -> Result<String, String> {
    let seq = args.get("seq").and_then(|v| v.as_u64()).ok_or("missing `seq`")?;
    let all = load(cfg)?;
    if !live(&all).iter().any(|e| e["seq"].as_u64() == Some(seq)) {
        return Err(format!("no live entry with seq {seq}"));
    }
    let reason: String = args.get("reason").and_then(|v| v.as_str()).unwrap_or("").chars().take(256).collect();
    let fields = json!({"source": source(args), "kind": "forget", "tombstone": seq, "reason": reason});
    let entry = append(cfg, fields)?;
    Ok(json!({"forgot": seq, "seq": entry["seq"]}).to_string())
}

/// Chain check plus signature check. An entry that claims a kid must carry a signature
/// that verifies under this machine's key; unsigned entries (pre-signing, or the Mac) pass
/// the hash check only and are counted separately.
pub fn verify(cfg: &Config) -> Result<String, String> {
    let signer = sign::signer();
    let signed = std::cell::Cell::new(0u64);
    let check = |e: &Value| -> Result<(), String> {
        let (kid, sig) = (e["kid"].as_str(), e["sig"].as_str());
        match (kid, sig) {
            (None, None) => Ok(()),
            (Some(_), None) | (None, Some(_)) => Err("entry has kid or sig but not both".into()),
            (Some(k), Some(s)) => {
                let Some(me) = signer else { return Err(format!("signed by kid {k} but this machine has no signing key")) };
                if k != me.kid {
                    return Err(format!("signed by unknown kid {k} (this machine is {})", me.kid));
                }
                if !me.verify(e["hash"].as_str().unwrap_or(""), s) {
                    return Err("signature does not verify".into());
                }
                signed.set(signed.get() + 1);
                Ok(())
            }
        }
    };
    let n = audit::verify_at_with(&file(cfg), Some(&check))?;
    let mut out = json!({"intact": true, "entries": n, "signed": signed.get(), "file": file(cfg).display().to_string()});
    if let Some(s) = signer {
        out["kid"] = json!(s.kid);
        out["scheme"] = json!(sign::SCHEME);
    }
    Ok(out.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn sandbox(tag: &str) -> PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("test-sandbox").join(format!("mem-{tag}-{n}"));
        fs::create_dir_all(&d).unwrap();
        d
    }

    /// Signing is off under cargo test (sign.rs), so no key material lands in the real config dir.
    fn cfg(dir: &Path) -> Config {
        serde_json::from_value(json!({
            "bind": "127.0.0.1:0", "roots": [dir], "deny": [], "max_read_bytes": 1024, "presets": {},
            "memory_dir": dir.join("memory")
        }))
        .unwrap()
    }

    fn seq_of(s: &str) -> u64 {
        serde_json::from_str::<Value>(s).unwrap()["seq"].as_u64().unwrap()
    }

    #[test]
    fn add_then_recent_and_search() {
        let c = cfg(&sandbox("basic"));
        add(&c, &json!({"text": "brain uses qwen3-8b in LM Studio", "topic": "Brain", "kind": "decision", "tags": ["LLM"], "source": "chat"})).unwrap();
        add(&c, &json!({"text": "video-watch nightly passes", "topic": "video-watch", "kind": "status"})).unwrap();
        let r: Vec<Value> = serde_json::from_str(&recent(&c, &json!({})).unwrap()).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0]["topic"], "video-watch"); // newest first
        let s: Vec<Value> = serde_json::from_str(&search(&c, &json!({"query": "llm"})).unwrap()).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0]["topic"], "brain"); // topics are lowercased
        let t: Vec<Value> = serde_json::from_str(&recent(&c, &json!({"topic": "BRAIN"})).unwrap()).unwrap();
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn supersede_hides_old_entry() {
        let c = cfg(&sandbox("supersede"));
        let a = seq_of(&add(&c, &json!({"text": "default model is gemma", "topic": "brain"})).unwrap());
        add(&c, &json!({"text": "default model is qwen3-8b", "topic": "brain", "supersedes": a})).unwrap();
        let r: Vec<Value> = serde_json::from_str(&recent(&c, &json!({})).unwrap()).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["text"], "default model is qwen3-8b");
        assert!(add(&c, &json!({"text": "x", "topic": "brain", "supersedes": a})).is_err(), "superseded seq is no longer live");
    }

    #[test]
    fn forget_is_a_tombstone_and_chain_stays_intact() {
        let c = cfg(&sandbox("forget"));
        let a = seq_of(&add(&c, &json!({"text": "to be forgotten", "topic": "t"})).unwrap());
        forget(&c, &json!({"seq": a, "reason": "test"})).unwrap();
        let r: Vec<Value> = serde_json::from_str(&recent(&c, &json!({})).unwrap()).unwrap();
        assert!(r.is_empty());
        assert!(forget(&c, &json!({"seq": a})).is_err());
        let v: Value = serde_json::from_str(&verify(&c).unwrap()).unwrap();
        assert_eq!(v["entries"], 2); // the fact plus its tombstone, both on the chain
        let st: Value = serde_json::from_str(&status(&c).unwrap()).unwrap();
        assert_eq!(st["live"], 0);
        assert_eq!(st["entries"], 2);
        assert_eq!(st["signing"], "off");
    }

    #[test]
    fn tampering_breaks_verify() {
        let c = cfg(&sandbox("tamper"));
        add(&c, &json!({"text": "one", "topic": "t"})).unwrap();
        add(&c, &json!({"text": "two", "topic": "t"})).unwrap();
        let f = file(&c);
        let edited = fs::read_to_string(&f).unwrap().replace("\"one\"", "\"uno\"");
        fs::write(&f, edited).unwrap();
        assert!(verify(&c).is_err());
    }

    #[test]
    fn entry_claiming_a_kid_without_a_key_is_refused() {
        let c = cfg(&sandbox("kid"));
        // Hand-build a hash-valid entry that claims a signature: the hash check passes,
        // the signature check must not.
        let mut e = json!({"seq": 1, "ts": 0, "prev": "GENESIS", "kind": "note", "topic": "t", "text": "x", "tags": [], "kid": "deadbeefdeadbeef"});
        e["sig"] = json!("00");
        e["hash"] = json!(audit::hash_of(&e));
        fs::create_dir_all(&c.memory_dir).unwrap();
        fs::write(file(&c), format!("{e}\n")).unwrap();
        let err = verify(&c).unwrap_err();
        assert!(err.contains("signed by kid"), "{err}");
    }

    #[test]
    fn validation() {
        let c = cfg(&sandbox("valid"));
        assert!(add(&c, &json!({"text": "", "topic": "t"})).is_err());
        assert!(add(&c, &json!({"text": "x", "topic": ""})).is_err());
        assert!(add(&c, &json!({"text": "x", "topic": "t", "kind": "rumor"})).is_err());
        assert!(add(&c, &json!({"text": "a".repeat(MAX_TEXT + 1), "topic": "t"})).is_err());
        assert!(search(&c, &json!({"query": "  "})).is_err());
        assert!(recent(&c, &json!({})).unwrap().starts_with('['));
    }
}
