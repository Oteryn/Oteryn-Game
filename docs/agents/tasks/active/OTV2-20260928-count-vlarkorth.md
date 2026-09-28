# OTV2-20260928-count-vlarkorth

```yaml
task_id: OTV2-20260928-count-vlarkorth
title: Count Vlarkorth per-vocation summon encounter (owner decision D34)
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: null
jira: KAN-16
base_sha: b6c75634c8e53a26f6f3ef3a3bfa80a00e1e9c44
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-count-vlarkorth.md
  - docs/agents/tasks/active/OTV2-20260928-arena-summons.md
  - docs/agents/tasks/archive/OTV2-20260928-arena-summons.md
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

Owner decision D34 ("Wszystkie 7") includes the per-vocation summon; this is its event. Count Vlarkorth
(`count_vlarkorth_transform` and the Good Remains actions) becomes the `count_vlarkorth` encounter:

- Every 11,250 damage (15% of 75,000 health) each player in the room gets one dark creature of its base vocation
  (`spawn_per_player`), which raises a shield; the good remains of that vocation used on the boss lowers it
  (`item_used` with `base_vocation`).
- Where Canary and the reference-date wiki (Fandom Count Vlarkorth rev 1140872) differ, the wiki decides (D25): two
  waves, no damage while the shield holds, and a Dark Merudri with its remains (item 50311) for a Monk (D44).
- Engaging the boss credits the players in the room with the 20-hour cooldown and 30-minute room timer (D27).
- The census rises from 1,552 to 1,553; Count Vlarkorth waits in the creature staging with the other encounter-covered
  monsters. This PR also archives the arena summons task record of #1111.

## Acceptance and evidence

- `verify_encounter_schema.py` 124/124; 83 encounters validate and 78 manifests resolve fully.
- The census, the staging, the tree regeneration and its validators pass.
- The Rust tests pass.
- Exact-head review before the Merge Queue because staged creature evidence changes.
