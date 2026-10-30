use crate::{audit, config::{self, Config}, tools};
use serde_json::{json, Value};
use std::{
    io::{Cursor, Read},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tiny_http::{Header, Method, Request, Response, Server};

const MAX_BODY: u64 = 1024 * 1024;
const DEFAULT_PROTOCOL: &str = "2025-06-18";
const WORKERS: usize = 8;
/// Audit at most one unauthorized/bad-origin hit per window; the rest are counted.
const NOISE_WINDOW_MS: u64 = 60_000;
/// Logged tool args are capped so one huge call can't outgrow the audit tail read.
const MAX_LOGGED_ARGS: usize = 2048;

static NOISE_LAST: AtomicU64 = AtomicU64::new(0);
static NOISE_SUPPRESSED: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Rate-limited audit for internet noise (scanners hitting the Funnel URL).
fn record_noise(mut fields: Value) {
    let now = now_ms();
    let last = NOISE_LAST.load(Ordering::Relaxed);
    if now.saturating_sub(last) < NOISE_WINDOW_MS
        || NOISE_LAST.compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed).is_err()
    {
        NOISE_SUPPRESSED.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let n = NOISE_SUPPRESSED.swap(0, Ordering::Relaxed);
    if n > 0 {
        fields["suppressed_since_last"] = json!(n);
    }
    audit::record(fields);
}

fn capped_args(args: &Value) -> Value {
    let s = args.to_string();
    if s.len() <= MAX_LOGGED_ARGS {
        return args.clone();
    }
    let cut: String = s.chars().take(MAX_LOGGED_ARGS).collect();
    json!({"truncated": true, "bytes": s.len(), "head": cut})
}

type Resp = Response<Cursor<Vec<u8>>>;

/// Streamable HTTP transport, JSON-response mode: one POST in, one JSON body out.
/// Same dispatch shape as video-watch's stdio loop.
pub fn serve(cfg: Arc<Config>) -> anyhow::Result<()> {
    let server = Server::http(&cfg.bind).map_err(|e| anyhow::anyhow!("bind {}: {e}", cfg.bind))?;
    eprintln!(
        "xing-pc-mcp on http://{}/mcp (writes {}), config {}",
        cfg.bind,
        if cfg.write_secret.is_some() { "ON, D: only" } else { "off" },
        config::dir().join("config.json").display()
    );
    // Fixed worker pool: a flood over Funnel can queue, but never spawn unbounded threads.
    let server = Arc::new(server);
    let workers: Vec<_> = (0..WORKERS)
        .map(|_| {
            let (server, cfg) = (server.clone(), cfg.clone());
            std::thread::spawn(move || loop {
                let mut req = match server.recv() {
                    Ok(r) => r,
                    Err(_) => break,
                };
                // A bug in one request must not kill the worker: answer 500 and keep serving.
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle(&cfg, &mut req)))
                    .unwrap_or_else(|_| {
                        audit::record(json!({"event": "panic"}));
                        empty(500)
                    });
                let _ = req.respond(r);
            })
        })
        .collect();
    for w in workers {
        let _ = w.join();
    }
    Ok(())
}

fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn header<'a>(req: &'a Request, name: &'static str) -> Option<&'a str> {
    req.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str())
}

fn json_resp(code: u16, body: Value) -> Resp {
    let r = Response::from_data(body.to_string().into_bytes()).with_status_code(code);
    match Header::from_bytes("Content-Type", "application/json") {
        Ok(h) => r.with_header(h),
        Err(_) => r,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Access {
    Read,
    Write,
}

fn empty(code: u16) -> Resp {
    Response::from_data(Vec::new()).with_status_code(code)
}

/// Read token -> read tools. Write token (only when enabled) -> read + write tools.
fn authorized(cfg: &Config, req: &Request) -> Option<Access> {
    let path = req.url().split('?').next().unwrap_or("");
    let check = |t: &str| -> Option<Access> {
        if let Some(w) = cfg.write_secret.as_deref() {
            if ct_eq(t, w) {
                return Some(Access::Write);
            }
        }
        ct_eq(t, &cfg.secret).then_some(Access::Read)
    };
    if let Some(t) = path.strip_prefix("/mcp/") {
        return check(t);
    }
    if path == "/mcp" {
        if let Some(b) = header(req, "Authorization").and_then(|h| h.strip_prefix("Bearer ")) {
            return check(b.trim());
        }
    }
    None
}

/// Never put file contents in the audit log: keep size + hash instead.
fn redact(args: &Value) -> Value {
    let mut a = args.clone();
    if let Some(c) = a.get("content").and_then(|v| v.as_str()).map(|s| (s.len(), blake3::hash(s.as_bytes()).to_hex().to_string())) {
        a["content"] = json!({"bytes": c.0, "blake3": c.1});
    }
    capped_args(&a)
}

/// DNS-rebinding guard from the MCP spec: browsers send Origin, Claude's backend and curl do not.
fn origin_ok(req: &Request) -> bool {
    match header(req, "Origin") {
        None => true,
        Some(o) => ["https://claude.ai", "https://claude.com", "https://www.claude.ai"].contains(&o),
    }
}

fn handle(cfg: &Config, req: &mut Request) -> Resp {
    let remote = req.remote_addr().map(|a| a.to_string()).unwrap_or_default();
    if !origin_ok(req) {
        let origin = header(req, "Origin").unwrap_or("").chars().take(100).collect::<String>();
        record_noise(json!({"event": "bad_origin", "remote": remote, "origin": origin}));
        return empty(403);
    }
    let http_method = req.method().to_string();
    let accept = header(req, "Accept").unwrap_or("").to_string();
    let ua = header(req, "User-Agent").unwrap_or("").chars().take(80).collect::<String>();
    let Some(access) = authorized(cfg, req) else {
        // Never log the path itself: it may carry a wrong or partial token.
        record_noise(json!({"event": "unauthorized", "remote": remote, "method": http_method, "ua": ua}));
        return empty(401);
    };
    if *req.method() != Method::Post {
        audit::record(json!({"event": "http", "method": http_method, "status": 405, "accept": accept, "ua": ua}));
        return empty(405);
    }
    let mut body = String::new();
    if req.as_reader().take(MAX_BODY).read_to_string(&mut body).is_err() {
        return empty(400);
    }
    let msg: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return json_resp(400, json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "parse error"}})),
    };
    if msg.as_array().is_some_and(|b| b.is_empty()) {
        return json_resp(400, json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32600, "message": "empty batch"}}));
    }
    let rpc_methods: Vec<&str> = match &msg {
        Value::Array(b) => b.iter().filter_map(|m| m["method"].as_str()).collect(),
        m => m["method"].as_str().into_iter().collect(),
    };
    audit::record(json!({"event": "http", "method": "POST", "rpc": rpc_methods, "accept": accept, "ua": ua}));
    let replies: Vec<Value> = match &msg {
        Value::Array(batch) => batch.iter().filter_map(|m| dispatch(cfg, m, &remote, access)).collect(),
        m => dispatch(cfg, m, &remote, access).into_iter().collect(),
    };
    match (msg.is_array(), replies.len()) {
        (_, 0) => empty(202),
        (false, _) => json_resp(200, replies.into_iter().next().unwrap()),
        (true, _) => json_resp(200, Value::Array(replies)),
    }
}

fn dispatch(cfg: &Config, req: &Value, remote: &str, access: Access) -> Option<Value> {
    let writable = access == Access::Write;
    let id = req.get("id").cloned()?; // notifications get no reply
    let method = req["method"].as_str().unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(json!({}));
    let result = match method {
        "initialize" => json!({
            "protocolVersion": params["protocolVersion"].as_str().unwrap_or(DEFAULT_PROTOCOL),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "xing-pc-mcp", "version": env!("CARGO_PKG_VERSION")},
            "instructions": if writable { "Access to Xing's PC (xing-pc): read tools plus write_file and prepare_commit (D: only; commits need Xing's approval at the PC, never pushed). Call `status` first." } else { "Read-only access to Xing's PC (xing-pc). Call `status` first to see allowed roots and presets." }
        }),
        "ping" => json!({}),
        "tools/list" => json!({"tools": tools::list(writable)}),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let r = if config::killed() {
                Err("kill switch is on (xing-pc-mcp unkill to resume)".to_string())
            } else {
                tools::call(cfg, name, &args, writable)
            };
            audit::record(json!({"event": "call", "remote": remote, "tool": name, "write": writable, "args": redact(&args), "ok": r.is_ok()}));
            match r {
                Ok(t) => json!({"content": [{"type": "text", "text": t}]}),
                Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
            }
        }
        _ => {
            return Some(json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("unknown method {method}")}}));
        }
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}
