# Task 6: Peer provisioning

Status: QUEUED (proposed September 8, 2026). Owner: DUCTEI. Not started
until Task 5 flips to DONE (one task ACTIVE at a time).

## What changes

DUCTEI only. A minimal provisioning story for cross-network peers:

- Static peer-list config (address + self-signed cert DER fingerprint),
  matching how QUIC clients already pin today ("config, not a PKI").
- A pairing-time fingerprint exchange so two nodes can bootstrap each
  other's peer entries without a certificate authority.
- gRPC/QUIC usable as cross-network transports behind explicit config.

## Done when

Two nodes on different machines exchange envelopes over `grpc` or `quic`
with persistence-first acks and the five invariants asserted by a smoke
scenario in CI.

## Unlocks / does not unlock

Unlocks: the spine leaves localhost. Does not unlock: CA-based mTLS,
service discovery (etcd/consul-style), new capabilities.
