# Task 3: Self-improvement loop

Status: DONE (August 29, 2026)
Repos: LIMEN (primary), DUCTEI

## Blocker

Resolved. Task 1 is DONE.

## Scope guard

Gates routing-policy changes ONLY, at first. Nothing in this task widens
to gating arbitrary code — see ../AGENTS.md section 4 (CANON
governance): witness requirement, no punishment realms, threshold
protocol.

## The loop

1. **Ledger baseline** — record the current state of LIMEN's
   memory.py-backed ledger before any proposal is made.
2. **Proposal with claim file** — a change to routing policy is proposed
   alongside a plain-English claim file stating what it changes, what it
   unlocks, and what it does not unlock (../AGENTS.md section 5).
3. **CI verdict via report.py** — the proposal runs through CI; report.py
   is the reference implementation for turning results into a
   plain-English verdict (../AGENTS.md section 4, witness requirement).
4. **Branch-protected merge** — only merges if the harness (not the
   agent) says so, per ../AGENTS.md section 5: "Agents propose; the
   harness disposes."
5. **Ledger learns** — the ledger is updated to reflect the merged
   change, closing the loop.

Failed proposals are witnessed and archived (failed/), never deleted,
never retried punitively — ../AGENTS.md section 4, "No punishment
realms."

## Done when

- One real routing improvement has traversed the full loop end to end.
- The transcript of that traversal is written up somewhere durable
  (ledger + report.py output + the claim file), not just implied by a
  merged diff.

## Out of scope

- Gating arbitrary code. Widening scope beyond routing-policy changes
  stays a deliberate, separately proposed decision.

## Progress (July 24 / August 29, 2026)

- Implemented LIMEN@13329733. Claim files live under
  `policy_proposals/pending/` and archive to `accepted/` or `failed/`.
  Schema is a Proposal claiming a named tunable (currently only
  `CRITICALITY_SPREAD_THRESHOLD`), plus what it changes / unlocks /
  does not unlock.
- One real routing improvement: raise CRITICALITY_SPREAD_THRESHOLD
  2.0 -> 8.0. Transcript:
  `policy_proposals/accepted/2026-07-24-raise-criticality-threshold.*`
  Verdict ACCEPTED (cost delta 0, physical-error exposure delta -698.7).
- CI job `routing-policy proposal verdict` green:
  https://github.com/xingxerx/LIMEN/actions/runs/30072093877
- LIMEN `main` branch protection now requires that verdict (plus
  `cargo test` and `pytest (py3.12)`), August 29 2026.

## Open questions

- Claim-file location and schema: resolved as above.
- "One real routing improvement": resolved as
  CRITICALITY_SPREAD_THRESHOLD 2.0 -> 8.0.
