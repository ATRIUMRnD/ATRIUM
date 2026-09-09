# Task 7: One-app integration proof

Status: QUEUED (proposed September 8, 2026). Owners: VEYN harness +
DUCTEI scripts; ATRIUM witnesses. Not started until its predecessors
flip to DONE (one task ACTIVE at a time).

## What changes

No runtime code in ATRIUM. A single-command end-to-end scenario in the
existing harness style (sibling of scripts/smoke_qallow.py and
scripts/smoke_veyn.py):

sensor event -> VEYN bridge -> DUCTEI -> qallow ingest -> LMDB ->
GET /export snapshot, plus the REM rising-edge path
(/oneiro/watch -> rem_detected -> Qallow snapshot).

Cross-machine transport is included only if Task 6 has landed by then;
otherwise the scenario runs local-first and the transport upgrade is a
follow-up scenario, not a scope change.

## Done when

The scenario replays in CI green, asserting the five invariants inline
like every other smoke script.

## Unlocks / does not unlock

Unlocks: "everything works as one app" becomes a demonstrated,
regression-gated property rather than a claim. Does not unlock: the
Øneiro charge path (owned by oneiro_website, private, outside this
workspace).

## Progress (September 8, 2026) - built on-box, nothing committed

Built ahead of Tasks 5 and 6 on explicit user direction. Runs
local-first on QSW v1 exactly as the scope above allowed.

- **Blocker found and repaired first.** Qallow main (Task 4) rejects
  every frame DUCTEI's relay emitted: `schema_ver` 1 and a raw blob with
  no bounded session. DUCTEI's own CI never saw it because its Qallow pin
  (42ce93f) predates Task 4. Repair, DUCTEI-owned:
  `ductei-qallow/src/persist.rs` (persist-v2 header: `u16 ver=2 |
  u16 scope_code | u64 session_id | u64 session_bound | u32 len | data`),
  used by ductei-qallow-relay so the bounded session it already used
  (`max_envelopes = 1`) is what lands on disk. Wire frame (QSW v1)
  untouched; conformance oracle unaffected. `cargo test --workspace`
  green; smoke_qallow (17 checks) and smoke_veyn (13 checks) green
  against Qallow main. ci.yml Qallow pins bumped to a05392e.
- **The arrow that was missing: mind -> hands.** Qallow gains
  `qallow propose <store> <key> <spool>` (qallow_cli/src/propose.rs,
  self-contained; main.rs gains the subcommand). From durable state
  only: unpersisted key -> exit 1, nothing written; non-REM record ->
  "no proposal", exit 0; REM cue (scope `veyn.rem_event`, value 1.0) ->
  atomic write of an `offline: true` LIMEN route request into
  `spool/pending/`, job id derived from the LMDB key, idempotent across
  pending/done/certs/sent/failed. Four unit tests.
- **The proof.** DUCTEI `scripts/smoke_loop.py`: VEYN bridge (real
  DucteiBridge path) -> ductei-qallow-relay -> one shared `--store-dir`
  -> `qallow propose` -> limend -> ductei-limen-relay ->
  ductei-qallow-relay -> the same store. Scenarios: good cue, restart
  replicability (every process fresh, node ids and cursors intact, no
  re-proposal), malformed/non-cue (HRV never proposes, unpersisted key
  never proposes, bad spool request witnessed in failed/, bad event
  aborts the bridge), invariants (sentinel absent from every log, frame,
  spool file and LMDB value; exact scopes per hop; sent/ == accepted
  lines; zero persist-gate rejections). 41 checks, all green on-box.
- **The launcher.** DUCTEI `scripts/atrium_up.py`: one command boots
  limend, ductei-limen-relay, both ductei-qallow-relay hops (shared
  store) and optionally the real veyn-core daemon (mock adapter + OSC,
  DUCTEI bridge logging into the loop's root), runs the proposer
  in-process with a persisted cursor, and can fire an Øneiro-shaped
  `/oneiro/watch` OSC cue (stage N2 -> REM, rising edge). Verified:
  `--once --inject` closes the loop in ~2 s (backend ibm_fez, tier 1,
  fidelity 0.933 on the offline path); `--veyn-daemon --osc-cue` closes
  it live through the real daemon in ~4 s, with the proposer correctly
  deferring once until the cue was persisted.
- **CI.** DUCTEI job `e2e-smoke-loop` added. It needs a Qallow rev that
  carries `propose`; bump the pin in a separate commit once Qallow's PR
  lands, then make the job required. Until then it is expected red and
  not required.
- **Not done / not in scope.** Qallow's FastAPI reasoning server (the
  `GET /export` target of the REM snapshot) is not in the Qallow tree, so
  the qallow-veyn-bridge `ws` path is not part of the launcher. Nothing
  is committed or pushed (AGENTS.md section 6). Landing order: DUCTEI
  persist-v2 + smoke_loop + atrium_up first; Qallow `propose` second
  (Task 5's v2-decoder work is in the same Qallow tree, uncommitted, and
  untouched by this); then the DUCTEI pin bump; then ATRIUM flips this
  task to DONE.
