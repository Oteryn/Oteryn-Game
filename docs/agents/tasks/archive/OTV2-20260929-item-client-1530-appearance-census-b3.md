# OTV2-20260929-item-client-1530-appearance-census-b3

```yaml
task_id: OTV2-20260929-item-client-1530-appearance-census-b3
title: B3 appearance-only census of the 15.30 client ids no engine defines
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: 1197
base_sha: 1f6aa1b7
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session (Item authoring lane)
created_at: 2026-09-29T05:30:00Z
updated_at: 2026-09-29T05:50:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/client_appearance_census.py
  - tools/content-schema/item-authoring/test_client_appearance_census.py
  - tools/content-schema/item-authoring/samples/client-appearance-census-15-30-2dfa943b.json
  - tools/content-schema/item-authoring/README.md
  - .github/workflows/item-authoring-schema.yml
  - docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md
  - docs/agents/tasks/archive/OTV2-20260929-item-client-1530-appearance-census-b3.md
public_contracts: []
depends_on:
  - "B2 #1191 (1f6aa1b7)"
blocks:
  - "identity for client-only ids (future epoch); WO-2 map geometry"
cross_repository_coordination_id: null
external_repositories:
  - "owner-supplied local Tibia 15.30 client assets (appearances.dat sha256 2dfa943b..., reference evidence only, never committed)"
jira: KAN-16
```

## Allocation

The claim is #162 comment 5880862991. B3 returns to the Item authoring lane per 5876385001, and the owner's standing
direction is to announce and proceed.

The owner supplied the client assets archive, and its sha256 `e48e478d…` equals the manifest. Only
`appearances-2dfa943b….dat` was extracted, to the session scratchpad. The archive was deleted, and nothing from it is
committed.

## Outcome

- **`client_appearance_census.py`.** It reads the pinned client file (size and sha256 are both enforced) and the three
  pinned engine `items.xml`. For every appearance id that no engine defines, it records the facts (name, description,
  a fixed flag subset), the appearance-only routing, and a provisional `client:` key. It mints no identity.
- **Committed census.** 9,310 undefined ids: 1,306 in the 15.30 range and 8,004 older.
  - 8,766 are routed: Terrain ground/border 2,896, WorldObject immovable 5,569, corpse 301.
  - 247 are pickupable candidates and 297 are unclassified.
- **CI lane.** A new fixture test step (19 checks).
- **Formal doc.** New §5l-b.

## Validation

- `test_client_appearance_census.py`: 19 checks PASS.
- `client_appearance_census.py --check` against the owner's file: PASS.
- Ruff 0.16.1 `check` and `format`: PASS.
- Validators and `git diff --check`: see the PR.

## Handoff

- **Client-only map geometry (8,766 routed ids).** It needs its own identity step before WO-2 can key it.
- **Pickupable candidates (247).** They need a family and an identity decision.
