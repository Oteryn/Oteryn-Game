# OTV2-20260930-npc-reviewed-definitions

```yaml
task_id: OTV2-20260930-npc-reviewed-definitions
title: NPC D16 - settle definition conflicts in review, confirm offers by wiki majority, drop dialogue Crystal wrote itself
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1358
jira: KAN-16
base_sha: 1852a69d801e6ac88ade0e917cc3cac386854000
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/archive/OTV2-20260930-npc-reviewed-definitions.md
  - tools/content-schema/npc-authoring/promotion_candidates.py
  - tools/content-schema/npc-authoring/validate_promotion.py
  - tools/content-schema/npc-authoring/test_promotion.py
  - tools/content-schema/npc-authoring/wiki_outfit_fit.py
  - tools/content-schema/npc-authoring/README.md
  - tools/content-migration/npc_dialogue_stage.py
  - tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json
  - tools/content-schema/npc-authoring/samples/tibiawiki-br-crosscheck-v1.json
  - tools/content-migration/npc_admission_stage.py
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
  - imports/crystalserver/batches.json
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
public_contracts: []
depends_on:
  - OTV2-20260930-npc-fan-wiki-parity
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decisions 2026-09-30 ("2 oraz 3 i 4", "tak" on the image-fit rule, "ok" on generated replies), rule D16:

- 42 left-out offers are admitted where two of three wikis state the source price (`WIKI_MAJORITY_ARBITER`).
- Straw Mat Foot Section maps to its registered Item name (`WIKI_ITEM_NAMES`).
- Eight of nine definition conflicts are settled in review (`DEFINITION_REVIEWED`); no wiki has outfit or movement
  fields, so the evidence is the TibiaWiki image rendered and fitted from the client sprites (`wiki_outfit_fit.py`)
  or the owner's observation. Testserver Assistant stays held.
- 35 single-source Crystal NPCs mark facts TODO (`SOURCE_UNCONFIRMED`); none has a transcript in s2ward/TibiaSecrets.
  30 lose the dialogue Crystal wrote itself (`SOURCE_AUTHORED_TEXT`, NPC-0 generated replies instead) and seven take
  wiki-fitted outfit colours.
- Wave A: 1,110 NPCs, 694 Dialogues, 324 trade and 56 travel Services, 12,725 offers, 2,376 bindings.

## Acceptance and evidence

- `test_promotion.py` 70 tests; `validate_promotion` with the source-backed rebuild 1112/1112.
- Dialogue and admission stages; materializer 1,110 NPCs; tree generator, validator and tests;
  `validate_materialized_game_tree`; `content_world_project_repository`, `content_world_project_v2` and
  `content_world_project_v2_npc_admission`; `cargo fmt`; governance.

## PR and closeout

- PR #1358; merge recorded as squash merge of #1358; the frozen head and review are in the PR's FREEZE_SHA entry.
