# OTV2-20260930-chest-content-2b

```yaml
task_id: OTV2-20260930-chest-content-2b
title: CHEST-CONTENT part 2b - RewardClaim family in content/interactions (231 plain once claims)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: 1364
allocation: "#162 5909237761 (part 2); scope ruling 5911004459; owner answer a (tree-first) 5912009064"
base_sha: 5dcfb724
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: CHEST-1/D39 worker (claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/reward-claim-authoring/**
  - content/interactions/reward_claims/**
  - content/interactions/index.json
  - content/project.json  # shared: RewardClaim registration only
  - content/manifest.json  # shared: RewardClaim registration only
  - content/content.lock.json  # shared: RewardClaim registration only
  - tools/content-migration/world_project_v2_to_tree.py  # shared: RewardClaim registration only
  - tools/content-migration/validate_world_project_v2_to_tree.py  # shared: RewardClaim checks only
  - tools/content-migration/test_world_project_v2_to_tree.py  # shared: RewardClaim checks only
  - docs/agents/tasks/archive/OTV2-20260930-chest-content-2b.md
public_contracts: []
depends_on: [CHEST-CONTENT part 1 (#1334), part 2a (#1356)]
blocks: [MAP-BUNDLE-1 binding of claim placements]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`content/interactions/reward_claims/` holds 231 RewardClaim records: the plain `once` claims of
the chest pilot, 247 placements in all. They are built by
`tools/content-schema/reward-claim-authoring/reward_claim_authoring.py` and registered in
`content/project.json`, `manifest.json` and `content.lock.json`.

The tree generator `world_project_v2_to_tree.py` knows the family, and a regenerated tree is
byte-identical to the committed one.

Owner answer "a" (5912009064) made the family tree-first, like CHARM-1. There is no Rust v2
parser: the server does not read `content/`, and MAP-BUNDLE-1 compiles it (ADR-0021).

- **Reward on the placement.** Architect ruling 5905746509: in 8 claims the chests reward
  differently.
- **No PlacementKey (5909237761).** Each placement binds to its project-frame position and the
  legacy unique ids of both servers; all 247 have at least one. The chest appearance is kept as
  `appearance_tibia_id`, evidence only, because 3 chest appearances (ids 28827 and 28828) are
  not Items.
- **Rewards.** Each is one A12 Item key that resolves in `content/items`. The G4
  `item_key_references` check passes.
- **Readiness.** 27 claims are `ready` and 204 are `waiting_item_semantics`: 188 Item ids go to
  ITEM-SEM-2b, list in 5909327087. One source check: 3081 x5 is NonStackable with count 5. It is
  not guessed and stays not ready.
- **Coupling.** Only a false `ready` is an error. A stale `waiting` is safe (the MINT fails
  closed), so an ITEM-SEM change to `content/items` never breaks this family; rebuilding
  promotes newly ready claims.
- **CI.** `test_world_project_v2_to_tree.py` runs the tool's tests, so the Content Tree
  Migration workflow covers them. There is no workflow change. Edits limited to
  `content/interactions/**` do not trigger that workflow; G4 (`content/**`) still checks the
  keys.

## Validation

- `reward_claim_authoring.py content --check`: ok. `test_reward_claim_authoring.py`: 7 tests
  pass. ruff check and ruff format are clean.
- `test_world_project_v2_to_tree.py` and `validate_world_project_v2_to_tree.py`: PASS, including
  `reward_claim_records=231`.
- A regenerated tree is byte-identical (`world_project_v2_to_tree.py`).
- `item_key_references.py`: PASS.

## Closeout

- merge commit/result: squash merge of this PR (pending)
- review: independent exact-head review after freeze (pending)
- follow-ups:
  - MAP-BUNDLE-1 binds the placements;
  - rebuild after ITEM-SEM-2b to promote claims;
  - cooldown, container, key, text, random and achievement claims are later children.
