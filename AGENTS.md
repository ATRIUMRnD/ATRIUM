# AGENTS.md - ATRIUM workspace rules

ATRIUM is the receiving chamber of the ecosystem: work enters here and is
routed onward to the room that owns it. No code lives in ATRIUM itself,
only rules, routing, and roadmap documents. The repos are the rooms.

This is a multi-root workspace (DUCTEI, LIMEN, Qallow, VEYN) plus one
application: Øneiro. Agents working here act as routers, not owners:
classify the change, send it to the owning repo, respect the gates. The
workspace mirrors the system it builds: jobs are routed the way LIMEN
routes QUBOs, and changes travel the way DUCTEI carries envelopes.

## 1. Ownership map (route work like LIMEN routes QUBOs)

| Concern | Owner repo | Notes |
|---|---|---|
| Quantum routing, fidelity tiers, budget router, certificates, ledger (memory.py), reports (report.py), limend daemon | LIMEN | Python. Never call fidelity/budget routing "Lumen". |
| Envelopes, relays, sessions, spool consumers, invariant enforcement (Rust), ductei-limen-relay | DUCTEI | Channel, not merger. |
| Persistence, LMDB, sync wire format, C-level enforcement, ql_persist_merge_blob() | Qallow | sync_wire.c is the conformance oracle. |
| Tracing, structured JSON logging, daemon/sensory-motor | VEYN | https://github.com/xingxerx/VEYN (private). Sensory-motor bus. Intero lives in this tree. |
| Lucid-dreaming instrument, watch REM cue, haptic, cohort site | Øneiro | Private `xingxerx/oneiro` + `xingxerx/oneiro_website`. Commercial door. Not a mesh room. |

SYNOID and Grand_Cross are outside this map. Do not route mesh work into them.

Cross-repo changes land in dependency order: DUCTEI first, then bump the
pinned rev in veyn-core/Cargo.toml as a separate, auditable commit. The
same rev-pin discipline applies to any CI pin of LIMEN inside DUCTEI.

A change that cannot be classified into exactly one owning repo is split
into per-repo changes before any work starts.

## 2. Hard invariants (DUCTEI layer - never negotiable)

1. LIMEN credentials are never representable in sync payloads. Env vars only.
2. Broadcast scopes are deny-by-default.
3. Byte-level wire compatibility with Qallow's sync_wire.c.
4. Persistence before ack, always, fsynced.
5. Sessions are bounded. One bounded session per job. Unbounded sessions
   must remain unrepresentable in the type system.

An agent that cannot complete a task without violating one of these stops
and reports instead of working around it.

## 3. Qallow governance (state layer)

- ql_persist_merge_blob() is the final gate: validate before write, reject
  malformed blobs before they touch LMDB.
- Lamport ordering and node identity are daemon-owned; agents never
  fabricate or reset counters.
- Restart replicability is a required property: any change to persistence
  must survive the kill-and-restart scenario from the smoke suite.

## 4. CANON governance (conduct layer)

- **No punishment realms.** Failed jobs, proposals, and runs are witnessed
  and archived (failed/), never deleted, never retried punitively. Failure
  records must be human-readable.
- **No adversary figures.** Agents do not frame components, repos, or prior
  work as enemies to defeat. Malformed input is handled, not fought.
- **Threshold protocol.** Every trial is bounded: bounded sessions, bounded
  pauses (a paused feature stays paused, e.g. Azure/Atom, do not
  resurrect it), and every gate names its threshold in plain English
  before the trial runs.
- **Witness requirement.** Nothing changes state silently. Every merge,
  failure, and verdict leaves a plain-English record (report.py is the
  reference implementation).
- [SLOT - remaining CANON invariants to be added on review]

## 5. Change gating (harness discipline, applied to agents now)

- Agents propose; the harness disposes. No agent merges to main in any
  repo without that repo's tests passing, and (once the CI harness lands)
  the end-to-end smoke job.
- A proposal states, before work starts: what changes, which repo owns
  it, what it unlocks, and what it does not unlock.
- Scope stays minimal: no drive-by refactors outside the owning repo.

## 6. Standing prohibitions

- No hardcoded credentials anywhere. Ever.
- Package name is limen-compiler, never limen, in all docs and manifests.
- No commiting, pushing, or merging without explicit user instruction.
