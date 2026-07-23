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

## Task 1: CI harness — GREEN, PROTECTION PENDING (July 22, 2026)
- Detail: harness-roadmap/01-ci-harness.md
- Repos: DUCTEI (primary), LIMEN (pinned rev, simulator path)
- Deliverable: checked-in smoke script + GitHub Actions job replaying the
  four scenarios per push, with invariant checks as assertions — DONE,
  DUCTEI@7433317, verified green on main:
  https://github.com/xingxerx/DUCTEI/actions/runs/29975187862
- Done when: the e2e job is green on main in DUCTEI (met) and required
  for merge (NOT MET — branch protection needs an explicit owner
  decision; an agent cannot flip repo-wide merge requirements
  unilaterally). Owner: set required status checks [test, e2e-smoke] on
  DUCTEI main at https://github.com/xingxerx/DUCTEI/settings/branches,
  then flip this line to DONE.
- Unlocks: Tasks 2 and 3 become safe to build; regressions caught same
  day. Treating this as unblocked for GAP 2/3 build work below since the
  check exists and is green — the only missing piece is enforcement.
- Does not unlock: any new runtime capability

## Task 2: Spine flow (Qallow, VEYN pairs) — BLOCKED ON TASK 1
- Detail: harness-roadmap/02-spine-flow.md
- Repos: DUCTEI, Qallow, then VEYN (rev-pin bump per landing)
- Order inside the task: Qallow pair first (conformance oracle exists),
  VEYN pair second (producer of trace events), network transport last
- Done when: three real consumers flow through DUCTEI under the five
  invariants, each with its four smoke scenarios in CI
- Unlocks: DUCTEI becomes the actual spine; precondition for Tasks 3 and 4
- Does not unlock: self-modification; flow is still human-initiated

## Task 3: Self-improvement loop — BLOCKED ON TASK 1
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
