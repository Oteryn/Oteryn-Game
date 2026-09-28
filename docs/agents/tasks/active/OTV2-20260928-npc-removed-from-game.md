# OTV2-20260928-npc-removed-from-game

```yaml
task_id: OTV2-20260928-npc-removed-from-game
title: NPC D11 and D12 - hold NPCs both wikis record as removed from Tibia Global, and take offer prices both wikis agree on
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1124
jira: KAN-16
base_sha: 943e17b07ba42c95a91f6a431990905846a69968
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-npc-removed-from-game.md
  - docs/agents/tasks/active/OTV2-20260928-npc-tibiawiki-br-crosscheck.md
  - docs/agents/tasks/archive/OTV2-20260928-npc-tibiawiki-br-crosscheck.md
  - tools/content-schema/npc-authoring/promotion_candidates.py
  - tools/content-schema/npc-authoring/test_promotion.py
  - tools/content-schema/npc-authoring/validate_promotion.py
  - tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json
  - tools/content-schema/npc-authoring/samples/tibiawiki-br-crosscheck-v1.json
  - docs/agents/evidence/OTV2-20260928-npc-dialogue-wave-a-staged.json
  - docs/agents/evidence/OTV2-20260927-npc-admission-wave-a-staged.json
  - docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - tools/content-migration/npc_admission_stage.py
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/world/**
  - content/manifest.json
  - content/content.lock.json
  - content/npcs/**
  - content/behaviors/**
  - content/presentations/**
  - content/services/**
  - content/dialogues/**
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
public_contracts: []
depends_on:
  - OTV2-20260928-npc-tibiawiki-br-crosscheck
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The TibiaWiki BR cross-check found five admitted NPCs that BR records as removed in 13.12:
Brom, Brutus, Roughington, Shadowpunch and Victor, the Duelling Arena supervisors. TibiaWiki Fandom
agrees (`status = deprecated`). Under D11 they are held `REMOVED_FROM_GAME` through a fixed table in
`promotion_candidates.py`, checked before any wiki matching, like the `OWNER_REJECTED` table.

Under D12 the candidates also read the committed TibiaWiki BR facts (`--br-facts`). An admitted offer
takes the price TibiaWiki Fandom and TibiaWiki BR both state for that NPC, item and direction, when it
differs from the source price and every BR row of the offer gives it. 21 offers of 13 NPCs change, for
example fishing rods and ropes at Ahmet, Bezil, Gree Dee, Halif and Nezil. One wiki alone, or two wikis
that disagree, never change a price. The cross-check's explicit price differences drop from 28 to 7.
A `WIKI_PRICE` row is valid only with the BR facts digest. The staged packet carries that digest, and
the World project records the BR facts as their own import `g4-npc-prices-tibiawiki-br-r1`
(`tibiawiki.com.br`, facts SHA-256), next to the Fandom NPC import.

Wave A becomes 1,088 NPCs, with 307 trade and 55 travel Services and 2,282 bindings (the D12 prices add
wiki page bindings). Victor's trade Service goes with him. The 701 Dialogues are unchanged, because none
of the five had one. Two stale shards left in `main` by #1082 (`behaviors-02000-02410.json`,
`presentations-02000-02410.json`, referenced by no index) are removed with the regenerated tree.

Seven other admitted NPCs are marked deprecated by Fandom only. BR does not record them as removed:
Carlos, Santiago and Zirella (Rookgaard tutorial, still in game but no longer reached), Elgar (Travora),
Lou Toose and Vad Inchi (Royal Painting Contest) and Sane Mage. They stay admitted (owner decision in
this session: keep them, they may be useful later).

Authority: owner request in this session to reproduce Tibia Global 1:1 and to cross-check every NPC
against TibiaWiki BR ("Kontynuuj" and "Tak kontynuuj" on the proposed fix order: removed NPCs, then prices both wikis agree on).

## Acceptance and evidence

- The candidates reproduced byte for byte before the change. After it, `validate_promotion` and
  `test_promotion` (51 tests, including REMOVED_FROM_GAME, WIKI_PRICE, missing-BR-digest, unknown-offer, offer-variant, wrong-item
  and wrong-price cases) pass; with the pinned Fandom snapshot and BR facts, `validate_promotion` confirms
  every WIKI_PRICE row, and every admitted offer both wikis price (877), carries the price both wikis state.
- The dialogue and admission stages re-run with the same inputs as #1095.
- `materialize_content_world_project_v2` verifies 1,088 NPCs and 2,151 NPC-side declarations.
- `content_world_project_repository` pins the new documents and the tree digest.
- The tree generator, its validator and tests, and `validate_materialized_game_tree` pass. The
  cross-check is regenerated.
