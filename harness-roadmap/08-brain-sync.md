# Task 8: brain joins the mesh (correction sync)

Status: BUILT, awaiting merge (October 26, 2026). Owners: DUCTEI
(ductei-sync), Qallow (tombstone retention), brain (adapt/ sync).
ATRIUM witnesses. Built on the user's explicit direction ("build both
then connect everything to atrium"), ahead of Task 6, the same way
Task 7 was built ahead of Tasks 5 and 6.

## Promotion (AGENTS.md section 7 rule)

brain moves from "rooms outside the core mesh" to a producer on the
spine, for one concern only:

- Owning concerns: brain owns the correction store, retrieval, adapter
  ledger and the reasoning loop. It does no networking and no
  persistence of anyone else's state.
- What crosses the spine: correction and delete events from
  `adapt/data/corrections.jsonl`, scope `brain.correction`, key
  `brain/correction/<id>`. Nothing else (not adapters, not run logs, not
  model weights).
- Invariants that apply: I1 (no credentials: payloads are correction
  JSON from local sources only, `SOURCES = local-cli | brain-user`), I2
  (receivers allow `brain.correction` explicitly; anything else is
  ack 0 and witnessed), I3 (Qallow ingest uses the existing QSW v1 frame,
  oracle unchanged), I4 (push cursor moves only after the peer's
  persistence-first ack), I5 (each Qallow delivery is the relay's
  existing bounded session, max_envelopes = 1).

## What changes

1. **Qallow** `src/mind/persist_lmdb.c`: a winning tombstone is kept as
   a header-only record (flags 0x1, zero data) instead of `mdb_del`.
   Before this, a delete forgot the (lamport, node_id) that won, so an
   older write from another peer merged afterwards resurrected the key.
   `ql_persist_get` already reported tombstoned keys as absent, so the
   read contract is unchanged. New test in `tests/test_persist_lmdb.c`
   (fails on the old code, passes on the new).
2. **DUCTEI** new crate `ductei-sync` (feature `net`, off by default so
   the default workspace build stays network-free): the peer sync agent
   that Qallow's old task list called "Task 3, sync agent". It is
   transport, so it lives in DUCTEI per the ownership map. Full mesh,
   origin-only push over the existing TCP transport
   (`TcpClient` / `serve_connection`, persistence-first ack), a state
   directory contract shared with producers, an IP allow-list
   (required for any non-loopback bind), and witnessed refusals.
   `ductei-core` gains `TcpClient::connect_timeout` (additive).
   `scripts/smoke_brain_sync.py` is the two-node proof; CI gains
   `cargo test -p ductei-sync --features net`.
3. **brain** `adapt/src/sync.rs` + `brain-adapt sync
   export|import|run|status --state DIR`. Export turns new local events
   into DUCTEI envelopes; import applies peer events through
   `CorrectionStore::import_correction` / `import_delete`, which
   re-run every local `add` check plus: id and prompt hash recomputed
   from content, split must match this store's pinned heldout_frac (F5),
   F8 conflicts refused unless the peer had allowed them. Last writer
   wins per id on (lamport, node id), same rule as Qallow. Each machine
   keeps its own hash chain; imported entries carry `origin` and are
   never exported again. Refusals go to `adapt/data/sync-refused.jsonl`.
   New dependency: `ductei-core` pinned by git rev.

## Topology on the two machines

```
xing-laptop                                 xing-pc
brain-adapt sync export                     brain-adapt sync export
  -> sync/local/accepted.jsonl                -> sync/local/accepted.jsonl
ductei-sync push --peer <pc>:7420  ------->  ductei-sync serve :7420
                                              -> sync/inbound/accepted.jsonl
ductei-sync serve :7420  <-------  ductei-sync push --peer <laptop>:7420
brain-adapt sync import                     brain-adapt sync import
                                            ductei-qallow-relay (local + inbound)
                                              -> qallow ingest -> LMDB
```

Qallow on the laptop is optional; xing-pc is where the durable kernel
copy lives.

## Done when

- Qallow PR with the tombstone fix merged (lands first: DUCTEI's smoke
  depends on it).
- DUCTEI PR with ductei-sync, the smoke script and the CI line merged,
  with its Qallow pin bumped to the Qallow merge commit in a separate,
  auditable commit.
- brain commit with sync.rs merged (its ductei-core pin can stay on the
  current DUCTEI rev; brain uses only Channel/LogStore/scoped).
- One real run across xing-pc and xing-laptop over Tailscale.

## Verified (October 26, 2026, Linux cloud workspace, not on-box)

- Qallow `make test`: green, including the new tombstone-retention case;
  the same test aborts on the previous persist_lmdb.c.
- DUCTEI `cargo test --workspace` (default and `ductei-core/net`) green;
  `cargo test -p ductei-sync --features net` 3/3.
- brain `cargo test --workspace` green (27 + 34), 4 new sync tests.
- `scripts/smoke_brain_sync.py`: 17/17 checks across two nodes on
  loopback with the real binaries (brain-adapt, ductei-sync,
  ductei-qallow-relay, qallow). Against the old Qallow the
  "older add does not resurrect" check fails, as it should.

Not yet run on xing-pc or xing-laptop, and not over Tailscale.

## Unlocks / does not unlock

Unlocks: a correction made on either machine is retrieved by brain on
the other; xing-pc's Qallow store holds the merged, Lamport-ordered
correction history; brain is a real consumer of DUCTEI (DUCTEI had zero
real cross-machine consumers before this).

Does not unlock: shared LoRA adapters or run logs (not synced), live
shared inference, certificate pinning or pairing (Task 6), relaying
through a third node (origin-only push needs a full mesh), anything on
mobile (Qallow aarch64 clone untouched).
