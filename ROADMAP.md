# ROADMAP.md — ATRIUM build order

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

## Task 1: CI harness — DONE (August 29, 2026)
- Detail: harness-roadmap/01-ci-harness.md
- Repos: DUCTEI (primary), LIMEN (pinned rev, simulator path)
- Deliverable: checked-in smoke script + GitHub Actions job replaying the
  four scenarios per push, with invariant checks as assertions — DONE,
  DUCTEI@7433317, verified green on main:
  https://github.com/xingxerx/DUCTEI/actions/runs/29975187862
- Done when: the e2e job is green on main in DUCTEI (met) and required
  for merge (met, August 29 2026). DUCTEI `main` now requires status
  checks [test, e2e-smoke, e2e-smoke-qallow, e2e-smoke-veyn], strict,
  no force-push, no deletions. ATRIUM `master` is also protected
  (pull-request reviews, no force-push, no deletions).
- Unlocks: Tasks 2 and 3 are safe to build; regressions caught same day.
- Does not unlock: any new runtime capability

## Task 2: Spine flow (Qallow, VEYN pairs) — NETWORK TRANSPORT REMAINING (July 22, 2026)
- Detail: harness-roadmap/02-spine-flow.md
- Repos: DUCTEI, Qallow, then VEYN (rev-pin bump per landing)
- Order inside the task: Qallow pair first (conformance oracle exists) —
  DONE, DUCTEI@2cb9e68 + Qallow@0a546b3. VEYN pair second (producer of
  trace events) — DONE, DUCTEI@5d750b8 + VEYN@1913f41. Network
  transport last — NOT STARTED.
- Done when: three real consumers flow through DUCTEI under the five
  invariants, each with its four smoke scenarios in CI — MET for all
  three (LIMEN, Qallow, VEYN), all green on DUCTEI main:
  https://github.com/xingxerx/DUCTEI/actions/runs/29976969294
- Unlocks: DUCTEI becomes the actual spine (true in fact now, not just
  design); precondition for Tasks 3 and 4
- Does not unlock: self-modification; flow is still human-initiated

## Task 3: Self-improvement loop — UNBLOCKED (Task 1 DONE August 29, 2026)
- Detail: harness-roadmap/03-self-improvement-loop.md
- Repos: LIMEN (primary), DUCTEI
- Scope guard: gates routing-policy changes ONLY at first
- Loop: ledger baseline -> proposal with claim file -> CI verdict via
  report.py in plain English -> branch-protected merge -> ledger learns
- Done when: one real routing improvement has traversed the full loop and
  the transcript is written up
- Unlocks: the research result itself; every future improvement generates
  its own evidence trail
- Does not unlock: gating arbitrary code; widening scope stays deliberate

## Task 4: Qallow kernel-level invariants — LONG HORIZON
- Detail: harness-roadmap/04-qallow-kernel-invariants.md
- Repo: Qallow (primary)
- Includes: aarch64 mobile clone, invariant checks in
  ql_persist_merge_blob(), C conformance suite, session bounds in the
  LMDB schema, sovereignty audit
- Done when: each invariant has a C-level test that fails on violation,
  and the sovereignty audit answers "none" for core-loop dependencies
- Unlocks: invariants become un-bypassable from above; harness state
  travels on-device
- Does not unlock: new capabilities; never preempts Tasks 1-3

## Rules of the table

- One task ACTIVE at a time. An agent proposing work on a BLOCKED task
  must first show the blocker is resolved.
- Completing a task means updating this file (status flip + date) in the
  same PR as the final change, per the witness requirement.
- Any change to task scope or order is proposed here first, in plain
  English, before implementation anywhere.
