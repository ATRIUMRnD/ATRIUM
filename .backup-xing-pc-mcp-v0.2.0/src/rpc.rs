use crate::{audit, config::{self, Config}, tools};
use serde_json::{json, Value};
use std::{
    io::{Cursor, Read},
    sync::Arc,
};
use tiny_http::{Header, Method, Request, Response, Server};

const MAX_BODY: u64 = 1024 * 1024;
const DEFAULT_PROTOCOL: &str = "2025-06-18";

type Resp = Response<Cursor<Vec<u8>>>;

/// Streamable HTTP transport, JSON-response mode: one POST in, one JSON body out.
/// Same dispatch shape as video-watch's stdio loop.
pub fn serve(cfg: Arc<Config>) -> anyhow::Result<()> {
    let server = Server::http(&cfg.bind).map_err(|e| anyhow::anyhow!("bind {}: {e}", cfg.bind))?;
    eprintln!(
        "xing-pc-mcp (read-only) on http://{}/mcp, config {}",
        cfg.bind,
        config::dir().join("config.json").display()
    );
    for req in server.incoming_requests() {
        let cfg = cfg.clone();
        std::thread::spawn(move || {
            let mut req = req;
            let r = handle(&cfg, &mut req);
            let _ = req.respond(r);
        });
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
    Response::from_data(body.to_string().into_bytes())
        .with_status_code(code)
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
}

fn empty(code: u16) -> Resp {
    Response::from_data(Vec::new()).with_status_code(code)
}

fn authorized(cfg: &Config, req: &Request) -> bool {
    let path = req.url().split('?').next().unwrap_or("");
    if let Some(t) = path.strip_prefix("/mcp/") {
        return ct_eq(t, &cfg.secret);
    }
    if path == "/mcp" {
        if let Some(b) = header(req, "Authorization").and_then(|h| h.strip_prefix("Bearer ")) {
            return ct_eq(b.trim(), &cfg.secret);
        }
    }
    false
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
        audit::record(json!({"event": "bad_origin", "remote": remote, "origin": origin}));
        return empty(403);
    }
    let http_method = req.method().to_string();
    let accept = header(req, "Accept").unwrap_or("").to_string();
    let ua = header(req, "User-Agent").unwrap_or("").chars().take(80).collect::<String>();
    if !authorized(cfg, req) {
        // Never log the path itself: it may carry a wrong or partial token.
        audit::record(json!({"event": "unauthorized", "remote": remote, "method": http_method, "ua": ua}));
        return empty(401);
    }
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
    let rpc_methods: Vec<&str> = match &msg {
        Value::Array(b) => b.iter().filter_map(|m| m["method"].as_str()).collect(),
        m => m["method"].as_str().into_iter().collect(),
    };
    audit::record(json!({"event": "http", "method": "POST", "rpc": rpc_methods, "accept": accept, "ua": ua}));
    let replies: Vec<Value> = match &msg {
        Value::Array(batch) => batch.iter().filter_map(|m| dispatch(cfg, m, &remote)).collect(),
        m => dispatch(cfg, m, &remote).into_iter().collect(),
    };
    match (msg.is_array(), replies.len()) {
        (_, 0) => empty(202),
        (false, _) => json_resp(200, replies.into_iter().next().unwrap()),
        (true, _) => json_resp(200, Value::Array(replies)),
    }
}

fn dispatch(cfg: &Config, req: &Value, remote: &str) -> Option<Value> {
    let id = req.get("id").cloned()?; // notifications get no reply
    let method = req["method"].as_str().unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(json!({}));
    let result = match method {
        "initialize" => json!({
            "protocolVersion": params["protocolVersion"].as_str().unwrap_or(DEFAULT_PROTOCOL),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "xing-pc-mcp", "version": env!("CARGO_PKG_VERSION")},
            "instructions": "Read-only access to Xing's PC (xing-pc). Call `status` first to see allowed roots and presets."
        }),
        "ping" => json!({}),
        "tools/list" => json!({"tools": tools::list()}),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let r = if config::killed() {
                Err("kill switch is on (xing-pc-mcp unkill to resume)".to_string())
            } else {
                tools::call(cfg, name, &args)
            };
            audit::record(json!({"event": "call", "remote": remote, "tool": name, "args": args, "ok": r.is_ok()}));
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
