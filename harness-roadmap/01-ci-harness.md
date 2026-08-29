# Task 1: CI harness

Status: DONE (August 29, 2026)
Repos: DUCTEI (primary), LIMEN (pinned rev, simulator path)

## Goal

Turn the already-verified harness loop (limend -> ductei-limen-relay ->
certified, persisted, invariant-checked results) into something that
runs on every push, not just on a laptop by hand.

## Scope

- A checked-in smoke script in DUCTEI that replays the four scenarios
  already proven to pass:
  1. good job
  2. restart replicability
  3. malformed request
  4. malformed cert
- A GitHub Actions job in DUCTEI that runs that script per push and per
  PR against main.
- The job pins a LIMEN rev (simulator path — no live limend required)
  per the rev-pin discipline in ../AGENTS.md.
- Invariant checks (../AGENTS.md section 2) run as assertions inside the
  smoke script, not as a separate, skippable step.

## Out of scope

- No new runtime capability. This task proves what already works keeps
  working; it does not add scenarios, repos, or consumers. That is
  Task 2.

## Done when

- The end-to-end job is green on main in DUCTEI.
- The job is a required check for merge (branch protection).
- A regression in any of the four scenarios fails the job, same day it
  lands.

## Progress (July 22, 2026)

- DUCTEI/scripts/smoke_e2e.py written: replays all four scenarios with
  the invariants asserted inline (I1 credential sentinel + closed-type
  field drop, I2 exact-scope check, I4 sent/ implies accepted.jsonl,
  I5 one accepted line per job; I3 stays with the existing conformance
  job). All 21 checks green locally on Windows.
- e2e-smoke job added to DUCTEI/.github/workflows/ci.yml: ubuntu-latest,
  LIMEN checked out at pinned rev fd3f505 (env LIMEN_REV — bumping it is
  a deliberate commit), pip-installed base package only, no QPU extras.
- Remaining for done: push to DUCTEI, see the job green on main, then
  mark it required for merge in branch protection.

## Resolved questions

- Runner: ubuntu-latest (matches the existing test job).
- Simulator path: requests carry "offline": true, which budget_router
  forces onto Tier 0 local statevector simulation — no network, no QPU
  credentials. CI pip-installs the pinned LIMEN checkout (maturin builds
  limen_core; the Rust toolchain is already on the runner) and the smoke
  script invokes `python -m limen.limend <spool> --once` per pass — a
  real limend, run to completion, not a mock.

## Unlocks / does not unlock

See ../ROADMAP.md Task 1 entry — kept there as the single source of
truth so status and unlock notes don't drift between the two files.
