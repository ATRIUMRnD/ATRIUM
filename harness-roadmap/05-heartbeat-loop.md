# Task 5 (PROPOSED): Heartbeat loop

Status: BUILT, scenarios run with scaled timings (October 24, 2026). Added
to ../ROADMAP.md on the owner's go-ahead. Results and findings:
../BUILD.md section 7. Watchdog scheduled tasks are not installed yet.
Repos: DUCTEI (primary: ductei-core, feature `net`)
Date drafted: October 24, 2026

## Why now

GAP 4 has a real two-machine result (BUILD.md section 6): xing-laptop to
xing-pc over the tailnet, persistence before ack, restart replay and local
fallback all held. This task turns that transport into the first closed
loop across machines: nodes detect a failed peer and heal themselves with
no human in the loop.

## Claim file (AGENTS.md section 5)

- Changes: a new `ductei-node` binary (feature `net`) that listens, sends
  periodic heartbeats to peers, and records peer silence in plain English.
  Plus a per-node watchdog that restarts that node's own process.
- Unlocks: failure of one node is detected by its peer and healed without
  you; evidence that the harness can supervise itself across machines.
- Does not unlock: recovery when every node is down at once (power cut);
  any remote command channel; a phone node (iOS cannot run the binary,
  the phone only views the status page); transport security (the tailnet
  provides encryption and authentication, DUCTEI's own wire has neither).

## Design

- Nodes: xing-pc (node 1) and xing-laptop (node 2).
- Heartbeat envelope: scope `ductei.heartbeat` (added to each node's
  allow list explicitly, deny by default stays), key `hb.<node>`, lamport
  from a counter the node resumes from its own log after restart, never
  reset. Payload is a closed type: node id, uptime seconds, build version.
  No credentials are representable (invariant 1).
- Delivery: send_local_first, one bounded session per beat (invariant 5),
  persistence before ack (invariant 4). Heartbeats are best effort: a beat
  that fell back to the local spool is not forwarded later, because a stale
  heartbeat carries no information.
- Cadence: one beat per peer every 30 seconds.
- Thresholds, stated before any trial (CANON threshold protocol):
  - SUSPECT: no beat from a peer for 90 seconds (3 missed beats).
  - UNREACHABLE: no beat for 5 minutes. Reported as "unreachable", not
    "failed", because a laptop on battery can be asleep.
- Healing is self-only: each node's watchdog restarts that node's own
  process, the same pattern as the existing GrokBotWatchdog task. Peers
  observe and report; they never restart each other. A remote restart would
  need a command channel, which breaks deny by default.
- Bounded retries: at most 3 self-restarts per hour. After that the node
  stops, and writes a human-readable record to failed/ (witnessed, never
  deleted, never retried punitively).
- Visibility: each node writes a plain-English status line per peer. The
  xing-pc status page (Tailscale Serve, tailnet only) shows last beat age
  and turns red at SUSPECT.

## Acceptance scenarios

1. Steady state: both nodes show each other healthy for 10 minutes.
2. Kill the laptop node: xing-pc reports SUSPECT at about 90 seconds, the
   laptop watchdog restarts it, the next beat carries a higher lamport,
   status returns to healthy with no human action.
3. Crash storm: kill the node 4 times within an hour. It restarts 3 times,
   then stops and writes the failure record.
4. Partition: stop Tailscale on one machine. Beats fall back locally, no
   crash, status shows UNREACHABLE after 5 minutes, recovery on reconnect.
5. Out-of-scope or malformed beat: rejected, log untouched, node keeps
   running.

Scenarios 1 and 5 can also run on loopback in CI. Scenarios 2 to 4 need
the two real machines and are recorded as manual transcripts.

## Open decisions for the owner

- Scheduled task for the watchdog: per-user (no admin, runs while logged
  in) or SYSTEM (runs with nobody logged in, needs admin on each machine).
- Whether the status page change belongs in this task or a follow-up.

## Done when

- Scenarios 1 to 5 pass, with a written transcript in BUILD.md.
- ROADMAP.md gains this task through an approved proposal, and its status
  is flipped in the same pull request as the final change.


## Update, October 25 2026

Persisted restart history and the status page output are built and tested on xing-pc; the watchdog task is installed on xing-pc. Laptop install and the 10-minute production-timing run wait for xing-laptop to come back online. Details: BUILD.md, section 7, Follow-ups.
