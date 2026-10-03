# OTV2-20261003-charm-desc-fence

```yaml
task_id: OTV2-20261003-charm-desc-fence
title: "CHARM-DESC-FENCE: live attacker authority for Ability damage commits (D295)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/charm-desc-fence-decision-20261003
pr: null
base_sha: 3d8c14d0f6326e278cf09e97acd2453b6af31926
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CHARM_DESC_FENCE_DECISION_2026-10-03.md
  - docs/agents/tasks/archive/OTV2-20261003-charm-desc-fence.md
public_contracts: []
depends_on: []
blocks:
  - "PR #1625 reply to Codex P1 4173012827"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

D295 is ruled C plus (a):
- the live attacker fence is deferred to composition;
- the `CharacterLease` seam stays;
- a binding gate allows no production caller of the three Ability damage bridges until the A2
  follow-up (a character-bound single live slot with immediate takeover eviction) is merged.

This also corrects the packet: `remove_terminal_session` has production callers on the
grace-expiry path.

## Architecture and source of truth

- `PROVEN`: `ability/commit.rs`, `ability/mod.rs`, `foundation/runtime_actor_carrier.rs` and
  `gameplay_transport/mod.rs` at `3f7f1700`.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. The A2 task carries the authority review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (authority, fencing).
