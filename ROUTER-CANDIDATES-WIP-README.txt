router-candidates WIP (bounded failover: route_candidates, _execute_plan, max_attempts)
was staged-but-uncommitted on this laptop's LIMEN main when the tree was moved to
origin/main 20ec599 (tier 0) on 2026-10-25T20:16:12. It is preserved as:
  - git stash entry: 'router-candidates WIP (laptop, before tier-0 pull ...)'
  - patch: C:\Users\there\ATRIUM\router-candidates-staged-*.patch
It conflicts with tier 0 in limen/pipeline.py, limen/router/__init__.py and
limen/router/budget_router.py (both touch run_route_request and route()).
Re-apply with: git stash pop   (then resolve the three files), or
               git apply --3way <patch>
