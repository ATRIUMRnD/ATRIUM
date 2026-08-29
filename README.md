# ATRIUM

ATRIUM is the receiving chamber of the xingxerx ecosystem. Work enters
here and is routed to the room that owns it. No application code lives
in this repo. Only rules, routing, and roadmap documents. The other
repos are the rooms.

Agents working against this ecosystem act as routers, not owners:
classify the change, send it to the owning repo, and respect the gates.

## Documents

| File | Role |
|---|---|
| [AGENTS.md](AGENTS.md) | Ownership map, hard invariants, CANON, change gating |
| [ROADMAP.md](ROADMAP.md) | Dispatch table: what is next, what is blocked, what is done |
| [BUILD.md](BUILD.md) | Connection audit and gap history |
| [harness-roadmap/](harness-roadmap/00-INDEX.md) | How each ROADMAP task is built |
| [PRODUCT.md](PRODUCT.md) | Commercial surface: VEYN / Intero |

ROADMAP.md is the source of truth for status. This README is not.

## Rooms

| Concern | Owner | Repo |
|---|---|---|
| Quantum routing, fidelity tiers, budget router, certificates, ledger, reports, limend | LIMEN | https://github.com/xingxerx/LIMEN |
| Envelopes, relays, sessions, spool consumers, invariant enforcement | DUCTEI | https://github.com/xingxerx/DUCTEI |
| Persistence, LMDB, sync wire format, C-level enforcement | Qallow | https://github.com/xingxerx/Qallow |
| Tracing, structured JSON logging, daemon / sensory-motor | VEYN | https://github.com/xingxerx/VEYN |

LIMEN is Python. Never call fidelity or budget routing "Lumen". The
package name is `limen-compiler`, never `limen`. DUCTEI is a channel,
not a merger. Qallow `sync_wire.c` is the conformance oracle. VEYN is
the most mature repo; its patterns are the reference for the others.

A change that cannot be classified into exactly one owning repo is split
into per-repo changes before any work starts. Cross-repo landings follow
dependency order: DUCTEI first, then a separate rev-pin bump in the
consumer.

## Hard invariants

These are never negotiable. Detail lives in AGENTS.md.

1. LIMEN credentials are never representable in sync payloads. Env vars only.
2. Broadcast scopes are deny-by-default.
3. Byte-level wire compatibility with Qallow `sync_wire.c`.
4. Persistence before ack, always, fsynced.
5. Sessions are bounded. One bounded session per job. Unbounded sessions
   must remain unrepresentable in the type system.

If a task cannot be completed without violating one of these, stop and
report. Do not work around it.

## How to land a change

1. Read AGENTS.md, then pick up the lowest-numbered incomplete task in
   ROADMAP.md.
2. State, before work starts: what changes, which repo owns it, what it
   unlocks, and what it does not unlock.
3. Implement in the owning repo. Open a PR. Do not merge unless that
   repo's tests pass.
4. Completing a ROADMAP task means flipping its status in ROADMAP.md in
   the same change as the final work, in plain English.

Failed jobs, proposals, and runs are witnessed and archived. They are
never deleted and never retried punitively.

## What this repo is not

ATRIUM is not an application. Do not add runtime code here. Do not clone
the room repos onto a shared machine to "just quickly" edit them. GitHub
is the source of truth. Use Cloud Agents for application-code changes
when they are available.
