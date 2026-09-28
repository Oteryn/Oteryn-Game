# OTV2-20260928-tile-damage

```yaml
task_id: OTV2-20260928-tile-damage
title: Untyped tile damage for named targets in monster area spells
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1056
jira: KAN-16
base_sha: ac6d820bf744ac3abe9674258607b9e830eb2306
head_sha: 17e7bc1927c676487fbc0d09a15cfca0ec45df4a
final_head_sha: 17e7bc1927c676487fbc0d09a15cfca0ec45df4a
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-tile-damage.md
  - docs/agents/tasks/active/OTV2-20260928-players-only-chain.md
  - docs/agents/tasks/archive/OTV2-20260928-players-only-chain.md
  - tools/content-migration/creature_admission_stage.py
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

This slice is monster spell work, which the owner approved ("Zaczynaj"). It covers the D18 pattern
`area_damage_named_target`: a tile callback takes a fixed or rolled amount of health from the top creature of each
area tile through `Creature:addHealth`, when that creature is a player or a named monster.

- The pattern is authored as extra `damage` effects of the new damage type `untyped`.
- The effects affect `players` (a new kind, players only) or `named_creatures`, with `top_creature_only`.
- The new kind is added to the monster schema, to the v2 `ProjectV2AffectsKind` and to the staging tool.

The census rises from 1,543 to 1,548:
- Ravenous Lava Lurker and The Remorseless Corruptor are admitted.
- The Corruptor of Souls and The Source of Corruption wait for the encounter runtime.
- Freed Soul waits for The Souldespoiler.

The completed task record of #1052 is archived.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: two new declarative enum values and two admitted creatures; no protocol, persistence or authority
change.

## Acceptance and evidence

- `verify_formal_schema.py` passes 231/231.
- The census reports 1,548 resolved and 102 blocked; the existing bundles are unchanged.
- The staging, the tree regeneration and its validators pass.
- The Rust tests pass, including the new `Players` case.
- The governance and policy validators pass.
- The PR gets an exact-head review because `content/world` changes.

## Completion

Merged as PR #1056 (`e4a463d09bbe5ad98c3dfa9d4761ac10db48ceab`) from final head `17e7bc1`; required checks passed on that head. The Codex finding
(probe a player summon before a players-only effect) was fixed in `17e7bc1`.
Owner released.
