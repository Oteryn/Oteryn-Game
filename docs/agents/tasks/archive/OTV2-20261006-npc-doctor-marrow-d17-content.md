# OTV2-20261006-npc-doctor-marrow-d17-content

```yaml
task_id: OTV2-20261006-npc-doctor-marrow-d17-content
title: "NPC authoring D17: PLACEMENT_HELD, dialogue links and source_incomplete, Rust model"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: content
base_branch: main
branch: agent/npc-doctor-marrow-d17-20261006
pr: 1903
base_sha: c5353e02
owner: claude-code-session-012HxnzhyMseH5DyNLy7dPmM
control_plane: claude-code-session-0114oBVR3osF1auvFMu6ksMH
created_at: 2026-10-06
updated_at: 2026-10-06
authority: "control-plane packet for D17 (a)-(e); rulings D867 and D868"
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owned_paths:
  - tools/content-schema/npc-authoring/**
  - tools/content-migration/npc_dialogue_stage.py
  - tools/content-migration/npc_admission_stage.py
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/src/content/npc_catalogue/service_tests.rs
  - apps/game-server/tests/content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_v2_npc_admission.rs
  - docs/agents/tasks/archive/OTV2-20261006-npc-doctor-marrow-d17-content.md
```

## Scope

D17 (a)-(e): `PLACEMENT_HELD` table (Doctor Marrow admitted with zero placements and a PLACEMENT_HELD
arbitration row), dialogue keyword `links` and `source_incomplete`, validators and tests, Rust model.

Deviation from D17: Marrow records two `NO_HANDLER` entries, `stopped`→`traitor` and `hesitate`→`helping`
(the second link is answered only by a Lua callback, which is no KeywordNode handler). Ruling D867.

## Decisions

- D868: shard regeneration is not shipped. The pinned snapshot sha `2fcbc6ff…` could not be reproduced:
  all 1038 pinned NPC revids are unchanged, but item-page and trade drift changes 71 other NPCs on a fresh
  fetch. Marrow shard regeneration is a follow-up blocked on snapshot reproducibility.

## Validation

- `python -m unittest discover` in `tools/content-schema/npc-authoring`: OK (180 tests, 8 skipped)
- `cargo test --test content_world_project_v2 --test content_world_project_v2_npc_admission`: pass
- `cargo test --lib content::` (game-server): pass (305 passed)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (59 tests)
- `git diff --check`: pass
