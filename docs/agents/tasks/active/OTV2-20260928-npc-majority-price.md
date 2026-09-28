# OTV2-20260928-npc-majority-price

```yaml
task_id: OTV2-20260928-npc-majority-price
title: NPC D13 - Tibiopedia as the third wiki; offer prices two of three wikis agree on
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/dazzling-brown-1u2xxo
issue: 162
pr: null
jira: KAN-16
base_sha: 7d1134f090ac249f964fede017efabba91e22b90
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01RTD1d7GsT7uFSBHg5syB4T
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260928-npc-majority-price.md
  - tools/content-schema/npc-authoring/tibiopedia.py
  - tools/content-schema/npc-authoring/promotion_candidates.py
  - tools/content-schema/npc-authoring/validate_promotion.py
  - tools/content-schema/npc-authoring/test_promotion.py
  - tools/content-schema/npc-authoring/README.md
  - tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json
  - tools/content-schema/npc-authoring/samples/tibiawiki-br-crosscheck-v1.json
  - imports/tibiawiki/npc-tibiopedia/**
  - imports/tibiawiki/batches.json
  - imports/tibiawiki/sources.json
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
  - content/**/index.json
  - content/services/**
  - content/npcs/**
public_contracts: []
depends_on:
  - OTV2-20260928-npc-removed-from-game
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Tibiopedia (tibiopedia.pl) becomes the third wiki for NPC offer prices, next to TibiaWiki Fandom and
TibiaWiki BR (D13). `tibiopedia.py fetch` reads the NPC list from the site's sitemap. For each page it
keeps only trade facts: the trade flag, and the item and price of each row. No place, profession or other
page text is kept (D3). The facts are committed under `imports/tibiawiki/npc-tibiopedia/@DATE@/`.

A wiki states a price for an offer when it lists that NPC, item and direction with one explicit price in
every row. When D12 does not apply and two of the three wikis state the same price, that price replaces a
differing source price (`WIKI_MAJORITY_PRICE`, with the agreeing wikis recorded). The offer is looked up
under its registered Item name, so a source's own item name never decides it. An offer with a count or sub
type (a fluid, charges) is not changed.

- @CHANGED@ offers of @NPCS@ NPCs change: @SPLIT@. Most are the part-by-part bed prices of 15 furniture
  sellers (Canary/Crystal price every part at 40), lesser gems and onyx chips at the jewellers, and
  machetes.
- Where only Tibiopedia disagrees with the source, and Fandom (and often BR) confirm the source price, the
  source price stays. Example: Christine's food prices, which Tibiopedia lists lower.
- The BR cross-check's explicit price differences drop from 7 to 5 (Sessek's roll, Sundara's blank rune).
- Wave A stays 1,088 NPCs, with 307 trade and 55 travel Services. Bindings go from 2,282 to @BINDINGS@,
  because the wiki now decides a fact for more NPCs and their Fandom page bindings are added. The World
  project records the Tibiopedia facts as their own import `g4-npc-prices-tibiopedia-r1` (`tibiopedia.pl`,
  facts SHA-256).

Measured before the change, on 998 matched NPCs: 972 offers have a price in all three wikis, and 98% of
them agree. Where Fandom and BR disagree (9 offers), Tibiopedia sides with Fandom 4 times and with BR 5
times, so it is not a copy of either. Its "Handel: nie" flag is wrong for some Dawnport shopkeepers
(Coltrayne, Richard, Hamish), so the flag alone never removes a shop.

Authority: owner decision in this session ("tak" on the proposed D13: a price two of three wikis agree on;
Tibiopedia as the third wiki; its facts committed without prose).

## Acceptance and evidence

- The candidates reproduce byte for byte without `--tibiopedia-facts`. With it, `validate_promotion` passes
  with the three pinned wikis: every `WIKI_MAJORITY_PRICE` row is recomputed, and no admitted plain offer
  keeps a price two wikis contradict.
- `test_promotion` (55 tests) passes, including majority, one-wiki, three-way-split, unparsed-price, D12
  precedence, fluid-offer, row-shape, missing-digest and pinned-wiki cases. `tibiopedia.py self-test`
  passes.
- The dialogue and admission stages re-run with the same inputs as #1124.
- `materialize_content_world_project_v2` verifies 1,088 NPCs and 2,151 NPC-side declarations.
- `content_world_project_repository` pins the new documents and the tree digest. The tree generator, its
  validator and tests pass. The BR cross-check is regenerated.

## Follow-ups

- Wiki-confirmed offers that Canary and Crystal lack (about 735 in existing shops) and the BR-only shops,
  under the same two-of-three rule (second D13 PR).
- The Crystal `summer-update` supplement, 65 NPCs with no dialogue match.
