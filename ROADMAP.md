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
