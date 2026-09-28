# OTV2-20260928-dark-merudri

```yaml
task_id: OTV2-20260928-dark-merudri
title: Wiki-authored Dark Merudri (owner decision D44)
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: null
jira: KAN-16
base_sha: 8d320703ec39b39c44c0b38268a07e4b0579e060
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-dark-merudri.md
  - docs/agents/tasks/active/OTV2-20260928-item-target-date.md
  - docs/agents/tasks/archive/OTV2-20260928-item-target-date.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json
  - tools/content-schema/monster-authoring/**
  - tools/content-migration/creature_admission_stage.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
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

Owner decision D44 ("uzupełnić to co masz, żeby było maksymalnie zbliżone do Tibii, ale zaznacz, że to wymaga kiedyś
weryfikacji"): Dark Merudri, which Tibia has at the 2026-09-27 target and neither Canary nor CrystalServer has on any
active branch, is authored from the wiki and admitted to `content/world`.

- Fandom (Dark Merudri rev 1098116, Good Remains of a Merudri rev 1116306, Monk Outfits rev 1102608): health 6500,
  experience 0, flags, immunities, corpse item 50311, Monk looktype 1824.
- TibiaWiki BR (rev 423733): melee 0-160, energy area 430-550, energy ball 290-460, element modifiers 100%, immunities,
  ignored fields, no loot.
- Dark Knight template (Canary): colours, speed, armor, defense, attack intervals, chances and effects, each marked
  NEEDS VERIFICATION in the manifest. The BR earth wave without damage is left out.
- The creature is bound to its Fandom page id under its own `oteryn:source.tibiawiki` import batch.

The Count Vlarkorth encounter rule that summons it for a Monk is a follow-up.

## Acceptance and evidence

- `wiki_authored.py self-test` passes; the bundle validates with no open manifest row.
- The census, the staging, the tree regeneration and its validators pass (1,319 creatures).
- The Rust tests pass.
- Exact-head review before the Merge Queue because `content/world` changes.
