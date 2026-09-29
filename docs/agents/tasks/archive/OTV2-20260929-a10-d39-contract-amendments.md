# OTV2-20260929-a10-d39-contract-amendments

```yaml
task_id: OTV2-20260929-a10-d39-contract-amendments
title: "A10 corpse container DUR-03 amendment and D39 chest USE GAME-INTERACTION amendment"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1210
base_sha: 005550daa1bc2ecc9345d07322bc013eddfa3d06
head_sha: 25cf3a5adc86da99b59dcd29a35011f5ec30a333
final_head_sha: 25cf3a5adc86da99b59dcd29a35011f5ec30a333
final_head_frozen_at: 2026-09-29T07:04Z
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
successor (§4.1, §4.3, §5.1, §5.3-§5.7, §17, §19.1) and sets the live-allocations row to READY for that slice.

No schema, migration, runtime or registry change is made.

## Architecture and source of truth

- `PROVEN`: B3 decision §4.1; DUR-03 §5.2 and §39; VSL Combat rows; WO-0 corpse row; D52; the
  reward chest decisions (D39, D40); the GAME-INTERACTION-01 successor; CHEST-1 (#1190).
- `UNKNOWN`: rat corpse capacity and decay; retire work units; the client USE wire.

## High-risk authority/recovery qualification

Not applicable. Contract text only; the Durability and Combat lanes implement under their own
allocations.

## Acceptance criteria

- [x] The amendments are on an exact frozen head with passing validators.
- [x] Independent exact-head review.
- [x] Protected Merge Queue integration.

## Excluded scope

- Migrations, tables, runtime code, protocol and content values.

## Finding dispositions

Codex review of `10ec768`: two P2, both ACCEPTED in repair generation 1 of 1. No owner decision
changed.

- 4130553990 (incomplete corpse-retire resource envelope): explicit retire container expansion,
  participants (1) and effect work units (17) rows; `DUR03-RL-08` stays the retry budget (A10 §4.4).
- 4130554004 (§5.1 identity dependencies): §4.1, §4.3, §5.3, §5.4, §5.6 and §5.7 are accepted for
  the chest slice with the revision binding restated (D39 §4.1, pointers).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Terminal integration

- PR #1210 merged through the Merge Queue on 2026-09-29 as `f5db55e3`.
- Review: one Codex review of `10ec768` (two P2), repaired in `25cf3a5`; the owner decided to merge.
- Protected-main readback: every owned file is byte-identical to `25cf3a5`, except DUR-03, which
  also carries #1198 (the D3 corpse decision), merged first.
- A10 overlaps #1198 D3; D3 is authoritative for the window, decay shape, rows, timing and corpse
  identity (#162 5886162546). The reconciliation lands in `OTV2-20260929-a9-adr-a10-reconcile`.

## Context checkpoint

```yaml
last_progress: protected-integrated as f5db55e3; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: 25cf3a5adc86da99b59dcd29a35011f5ec30a333
pr: 1210
owner_action_required: null
blocker: null
next_action: null
```
