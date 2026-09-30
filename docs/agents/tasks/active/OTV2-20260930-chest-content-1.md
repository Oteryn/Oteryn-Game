# OTV2-20260930-chest-content-1

```yaml
task_id: OTV2-20260930-chest-content-1
title: CHEST-CONTENT part 1 - RewardClaim Content family and D39 reward from Content
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s  # allocation named claude/chest-content-1; this session may push only here (#162 5909327087)
issue: 162
pr: null
allocation: "#162 comment 5909237761 (request 5908857336; ruling 5905746509)"
base_sha: 58429a26
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: CHEST-1/D39 worker (claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/reference_playable.rs  # shared: RewardClaim family, kind, validation only
  - apps/game-server/src/content/project/v2.rs  # shared: one exhaustive-match arm
  - apps/game-server/src/combat/pickup.rs  # shared: one family wire-name arm
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/tests/content_reference_playable.rs  # shared: RewardClaim cases appended
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - docs/agents/tasks/active/OTV2-20260930-chest-content-1.md
  # after #1320 merges (LOW items of 5907886746):
  - apps/game-server/src/durability/reward_claim_mint.rs
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs
public_contracts:
  - GAME-INTERACTION-01 (chest USE slice, consumed)
  - reward chest decisions D39-D42 (consumed)
depends_on: [D39 (#1297), ruling 5905746509]
blocks: [CHEST-CONTENT part 2 (231 generated claims)]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome (so far)

- **RewardClaim family.** `DefinitionFamily::RewardClaim` and
  `ReferenceDefinitionKind::RewardClaim(ReferenceRewardClaimDefinition)` hold `placements[]`.
  Each placement has a `PlacementKey` and its own reward items, following ruling 5905746509:
  claims with several chests may reward differently per chest.
- **Link validation, fail closed:**
  - server-only;
  - at least one placement;
  - no placement listed twice in a claim;
  - exactly one reward item per placement, as in the first slice (CHEST-1 §5.1);
  - positive count;
  - reward items resolve to Item definitions;
  - a placement belongs to at most one claim (D40);
  - no placement may place a RewardClaim definition.

  Placements are canonically sorted, and the family is never in the client projection.
- **D39 follow-up.** `ChestUseRequest` names only the chest. `resolve_chest` finds the Item
  placement, the claim that lists it and that placement's reward, all from Content. A chest that
  no claim lists is refused with `ChestHasNoClaim`, and a non-Item reward with `NotAnItem`,
  before any write.
- **Authoring vocabulary.** `ProjectV2Family::from_reference` maps RewardClaim to `Interaction`,
  its authoring home (`content/interactions/`). `parse_family` never yields it.

## Pending in this task

- After #1320 merges: merge main and fold in the LOW items of 5907886746:
  - the `admit` comment (EXCLUSIVE table locks);
  - a real state-3 session case;
  - a commit-time race with two seeded reservations;
  - a dispatch note on replaying the same `CommandRef`.

## Excluded scope

- The 231 generated claims (part 2).
- Cooldowns, container rewards, key_binding, written_text, random_one_of, achievements on
  claims (#1320 owns the MINT side).
- Runtime Content loading (ADR-0021 MAP lanes), the client `USE` dispatch and migrations.

## Validation (so far)

- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `--lib`: 1129 passed. `content_reference_playable`: 45, including 5 RewardClaim cases.
  `interaction_workflow`: 15.
- PostgreSQL 17.11 (local; CI pins 17.6): `chest_use_postgres` passes, 3 cases.
