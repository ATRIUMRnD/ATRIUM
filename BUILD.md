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

## Status update (October 24 2026)

Checked on xing-pc against repo history (all three at origin/main, no
unpushed commits): DUCTEI d951787, LIMEN 1332973, Qallow 218fb76.

- GAP 4 (network transport): STARTED. DUCTEI@d951787 gates the transport
  behind the `net` feature and adds send_local_first(), so offline
  degrades to the local Channel by construction. Restart replay,
  malformed-frame rejection, and local-first degrade are tested. CI runs
  default, `net`, and `pq`. No multi-node deployment yet.
- GAP 6 (self-improvement loop): BUILT in LIMEN@1332973, scoped to the
  CRITICALITY_SPREAD_THRESHOLD tunable. One demonstration proposal is
  archived. The policy-proposal-verdict job exits non-zero on rejection;
  it is a required check on LIMEN main (confirmed October 24 2026). Extending the gate to
  ATRIUM's own AGENTS.md / ROADMAP.md is not done.
- GAP 0 and GAP 5 branch protection: DONE, confirmed by the owner October
  24 2026 (DUCTEI main, LIMEN main, ATRIUM master; see ROADMAP.md).
- LIMEN working tree has uncommitted edits (budget_router, certificate,
  pipeline, report, CHANGELOG, tests, plus a new tests/test_ecc_term.py).
  Not touched by this doc update.
- VEYN (D:\VEYN) history now starts at 69f7c5d (2026-08-29), so older
  VEYN commit refs in section 5 are historical only.

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

### GAP 3 — DONE. VEYN pair (in-process bridge, not a separate relay)
Repos: DUCTEI, VEYN
- Audit premise was stale. VEYN's dispatcher writes a DUCTEI channel
  in-process (veyn-core/src/ductei_bridge.rs). There is no
  ductei-veyn-relay binary; smoke_veyn.py reuses ductei-qallow-relay as
  the second hop. Required CI job e2e-smoke-veyn is green on DUCTEI main.
- OSC rem path on VEYN/Qallow main 2026-08-29: VEYN OSC 9000 ingests
  /oneiro/state and /oneiro/watch, rising-edge rem_detected; Muse /muse/*
  unchanged. Example veyn.toml sets adapters.eeg = true; compiled default
  stays eeg_enabled=false. Qallow snapshots via GET /export on
  rem_detected (value 1.0); sleep_stage==3.0 is legacy. See section 5.

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
3. GAP 3 (DONE: in-process DucteiBridge + smoke_veyn.py + e2e-smoke-veyn)
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

## 6. GAP 4 network smoke, October 24 2026 (xing-pc leg only)

Test binary: ductei-core/src/bin/ductei-net-smoke.rs (feature `net`,
working folder only, not committed, not yet through a pull request).
Run: `ductei-net-smoke scenarios 100.106.147.75:7710` on xing-pc.

- Result: 14 of 14 checks passed across five scenarios: good session,
  scope denial plus stale rejection, restart replay (listener process
  killed and restarted over the same log), malformed frame (listener
  survived, log untouched), and unreachable peer (degraded to the local
  channel).
- Honest scope: both nodes ran on xing-pc, as separate processes, over the
  Tailscale address 100.106.147.75. Traffic to a machine's own Tailscale
  address does not cross the physical network, so this is not yet a
  two-machine result. The xing-laptop leg (listener on xing-pc, sender on
  xing-laptop) is pending.
- Not covered: transport security (the wire has no encryption or
  authentication of its own; the tailnet provides both) and the `pq` key
  exchange.

- xing-laptop leg, partial (October 24 2026): sender on xing-laptop
  (100.121.3.55), listener on xing-pc (100.106.147.75:7710). Good envelope
  delivered over the network and persisted on xing-pc (listener log shows
  the connection from 100.121.3.55; remote log holds key laptop-a,
  lamport 10). Out-of-scope envelope denied by the peer and not written.
  Not yet run from the laptop: stale rejection, restart replay, unreachable
  peer fallback.

- xing-laptop leg, complete (October 24 2026): sender on xing-laptop,
  listener on xing-pc over the tailnet. Good envelope delivered and
  persisted. Out-of-scope envelope denied. Replayed envelope denied as
  stale. Listener killed: sender degraded to its local channel
  (LocalFallback) and the remote log was unchanged. Listener restarted over
  the same log: new envelope accepted, old envelope still denied as stale
  (gate rebuilt from the log), remote log holds laptop-a, laptop-c,
  laptop-e with nothing lost or duplicated.
- Observation: when the peer is unreachable and the envelope is already in
  the local spool, send_local_first returns the local Channel's
  StaleDelta error instead of LocalFallback. Correct for idempotence, but a
  retrying caller must treat StaleDelta from the local channel as "already
  spooled", not as a failure.
- Still not covered: transport security (the wire has no encryption or
  authentication of its own; the tailnet provides both) and the pq key
  exchange.

## 7. Heartbeat loop run, October 24 2026 (xing-pc and xing-laptop)

Binary: ductei-core/src/bin/ductei-node.rs (feature `net`, std only, working
folder only, not committed). Nodes: pc (100.106.147.75:7720) and laptop
(100.121.3.55:7720) over the tailnet, each under its own `supervise`
process. Timings were SCALED for the test: beat 2 s, SUSPECT after 6 s,
UNREACHABLE after 20 s (production plan: 30 s, 90 s, 300 s).

- S1 steady state: PASS, but only about 25 seconds, not the 10 minutes in the
  plan. Both nodes HEALTHY, counters advancing in both directions.
- S2 kill the laptop node process: PASS as healing, not as detection. The
  supervisor restarted it in about 1 s (counter resumed 19 -> 20), faster than
  the 6 s SUSPECT threshold, so the pc never noticed. That is the intended
  outcome of self-healing.
- S2b whole laptop node stopped (node and supervisor): PASS. pc reported
  SUSPECT at 7 s, UNREACHABLE at 21 s, and HEALTHY again at counter 34 after
  the laptop was started, resuming above its last counter (33).
- S3 crash storm: PASS. Four kills: three restarts, then the supervisor
  stopped and wrote a plain-English record to laptop\failed\ (nothing
  deleted, no retry).
- S4 partition: PASS, one-way. Instead of stopping Tailscale (which would cut
  the SSH route), a temporary outbound firewall rule on the laptop blocked
  connections to pc:7720. Laptop heartbeats were spooled locally, pc reported
  UNREACHABLE, the laptop still saw the pc as HEALTHY, and on removal pc
  reported HEALTHY again at counter 73. The 15 spooled heartbeats (58 to 72)
  were not forwarded, by design.
- S5 out-of-scope and malformed frames sent at the pc node: PASS. Scope
  denied (ack 0), malformed frame dropped with a witness line, same node
  process, inbox holds heartbeats only, laptop still HEALTHY.
- Cleanup verified: no ductei-node processes on either machine, no listeners
  on 7710 or 7720, temporary firewall rules removed (sshd-tailnet kept).

Findings to fix:
- No connect timeout in the transport. When the peer did not answer, the pc
  beat loop produced no "not reachable" line for about 20 seconds, so I
  infer connect() blocked. With three or more nodes one dead peer would
  delay beats to live peers and cause false SUSPECT. Candidate fix: use
  TcpStream::connect_timeout in TcpClient::connect (ductei-core transport).
- The 3-restarts-per-hour count lives in the supervisor's memory. If the
  supervisor itself is restarted the count resets. Persist restart times to
  disk before installing it as a scheduled task.
- Spooled heartbeats are never trimmed. Small (15 lines after the partition)
  but unbounded.
- Not done: watchdog scheduled tasks on either machine (owner decision:
  per-user or SYSTEM), a 10 minute steady-state run at production timings,
  the status page change, and the pull request.

Fix applied, October 25 2026 (connect timeout): ductei-core transport.rs now
connects with a 2 s bound (TcpClient::connect_timeout, CONNECT_TIMEOUT) and
bounds the ack wait at 5 s (ACK_TIMEOUT). New test
transport_connect_is_bounded_when_peer_does_not_answer. Measured on xing-pc
sending to the laptop port while nothing answered: old build 66.4 s before
falling back, new build 2.2 s, same LocalFallback result. Suites pass with
default features (15) and with net (21). Working folder only, not
committed. Side effect to know: if the ack times out after the peer already
persisted the envelope, the sender also spools it locally (at-least-once, a
possible duplicate). The ductei-node.exe still on xing-laptop is the old
build; redeploy it before the next run.


### Follow-ups, October 25 2026 (heartbeat loop)

Done on xing-pc:
- Restart history now survives a supervisor restart. The supervisor keeps unix timestamps in `restarts.txt` in the node folder, drops entries older than an hour on load, and counts them toward the limit of 3 per hour. Test: two kills, supervisor killed, new supervisor, two more kills. The node stopped on the 4th exit in total (it needed 5 without persistence) and wrote `failed\<unix>-restart-limit.txt`.
- `--html PATH` writes a small auto-refreshing status page (peer, state, age, last counter). Checked against an unreachable peer: shows UNREACHABLE in red and refreshes every second. If the swap-in rename fails (Windows file in use), it writes the page in place.
- Per-user scheduled task `ductei-node-watchdog` installed on xing-pc (logon trigger, runs hidden, production timings 30/90/300 s by default, install dir D:\dev\ductei-node, data in D:\dev\ductei-node\data). It starts and heartbeats.
- Test suite: 21 passed (`cargo test -p ductei-core --features net`).

Still open:
- The status page cannot be published yet: `C:\me\www` is admin-write only, and `tailscale serve --set-path` needs a Windows admin. One elevated command fixes it (grant the user modify rights on `C:\me\www`).
- xing-laptop was offline (last seen about an hour before the attempt), so the laptop install (`laptop-go.ps1`) and the 10-minute steady-state run (`steady-state.ps1`) have not run. Scripts are ready in `D:\dev\ductei-test`.
- Spool is still unbounded.