# OTV2-20260929-item-tibia-id-mirror-decision-m0

```yaml
task_id: OTV2-20260929-item-tibia-id-mirror-decision-m0
title: ITEM-ID-M0 candidate decision record for Tibia-id Item keys
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: null
base_sha: b90f85c9
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session (Item authoring lane)
created_at: 2026-09-29T15:30:00Z
updated_at: 2026-09-29T15:30:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ITEM_TIBIA_ID_MIRROR_IDENTITY_DECISION_2026-09-29.md
  - docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md
  - docs/agents/tasks/archive/OTV2-20260929-item-tibia-id-mirror-decision-m0.md
public_contracts: []
depends_on:
  - "B3 #1197 (4484ea98): client 15.30 appearance census"
blocks:
  - "ITEM-ID-M1 migration"
  - "WO-2 key minting"
cross_repository_coordination_id: null
external_repositories: []
jira: KAN-16
```

## Allocation

The owner decision and claim are in #162 comments 5892865958 and 5892949642, under announce-and-proceed.

## Outcome

- **New candidate decision record `ITEM-ID-TIBIA-MIRROR-V1`** (`docs/architecture/reviews/`). The owner decided (ID1)
  that the Item key number is the Tibia item id for every id, known or not, as `oteryn:item.tibia.iNNNNNNNN`.
  - Old `item.registry` keys are retired, never re-meant.
  - Oteryn-own items use semantic keys.
  - WO-0 D93 is unchanged.
  - It supersedes A8 §4.1.
  - It lists open questions Q1 (the 65 semantic keys) and Q2 (removed-id marker) for review.
- **G4 carries a one-line amendment pointer.**
- **No minting and no runtime, binding or content change.** The migration is ITEM-ID-M1.

## Validation

- `validate_governance.py`, `validate_repository_policy.py` and `git diff --check`: see the PR.
