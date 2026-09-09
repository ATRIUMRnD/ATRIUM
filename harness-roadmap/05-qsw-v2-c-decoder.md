# Task 5: QSW proto v2 C decoder

Status: ACTIVE (proposed September 8, 2026). Owner: Qallow, then DUCTEI.

## What changes

Qallow C persist/sync path only, plus one DUCTEI conformance commit:

1. `include/qallow/sync_wire.h`: v2 constants (`QSW_PROTO_VER_V2`,
   `QSW_MAX_SCOPES`, `QSW_MAX_SCOPE_LEN`, v2 body-size bound), the
   `qsw_scope` / `qsw_envelope_v2` / `qsw_frame_v2` types, and prototypes
   for `qsw_encode_hello_v2`, `qsw_encode_envelope_v2`,
   `qsw_decode_v2`, `qsw_decode_v2_envelope_body`,
   `qsw_hello_validate_v2`.
2. `src/mind/sync_wire.c`: v2 encode/decode alongside v1. v1 functions
   stay byte-identical (the conformance oracle must keep passing
   unchanged). The v2 ENVELOPE body mirrors
   `DUCTEI/ductei-qallow/src/v2.rs` byte for byte:
   `node_id[16] | u64 lamport | u16 schema_ver | u16 flags |
   u16 scope_count | (u16 scope_len | scope bytes){scope_count} |
   u32 key_len | u32 blob_len | key | blob`.
   The encoder sorts scopes bytewise (matching Rust `sort_unstable`);
   the decoder enforces DUCTEI's exact-length rule
   (`body_len == consumed + key_len + blob_len`).
3. `tests/test_sync_wire.c`: v2 cases - multi-scope roundtrip with a
   comma in a scope name (the exact corruption class v1's shim has),
   wire-layout spot checks, malformed rejections (truncated scope,
   scope_count past the body, key/blob length mismatch), v1 stream
   rejected by the v2 decoder, and HELLO proto-version separation
   (`qsw_hello_validate_v2` accepts only proto_ver=2).
4. `qallow_cli/src/ingest.rs`: auto-negotiation - sniff HELLO
   proto_ver; on a stream that starts with an ENVELOPE, fall back from
   v1 decode to v2 decode on the first MALFORMED. A v2 envelope is
   merged by reconstructing the v1 shim key
   (`scopes.join(",") + "|" + key`, scopes sorted) so the LMDB merge key
   is identical regardless of proto version. Scopes are routing
   metadata; the merge contract stays key -> blob.
5. DUCTEI (separate commit, after Qallow lands): conformance emitter
   emits a v2 stream, `conformance/verify.c` verifies it through the
   real C decoder, CI gains the v2 step, and the Qallow rev pin is
   bumped. Until then DUCTEI CI stays green: v1 is untouched.

## Status: implemented, verified locally - awaiting commit/PR
(September 8, 2026. No-merge-without-instruction rule: nothing has been
committed or pushed; the evidence below is local but real.)

- Qallow `make test` equivalent green on MSVC /W4 /WX:
  - test_sync_wire: v1 suite unchanged and green, plus v2 cases -
    multi-scope roundtrip with a comma inside a scope name ("veyn,x"),
    wire-layout spot checks (body 87, frame 92, scopes sorted bytewise),
    1-byte drip-feed decode, HELLO proto-version separation
    (v1 validator rejects v2 HELLO and vice versa), malformed rejections
    (scope_len past body, scope_count 0xFFFF, key/blob exact-length
    violation), v1 stream rejected by the v2 decoder, encoder arg caps.
  - test_persist_lmdb: green (one pre-existing MSVC gap fixed: guarded
    `#include <windows.h>` for GetCurrentProcessId; zero effect on the
    gcc/CI build).
- DUCTEI `cargo test -p ductei-core`: 16/16 pass (incl. the existing
  qsw_v2 Rust tests).
- Conformance oracle, run locally: DUCTEI's Rust v2 encoder -> Qallow's
  real C qsw_decode_v2, one byte at a time: CONFORMANCE PASS (v1) and
  CONFORMANCE PASS (v2), the v2 stream carrying scopes
  ["qallow.semantic.cert", "veyn,x"].
- End-to-end merge parity, run locally: a v2 frame with a
  persist-payload-v2 blob merged through `qallow ingest` into real LMDB
  ("ingest: ... applied=true ... [proto v2]"), the merge key rebuilt as
  the v1 shim "qallow.semantic.cert,veyn,x|limen.cert.j1" (comma-scope
  intact as metadata), and `qallow get` returned the payload (FOUND:OK!).

Landing order (rev-pin discipline): Qallow commit first (additive, v1
untouched - existing CI stays green), then the DUCTEI commit (emitter v2
mode, verify.c v2 mode, CI step, which requires Qallow main to carry the
v2 decoder).

## Done when

- `make test` green in Qallow with the v2 cases failing on violation.
- One real v2 stream emitted by DUCTEI's Rust encoder decodes through
  Qallow's real C decoder, byte-for-byte, including a comma inside a
  scope name.
- v1 conformance unchanged and green.

## Unlocks / does not unlock

Unlocks: v2 adoption end-to-end. Does not unlock: peer provisioning
(Task 6), new transports, new capabilities.
