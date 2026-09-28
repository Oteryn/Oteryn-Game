# OTV2-20260928-players-only-chain

```yaml
task_id: OTV2-20260928-players-only-chain
title: Players-only chain picker for monster chain spells
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1052
jira: KAN-16
base_sha: e91cafdd95c165668126e1dccaf93422af29d55c
head_sha: e8129de4f7a3bfce6ee1fe13f7bf5f1d425255b7
final_head_sha: e8129de4f7a3bfce6ee1fe13f7bf5f1d425255b7
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-players-only-chain.md
  - docs/agents/tasks/active/OTV2-20260927-br-health-experience.md
  - docs/agents/tasks/archive/OTV2-20260927-br-health-experience.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json
  - tools/content-schema/monster-authoring/**
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - apps/game-server/**
  - content/world/**
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - content/items/index.json
  - content/cosmetics/mounts/index.json
  - content/npcs/**
  - content/services/**
  - content/creatures/definitions/**
  - content/presentations/definitions/**
  - content/behaviors/**
  - content/loot/**
  - content/abilities/**
  - imports/canary/**
  - imports/tibiawiki/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The owner approved the monster spell work ("Zaczynaj"). This slice handles the Canary chain spells `poison chain` and
`podzillaphyschain`, whose `CALLBACK_PARAM_CHAINPICKER` keeps only players outside a protection zone.

- The picker is recognised as an exact template and authored as `Ability.chain.target_filter: players`.
- The field is added to the monster schema and to the v2 `ProjectV2Chain` profile.
- Any other picker body stays unresolved.

Result: four monsters resolve (the census rises from 1,539 to 1,543).

- Quara Looter is admitted.
- Rootthing Bug Tracker waits for an unregistered Item.
- Mould Phantom and Rotten Golem wait for the encounter runtime (the Soul War taint zones).

The completed task record of #1051 is archived.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: an optional declarative profile field and one admitted creature; no protocol, persistence or
authority change.

## Acceptance and evidence

- `verify_formal_schema.py` passes 231/231.
- The census reports 1,543 resolved, 107 blocked and 6 not converted.
- The staging, the tree regeneration and its validators pass.
- The Rust tests pass, including a round trip of the new field.
- The governance and policy validators pass.
- The PR gets an exact-head review because `content/world` changes.

## Completion

Merged as PR #1052 (`ac6d820bf744ac3abe9674258607b9e830eb2306`) from final head `e8129de`; required checks passed on that head. The Codex finding
(bind the picker to its combat) was fixed in `e8129de`.
Owner released.
