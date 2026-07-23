# Task 4: Qallow kernel-level invariants

Status: LONG HORIZON
Repo: Qallow (primary)

## Goal

Move invariant enforcement from "checked by agents/harness above" to
"unbypassable from above" — enforced at the C/kernel level inside
Qallow itself.

## Includes

- **aarch64 mobile clone** — a build target for on-device use.
- **Invariant checks in ql_persist_merge_blob()** — this function is
  already the final gate (../AGENTS.md section 3): validate before
  write, reject malformed blobs before they touch LMDB. This task makes
  that gate check the full invariant set, not just blob validity.
- **C conformance suite** — tests against sync_wire.c, the byte-level
  conformance oracle for wire compatibility with DUCTEI
  (../AGENTS.md section 2, invariant 3).
- **Session bounds in the LMDB schema** — bounded sessions is a hard
  invariant (../AGENTS.md section 2, invariant 5: "Unbounded sessions
  must remain unrepresentable in the type system"). This task makes that
  true at the schema level, not just in application code.
- **Sovereignty audit** — an audit of core-loop dependencies, answering
  plainly whether any external dependency exists in the core loop.

## Done when

- Each invariant listed above has a C-level test that fails on
  violation (not just an application-level check that can be skipped).
- The sovereignty audit answers "none" for core-loop dependencies.

## Out of scope

- New capabilities. This task hardens what exists; it does not add
  features.
- Never preempts Tasks 1-3. This is explicitly long-horizon and does not
  block or get blocked by the CI harness, spine flow, or
  self-improvement loop landing first.

## Open questions

- [SLOT — aarch64 clone build/test infra]
- [SLOT — exact invariant-to-test mapping for ql_persist_merge_blob()]
- [SLOT — sovereignty audit methodology and who signs off on "none"]
