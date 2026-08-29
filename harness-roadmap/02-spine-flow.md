# Task 2: Spine flow (Qallow, VEYN pairs)

Status: DONE (August 29, 2026)
Repos: DUCTEI, Qallow, then VEYN (rev-pin bump per landing)

## Blocker

Resolved. Task 1 is DONE: e2e jobs green and required on DUCTEI main.

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

## Progress (July 22, 2026)

- **Qallow pair: DONE.** ductei-qallow-relay (DUCTEI@2cb9e68) tails
  ductei-limen-relay's accepted.jsonl, frames each envelope as QSW v1
  bytes, hands it to a real `qallow ingest` process (Qallow@0a546b3,
  qallow_cli/src/ingest.rs) which calls the real
  ql_persist_merge_blob() into LMDB. Verified live end to end on-box
  (value round-tripped via the new `qallow get` subcommand) and in CI:
  scripts/smoke_qallow.py, 4 scenarios / 17 checks, wired as
  e2e-smoke-qallow in ci.yml, green:
  https://github.com/xingxerx/DUCTEI/actions/runs/29976231339
- **VEYN pair: DONE.** No separate "ductei-veyn-relay" binary needed --
  VEYN's daemon already writes directly into its own DUCTEI channel
  in-process (veyn-core/src/ductei_bridge.rs, built in an earlier
  session and previously unnoticed by the GAP audit that proposed this
  task). Added a `[lib]` target to veyn-core (VEYN@1913f41) so a
  standalone example, ductei_bridge_smoke, can drive the exact
  production DucteiBridge::open/.forward path without booting the full
  async daemon. ductei-qallow-relay is reused completely unmodified as
  the second hop -- it only ever needed "a directory with an
  accepted.jsonl", never anything LIMEN-specific. Verified live on-box
  and in CI: scripts/smoke_veyn.py, 4 scenarios tailored to this pair's
  real architecture (good event, restart replicability, coalescing via
  the real CoalescePolicy, malformed input rejected) / 13 checks, wired
  as e2e-smoke-veyn, green:
  https://github.com/xingxerx/DUCTEI/actions/runs/29976969294
  (two CI-only fixes needed along the way: system dev packages
  libdbus-1-dev/libasound2-dev/libudev-dev for VEYN's BLE/audio/HID
  adapters, and surfacing cargo's stderr on build failure so the first
  of those wasn't a silent exit 101).
- **All three pairs done: DUCTEI is a spine in fact, not just in
  design.**
- **Network transport: DONE.** DUCTEI@d951787 (July 24 2026) gates the
  existing TCP transport behind a `net` feature (implied by grpc/quic)
  so a default build has zero network code. `transport::send_local_first()`
  tries the network, then falls back to the local persistence-before-ack
  Channel only on unreachability. CI runs the workspace suite with
  default features, `net`, and `pq`. Green:
  https://github.com/xingxerx/DUCTEI/actions/runs/30066003124

## Open questions

- Transport choice for the network step: resolved as the existing
  ductei-core TCP path, feature-gated (`net`), ML-KEM-768 still under
  `pq`. Not a new protocol.
- [SLOT — how VEYN's ductei_bridge (veyn-core/src/ductei_bridge.rs) maps
  onto this task's done-criteria smoke scenarios]
