# OTV2-20260928-npc-wiki-offers

```yaml
task_id: OTV2-20260928-npc-wiki-offers
title: NPC D13 offers - admit the plain offers two of three wikis list with the same price
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: 1185
jira: KAN-16
base_sha: 0b5c92c21fe8992666234b0cdf3210dc1e733786
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-npc-wiki-offers.md
  - tools/content-schema/npc-authoring/promotion_candidates.py
  - tools/content-schema/npc-authoring/validate_promotion.py
  - tools/content-schema/npc-authoring/test_promotion.py
  - tools/content-schema/npc-authoring/README.md
  - tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json
  - tools/content-schema/npc-authoring/samples/tibiawiki-br-crosscheck-v1.json
  - docs/agents/evidence/OTV2-20260928-npc-dialogue-wave-a-staged.json
  - docs/agents/evidence/OTV2-20260927-npc-admission-wave-a-staged.json
  - docs/architecture/OTERYN_NPC_AUTHORING_SCHEMA_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1.md
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/world/**
  - content/manifest.json
  - content/content.lock.json
  - content/**/index.json
  - content/services/**
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
public_contracts: []
depends_on:
  - OTV2-20260928-npc-majority-price
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The second half of D13. With the Tibiopedia facts, an admitted gold shop gets every plain offer that two of the
three wikis (Fandom, TibiaWiki BR, Tibiopedia) list for that NPC and direction with the same price. Three
conditions apply: the wiki item name is exactly one registered Item's name; the sources have no offer of that
Item and direction; and the sources do not offer that Item in a left-out offer (gated, unconfirmed or
conflicting). An admitted NPC with no source trade gets a gold trade
Service from such offers alone. Each added offer carries a `WIKI_OFFER` row with its agreeing wikis. Given the
pinned item map, the validator re-derives every candidate's rows, so an omitted or invented wiki offer fails.
A fixed `WIKI_SHOP_HELD` table keeps quest, event and token shops out: Cillia, Grizzly Adams, Gnomux, Walter
Jaeger and Ruprecht.

- 894 offers at 91 NPCs. The largest are at Rock In A Hard Place, Shanar, Yasir, Raffael and Brengus.
- 15 new gold trade Services: Ashtamor, Blind Orc, Captain Haba, Dal the Huntress, Fral the Butcher,
  Hjaern, Humgolf, Iyad, Jehan the Baker, Lucius, Partos, Raffael, Rock In A Hard Place, Tefrit and Vigintius.
- Wave A: 1,088 NPCs, 2,166 NPC-side declarations, 322 trade and 55 travel Services, 12,626 offer rows and
  2,330 bindings. Dialogues are unchanged. The BR cross-check's BR-only shops drop from 64 to 52.

Authority: owner decision in this session ("tak" on D13, which includes new offers backed by two wikis).

## Acceptance and evidence

- `test_promotion` (58 tests) passes. `validate_promotion` passes (1,089/1,089) with the three pinned wikis and
  the item map.
- The dialogue and admission stages re-run; `materialize_content_world_project_v2` verifies 1,088 NPCs and
  2,166 declarations.
- The tree generator, its validator and tests, and `validate_materialized_game_tree` pass.
  `content_world_project_repository` passes with new pins.
