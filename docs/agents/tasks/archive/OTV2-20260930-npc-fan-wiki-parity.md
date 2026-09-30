# OTV2-20260930-npc-fan-wiki-parity

```yaml
task_id: OTV2-20260930-npc-fan-wiki-parity
title: NPC D15 - admit NPCs Tibiopedia or BR confirm and mark disputed prices PARITY_PENDING
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1337
jira: KAN-16
base_sha: 67cd501a8d2cf0073368622d5b59c414ce27369c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/archive/OTV2-20260930-npc-fan-wiki-parity.md
  - tools/content-schema/npc-authoring/promotion_candidates.py
  - tools/content-schema/npc-authoring/validate_promotion.py
  - tools/content-schema/npc-authoring/test_promotion.py
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
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - apps/game-server/tests/content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_v2_npc_admission.rs
  - content/**
  - imports/crystalserver/batches.json
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
public_contracts: []
depends_on:
  - OTV2-20260929-npc-crystal-supplement
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This applies the architect ruling of 2026-09-30 on #162 (answers 5a and 6b) as NPC authoring rule D15.

- **Fan-wiki confirmation.** Nine single-source Crystal NPCs are unknown to Fandom, and Tibiopedia confirms all
  of them. S'Zallar M'Andar and The Silent Oarsman are also on TibiaWiki BR.
  - Each gets a `FAN_WIKI_CONFIRMED` row that records the confirming wikis and their pages.
  - Eight enter wave A. Seven of them have no Crystal spawn and are admitted with no placements, as D8 does for
    Fandom: six Lion knights and archers and Raubritter Chastener.
  - S'Zallar M'Andar stays deferred as `MOVEMENT_UNDECLARED`.
- **Pending prices.** 77 admitted plain source offers have a price that no two wikis settle, while some wiki
  states a different price. They keep the source price and carry `parity_pending`.
  - The ruling cited 22, an earlier ad hoc count that cannot be reproduced. The rule as written gives 77: 50
    offers where two wikis disagree (mostly Fandom and Tibiopedia on parcels and letters) and 27 where a single
    wiki disagrees with the source.
  - An official source settles these later. Exact price evidence stays a parity gate before a production
    release (NPC-0 §7).
- **ITEM-ID-1b regression fixed.** Since #1305 the D13 offer rule had dropped all 894 wiki offers, and the
  admission stage rejected the retired Item keys of the pinned item map. With both fixes, main's candidates and
  both staged packets regenerate byte for byte.
- **Wave A** now has 1,102 NPCs, 715 Dialogues, 322 trade and 56 travel Services, 12,626 offers and 2,352
  bindings.
- **Not in scope.** The ruling's minimal generated replies (answer 3b) belong to NPC-CONTENT-1 (NPC-0 §3.3), so
  this task does not add them.

## Acceptance and evidence

- `test_promotion.py`: 67 tests pass.
- `validate_promotion`, with the source-backed rebuild: 1104/1104 candidates valid.
- The dialogue and admission stages re-run.
- The materializer run gives 1,102 NPCs and 2,195 declarations.
- These pass:
  - the tree generator, its validator and its tests;
  - `validate_materialized_game_tree`;
  - `content_world_project_repository`, `content_world_project_v2` and
    `content_world_project_v2_npc_admission`;
  - `cargo fmt` and governance.

## PR and closeout

- PR #1337. The merge is recorded as squash merge of #1337. The frozen head and its review are recorded in the
  PR's FREEZE_SHA entry.
