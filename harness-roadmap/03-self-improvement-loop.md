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

## Progress (October 27, 2026)

Landed in LIMEN#38 (merged October 27, 2026).

- Held-out gate: `evaluate_proposal` also replays `HELDOUT_SCENARIOS`
  (certified at spreads ~11.1, ~15.6 and ~20.2, plus two Tier 1 cases) under
  the same rules, and no single scenario may cost more than under the live
  policy. The proposer never reads the held-out set.
- Rollback guard wired: limend runs `monitor_landed_proposals` at startup and
  after each batch when `--proposals-dir` is set (on in
  `scripts/limend.service`). The window is anchored in the live ledger, each
  proposal resolves exactly once (`landed:`, then `monitor window closed:` or
  `rollback:` rows), and a recorded rollback is re-applied after a restart.
  The rollback is in-process, so it also files a `-revert-` claim under
  `policy_proposals/pending/` for the CI verdict; once accepted,
  `scripts/apply_proposal.py <id>` makes the one-line source edit (accepted
  verdicts only, refuses a stale source) and a human commits it through the
  PR gate.
- Proposer: `scripts/propose.py` writes claims to `policy_proposals/pending/`
  for the CI verdict. On the one gated knob it reports a fixed point: the meet
  gate forbids every move that could lower exposure, and cost is equal in both
  tiers, so the replay has no gradient there. Widening `GATED_POLICIES` is
  still a separate decision, and a cost knob (for example MAX_SHOTS) would need
  an outcome-based fidelity check first or the proposer would drive it to the
  floor.

## Open questions

- Claim-file location and schema: resolved as above.
- "One real routing improvement": resolved as
  CRITICALITY_SPREAD_THRESHOLD 2.0 -> 8.0.
