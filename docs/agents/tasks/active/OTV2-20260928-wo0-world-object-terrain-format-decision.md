# OTV2-20260928-wo0-world-object-terrain-format-decision

```yaml
task_id: OTV2-20260928-wo0-world-object-terrain-format-decision
title: "WO-0 WorldObject and Terrain authoring format decision (D93-D94)"
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: null
base_sha: 9961e4d4afc90d06a92ad7af59cfdbe6857e99b3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260928-wo0-world-object-terrain-format-decision.md
  - docs/agents/tasks/active/OTV2-20260928-spell-d7-cw1-chest-amendments.md   # archive move after #1151
  - docs/agents/tasks/archive/OTV2-20260928-spell-d7-cw1-chest-amendments.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the durable text for architecture package item A7 (WO-0, #162 comments
5876505672 and 5876526764) with owner decisions D93-D94 (#162 comment 5876870559): family-scoped
Terrain and WorldObject keys that reuse the frozen CW2-B1 sequence number, the first catalog
scope, the Terrain and WorldObject record fields from the existing field dispositions, and the
relation to the runtime `LocalObject` overlay.

No identity is minted and no schema, runtime or registry change is made; WO-1 and WO-2 follow.

## Architecture and source of truth

- `PROVEN`: `cw2_b1_import.rs` and task #504 (frozen allocation); the converter routing sample;
  `crystal-field-dispositions.json` and the Item formal schema §4; `reference_playable.rs` and the
  owners proposal §4, §9; the target-date source policy.
- `UNKNOWN`: Global evidence for contested routes.

## High-risk authority/recovery qualification

Not applicable. The decision fixes an identity rule and record shapes but mints nothing; WO-2,
which mints identity, requires independent identity review when allocated.

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Identity minting, schemas, validators, runtime code, map placements.

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
next_action: open the PR, bind this record to it, freeze and route one external review
```
