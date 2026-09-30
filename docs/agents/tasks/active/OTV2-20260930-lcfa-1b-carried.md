# OTV2-20260930-lcfa-1b-carried

```yaml
task_id: OTV2-20260930-lcfa-1b-carried
title: LCFA-1b - close the five carried LCFA-1 review findings (migration 0028)
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/lcfa-1b-carried
issue: 162
lane_id: durability
pr: null
base_sha: 07138ee5
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "LCFA-1b hard worker (Oteryn work coordinator, #162)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0028_account_characters_projection_retry.sql (migration lease 0028)
  - apps/game-server/src/durability/account_characters_projection.rs
  - apps/game-server/src/native_admission_source/account_characters.rs (+ _tests.rs)
  - apps/game-server/tests/character_authority_postgres.rs, tests/support/account_characters_projection_postgres_cases.rs
  - docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md (one sentence)
  - docs/agents/tasks/active/OTV2-20260930-lcfa-1b-carried.md -> docs/agents/tasks/archive/
public_contracts:
  - docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md
depends_on: [PR 1330 (LCFA-1, merged)]
blocks: [LCFA publisher enablement]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Closes the five carried findings of the LCFA-1 review (#1330, comment 5910198689) before the
publisher is enabled on any host. The publisher stays off (A1); no wire change; 0024 is unchanged.

## Excluded scope

Publisher enablement, the Platform repository, wire changes, edits to migration 0024.
