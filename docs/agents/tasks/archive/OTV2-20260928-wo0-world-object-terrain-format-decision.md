# OTV2-20260928-wo0-world-object-terrain-format-decision

```yaml
task_id: OTV2-20260928-wo0-world-object-terrain-format-decision
title: "WO-0 WorldObject and Terrain authoring format decision (D93-D94)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1157
base_sha: 9961e4d4afc90d06a92ad7af59cfdbe6857e99b3
head_sha: db4eb5e266bc7e2aa83c70e7d517a8ab81a79ef5
final_head_sha: db4eb5e266bc7e2aa83c70e7d517a8ab81a79ef5
final_head_frozen_at: 2026-09-28T19:37Z
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

- [x] The decision document is on an exact frozen head with passing validators.
- [x] Independent exact-head review.
- [x] Protected Merge Queue integration.

## Excluded scope

- Identity minting, schemas, validators, runtime code, map placements.

## Finding dispositions

Codex review of `0b18850`: three P1, all ACCEPTED in repair generation 1 of 1 (#162 convergence
rule 5869165340). No owner decision changed.

- 4126280751 (no WorldObject reference on LocalObject states): an optional typed `presentation`
  reference `{family: WorldObject, key, revision}`, added by the new WO-3 child (§4.4, §5).
- 4126280766 (`routed_to` not admitted and untyped): a typed versioned reference; WO-1 amends the
  Item formal schema and `ProjectReferenceRecord::Item` (§4.1, §5).
- 4126280775 (Item-owned corpse fields): decay target, duration and container capacity stay on the
  routed Item record; the WorldObject corpse row keeps only corpse flags (§4.3).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Terminal integration

- PR #1157 merged through the Merge Queue on 2026-09-28 as `e19b19d6`.
- Review: one Codex review of `0b18850` (three P1s), repaired in the single repair generation
  `db4eb5e`; the owner decided to merge after green CI.
- Protected-main readback: all three changed files on `e19b19d6` are byte-identical to the frozen
  head `db4eb5e`.
- The PR body predates the repair (the LocalObject `presentation` ref and WO-3, the typed
  `routed_to`, the Item-owned corpse fields); the documents are authoritative.
- Archived under `OTV2-20260928-a8-donor-item-identity-epoch-decision`.

## Context checkpoint

```yaml
last_progress: protected-integrated as e19b19d6; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: db4eb5e266bc7e2aa83c70e7d517a8ab81a79ef5
pr: 1157
owner_action_required: null
blocker: null
next_action: null
```
