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
  veyn-core daemon, �neiro-shaped /oneiro/watch OSC cue (N2 -> REM),
  rising-edge rem_detected, certificate in the shared store in ~4 s.
- Done when: e2e-smoke-loop is green on DUCTEI main and required - NOT
  YET (nothing here is committed; see AGENTS.md section 6).
- Unlocks: "everything works as one app" is a demonstrated, replayable
  property with one durable state, not a claim.
- Does not unlock: GET /export snapshot on REM (Qallow's FastAPI reasoning
  server is not in the tree; the qallow-veyn-bridge `ws` path stays
  optional), cross-machine transport (Task 6), the �neiro charge path.

## Rules of the table

- One task ACTIVE at a time. An agent proposing work on a BLOCKED task
  must first show the blocker is resolved.
- Completing a task means updating this file (status flip + date) in the
  same PR as the final change, per the witness requirement.
- Any change to task scope or order is proposed here first, in plain
  English, before implementation anywhere.
