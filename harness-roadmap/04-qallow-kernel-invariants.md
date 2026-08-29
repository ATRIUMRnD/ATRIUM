# Task 4: Qallow kernel-level invariants

Status: ACTIVE (August 29, 2026)
Repo: Qallow (primary)

## Proposal (AGENTS.md section 5)

Stated before Qallow application-code work starts.

- What changes: Qallow C persist/sync path only.
  `ql_persist_merge_blob()`, the LMDB record layout, `test_persist_lmdb.c`,
  `test_sync_wire.c`, an aarch64 Makefile target, and a sovereignty
  audit checked into Qallow. ATRIUM only updates this file and ROADMAP.md
  when a piece lands.
- Which repo owns it: Qallow.
- What it unlocks: the five DUCTEI-layer invariants become unbypassable
  from above, and harness state can travel on-device.
- What it does not unlock: new capabilities, new consumers, network
  features, or any LIMEN/DUCTEI/VEYN code.

## Goal

Move invariant enforcement from "checked by agents/harness above" to
"unbypassable from above", enforced at the C level inside Qallow.

## Live audit (Qallow main, August 29 2026)

- `ql_persist_merge_blob()` in `src/mind/persist_lmdb.c` already
  validates args, rejects oversize keys/blobs (`QLP_E_TOO_BIG`),
  last-writer-wins on lamport, memcmp node_id tie-break, tombstone
  delete on flags bit 0. It does not inspect blob contents for
  credentials, does not know about broadcast scopes, and does not
  store a session bound in the record.
- On-disk record: `u64 lamport | u8[QSW_NODE_ID_LEN] node_id | u16 flags | blob`.
  No session id, no TTL, no bound. Invariant 5 is not true at schema
  level yet.
- `tests/test_persist_lmdb.c` covers LWW, tie-break, tombstone, get.
  Missing: C tests that fail on credential-shaped payloads, unbounded
  session records, and oversize-session writes.
- `tests/test_sync_wire.c` exists and is wired in the Makefile. That is
  the start of the C conformance suite, not the finish.
- Makefile `TESTS` is `test_sync_wire` and `test_persist_lmdb`. No
  aarch64 / mobile clone target.
- `third_party/lmdb` is vendored. Good for sovereignty. README still
  describes Gemma/Ollama/FastAPI as a reasoning surface. The audit must
  classify those as outside the persist/sync core loop, or the answer
  is not "none".

## Includes (build order inside Qallow)

1. **C conformance suite** (already started). Expand `test_sync_wire.c`
   so every QSW v1 frame DUCTEI emits is accepted, and a mutated byte
   is rejected. Fail the test on violation, not skip.
2. **Session bounds in the LMDB schema.** Add a session field to the
   record layout (id + bound). Reject merge of a record with no bound
   or a bound of 0. Keep a version nibble or flags bit so old stores
   fail closed (QLP_E_IO / new status), never silently accept unbounded
   rows. Unbounded sessions stay unrepresentable.
3. **Invariant checks in ql_persist_merge_blob().** After schema decode,
   reject before write:
   - payload that could carry LIMEN credentials (env vars only; a
     reserved key prefix or well-known env-shaped field is enough to
     fail closed)
   - broadcast / open scope (deny-by-default)
   - missing or zero session bound
   Malformed blobs still reject before they touch LMDB.
4. **C tests that fail on violation** for each of those, in
   `test_persist_lmdb.c` (or a sibling). Same Makefile `test` target.
5. **aarch64 mobile clone.** Makefile target `test-aarch64` that
   cross-compiles the two test bins (`aarch64-linux-gnu-gcc` or
   equivalent) and, where a runner exists, executes them. Compile-only
   is the first landing; run-on-device is the same tests, not a new
   capability.
6. **Sovereignty audit.** A checked-in `SOVEREIGNTY.md` in Qallow that
   lists every link in the persist/sync core loop (sync_wire.c,
   persist_lmdb.c, vendored lmdb) and answers "none" or names the
   dependency. Gemma/Ollama/FastAPI/VEYN bridge are not the core loop
   unless the persist path calls them. Sign-off is the Qallow PR that
   adds the file, reviewed against `ldd` / `#include` of those two
   translation units.

## Done when

- Each invariant has a C-level test that fails on violation.
- The sovereignty audit answers "none" for core-loop dependencies.

## Out of scope

- New capabilities. This task hardens what exists.
- LIMEN, DUCTEI, VEYN application code.
- Cloning Qallow onto a shared machine. The Qallow code PR waits on a
  Cursor Cloud Agent (or a checkout the owner runs).

## Open questions (proposed answers, not locked)

- aarch64 clone build/test infra: compile with a cross gcc in Qallow
  CI first. Run-on-device later if a runner exists. Hypothesis, verify
  in the Qallow PR.
- exact invariant-to-test mapping for ql_persist_merge_blob(): the
  three rejects listed in step 3, plus existing TOO_BIG / ARG cases.
- sovereignty audit methodology and who signs off on "none":
  `SOVEREIGNTY.md` plus the Qallow PR. No separate committee.
