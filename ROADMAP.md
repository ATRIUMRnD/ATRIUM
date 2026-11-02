# ROADMAP.md  - ATRIUM build order

Single source of truth for what gets built next. Agents: read AGENTS.md
first for routing and invariants, then pick up the lowest-numbered
incomplete task here. Detailed steps live in harness-roadmap/; this file
is the dispatch table.

## Status baseline (July 22, 2026)

The first verified harness loop exists: limend (LIMEN spool daemon) ->
ductei-limen-relay (DUCTEI consumer) -> certified, persisted,
invariant-checked results. Smoke test passed on all four scenarios:
good job, restart replicability, malformed request, malformed cert.
Everything below grows that loop. Nothing starts over.

## Status update (October 24, 2026)

Verified against live repo history on xing-pc. Two roadmap items moved
since the July 22 baseline:

- Task 2, network transport: DUCTEI@d951787 gates the TCP transport
  behind a `net` feature (implied by grpc/quic), so a default build has
  zero network code, and adds transport::send_local_first() which falls
  back to the local persistence-before-ack Channel only when the peer is
  unreachable. CI runs default, `net`, and `pq` paths. Remaining: the
  ML-KEM-768 scoped design is exercised under `pq` in CI, but no real
  multi-node deployment exists yet.
- Task 3, self-improvement loop: LIMEN@1332973 adds the ledger-backed
  routing-policy proposal gate (proposal.py, append-only
  policy_proposals table, build_proposal_report, run_proposal.py, and the
  policy-proposal-verdict CI job), scoped to CRITICALITY_SPREAD_THRESHOLD
  only. One demonstration proposal is archived under
  policy_proposals/accepted/. The verdict check is now required on LIMEN
  main (routing-policy proposal verdict, confirmed October 24, 2026).
- Branch protection confirmed by the owner October 24, 2026 (repos live
  under ATRIUMRnD): DUCTEI main, LIMEN main (requires cargo test, pytest
  py3.12, routing-policy proposal verdict), and ATRIUM master (pull
  request required, 0 approvals, no checks). Admin bypass is on. DUCTEI
  and LIMEN also require branches to be up to date before merging.
- Rooms outside the core mesh are now listed in AGENTS.md section 7.
  They are tracked there, not dispatched from this table.
- Known drift: VEYN on xing-pc (D:\VEYN) shows a single "Initial commit"
  (69f7c5d, 2026-08-29), so the VEYN@1913f41 and VEYN@da747ec refs in this
  file and BUILD.md no longer resolve locally. Record only, no action.

## Status update (October 25, 2026)

Quantum backbone on xing-pc: LIMEN gained a local GPU statevector tier 0
and the loop now exercises it nightly. Verified on the RTX 5080.

- LIMEN@ee4caed (on main, CI green): fidelity tier 0 `aer_gpu` in the
  budget router. Opt-in via LIMEN_LOCAL_GPU=1 or
  run_route_request(local_gpu_first=True); limend inherits it from the
  environment. Cap is read from nvidia-smi at call time:
  floor(log2(0.8 * free_bytes / 8)), max 30, single-precision Aer, no
  eviction of other GPU tenants. Idle 5080 reads cap 30; with LM Studio
  holding ~7 GB it reads 29. Prepend-only: Tier 2 plans and pinned
  hardware tiers are returned unchanged, anything that does not fit falls
  through to the ordinary route() decision. run_pipeline(backend="aer")
  now runs the QAOA grid search on Aer too, so 24+ variable problems run
  on the GPU end to end (24 vars: 19 s). Decided: no limend daemon or
  systemd unit until brain, jspace-lab, or Self.Define import limen in
  code; a scheduled run covers today's two consumers.
- LIMEN@d029fe5 (#37) widened the limend cert summary
  (success_probability, fidelity_target, target_met; fidelity_estimate
  kept as a deprecated alias for one release). DUCTEI working folder
  (uncommitted): ductei-limen CertSummary now reads success_probability
  with the alias accepted, tolerates both keys during the window, carries
  target_met, and still emits fidelity_estimate on the wire;
  scripts/smoke_e2e.py asserts required-fields-plus-known-set instead of
  an exact field match, and adds an explicit no-credential-key check (I1).
  The nightly caught this drift the first night it ran.
- Nightly on xing-pc (Windows Task Scheduler): limen-ductei-nightly-smoke
  (03:30, C:\me\scripts\nightly-smoke.ps1: four DUCTEI scenarios on the
  Windows venv plus a 24-var tier-0 GPU route in WSL, logs
  C:\me\logs\smoke) and limen-nightly-smoke (03:37, WSL,
  C:\me\scripts\limen-nightly-smoke.sh: the four scenarios through the
  WSL venv with tier 0 enabled, logs C:\me\logs\limen-smoke). Both passed
  October 25. The two overlap; consolidating into one is a cleanup item,
  not a blocker.
- WSL2 Ubuntu-24.04 fixed for unattended runs: pam_lastlog.so (removed in
  24.04) no longer referenced in /etc/pam.d/login, snapd and wsl-pro
  masked; systemd reports running for system and user.
- Does not unlock: quantum speedup for LLM work, qubit counts above 30,
  or revenue by itself. It is harness plumbing that makes the loop free
  to run every night.
- Oct 26, 2026, what tier 0 is for, measured (limen/study/, uncommitted in
  D:\LIMEN; `python -m limen.study <gap|cut|bench|verify|all>`):
  gap: ideal-vs-hardware on the final QAOA circuit; success_probability
  collapses like 2**-n (0.0005 at 20 qubits) so the record now carries
  distribution fidelity / TVD (exact up to 18 qubits) and expected-energy
  gap (valid at any width). cut: 16 vars into 8q pieces took 57 min of
  reconstruction vs 8 s exact, mean marginal error 0.031, 15/16 signs
  right; cost is n * 4**cuts classical post-processing, not the GPU, so
  tier-0 cutting is a correctness oracle, not a throughput path. bench on
  aer_gpu: TSP 3 cities ratio 1.0 (1.4 s); 4 and 5 cities decode to
  infeasible tours (p=1 QAOA + argmax, the 0.15% feasible-rate weakness
  from June); Max-Cut 3-regular 12/20/28 nodes ratio 0.93 / 0.83 / 0.37
  (28-node ratio is against a local-search lower bound, labelled). verify:
  Tier 2 (d=5 ECC) at 20 vars reproduces the ideal solution exactly;
  predicted logical error bound 5.8e-6. Takeaway: tier 0 is doing its job
  as ground truth; the weak link it exposes is p=1 QAOA with argmax
  decoding, which is where the next LIMEN work should go.

## Task 1: CI harness  - DONE (August 29, 2026)
- Detail: harness-roadmap/01-ci-harness.md
- Repos: DUCTEI (primary), LIMEN (pinned rev, simulator path)
- Deliverable: checked-in smoke script + GitHub Actions job replaying the
  four scenarios per push, with invariant checks as assertions  - DONE,
  DUCTEI@7433317, verified green on main:
  https://github.com/xingxerx/DUCTEI/actions/runs/29975187862
- Done when: the e2e job is green on main in DUCTEI (met) and required
  for merge (met, August 29 2026). DUCTEI `main` now requires status
  checks [test, e2e-smoke, e2e-smoke-qallow, e2e-smoke-veyn], strict,
  no force-push, no deletions. ATRIUM `master` is also protected
  (pull-request reviews, no force-push, no deletions).
- Unlocks: Tasks 2 and 3 are safe to build; regressions caught same day.
- Does not unlock: any new runtime capability

## Task 2: Spine flow  - DONE (August 29, 2026)
- Detail: harness-roadmap/02-spine-flow.md
- Repos: DUCTEI, Qallow, then VEYN (rev-pin bump per landing)
- Order inside the task: Qallow pair first  - DONE, DUCTEI@2cb9e68 +
  Qallow@0a546b3. VEYN pair second  - DONE, DUCTEI@5d750b8 + VEYN@1913f41.
  Network transport last  - DONE, DUCTEI@d951787 (July 24 2026): `net`
  feature gate, local-first fallback, CI runs default/`net`/`pq`. Green:
  https://github.com/xingxerx/DUCTEI/actions/runs/30066003124
- Done when: three real consumers flow through DUCTEI under the five
  invariants, each with its four smoke scenarios in CI  - MET (LIMEN,
  Qallow, VEYN), and network transport is gated and tested  - MET.
- Unlocks: DUCTEI is the actual spine; Tasks 3 and 4 unblocked on this
  axis
- Does not unlock: self-modification; flow is still human-initiated

## Task 3: Self-improvement loop  - DONE (August 29, 2026)
- Detail: harness-roadmap/03-self-improvement-loop.md
- Repos: LIMEN (primary), DUCTEI
- Scope guard: gates routing-policy changes ONLY at first (closed set:
  currently CRITICALITY_SPREAD_THRESHOLD)
- Loop: ledger baseline -> proposal with claim file -> CI verdict via
  report.py in plain English -> branch-protected merge -> ledger learns
   - implemented LIMEN@13329733 (July 24 2026). Demonstration transcript:
  policy_proposals/accepted/2026-07-24-raise-criticality-threshold.*
  (threshold 2.0 -> 8.0, ACCEPTED, cost delta 0, physical-error exposure
  delta -698.7). CI job `routing-policy proposal verdict` green:
  https://github.com/xingxerx/LIMEN/actions/runs/30072093877
- Done when: one real routing improvement has traversed the full loop and
  the transcript is written up  - MET. LIMEN `main` now requires
  [cargo test, pytest (py3.12), routing-policy proposal verdict],
  strict, no force-push, no deletions (August 29 2026).
- Unlocks: the research result itself; every future improvement generates
  its own evidence trail
- Does not unlock: gating arbitrary code; widening scope stays deliberate

## Task 4: Qallow kernel-level invariants - DONE (August 29, 2026)
- Detail: harness-roadmap/04-qallow-kernel-invariants.md
- Repo: Qallow (primary)
- Proposal (before Qallow code): what changes is C-level enforcement in
  Qallow only (persist gate, LMDB session bounds, conformance tests,
  aarch64 target, sovereignty audit). ATRIUM only witnesses. Unlocks
  unbypassable invariants and on-device state. Does not unlock new
  capabilities.
- Landed Qallow@2b0009e (PR 14). ql_persist_merge_blob() rejects
  schema < 2, reserved key prefixes (env/ cred/ secret/ token/
  password/), open scope 0, zero session id, zero session bound, and
  malformed v2 payloads, before LMDB write. Record layout stores
  session_id and session_bound. test_sync_wire.c and test_persist_lmdb.c
  fail on those violations. Makefile test-aarch64 cross-compiles.
  SOVEREIGNTY.md: sync_wire none; persist vendored LMDB only. CI job
  `C persist/sync tests` green:
  https://github.com/xingxerx/Qallow/actions/runs/33280770814
- Done when: each invariant has a C-level test that fails on violation,
  and the sovereignty audit answers "none" for core-loop dependencies
  - MET.
- Unlocks: invariants become un-bypassable from above; harness state
  travels on-device
- Does not unlock: new capabilities; never preempts Tasks 1-3 (those
  are already DONE)

## Task 5: QSW proto v2 C decoder - ACTIVE (proposed September 8, 2026)
- Status note (September 8, 2026): implemented and verified locally -
  all Qallow C tests, DUCTEI ductei-core tests, v1+v2 conformance
  oracle runs, and an end-to-end v2 ingest into real LMDB with
  merge-key parity are green. Evidence in harness-roadmap/05. Nothing
  committed or pushed yet; task stays ACTIVE until the commits land in
  the order documented there (Qallow first, then DUCTEI).
- Detail: harness-roadmap/05-qsw-v2-c-decoder.md
- Repos: Qallow (primary), then DUCTEI (rev-pin bump + conformance
  extension as a separate, auditable commit after Qallow lands)
- Proposal: sync_wire.c gains a v2 decoder (scopes as a native wire field)
  alongside v1 unchanged - v1 stays the byte-compatible oracle. C struct
  qsw_envelope_v2 mirrors ductei-qallow/src/v2.rs exactly. Merge-key
  parity: v2 ingest reconstructs the v1 "scopes|key" shim so a record
  lands under the same LMDB key regardless of proto version.
- Unlocks: v2 adoption end-to-end; closes the wire-format gap; multi-scope
  envelopes without the v1 comma-shim corruption class.
- Does not unlock: peer provisioning (Task 6), new transports, new
  capabilities.

## Task 6: Peer provisioning - QUEUED (proposed September 8, 2026)
- Detail: harness-roadmap/06-peer-provisioning.md
- Repo: DUCTEI
- Proposal: minimal cross-network peer story - static peer-list config +
  self-signed cert fingerprint exchange at pairing time (config, not a
  PKI, matching how QUIC clients pin today) so the grpc/quic transports
  become usable cross-machine.
- Unlocks: the spine leaves localhost; multi-machine deployments become a
  supported path.
- Does not unlock: CA-based mTLS, service discovery (etcd/consul-style).

## Task 7: One-app integration proof - BUILT, awaiting merge (September 8, 2026)
- Detail: harness-roadmap/07-one-app-integration.md
- Repos: DUCTEI (scripts + persist-v2 framing), Qallow (`qallow propose`);
  ATRIUM witnesses. Built ahead of Tasks 5 and 6 on the user's explicit
  direction ("build this"); it depends on neither (runs local-first on
  QSW v1, as the detail file allowed).
- What was found first: the spine was broken against Qallow main. Task 4
  made `ql_persist_merge_blob()` reject `schema_ver < 2` and any blob
  without a bounded session, and DUCTEI's relay still emitted schema 1
  with raw blobs. DUCTEI CI stayed green only because its Qallow pin
  (42ce93f) predates Task 4. Repaired in DUCTEI: `ductei_qallow::persist`
  wraps each forwarded envelope in the persist-v2 header (scope code,
  session id, session bound = the relay's own bounded session) and the
  Qallow pins move to a05392e. smoke_qallow and smoke_veyn pass again
  against Qallow main on-box.
- The missing arrow, mind -> hands: `qallow propose <store> <key> <spool>`
  (Qallow, qallow_cli/src/propose.rs) reads a persisted `veyn.rem_event`
  cue from LMDB and writes an offline LIMEN route request into limend's
  spool. Idempotent by job id, refuses unpersisted keys, never carries
  credentials.
- The proof: DUCTEI scripts/smoke_loop.py runs VEYN cue -> DucteiBridge ->
  ductei-qallow-relay -> ONE shared LMDB store -> `qallow propose` ->
  limend -> ductei-limen-relay -> ductei-qallow-relay -> the same store.
  Four scenarios, 41 checks, invariants I1/I2/I4/I5 asserted inline. Green
  on-box (Windows, September 8 2026). CI job `e2e-smoke-loop` added; it
  goes green once the Qallow pin is bumped to a rev carrying `propose`
  (separate, auditable commit) and must not be a required check before.
- The launcher: DUCTEI scripts/atrium_up.py runs the same loop as
  long-lived processes from one command. Verified live on-box: real
  veyn-core daemon, watch-shaped /oneiro/watch OSC cue (N2 -> REM),
  rising-edge rem_detected, certificate in the shared store in ~4 s.
- Done when: e2e-smoke-loop is green on DUCTEI main and required - NOT
  YET (nothing here is committed; see AGENTS.md section 6).
- Unlocks: "everything works as one app" is a demonstrated, replayable
  property with one durable state, not a claim.
- Does not unlock: GET /export snapshot on REM (Qallow's FastAPI reasoning
  server is not in the tree; the qallow-veyn-bridge `ws` path stays
  optional), cross-machine transport (Task 6), any commercial charge path.

## Task 8: brain joins the mesh (correction sync) - BUILT, awaiting merge (October 26, 2026)
- Detail: harness-roadmap/08-brain-sync.md
- Repos, in landing order: Qallow (tombstone retention in
  persist_lmdb.c), DUCTEI (ductei-sync crate behind `net`, Qallow pin
  bump as its own commit, scripts/smoke_brain_sync.py), brain
  (adapt/src/sync.rs, `brain-adapt sync`).
- Built ahead of Task 6 on the user's explicit direction; depends on
  neither Task 5 nor Task 6 (QSW v1 into Qallow, plain TCP between
  peers, IP allow-list on a Tailscale network).
- Promotion: brain becomes a spine producer for scope
  `brain.correction` only (AGENTS.md sections 1 and 7 updated).
- Bug found and fixed on the way: Qallow deleted tombstones from LMDB,
  so an older write merged after a delete resurrected the key. Now the
  tombstone record is kept; the new C test fails on the old code.
- Proof: two-node smoke, 17/17 checks, real binaries, loopback. Not yet
  run on xing-pc / xing-laptop.
- Unlocks: corrections made on either machine reach brain on the other;
  xing-pc's Qallow holds the merged history.
- Does not unlock: adapter or run-log sync, cert pinning (Task 6),
  relay through a third node, mobile.

## Rules of the table

- One task ACTIVE at a time. An agent proposing work on a BLOCKED task
  must first show the blocker is resolved.
- Completing a task means updating this file (status flip + date) in the
  same PR as the final change, per the witness requirement.
- Any change to task scope or order is proposed here first, in plain
  English, before implementation anywhere.

## Status update (October 26, 2026, evening optimize pass)

- Nightly limen-ductei smoke failed at 03:32: limend adds an optional `note` field when a job misses its fidelity target (the Windows CPU path), and smoke_e2e.py did not allow it. Allowed set widened in DUCTEI (uncommitted); rerun 18:30 ALL PASS. The 03:37 WSL GPU task passed because GPU runs meet the target, so the two tasks cover different paths and both stay.
- Clippy auto-fixes (style only) in Qallow native_app/qallow_cli, universal-clock, video-watch chats.rs, DUCTEI transport.rs; all suites green. VEYN clippy-clean.
- Blocked: ductei-mcp re-mirror and tailnet anchor peers (claimed by mcp-gate-lockdown on the laptop), laptop watchdog (laptop SSH timed out).

## Status update (October 28, 2026, decision gates stay unstuck)

- brain: xing-pc's 5 unpushed Oct 27 commits merged with GitHub's 4 (probe v2, security baseline) in one local merge commit, c44df4a, not pushed. The duplicate key: value parser was folded into parse_proposal.
- brain, uncommitted: gate fallback (a failed or hung lens/decider call no longer ends `ask`), gate timeout 300 s to 30 s, repeat guard, act guard, reply timeout. All are on for `ask` and off for eval unless set, and labeled in run names. 84 tests pass, clippy clean. Live on xing-pc: with a dead sidecar, `ask` answers; with a hung test sidecar and a 3 s limit, it finishes in 3.2 s; on pr-abort-20 the act guard keeps the model's abort, where without it the gate wasted a step and then waited on stdin.
- brain: decider-4b on the GPU. llama-cpp-python 0.3.36 was rebuilt with CUDA 13.1 in `.venv`, and `sidecar/cuda_dlls.py` fixes the CUDA 13 `bin\x64` DLL lookup. 0.1 s per decision against 6.6 s on the CPU; dev 22/24 and probe 110/120, matching the CPU's 22/24 and 109/120.
- player-harness, uncommitted: one shared `/v1/decision` request (15 s timeout, was 60 s), backoff after failures for the move labeller and the name reviewer, and an API key for the decision server (it was open to any local origin with no key). Measured: no key or a wrong key gets 401, the right key 200, and a foreign origin gets no CORS header. `tools/ci-check.ps1` passes.
- Unlocks: brain's live gate survives sidecar crashes and hangs, and decider-4b is fast enough for the live loop.
- Does not unlock: Jev itself (no code calls it), or the heldout run (brain's PREREG is still not locked).

## Status update (October 29, 2026, QFLTL transfers complete over Tailscale)

- QFLTL, uncommitted on `hardening/defensive-pass`: receiver lanes now get 4 MB receive buffers (the OS default dropped about 4% even on loopback). NACK repair: the sender sends END for round r, and the receiver answers END_ACK when complete, or a sealed bitmap of missing chunks (8192 chunks per datagram, about 1.1 KB). The sender resends only those chunks. Sender pacing starts from the metadata RTT and is re-aimed every round at the measured goodput. qfltl-crypto has 7 new tests and 56/56 pass.
- Measured on xing-pc: 50 MB over loopback, 0 loss, paced and unpaced. A lossy relay (5% loss, or a 3000/s or 800/s bottleneck with 10 to 30% NACK loss) completes with matching SHA-256 in 4 to 6 rounds. The same 3000/s link unpaced fails after 64 rounds with 1,169 chunks missing.
- Real link: 10 MB from xing-pc to the laptop over Tailscale on Wi-Fi completed in 22.3 s with 3 repair rounds and a matching hash (before: 93% loss, never completed).
- Limits: round 0 runs at the initial RTT-based rate (91% lost on the laptop link before the rate adapted), a new sender talking to an old receiver gets END_ACK with no repair, and the GUI sender (gui.rs) has no repair yet.
- realtime-translate (D:\dev, not a git repo): `--pause` (default 450 ms), and a `--llm` translation stage that sends text to Ollama on loopback only (gemma2:9b). Whisper retry fallback is now off by default. End to end, lines land 0.56 to 0.79 s behind with translation on. Patois accuracy is untested on real speech.
### 2026-10-29 (morning): QFLTL feedback, Øneiro panel, cue QUBO, waitlist page
- QFLTL: shared sender engine `qfltl-core/src/repair.rs` (pacing, mid-transfer rate feedback from receiver PROGRESS reports every 25 ms, NACK repair, END_OK verified completion), used by both `shred` and the Mission Control GUI. The GUI used to report "Mission Success" with no confirmation at all; it now repairs and only succeeds on a verified ack. A sender that gets END_ACK without END_OK (older receiver) reports NOT VERIFIED. Relay tests on xing-pc: 400/s Wi-Fi-like link, 10 MB in 22.3 s (link limit 21.8 s, previous build 38 s); 3000/s with 1% loss, 20 MB in 6.5 s with first-pass loss 8.5% (was 97.6%, resends 1,498 vs 17,530). 85 core + 59 crypto tests pass. Laptop rerun pending (laptop offline on Tailscale this morning).
- Øneiro: `phase_sync` gains `panel` (private night panel, binds the Tailscale IP only, refuses non-tailnet peers, reads VEYN's event log read-only) and `cue_qubo` (cue timing as a QUBO from Watch heart rate; exact Gray-code solver, annealing, LIMEN offline request). 13-variable run: exact = annealing = -27.87; LIMEN simulator p=1 QAOA +39.30 with gap violations. Data window was mostly awake (median HR 93), so it validates the pipeline, not the cue times.
- Øneiro website: `website/waitlist.html` (Watch-first product page with waitlist form, no health claims, not deployed; form endpoint empty so it stores nothing).
## Status update (October 31, 2026, late)

Harness tightening pass (working folders only, nothing committed):

- Root cause of tonight's mass sign-out and of three broken services: the DPAPI master key {91ae3965-5ca7-4ce8-8f18-66c7e8dea4ae} can no longer be opened (Crypto-DPAPI event 8198/8203 "Could not get the master key"). Every secret wrapped before tonight is unreadable: ductei-sync QFLTL identity (D:\brain\adapt\data\sync\qfltl\identity.seed.dpapi), video-watch gate signing key and anchor identity (D:\dev\video-watch\ductei\*.dpapi), xing-pc-mcp bearer tokens (config token_dpapi). Rotation is a cross-machine decision (laptop pins), left to Xing.
- ductei-router: T0 brain host now follows DUCTEI_T0_HOST / OLLAMA_HOST and must be an address of this machine (was hardcoded 127.0.0.1 while OLLAMA_HOST points at the Tailscale IP, so T0 silently fell back). classify now returns {task, brain, fallback}; fallback is criticality 3 and visible. One shared decide() for CLI, MCP and hook. canon.toml (roots, extra denied drives; C can never be unbanned) and sign.toml in the state dir. Receipts are now ML-DSA-65 signed (DPAPI key, fingerprint pinned), chain continued from the old unsigned receipts. New verify subcommand.
- New ductei-hook (Claude Code PreToolUse): Write/Edit/Bash mapped to effects, C: writes deny, commit/push/delete ask, approvals via D:\dev\ductei-router-state\approvals.txt. Registered in ~/.claude/settings.json; router MCP re-registered to the signed release build.
- Ops: Ollama logon task (ollama-serve); xing-pc-mcp task started (Funnel answers again, 401 = auth); nightly consolidated: limen-ductei-nightly-smoke now runs D:\ATRIUM\ops\nightly.ps1, limen-nightly-smoke disabled (not deleted), tier0_probe.py copied to D:\ATRIUM\ops. Task XML backups in D:\ATRIUM\ops\logs.
- Open: DUCTEI feat/ductei-router local branch diverged from origin (28 ahead / 16 behind, rebased history); laptop offline so its loop node is unverified; Ollama app currently listens on :: (firewall rules scope it), the logon task will bind the Tailscale IP only.

## Status update (November 1, 2026): QHR fast lane

- ductei-mcp receipts gained a durability knob (`log.sync_every` / `log.sync_max_ms`; default = fsync every receipt, the verified tier).
- Bench on xing-pc (release, 2000 iterations): per-receipt fsync p50 0.52 ms / p99 0.99 ms; batched p50 0.002 ms; ML-DSA-65 sign p50 0.39 ms, verify p50 0.12 ms.
- Anchors now fsync before asserting a head; gate flushes at exit.
- ductei-router: receipt-log loader no longer spins on a repeating read error (filter_map -> map_while).
- Lint-clean: ductei-mcp, ductei-core, ductei-qallow, ductei-router. Uncommitted.
- ductei-mcp: optional background fsync thread for the fast lane (log.sync_background); QHR fast tier p99 0.55-0.75 ms -> 0.19 ms (p50 0.10 ms, floor 0.06 ms). New crate ductei-qhr: gated, deadline-bound local job plane with predicted-miss pre-flight; 11 tests.

## 2026-11-01 QHR lane weighting (uncommitted, D:\ATRIUM\DUCTEI\ductei-qhr)
- Weighter trait (uniform | evidence | jlens flagged off) scores pre-flight survivors; pick_why bound into receipt hash.
- ductei-qhr: 22 tests pass (11 core, 4 qpu, 7 weight), clippy clean; workspace green.
- Bench (release, xing-pc): fast-bg submit p50 0.110 ms p99 0.229 ms; verified p50 1.036 ms (USB fsync floor). No regression from the candidate scan.
- Next: run jspace-lab readout PREREG; flip jlens on only if it passes.

## Status update (November 1, 2026, early morning): timeline branch, rotation, self-heal

Everything above is now committed. Branch of record: `timeline` (DUCTEI merged feat/ductei-router into it, player-harness cut it from main, ATRIUM already on it).

- DUCTEI router MCP is extensible (`ToolExt`); `qhr serve` runs one stdio server with decide / classify / outcome / qhr_submit / qhr_verify. Smoke over stdio: cpu.sha256 job done in 938 us with a receipt; decide allowed a private read.
- Rotation after the DPAPI master-key loss (all in-stack, old material kept in `retired-20261101` folders next to the live dirs):
  - ductei-sync identity: new, fingerprint 6e1862bf38dfc10b5a5435f80ad04a10f74033d388bd2672f09723e07e9c39c6. Laptop must re-pin (`ductei-sync key-pin <laptop state> --role server --peer <xing-pc addr>:7420 --file identity.pub --replace`, and as client). Laptop offline at the time.
  - video-watch gate: new ML-DSA-65 key 73095f9240c66c5588523913b226baa38636c79c8cdd5fbb67e108af2a930944, pinned in ductei-mcp.toml; fresh chain. Old chain verifies with retired pk but already had prev-hash mismatches at seq 22148 and 74205 (pre-existing). Anchor-state identity regenerated; anchor peers must re-pin it.
  - xing-pc-mcp tokens: rotated Oct 31, connector links pasted by Xing.
  - GitHub credentials on xing-pc were also DPAPI-backed and are gone: `gh auth login -h github.com -w` needed before xing-pc can push. player-harness timeline was pushed from the cloud session; DUCTEI and ATRIUM pushes wait on auth (ATRIUMRnD org has no Claude GitHub App).
- Bug found and fixed: ductei-mcp-gate `restrict_acl` applied (OI)(CI) grants with /T, which on files is inherit-only and left every log file with an empty ACL. Now restricts the dir and resets children. The gate dir had been owned by Administrators (icacls could not touch it); recreated as xing-owned.
- xing-pc-mcp self-heal: an unreadable keystore token no longer kills the server; it rotates the failed token(s), saves, writes `healed.json` (no secrets) and `status` reports it; `heal-status` / `heal-ack` CLI. Tested against dead blobs: heals once, second load clean.
- player-harness moved to D:\dev\player-harness; xing-pc-mcp default roots are now D:\dev and D:\ATRIUM.
- Parked for the Mac mini: Oneiro cue timing fixes from the Oct 20-31 sleep comparison (HR gate treats REM as awake; cue 3-5 min after first REM detect; cap ramp at level 5).
- Still pending: qhr-cell / qhr-sched patch (built in a cloud session, file not on xing-pc; needs the .patch attached), MCP registration of `qhr serve` in Claude Code (C: file, Xing's call).

## 2026-11-01 05:42 status: QHR branch weighter + QPU lane in nightly
- qhr-sched/src/weight.rs (new): BranchWeighter (Uniform, Prior, flagged JLens), Scheduler::superpose_weighted, Prior::replay rebuilds the learned prior from DUCTEI receipts after verifying the chain (tampered or wrong-genesis chains teach nothing). BranchReceipt gains optional class (skipped when None, old receipt hashes unchanged). qhr-sched 27/27, clippy clean.
- ops/qhr_qpu_probe.ps1 (new): qhr submit qpu.run -> limend -> cert, 257 to 504 ms wall on CPU statevector; 65-var job refused in pre-flight, never spooled; receipt chain verifies.
- ops/nightly.ps1: steps F (qhr_qpu) and G (qhr_sched) added. Full run ALL PASS (7/7).
- Caveat: 4-var p=1 QAOA put 0.178 on the optimum (target 0.9). The probe gates on the lane contract, not QAOA quality.
- Uncommitted. J-lens stays off until jspace-lab PREREG passes.

## 2026-11-01 06:16 status: QPU lane cache, prefetch, exact check
- ductei-qhr/src/qpu.rs: offline-result cache (spool/qhr-cache, sha256 over the request minus job_id, terms sorted), prefetch (spools under prefetch-<key16>, next run adopts the cert), exact check (Rust brute force up to 16 vars: exact_energy, exact_assignment) and shots_to_optimum_99 from limend's success_probability. CertSummary unchanged. Hardware (non-offline) requests never cached.
- qhr prefetch --spool <dir> --request <file> (new CLI).
- Live vs real limend: miss 241 ms, hit 2.2 ms inside QHR incl. gate and receipt (14 ms wall with process start). Star QUBO exact optimum -3 (hub=0, leaves=1); p=0.178 means 24 shots find it at 99%.
- Tests: ductei-qhr qpu 10 (6 new), workspace 170 pass, clippy clean, nightly ALL PASS 7/7 (F now checks exact + cache hit).
- Limit: lane health() still checks the limend backlog before preflight, so a cache hit is refused if limend is down. Uncommitted.


## Status update (November 1, 2026): atrium, one loop

- New crate `DUCTEI/atrium-ctl` (binary `atrium`, installed at `D:\dev\bin\atrium.exe`): `up | status | verify | down` over `D:\ATRIUM\ops\atrium.toml`. 6/6 tests, clippy clean.
- Rooms: ward (timeline freshness), ductei-node (managed, replaces the disabled `ductei-node-watchdog` task), ductei-sync, xing-pc-mcp, loop (`atrium_up.py`, managed, offline simulator). All five up live.
- One chain: every room state change plus a beat per 60 ticks is a receipt on `ops\atrium-state\receipts.jsonl`, ML-DSA-65, DPAPI key, pinned fingerprint `f46ba76e...b0a3`. Reuses `ductei_mcp::ReceiptLog`.
- `atrium verify`: qhr chain, gate chain (5621 receipts), ductei e2e smoke, ward I0..I7, own chain. 5/5 PASS.
- Proven live: killed ductei-node, back in under 6s; second `up` and live `verify` refuse (75).
- Root cause found: `ductei-node supervise` stops for good after 3 restarts/hour and its shell stays alive, so the old watchdog reported healthy while the node was dead since 02:24.
- Task `atrium` registered at logon. Open: anchor the atrium chain to an off-box peer (laptop offline 3 days); nightly still runs separately because verify needs the supervisor down.

- Later Nov 1: nightly folded into atrium. Task `limen-ductei-nightly-smoke` now runs `ops\atrium-nightly.ps1`: down, verify (qhr, gate, nightly A..G, ward, atrium chain), up via task `atrium`. Real task run 07:41: 5/5, task result 0, nightly 7/7 in 3 min.
- Bugs found and fixed: (0) `atrium down` left `atrium_up.py`'s children running; 28 orphans from four loop copies shared one LMDB store. Managed rooms now live in kill-on-close Job Objects (tests: grandchild dies on shutdown and on supervisor drop). (1) The wrapper hung because the background supervisor inherited its stdout pipe; it now starts through the task. (2) Gate verify had a read-order race on a live gate (2/15 false failures); receipts are now counted before the channel is read, 0/15. (3) Gate verify matched channel entries by hash anywhere, so deleting a repeated entry (12 distinct calls over 6,466 polls) went unseen; it now checks the entry at the receipt's own Lamport stamp (new test, 36/36).
- The live video-watch gate runs from `DUCTEI\target\release`, which locks that build path; installs now build into `target\bin-install`.
