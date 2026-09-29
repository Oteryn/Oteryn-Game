# OTV2-20260928-official-library-sources

```yaml
task_id: OTV2-20260928-official-library-sources
title: Tibia.com library as a monster source and wiki summon/convince costs (owner decision D47)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1175
jira: KAN-16
base_sha: 39d5681e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-official-library-sources.md
  - docs/agents/tasks/active/OTV2-20260928-encounter-admission-wave.md
  - docs/agents/tasks/archive/OTV2-20260928-encounter-admission-wave.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json
  - tools/content-schema/monster-authoring/**
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - content/**
  - imports/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision D47 (2026-09-28) sets the source order for the reference-date monster state:
1. a Tibia.com news item dated on or before the target date;
2. the Tibia.com creature library, captured through TibiaData, while no news item after the target date changes the creature;
3. the wiki at the target date;
4. Canary and CrystalServer as hypotheses.

- `official_library.py` writes `samples/official-library-2026-09-28.json`: library health and experience for 647 Canary monsters.
- The converter adopts those values over Canary and the wiki as an `official_capture` source.
- The converter also adopts the wiki summon/convince costs, which it used to leave out (D15). This affects about 20 monsters, for example Fox, Rorc and White Tiger.

## Acceptance and evidence

- The census, the staging, the materializer, the tree validators and the Rust tests pass.
- Exact-head review before the Merge Queue, because `content/world` changes.
