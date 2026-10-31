//! Unified GPU + memory gateway (v0.6.0): one OpenAI-compatible endpoint on xing-pc that
//! fronts every model the 5080 can serve, swaps backends so they never fight over VRAM, and
//! gives every model (and every client) the same unified memory store (memory.rs).
//!
//!   client (any tailnet device, any OpenAI client)
//!      -> https://xing-pc.tail8b5cd4.ts.net:8444/v1   (tailscale serve -> this listener)
//!      -> gpu.bind (127.0.0.1:8445)
//!      -> picks a backend by model name:
//!           deepseek* / inferssd*  -> InferSSD (WSL, 127.0.0.1:8080)
//!           anything else          -> Ollama   (127.0.0.1:11434)
//!      -> stops the other backend first (16 GB VRAM holds one at a time)
//!      -> prepends the matching unified-memory entries as a system message
//!      -> streams the backend's reply back unchanged (SSE passes through)
//!
//! Memory over HTTP so local models and non-MCP tools share the same chain:
//!   GET  /v1/memory/recent?topic=&limit=
//!   GET  /v1/memory/search?q=&topic=&limit=
//!   POST /v1/memory/add        (write token only; same validation as memory_add)
//!   GET  /v1/gpu/status
//!
//! Auth: `Authorization: Bearer <read or write token>` (the same tokens as the connector).
//! The listener stays on 127.0.0.1; Tailscale serve is the only way in, tailnet-only, never Funnel.

use crate::{audit, config::Config, memory};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    io::Read,
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const MAX_BODY: u64 = 4 * 1024 * 1024;
const WORKERS: usize = 4;
/// How long a backend gets to come up before the request is refused.
const START_TIMEOUT: Duration = Duration::from_secs(90);
/// Read timeout on the backend: a slow 748B model on a USB drive can sit for minutes per token.
const BACKEND_TIMEOUT: Duration = Duration::from_secs(1800);

#[derive(Serialize, Deserialize, Clone)]
pub struct GpuConfig {
    /// Local bind for the gateway. Keep it on 127.0.0.1; `tailscale serve` publishes it.
    #[serde(default = "default_gpu_bind")]
    pub bind: String,
    #[serde(default = "default_ollama")]
    pub ollama_url: String,
    #[serde(default = "default_inferssd")]
    pub inferssd_url: String,
    /// Model-name prefixes (lowercase) that route to InferSSD.
    #[serde(default = "default_inferssd_models")]
    pub inferssd_models: Vec<String>,
    /// argv that starts InferSSD (the WSL serve script).
    #[serde(default = "default_inferssd_start")]
    pub inferssd_start: Vec<String>,
    /// argv that starts Ollama when its port is closed.
    #[serde(default = "default_ollama_start")]
    pub ollama_start: Vec<String>,
    /// Prepend matching unified-memory entries to every chat as a system message.
    #[serde(default = "default_true")]
    pub memory_inject: bool,
    /// Max memory entries injected per request.
    #[serde(default = "default_memory_hits")]
    pub memory_hits: usize,
}

fn default_gpu_bind() -> String { "127.0.0.1:8445".into() }
fn default_ollama() -> String { "http://127.0.0.1:11434".into() }
fn default_inferssd() -> String { "http://127.0.0.1:8080".into() }
fn default_true() -> bool { true }
fn default_memory_hits() -> usize { 6 }
fn default_inferssd_models() -> Vec<String> { vec!["deepseek".into(), "inferssd".into()] }
fn default_inferssd_start() -> Vec<String> {
    ["wsl", "-d", "Ubuntu-24.04", "--", "bash", "/mnt/d/dev/qfltl.save.home/inferssd-serve.sh"].iter().map(|s| s.to_string()).collect()
}
fn default_ollama_start() -> Vec<String> { vec!["ollama".into(), "serve".into()] }

impl Default for GpuConfig {
    fn default() -> Self {
        serde_json::from_value(json!({})).expect("GpuConfig defaults")
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Backend {
    Ollama,
    InferSsd,
}

impl Backend {
    fn name(self) -> &'static str {
        match self {
            Backend::Ollama => "ollama",
            Backend::InferSsd => "inferssd",
        }
    }
}

/// Which backend owns the GPU right now, plus the child we started (if we started it).
struct State {
    active: Option<Backend>,
    child: Option<Child>,
    since: Instant,
}

type Shared = Arc<Mutex<State>>;

pub fn serve(cfg: Arc<Config>) -> anyhow::Result<()> {
    let g = &cfg.gpu;
    let server = Server::http(&g.bind).map_err(|e| anyhow::anyhow!("gpu bind {}: {e}", g.bind))?;
    eprintln!("xing-pc-mcp gpu gateway on http://{}/v1 (ollama {}, inferssd {})", g.bind, g.ollama_url, g.inferssd_url);
    let state: Shared = Arc::new(Mutex::new(State { active: None, child: None, since: Instant::now() }));
    let server = Arc::new(server);
    let workers: Vec<_> = (0..WORKERS)
        .map(|_| {
            let (server, cfg, state) = (server.clone(), cfg.clone(), state.clone());
            std::thread::spawn(move || loop {
                let mut req = match server.recv() {
                    Ok(r) => r,
                    Err(_) => break,
                };
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle(&cfg, &state, &mut req)))
                    .unwrap_or_else(|_| {
                        audit::record(json!({"event": "gpu_panic"}));
                        json_resp(500, json!({"error": "internal"}))
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

// ── HTTP plumbing ───────────────────────────────────────────────────────────

type Resp = Response<Box<dyn Read + Send>>;

fn json_resp(code: u16, body: Value) -> Resp {
    let bytes = body.to_string().into_bytes();
    let len = bytes.len();
    let mut r = Response::new(StatusCode(code), Vec::new(), Box::new(std::io::Cursor::new(bytes)) as Box<dyn Read + Send>, Some(len), None);
    if let Ok(h) = Header::from_bytes("Content-Type", "application/json") {
        r = r.with_header(h);
    }
    r
}

fn err(code: u16, msg: impl Into<String>) -> Resp {
    json_resp(code, json!({"error": {"message": msg.into(), "type": "gateway_error"}}))
}

fn header<'a>(req: &'a Request, name: &'static str) -> Option<&'a str> {
    req.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str())
}

fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Clone, Copy, PartialEq)]
enum Access {
    Read,
    Write,
}

fn authorized(cfg: &Config, req: &Request) -> Option<Access> {
    let bearer = header(req, "Authorization")?.strip_prefix("Bearer ")?.trim();
    if let Some(w) = cfg.write_secret.as_deref() {
        if ct_eq(bearer, w) {
            return Some(Access::Write);
        }
    }
    ct_eq(bearer, &cfg.secret).then_some(Access::Read)
}

fn query(req: &Request) -> Value {
    let mut out = json!({});
    if let Some(q) = req.url().split_once('?').map(|x| x.1) {
        for pair in q.split('&') {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let v = percent_decode(&v.replace('+', " "));
            match k {
                "limit" => out[k] = json!(v.parse::<u64>().unwrap_or(20)),
                "q" => out["query"] = json!(v),
                _ => out[k] = json!(v),
            }
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(x) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(x);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_body(req: &mut Request) -> Result<String, Resp> {
    let mut body = String::new();
    if req.as_reader().take(MAX_BODY).read_to_string(&mut body).is_err() {
        return Err(err(400, "unreadable body"));
    }
    Ok(body)
}

// ── Backend control ─────────────────────────────────────────────────────────

fn port_open(url: &str) -> bool {
    let Some(addr) = url.trim_start_matches("http://").trim_start_matches("https://").split('/').next() else { return false };
    let Ok(sa) = addr.parse::<SocketAddr>() else { return false };
    TcpStream::connect_timeout(&sa, Duration::from_millis(400)).is_ok()
}

fn spawn(argv: &[String]) -> Option<Child> {
    if argv.is_empty() {
        return None;
    }
    Command::new(&argv[0]).args(&argv[1..]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok()
}

fn quiet(prog: &str, args: &[&str]) {
    let _ = Command::new(prog).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
}

fn stop_ollama() {
    for name in ["ollama.exe", "ollama app.exe"] {
        quiet("taskkill", &["/IM", name, "/F"]);
    }
}

fn stop_inferssd(st: &mut State) {
    if let Some(mut c) = st.child.take() {
        let _ = c.kill();
        let _ = c.wait();
    }
    // The Linux process outlives wsl.exe; kill it inside the distro too.
    quiet("wsl", &["-d", "Ubuntu-24.04", "--", "pkill", "-f", "inferssd serve"]);
}

/// Make `want` the only backend on the GPU. Serialised by the state mutex so two requests
/// can't flip it back and forth.
fn ensure(cfg: &Config, state: &Shared, want: Backend) -> Result<(), String> {
    let g = &cfg.gpu;
    let mut st = state.lock().map_err(|_| "gpu state poisoned")?;
    let (url, start) = match want {
        Backend::Ollama => (&g.ollama_url, &g.ollama_start),
        Backend::InferSsd => (&g.inferssd_url, &g.inferssd_start),
    };
    if port_open(url) {
        if st.active != Some(want) {
            st.active = Some(want);
            st.since = Instant::now();
        }
        return Ok(());
    }
    // Free the VRAM first.
    match want {
        Backend::Ollama => stop_inferssd(&mut st),
        Backend::InferSsd => stop_ollama(),
    }
    audit::record(json!({"event": "gpu_swap", "to": want.name()}));
    let child = spawn(start);
    if child.is_none() {
        return Err(format!("could not start {}: `{}`", want.name(), start.join(" ")));
    }
    if want == Backend::InferSsd {
        st.child = child;
    } else {
        // Ollama detaches itself; keep nothing.
        drop(child);
    }
    let t0 = Instant::now();
    while t0.elapsed() < START_TIMEOUT {
        if port_open(url) {
            st.active = Some(want);
            st.since = Instant::now();
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(format!("{} did not open {} within {}s (first InferSSD run downloads the model; try again later)", want.name(), url, START_TIMEOUT.as_secs()))
}

fn pick(cfg: &Config, model: &str) -> Backend {
    let m = model.to_lowercase();
    if cfg.gpu.inferssd_models.iter().any(|p| m.starts_with(p)) {
        Backend::InferSsd
    } else {
        Backend::Ollama
    }
}

fn base(cfg: &Config, b: Backend) -> &str {
    match b {
        Backend::Ollama => &cfg.gpu.ollama_url,
        Backend::InferSsd => &cfg.gpu.inferssd_url,
    }
}

// ── Memory injection ────────────────────────────────────────────────────────

/// Last user message, used as the search text for memory recall.
fn last_user_text(body: &Value) -> Option<String> {
    let msgs = body["messages"].as_array()?;
    let m = msgs.iter().rev().find(|m| m["role"] == "user")?;
    match &m["content"] {
        Value::String(s) => Some(s.clone()),
        Value::Array(parts) => Some(parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join(" ")),
        _ => None,
    }
}

/// Query terms: words of 4+ chars, lowercase, deduped, at most 8.
fn terms(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for w in text.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_') {
        let w = w.to_lowercase();
        if w.len() >= 4 && !out.contains(&w) {
            out.push(w);
        }
        if out.len() >= 8 {
            break;
        }
    }
    out
}

fn recall(cfg: &Config, body: &Value) -> Vec<Value> {
    let Some(text) = last_user_text(body) else { return Vec::new() };
    let mut hits: Vec<Value> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for t in terms(&text) {
        if let Ok(s) = memory::search(cfg, &json!({"query": t, "limit": cfg.gpu.memory_hits})) {
            if let Ok(Value::Array(a)) = serde_json::from_str::<Value>(&s) {
                for e in a {
                    let seq = e["seq"].as_u64().unwrap_or(0);
                    if seen.insert(seq) {
                        hits.push(e);
                    }
                }
            }
        }
        if hits.len() >= cfg.gpu.memory_hits {
            break;
        }
    }
    hits.truncate(cfg.gpu.memory_hits);
    hits
}

fn inject(cfg: &Config, body: &mut Value) -> usize {
    let want = body.get("memory").and_then(|v| v.as_bool()).unwrap_or(cfg.gpu.memory_inject);
    if let Some(o) = body.as_object_mut() {
        o.remove("memory");
    }
    if !want {
        return 0;
    }
    let hits = recall(cfg, body);
    if hits.is_empty() {
        return 0;
    }
    let mut txt = String::from("Shared memory on xing-pc (unified store, newest first). Use it as context; it is Xing's own stated facts and decisions:\n");
    for h in &hits {
        txt.push_str(&format!("- [{} #{}] {}: {}\n", h["kind"].as_str().unwrap_or("note"), h["seq"], h["topic"].as_str().unwrap_or(""), h["text"].as_str().unwrap_or("")));
    }
    let sys = json!({"role": "system", "content": txt});
    if let Some(msgs) = body["messages"].as_array_mut() {
        msgs.insert(0, sys);
    }
    hits.len()
}

// ── Forwarding ──────────────────────────────────────────────────────────────

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(5)).timeout_read(BACKEND_TIMEOUT).timeout_write(Duration::from_secs(60)).build()
}

/// Forward a request to `url` and stream whatever comes back (status, content-type, body).
fn forward(method: &Method, url: &str, body: Option<&str>) -> Resp {
    let a = agent();
    let req = match method {
        Method::Get => a.get(url),
        _ => a.post(url).set("Content-Type", "application/json"),
    };
    let res = match body {
        Some(b) => req.send_string(b),
        None => req.call(),
    };
    let res = match res {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(ureq::Error::Transport(t)) => return err(502, format!("backend unreachable: {t}")),
    };
    let status = res.status();
    let ct = res.header("Content-Type").unwrap_or("application/json").to_string();
    let reader: Box<dyn Read + Send> = Box::new(res.into_reader());
    let mut r = Response::new(StatusCode(status), Vec::new(), reader, None, None);
    if let Ok(h) = Header::from_bytes("Content-Type", ct.as_bytes()) {
        r = r.with_header(h);
    }
    if let Ok(h) = Header::from_bytes("Cache-Control", "no-cache") {
        r = r.with_header(h);
    }
    r
}

fn models(cfg: &Config, state: &Shared) -> Resp {
    let mut data: Vec<Value> = Vec::new();
    let active = state.lock().ok().and_then(|s| s.active);
    // Ollama's own list, without waking it up.
    if port_open(&cfg.gpu.ollama_url) {
        if let Ok(r) = agent().get(&format!("{}/api/tags", cfg.gpu.ollama_url)).call() {
            if let Ok(v) = r.into_json::<Value>() {
                for m in v["models"].as_array().cloned().unwrap_or_default() {
                    if let Some(n) = m["name"].as_str() {
                        data.push(json!({"id": n, "object": "model", "owned_by": "ollama", "backend": "ollama"}));
                    }
                }
            }
        }
    }
    for p in &cfg.gpu.inferssd_models {
        data.push(json!({"id": p, "object": "model", "owned_by": "inferssd", "backend": "inferssd"}));
    }
    json_resp(200, json!({"object": "list", "data": data, "active_backend": active.map(|b| b.name())}))
}

fn gpu_status(cfg: &Config, state: &Shared) -> Resp {
    let (active, since, child) = match state.lock() {
        Ok(s) => (s.active.map(|b| b.name()), s.since.elapsed().as_secs(), s.child.is_some()),
        Err(_) => (None, 0, false),
    };
    json_resp(200, json!({
        "active": active,
        "active_for_secs": since,
        "inferssd_child_owned": child,
        "ollama_up": port_open(&cfg.gpu.ollama_url),
        "inferssd_up": port_open(&cfg.gpu.inferssd_url),
        "memory_inject": cfg.gpu.memory_inject,
        "memory": memory::status(cfg).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok())
    }))
}

fn handle(cfg: &Config, state: &Shared, req: &mut Request) -> Resp {
    let path = req.url().split('?').next().unwrap_or("").trim_end_matches('/').to_string();
    let method = req.method().clone();
    let Some(access) = authorized(cfg, req) else {
        audit::record(json!({"event": "gpu_unauthorized", "path": path}));
        return err(401, "missing or wrong bearer token");
    };
    if crate::config::killed() {
        return err(503, "kill switch is on (xing-pc-mcp unkill to resume)");
    }
    match (method.clone(), path.as_str()) {
        (Method::Get, "/v1/models") => models(cfg, state),
        (Method::Get, "/v1/gpu/status") => gpu_status(cfg, state),
        (Method::Get, "/v1/memory/recent") => match memory::recent(cfg, &query(req)) {
            Ok(s) => json_resp(200, serde_json::from_str(&s).unwrap_or(json!([]))),
            Err(e) => err(400, e),
        },
        (Method::Get, "/v1/memory/search") => match memory::search(cfg, &query(req)) {
            Ok(s) => json_resp(200, serde_json::from_str(&s).unwrap_or(json!([]))),
            Err(e) => err(400, e),
        },
        (Method::Post, "/v1/memory/add") => {
            if access != Access::Write {
                return err(403, "memory_add needs the write token");
            }
            let body = match read_body(req) { Ok(b) => b, Err(r) => return r };
            let mut args: Value = match serde_json::from_str(&body) { Ok(v) => v, Err(_) => return err(400, "body is not JSON") };
            if args.get("source").is_none() {
                args["source"] = json!("gateway");
            }
            match memory::add(cfg, &args) {
                Ok(s) => {
                    audit::record(json!({"event": "gpu_memory_add", "topic": args["topic"]}));
                    json_resp(200, serde_json::from_str(&s).unwrap_or(json!({})))
                }
                Err(e) => err(400, e),
            }
        }
        (Method::Post, "/v1/chat/completions") => {
            let body = match read_body(req) { Ok(b) => b, Err(r) => return r };
            let mut v: Value = match serde_json::from_str(&body) { Ok(v) => v, Err(_) => return err(400, "body is not JSON") };
            let model = v["model"].as_str().unwrap_or("").to_string();
            let backend = pick(cfg, &model);
            if let Err(e) = ensure(cfg, state, backend) {
                return err(503, e);
            }
            let injected = inject(cfg, &mut v);
            audit::record(json!({"event": "gpu_chat", "backend": backend.name(), "model": model, "memory_hits": injected, "stream": v["stream"].as_bool().unwrap_or(false)}));
            forward(&Method::Post, &format!("{}/v1/chat/completions", base(cfg, backend)), Some(&v.to_string()))
        }
        (Method::Post, p) if p == "/v1/completions" || p == "/v1/embeddings" => {
            let body = match read_body(req) { Ok(b) => b, Err(r) => return r };
            if let Err(e) = ensure(cfg, state, Backend::Ollama) {
                return err(503, e);
            }
            forward(&Method::Post, &format!("{}{}", cfg.gpu.ollama_url, p), Some(&body))
        }
        _ => err(404, format!("no route {} {}", method, path)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Stays under this repo's target dir on D:, never the system temp on C:.
    fn cfg() -> Config {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("test-sandbox").join(format!("gpu-{n}"));
        serde_json::from_value(json!({
            "bind": "127.0.0.1:0", "roots": [], "deny": [], "max_read_bytes": 1024, "presets": {},
            "memory_dir": d.join("memory")
        }))
        .unwrap()
    }

    #[test]
    fn routes_by_model_prefix() {
        let c = cfg();
        assert_eq!(pick(&c, "DeepSeek-V4.1-Flash"), Backend::InferSsd);
        assert_eq!(pick(&c, "inferssd"), Backend::InferSsd);
        assert_eq!(pick(&c, "qwen3:8b"), Backend::Ollama);
        assert_eq!(pick(&c, ""), Backend::Ollama);
    }

    #[test]
    fn terms_are_short_and_deduped() {
        let t = terms("the Brain uses qwen3-8b in LM Studio, brain brain");
        assert_eq!(t, vec!["brain", "uses", "qwen3-8b", "studio"]);
    }

    #[test]
    fn memory_flag_is_stripped_and_honoured() {
        let c = cfg();
        let mut v = json!({"model": "x", "memory": false, "messages": [{"role": "user", "content": "hello brain"}]});
        assert_eq!(inject(&c, &mut v), 0);
        assert!(v.get("memory").is_none());
        assert_eq!(v["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn injection_prepends_system_message() {
        let c = cfg();
        memory::add(&c, &json!({"text": "the brain model is qwen3-8b", "topic": "brain", "kind": "decision"})).unwrap();
        let mut v = json!({"model": "x", "messages": [{"role": "user", "content": "which brain model?"}]});
        assert_eq!(inject(&c, &mut v), 1);
        let m = v["messages"].as_array().unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0]["role"], "system");
        assert!(m[0]["content"].as_str().unwrap().contains("qwen3-8b"));
    }

    #[test]
    fn percent_decoding() {
        assert_eq!(percent_decode("a%20b%2Fc"), "a b/c");
        assert_eq!(percent_decode("plain"), "plain");
        assert_eq!(percent_decode("bad%"), "bad%");
    }

    #[test]
    fn gpu_config_defaults_parse() {
        let g = GpuConfig::default();
        assert_eq!(g.bind, "127.0.0.1:8445");
        assert_eq!(g.memory_hits, 6);
        assert!(g.memory_inject);
    }
}
