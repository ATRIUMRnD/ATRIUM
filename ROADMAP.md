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

## Task 4: Qallow kernel-level invariants - ACTIVE (August 29, 2026)
- Detail: harness-roadmap/04-qallow-kernel-invariants.md
- Repo: Qallow (primary)
- Proposal (before Qallow code): what changes is C-level enforcement in
  Qallow only (persist gate, LMDB session bounds, conformance tests,
  aarch64 target, sovereignty audit). ATRIUM only witnesses. Unlocks
  unbypassable invariants and on-device state. Does not unlock new
  capabilities.
- Audited live on Qallow main: ql_persist_merge_blob() already does
  arg/size checks, LWW merge, tombstones. It does not yet enforce
  bounded sessions in the record layout, deny-by-default scopes, or
  credential-non-representability. test_sync_wire.c and
  test_persist_lmdb.c exist. Makefile has no aarch64 target.
- Done when: each invariant has a C-level test that fails on violation,
  and the sovereignty audit answers "none" for core-loop dependencies
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
