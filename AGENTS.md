# AGENTS.md - ATRIUM workspace rules

ATRIUM is the receiving chamber of the ecosystem: work enters here and is
routed onward to the room that owns it. No code lives in ATRIUM itself,
only rules, routing, and roadmap documents. The repos are the rooms.

This is a multi-root workspace (DUCTEI, LIMEN, Qallow, VEYN, brain).
Agents working here act as routers, not owners:
classify the change, send it to the owning repo, respect the gates. The
workspace mirrors the system it builds: jobs are routed the way LIMEN
routes QUBOs, and changes travel the way DUCTEI carries envelopes.

## 1. Ownership map (route work like LIMEN routes QUBOs)

| Concern | Owner repo | Notes |
|---|---|---|
| Quantum routing, fidelity tiers, budget router, certificates, ledger (memory.py), reports (report.py), limend daemon | LIMEN | Python. Never call fidelity/budget routing "Lumen". |
| Envelopes, relays, sessions, spool consumers, invariant enforcement (Rust), ductei-limen-relay, ductei-sync (peer sync agent) | DUCTEI | Channel, not merger. All machine-to-machine transport lives here. |
| Persistence, LMDB, sync wire format, C-level enforcement, ql_persist_merge_blob() | Qallow | sync_wire.c is the conformance oracle. REM snapshot cue is GET /export on rem_detected (sleep_stage==3.0 legacy). |
| Tracing, structured JSON logging, daemon/sensory-motor, OSC ingest | VEYN | https://github.com/xingxerx/VEYN (private). Sensory-motor bus. Intero lives in this tree. OSC 9000 owns /muse/* plus /oneiro/state and /oneiro/watch (rising-edge rem_detected). |
| Correction store, retrieval, adapter ledger (adapt/), reasoning loop, J-lens gate | brain | Rust. `xingxerx/brain`. Spine producer for scope `brain.correction` only (ROADMAP Task 8). No networking or foreign state in brain: transport is ductei-sync, durable kernel copy is Qallow. |

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

## 7. Rooms outside the core mesh (registered October 24, 2026)

ATRIUM covers the whole software stack. The four repos in section 1 are
the routed core. The rooms below sit under ATRIUM but are not yet wired
through DUCTEI, so the section 2 invariants and section 3 governance do
not apply to them automatically. An agent working in one of these acts in
that room only, and a change that would touch a core repo is split out and
routed per section 1.

| Room | Path on xing-pc | What it is |
|---|---|---|
| Player Harness | D:\player-harness | Game-agnostic, brain-agnostic AI embodiment runtime (Rust) |
| jspace-lab | D:\jspace-lab | J-lens readout, intent gate, quantum vs pseudo-random sampling test |
| Self.Define | D:\self-define | Choice-driven Rust game with the QFL power system |
| SYNOID | D:\SYNOID | Application layer, outside the core mesh |
| Grand_Cross | D:\Grand_Cross | AI-driven NPC/narrative system with a Gemma DM engine |
| ward | D:\dev\ward | Security monitoring tool with sequential integrity checks I0 to I7 |
| VEYN | D:\VEYN | Core repo, but cloned outside the D:\ATRIUM folder |

Private rooms are listed in private/ROOMS.md on xing-pc (gitignored, never published).

brain left this table on October 26, 2026: promoted to the core mesh
as a `brain.correction` producer (ROADMAP Task 8). It still lives at
D:\brain on xing-pc and C:\Users\there\Code\brain on the laptop.

Standalone projects kept separate on purpose, not part of routing:
universal-clock (D:\dev\universal-clock) and the TI-84 algebra solver
(D:\dev\TI).

Rule for promotion: a room joins the core mesh only through a proposal in
ROADMAP.md that names its owning concerns, which invariants apply, what
it unlocks, and what it does not unlock.
