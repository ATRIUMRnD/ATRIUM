# Task 2: Spine flow (Qallow, VEYN pairs)

Status: BLOCKED ON TASK 1
Repos: DUCTEI, Qallow, then VEYN (rev-pin bump per landing)

## Blocker

Task 1's CI harness must be green and required on main in DUCTEI first.
An agent proposing work here must show that's true before starting.

## Goal

Grow the verified LIMEN <-> DUCTEI loop into a real spine: DUCTEI
carrying traffic from multiple real consumers, not just LIMEN.

## Order (do not reorder without proposing it in ROADMAP.md first)

1. **Qallow pair** — first, because Qallow's sync_wire.c is already the
   conformance oracle (../AGENTS.md section 1 and 3). Wiring Qallow
   through DUCTEI validates the spine against a byte-level contract that
   already exists, before adding a producer with no oracle of its own.
2. **VEYN pair** — second. VEYN is the producer of trace events
   (../AGENTS.md section 1: tracing, structured JSON logging). Bring its
   events through DUCTEI's channel once the Qallow pair has proven the
   spine holds under the five invariants.
3. **Network transport** — last. Only after both pairs work over
   whatever local/in-process transport DUCTEI already has; network
   transport is additive risk, not a precondition for correctness.

Each landing follows the rev-pin discipline: DUCTEI change first, then
a separate, auditable commit bumping the pinned rev in the consuming
repo (e.g. veyn-core/Cargo.toml for VEYN).

## Done when

- Three real consumers (LIMEN, Qallow, VEYN) flow through DUCTEI.
- All five hard invariants (../AGENTS.md section 2) hold for each.
- Each pair has its own four smoke scenarios (good job, restart
  replicability, malformed request, malformed cert) running in CI.

## Out of scope

- Self-modification. Flow stays human-initiated; nothing here proposes
  or merges changes on its own. That is Task 3.

## Open questions

- [SLOT — transport choice for the "network transport last" step]
- [SLOT — how VEYN's ductei_bridge (veyn-core/src/ductei_bridge.rs) maps
  onto this task's done-criteria smoke scenarios]
