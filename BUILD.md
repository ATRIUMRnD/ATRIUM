# BUILD.md — ATRIUM connection audit and build plan

Audited from live repos, July 22 2026:
DUCTEI @ 7433317, LIMEN @ fd3f505, VEYN main, Qallow main, ATRIUM master.

## Status update (July 22 2026, later same day)

GAPs 0, 1, 2, 3, 5 built and verified — see section 5 below for what
actually happened, including two things this audit didn't predict
(VEYN's bridge was already daemon-wired; VEYN's CI build needed system
packages the audit didn't flag). GAP 4 (network transport) landed
DUCTEI@d951787 (July 24). GAP 6 (self-improvement loop) landed
LIMEN@13329733 (July 24). Witnessed in ROADMAP.md August 29 2026.

## 1. What exists and is connected (verified in source)

| Connection | State | Evidence |
|---|---|---|
| limend -> ductei-limen-relay | BUILT + CI'd | limen/limend/ (daemon, spool); ductei-limen/src/bin/relay.rs; smoke_e2e.py replays all 4 scenarios with invariants I1-I5 asserted inline |
| DUCTEI e2e CI job | BUILT | ci.yml e2e-smoke job: clones LIMEN at pinned rev fd3f505 (= current LIMEN HEAD), pip installs simulator path, runs smoke_e2e.py |
| DUCTEI <-> Qallow wire conformance | BUILT + CI'd | ci.yml conformance job compiles Qallow's sync_wire.c against the Rust emitter per push |
| ductei-qallow codec | BUILT (library) | Real QSW v1 byte-compatible frames, ingest.rs, v2.rs. Not a mock. |
| ductei-veyn adapter | BUILT (library) | Real design: closed VeynEvent type (no credentials representable), deny-by-default scopes, CoalescePolicy throttling. Targets VEYN's MCP server on :7700. |
| Qallow persistence | BUILT | persist_lmdb.c with ql_persist_merge_blob(), test_persist_lmdb.c, test_sync_wire.c |
| VEYN test harness | BUILT | VEYN/harness/: Python daemon manager, scenarios, benchmarks, plugins |
| LIMEN memory in run path | WIRED | limend daemon.py takes a memory parameter through the loop (Milestone 1 write-back live in the daemon) |
| Ecosystem CANON/governance | PARTIAL | VEYN carries CANON.md + governance.json; other repos do not |

## 2. Gaps (ordered by build priority)

### GAP 0 — Verify, don't build: Task 1 close-out
Repos: DUCTEI, ATRIUM
- Confirm e2e-smoke is green on DUCTEI main (Actions tab).
- Make e2e-smoke a required status check on DUCTEI main (branch protection).
- Flip Task 1 status in ATRIUM/ROADMAP.md in the same PR that turns on
  the required check (witness requirement).
Unlocks: Task 2 formally unblocked; every later change regression-gated.

### GAP 1 — Stale VEYN rev pin
Repo: VEYN
- veyn-core/Cargo.toml pins ductei-core and ductei-veyn at rev dcd4657;
  DUCTEI HEAD is 7433317. The rev-pin rule says bumps are deliberate, but
  a pin this far behind means VEYN is building against a pre-relay DUCTEI.
- Build step: single audit commit in VEYN bumping both pins to 7433317,
  run VEYN's test suite against the new rev, note any breakage in the
  commit message.
Unlocks: VEYN sees the current envelope/session types; prerequisite for
GAP 3. Does not unlock new traffic by itself.

### GAP 2 — Qallow pair: codec exists, process does not
Repos: DUCTEI (producer side), Qallow (consumer side)
The bytes are compatible (CI proves it) but nothing moves them. Missing
both endpoints of the actual flow:
- **ductei-qallow-relay** (new bin in DUCTEI/ductei-qallow/src/bin/):
  mirror relay.rs exactly — spool consumption, persisted node id, one
  bounded session per delivery, sent/failed archiving, --once flag from
  day one. Payload: LIMEN certificates (from accepted.jsonl) framed as
  QSW envelopes via the existing codec.
- **Qallow ingest entrypoint**: a small C or CLI path (qallow_cli is the
  natural host) that reads QSW frames from a file/socket and calls
  ql_persist_merge_blob() per envelope. Reject-before-write per Qallow
  governance.
- **CI**: add the four smoke scenarios for this pair to smoke_e2e.py (or
  a sibling smoke_qallow.py), running the real C ingest binary built in
  the job (the conformance job already compiles Qallow sources, reuse
  that pattern).
Unlocks: certified quantum results land in durable LMDB state — the
harness gains long-term memory outside LIMEN's own ledger. Second real
consumer proves the pattern replicates.

### GAP 3 — VEYN pair: adapter exists, daemon wiring does not
Repos: DUCTEI, VEYN
- The ductei-veyn adapter converts VeynEvents but no process feeds it.
  Missing: **ductei-veyn-relay** bin subscribing to VEYN's local daemon
  (MCP :7700), applying CoalescePolicy, emitting scoped envelopes.
  Local only, per standing decision.
- VEYN side: expose a stable local event stream endpoint if :7700 does
  not already provide one consumable without MCP handshake overhead
  (check VEYN/harness/daemon.py for the test-harness pattern to reuse).
- Direction of first traffic: VEYN as producer (trace/sensor events in),
  with veyn.rem_event scoped for Qallow consumption later (Øneiro path).
- CI: VEYN's own harness/ already knows how to spawn the daemon; reuse it
  to run a smoke scenario in DUCTEI CI or VEYN CI (pick DUCTEI, where the
  other smokes live, cloning VEYN pinned by rev).
Unlocks: third consumer; DUCTEI is then the spine in fact. Sensor->state
path opens the Øneiro REM-trigger route without new plumbing.

### GAP 4 — Network transport (after 2 and 3)
Repo: DUCTEI
- Both pairs above run on local spools/sockets. Transport generalizes
  them; ML-KEM-768 under the pq feature is the scoped design.
- Do not start until two local pairs run in CI.

### GAP 5 — Governance propagation
Repos: DUCTEI, LIMEN, Qallow, ATRIUM
- VEYN has CANON.md and governance.json; nothing else does. Minimum:
  each repo's README links to ATRIUM as the constitution repo, and
  ATRIUM gains the branch protection it prescribes (currently pushable
  straight to master).
- LIMEN remains the CODE_OF_CONDUCT.md candidate (public, on PyPI).
Unlocks: the routing/invariant rules become discoverable from any room
of the house, and the constitution stops being self-exempt.

### GAP 6 — Self-improvement loop (Task 3, unchanged)
Repos: LIMEN, DUCTEI, ATRIUM
- Prereqs now met earlier than expected: memory is wired into the daemon
  loop, report.py exists, CI verdict infrastructure exists (GAP 0).
- Build per harness-roadmap/03: baseline snapshot API in memory.py,
  claim-file proposal format, verdict CI job comparing ledger metrics,
  branch protection making the verdict required, one real routing
  proposal through the loop as the demonstration transcript.
- Extend the gate to ATRIUM itself once running: changes to AGENTS.md /
  ROADMAP.md require the e2e check. The constitution becomes self-gating.

## 3. Build order

0. GAP 0 (minutes: verify green, flip protection, update ROADMAP.md)
1. GAP 1 (one audit commit in VEYN)
2. GAP 2 (the main build: ductei-qallow-relay + qallow_cli ingest + CI)
3. GAP 3 (ductei-veyn-relay + VEYN endpoint + CI)
4. GAP 5 can interleave anywhere (doc-only)
5. GAP 6 after GAP 0, parallel to 2/3 if desired (different repos)
6. GAP 4 last

## 4. Honest status line (as audited, superseded by section 5)

The harness exists and is CI-verified for one producer/consumer pair.
Two more pairs have their hard parts (codecs, adapters, persistence,
conformance) already built; what is missing everywhere is the same
thing: the small process at each end that actually moves the bytes,
plus the smoke scenarios that prove it. That is deliberate, bounded
work with a proven template (relay.rs). No unknown-unknowns surfaced
in the audit.

## 5. What actually happened

- **GAP 0**: e2e-smoke confirmed green on DUCTEI main
  (run 29975187862). Branch protection on DUCTEI `main` is now LIVE
  (August 29 2026): required checks test, e2e-smoke, e2e-smoke-qallow,
  e2e-smoke-veyn; strict; no force-push; no deletions. ROADMAP.md
  Task 1 flipped to DONE.
- **GAP 1**: VEYN's DUCTEI rev pin bumped dcd4657 -> 7433317
  (VEYN@da747ec). 71/71 tests passed against the new rev.
- **GAP 2**: built exactly as scoped — ductei-qallow-relay
  (DUCTEI@2cb9e68) mirroring relay.rs, `qallow ingest`/`qallow get`
  subcommands (Qallow@0a546b3) calling the real ql_persist_merge_blob(),
  scripts/smoke_qallow.py (4 scenarios / 17 checks), e2e-smoke-qallow in
  CI. Verified live on-box before CI: real LMDB round-trip via
  `qallow get`. Green in CI: run 29976231339.
- **GAP 3**: the audit's own premise ("adapter exists, daemon wiring
  does not") turned out to be stale — VEYN's dispatcher already writes
  directly into a DUCTEI channel in-process
  (veyn-core/src/ductei_bridge.rs, built in an earlier session, missed
  by this audit). No ductei-veyn-relay binary was needed; instead added
  a `[lib]` target to veyn-core (VEYN@1913f41) so a smoke harness
  (ductei_bridge_smoke) can drive the real bridge without booting the
  full daemon, then reused ductei-qallow-relay completely unmodified as
  the second hop. scripts/smoke_veyn.py: 4 scenarios tailored to this
  pair's actual architecture (good event, restart replicability,
  coalescing via the real CoalescePolicy, malformed input rejected) / 13
  checks. Two CI-only fixes needed along the way that the audit didn't
  predict: system dev packages (libdbus-1-dev, libasound2-dev,
  libudev-dev) for VEYN's BLE/audio/HID adapter crates, and surfacing
  cargo's stderr on build failure (the first fix attempt failed silently
  without it). Green in CI: run 29976969294.
- **GAP 5**: all four repos (DUCTEI, LIMEN, Qallow, VEYN) now link their
  README to ATRIUM as the governance/constitution repo. ATRIUM `master`
  branch protection is now LIVE (August 29 2026): pull-request reviews,
  no force-push, no deletions. LIMEN's CODE_OF_CONDUCT.md candidacy
  noted but not actioned (a future decision, not a build step this
  pass).
- **GAP 4**: DONE. DUCTEI@d951787 gates TCP transport behind `net`,
  local-first fallback on unreachability, CI covers default/`net`/`pq`.
  Green: https://github.com/xingxerx/DUCTEI/actions/runs/30066003124
- **GAP 6**: DONE. LIMEN@13329733 ledger-backed routing-policy proposal
  gate, demonstration transcript
  `2026-07-24-raise-criticality-threshold` ACCEPTED, CI job
  `routing-policy proposal verdict` green
  (https://github.com/xingxerx/LIMEN/actions/runs/30072093877). LIMEN
  `main` requires that verdict as of August 29 2026.

Every commit above is pushed to each repo's real `main`/`master` and
independently green on GitHub Actions — nothing here is a local-only or
simulated result.
