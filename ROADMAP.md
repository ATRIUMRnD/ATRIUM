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

## Rules of the table

- One task ACTIVE at a time. An agent proposing work on a BLOCKED task
  must first show the blocker is resolved.
- Completing a task means updating this file (status flip + date) in the
  same PR as the final change, per the witness requirement.
- Any change to task scope or order is proposed here first, in plain
  English, before implementation anywhere.