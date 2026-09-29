# OTV2-20260928-arena-summons

```yaml
task_id: OTV2-20260928-arena-summons
title: Arena summon spells through encounters (owner decision D45)
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1111
jira: KAN-16
base_sha: 7c6b0df97832b363a9fd40dd4d3bac282db53fe1
head_sha: 7d537c8e4ccb6f53d6cfbbe7f09ca8751f1b611e
final_head_sha: 7d537c8e4ccb6f53d6cfbbe7f09ca8751f1b611e
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-arena-summons.md
  - docs/agents/tasks/active/OTV2-20260928-dark-merudri.md
  - docs/agents/tasks/archive/OTV2-20260928-dark-merudri.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md
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

Owner decision D45 ("Tak, przez encountery"): a monster spell whose Canary script summons creatures stays an ability of
its monster, but the ability points to an encounter (`encounter`) instead of listing effects, and the encounter's
`ability_cast` rule does the summon with the counters, flags and timers the script keeps.

- Encounter format: the `summon_count(role, op, value)` condition; `offset_tiles(n)` documented as a random free tile
  within n tiles of the subject.
- Monster format: the ability `encounter` branch (D45).
- Razzagorn (four Demons anywhere in the arena; Canary only, the reference-date wiki lists none, so it needs checking),
  Shulgrax (four Sin Devourers as summons and four Damned Souls while he has fewer than 8 summons), The Rage (Frenzy)
  and The Destruction (Disruption, 15 s apart), each fewer than 3.
- The census rises from 1,548 to 1,552 fully resolved monsters. Like every encounter-covered monster, the four are
  deferred by the creature staging until encounters are admitted (`deferred_encounter` 155 -> 159); `content/world`
  keeps 1,319 creatures and only the staged evidence and its pins change.

The Hunger (vortex counter) and the Glooth Generator (per-creature timer) stay unresolved. This PR also archives the
Dark Merudri task record of #1082.

## Acceptance and evidence

- `verify_encounter_schema.py` 116/116 and `verify_formal_schema.py` 233/233; 82 encounters validate.
- The census, the staging, the tree regeneration and its validators pass.
- The Rust tests pass.
- Exact-head review before the Merge Queue because `content/world` changes.

## Completion

Merged as PR #1111 (`b6c75634c8e53a26f6f3ef3a3bfa80a00e1e9c44`) from final head `7d537c8`; required checks passed on that
head and the exact-head Codex review found no issues. The review round restricted encounter-backed abilities to spells.
The census is at 1,552 fully resolved monsters; the four arena bosses wait in the creature staging with the other
encounter-covered monsters (159). Count Vlarkorth, The Hunger and the Glooth Generator remain follow-ups.
Owner released.
