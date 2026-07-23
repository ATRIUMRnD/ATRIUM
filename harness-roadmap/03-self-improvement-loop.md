# Task 3: Self-improvement loop

Status: BLOCKED ON TASK 1
Repos: LIMEN (primary), DUCTEI

## Blocker

Task 1's CI harness must be green and required on main in DUCTEI first.
An agent proposing work here must show that's true before starting.

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

## Open questions

- [SLOT — where claim files live and their exact schema]
- [SLOT — what "one real routing improvement" means concretely: a
  specific budget-router parameter, a fidelity-tier threshold, etc.]
