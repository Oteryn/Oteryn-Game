# OTV2-20260929-a10-d39-contract-amendments

```yaml
task_id: OTV2-20260929-a10-d39-contract-amendments
title: "A10 corpse container DUR-03 amendment and D39 chest USE GAME-INTERACTION amendment"
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: null
base_sha: 005550daa1bc2ecc9345d07322bc013eddfa3d06
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_A10_CORPSE_CONTAINER_DUR03_AMENDMENT_DECISION_2026-09-29.md
  - docs/architecture/reviews/OTERYN_GAME_D39_CHEST_USE_GAME_INTERACTION_AMENDMENT_DECISION_2026-09-29.md
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # §39.3 amendment pointer
  - docs/architecture/GAME-INTERACTION-01_SUCCESSOR_CHILD_IDENTITY_RETRY_CONTRACT_CANDIDATE.md   # amendment pointer
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md   # Interaction Use row
  - docs/agents/tasks/active/OTV2-20260929-a10-d39-contract-amendments.md
  - docs/agents/tasks/active/OTV2-20260928-owner-decision-batch-d118-d128.md   # archive move after #1180
  - docs/agents/tasks/archive/OTV2-20260928-owner-decision-batch-d118-d128.md
public_contracts: [DUR-03, GAME-INTERACTION-01]
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records two architect rulings as contract amendments: A10 (#162 5882274851), a DUR-03 §39
corpse container destination, corpse-entry transfer and corpse retire for decay under owner decision
D121; and D39 (#162 5884689001), which accepts only the chest USE slice of the GAME-INTERACTION-01
successor (§5.1, §5.5, §17, §19.1) and sets the live-allocations row to READY for that slice.

No schema, migration, runtime or registry change is made.

## Architecture and source of truth

- `PROVEN`: B3 decision §4.1; DUR-03 §5.2 and §39; VSL Combat rows; WO-0 corpse row; D52; the
  reward chest decisions (D39, D40); the GAME-INTERACTION-01 successor; CHEST-1 (#1190).
- `UNKNOWN`: rat corpse capacity and decay; retire work units; the client USE wire.

## High-risk authority/recovery qualification

Not applicable. Contract text only; the Durability and Combat lanes implement under their own
allocations.

## Acceptance criteria

- [ ] The amendments are on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Migrations, tables, runtime code, protocol and content values.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Context checkpoint

```yaml
last_progress: authored
status: implementing
branch: claude/gifted-rubin-a0axzx
pr: null
owner_action_required: null
blocker: null
next_action: open the PR, freeze the head, exact-head review and Merge Queue integration
```
