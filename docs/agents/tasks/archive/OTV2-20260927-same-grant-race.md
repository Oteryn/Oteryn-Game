---
task_id: OTV2-20260927-same-grant-race
title: Retry a fresh admission round left stale by a concurrent evidence refresh
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: bab42d5c
branch: agent/same-grant-race-20260927
issue: 162
jira: KAN-13
allocation_comment: 5860157234
owned_paths:
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - docs/agents/tasks/archive/OTV2-20260927-same-grant-race.md
---

# Retry a fresh admission round left stale by a concurrent evidence refresh

This is a #822 follow-up. The race was first observed on #974, when node boot run 36320519328 reported `concurrent same-grant admission accepted 0`.

## Defect

Each fresh admission runs one sequence of steps: refresh the account owner evidence into S2, publish the guard sources, compose, verify, and commit with owner revalidation. A concurrent attempt for the same account refreshes the same S2 floor. When that happens between this attempt's publication, composition and commit, this attempt becomes stale:
- publication returns `Stale` or `Conflict`;
- composition fails its floor match;
- or the commit returns `RejectedStaleAuthority`.

An older upstream revision can also lose the S2 floor race and fail the refresh itself. `admit` had no retry, so a valid grant could be refused under a concurrent same-account attempt. Safety held (at most one GameSession), but liveness did not.

## Outcome

- `admit` runs up to three rounds (`FRESH_ADMISSION_ROUNDS`) with the reconcile backoff. It retries only stale outcomes: a failed refresh, `Stale`/`Conflict` publication, a failed composition, and a proven `RejectedStaleAuthority` commit. A retry rolls back the runtime reservation of the stale round first.
- Semantic refusals stay final and unchanged: token verification, authorization, prepare, replay conflict, incumbent, collision, capacity and ambiguous-commit reconciliation.
- The grant replay key still gates the commit. A retried loser receives `RejectedReplayConflict` once the winner has committed, so no retry can create a second GameSession.
- The seam's concurrent stage now races three attempts on one grant and requires exactly one GameSession.
