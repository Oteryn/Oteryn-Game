# OTV2-20260928-more-summons

```yaml
task_id: OTV2-20260928-more-summons
title: Remaining summon spells through encounters (owner decision D45)
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1121
jira: KAN-16
base_sha: a1d0e4796210c38cd185790b44f8a3a6fb59d1e8
head_sha: 68065760f229b2fb88ec8978fadc1ec87c69f80f
final_head_sha: 68065760f229b2fb88ec8978fadc1ec87c69f80f
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-more-summons.md
  - docs/agents/tasks/active/OTV2-20260928-count-vlarkorth.md
  - docs/agents/tasks/archive/OTV2-20260928-count-vlarkorth.md
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json
  - tools/content-schema/monster-authoring/**
  - tools/content-schema/encounter-authoring/**
  - tools/content-migration/test_world_project_v2_to_tree.py
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/world/**
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - content/imports/**
  - imports/**
  - content/creatures/definitions/**
  - content/presentations/definitions/**
  - content/behaviors/**
  - content/loot/**
  - content/abilities/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision D45: the remaining summon spells that the vocabulary already expresses become ability_cast rules of their
bosses' existing encounters.

- `devourer summon` (World Devourer): one of Greed, Frenzy or Disruption while fewer than three are out.
- `plagirath summon`: Plagirath fills its summons up to four Disgusting Oozes in its room.
- `tenebris summon`: a Shadow Fiend in Lady Tenebris's room, which speaks as it appears.
- `thorn summon`: a Thorn Minion within three tiles of the Mounted Thorn Knight (Canary evidence, to be verified).
- The census rises from 1,553 to 1,555 (World Devourer and Mounted Thorn Knight); both wait in the creature staging
  with the other encounter-covered monsters. This PR also archives the Count Vlarkorth task record of #1116.

## Acceptance and evidence

- 83 encounters validate and 78 manifests resolve fully.
- The census, the staging, the tree regeneration and its validators pass.
- The Rust tests pass.
- Exact-head review before the Merge Queue because staged creature evidence changes.

## Completion

Merged as PR #1121 (`a311c4eb2b16d88b45b24f20a12f7f2b76fb78c3`) from final head `6806576`; required checks passed on that head and the exact-head Codex review
found no issues. The census is at 1,555; World Devourer and Mounted Thorn Knight wait in the creature staging with the
other encounter-covered monsters (162).
Owner released.
