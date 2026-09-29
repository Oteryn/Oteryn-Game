# OTV2-20260929-npc-crystal-supplement

```yaml
task_id: OTV2-20260929-npc-crystal-supplement
title: NPC D14 - admit the NPC files Crystal added after its pinned revision from the pinned summer-update commit
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1200
jira: KAN-16
base_sha: fb204ea3bac7b89b051e7a60c2036c65e506d7e7
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260929-npc-crystal-supplement.md
  - tools/content-schema/npc-authoring/convert.py
  - tools/content-schema/npc-authoring/promotion_candidates.py
  - tools/content-schema/npc-authoring/validate_promotion.py
  - tools/content-schema/npc-authoring/test_promotion.py
  - tools/content-schema/npc-authoring/README.md
  - tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json
  - tools/content-schema/npc-authoring/samples/tibiawiki-br-crosscheck-v1.json
  - tools/content-migration/npc_admission_stage.py
  - tools/content-migration/npc_dialogue_stage.py
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - docs/agents/evidence/OTV2-20260928-npc-dialogue-wave-a-staged.json
  - docs/agents/evidence/OTV2-20260927-npc-admission-wave-a-staged.json
  - docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/**
  - imports/crystalserver/**
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
public_contracts: []
depends_on:
  - OTV2-20260928-npc-wiki-offers
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Crystal's `summer-update` branch adds 20 NPC files after the pinned revision `ff7ede59`. Under D14 they come
from one more pinned commit, `00ce02a5`.

- `convert.py --source crystal-summer` converts that commit with the same `crystal:npc/<stem>` keys. The
  committed converter reproduces all 20 listed files.
- `promotion_candidates.py --crystal-supplement` reads only the listed files.
- Seven are admitted:
  - six are new: Captain Corsarah, Javala, Mayor Pocaro, Pescadu, Thorim and Wayland Smythers;
  - Uzon Back, already admitted from Canary, gains its Crystal file.
- Thirteen are held `SUPPLEMENT_HELD`, each with the reason found in review:
  - a placeholder outfit: Dhira, Nilavarna, Niral, Saraki, Sharai, Tarisu, Udu;
  - dialogue Crystal wrote itself: G'ezkho, Goldro, Nekaret, Omar, Zofia Bolter;
  - not an NPC on the wiki: Doctor Marrow.
- Provenance of a supplement file records its revision, and its NPC file binding names that revision.
- The dialogue stage authenticates the supplement reference bundles by the census-style digest the candidates
  record.
- The World project records the supplement as its own Crystal import, `g4-npc-crystal-summer-supplement-r1`.

Wave A becomes 1,094 NPCs with 707 Dialogues, 322 trade and 56 travel Services and 2,344 bindings.

Authority: owner decision on the Crystal `summer-update` supplement ("admit the seven, hold the thirteen with
their reasons") and "kontynuuj" once the permission mode allowed the `convert.py` change.

## Acceptance and evidence

- Without `--crystal-supplement`, the candidates reproduce `main` byte for byte.
- With it, `validate_promotion` passes: 1,095/1,095 candidates, including the rebuild from the Canary, Crystal
  and supplement bundles.
- `test_promotion` (62 tests) passes.
- The dialogue and admission stages re-run.
- `materialize_content_world_project_v2` verifies 1,094 NPCs and 2,179 declarations.
- The tree generator, its validator and tests, `validate_materialized_game_tree` and
  `content_world_project_repository` pass.
