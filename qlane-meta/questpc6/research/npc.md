# QUESTPC NPC, fee and book research

Scope: 13 source-qualified NPC programs, A Father's Burden, quest-fee boundaries, and ordinary book text transport. Donor code is `OTS_HYPOTHESIS_ONLY` unless stated otherwise. Pinned donors: Crystal `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`; Canary `opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446`.

Fresh 2026-11-06 MediaWiki API GETs used only `https://tibia.fandom.com/api.php`. `A Father's Burden Quest` returned revid 903239 (2022-01-31); its infobox lists 28,000 XP, 8,000 gp and Old Cape. `Tereban` returned revid 1116735 (2025-07-26); its infobox places him in Edron above the Depot. These are current API revisions, although the quest page itself is old. No CipSoft site was queried.

## NPC source catalogue

Counts below were recomputed from each document's `rules`, `tracks`, and distinct `npc` keys. The build script scans `quest-batch3-*.json` from this directory and sorts the filenames (`crates/gameplay/quests/build_npc_programs.rs:43-60`).

| Program | Rules | Tracks | NPCs | Source document |
|---|---:|---:|---:|---|
| Bigfoot's Burden | 284 | 43 | 11 | `content/dialogues/quest-rules/quest-batch3-a-bigfoot_burden.json:4,436` |
| A Father's Burden | 55 | 11 | 1 | `content/dialogues/quest-rules/quest-batch3-a-fathers_burden.json:4,116` |
| Hot Cuisine | 131 | 50 | 1 | `content/dialogues/quest-rules/quest-batch3-a-hot_cuisine.json:4,506` |
| In Service of Yalahar | 259 | 46 | 16 | `content/dialogues/quest-rules/quest-batch3-a-in_service_of_yalahar.json:4,466` |
| The Cursed Crystal / Tibia Tales | 16 | 6 | 1 | `content/dialogues/quest-rules/quest-batch3-a-the_cursed_crystal.json:4,66` |
| The Hidden City of Beregar | 56 | 11 | 10 | `content/dialogues/quest-rules/quest-batch3-a-the_hidden_city_of_beregar.json:4,116` |
| The Way of the Monk | 30 | 17 | 2 | `content/dialogues/quest-rules/quest-batch3-a-the_way_of_the_monk.json:4,176` |
| A Pirate's Tail | 12 | 2 | 2 | `content/dialogues/quest-rules/quest-batch3-b-a_pirates_tail_quest.json:4,26` |
| Hero of Rathleton | 24 | 4 | 4 | `content/dialogues/quest-rules/quest-batch3-b-hero_of_rathleton.json:4,50` |
| Oramond | 36 | 10 | 5 | `content/dialogues/quest-rules/quest-batch3-b-oramond.json:4,144` |
| The Shattered Isles | 115 | 22 | 17 | `content/dialogues/quest-rules/quest-batch3-b-the_shattered_isles_quest.json:4,333` |
| The Order of the Stag | 99 | 6 | 6 | `content/dialogues/quest-rules/quest-batch3-c-the_order_of_the_stag.json:1` |
| Tower Defence | 6 | 2 | 2 | `content/dialogues/quest-rules/quest-batch3-c-tower_defence_quest.json:1` |
| **Total** | **1,123** | **230** | **78 unique NPC keys** | All 13 source documents above |

### Track defaults and range behavior

The catalogue has 180 tracks with `initial=-1` and 50 with `initial=0`; bounds are declared per track, not globally. In A Father's Burden all 11 tracks explicitly use `initial=-1`, `min=-1`, `max=2147483647` (`content/dialogues/quest-rules/quest-batch3-a-fathers_burden.json:4-114`; fixture `crates/gameplay/quests/src/quest/npc_rules/test-fixtures/father_acceptance.json:1-114`). The pure planner substitutes the declared `initial` when the incoming character snapshot omits a declared track; it validates the read and every write against the inclusive `min..=max` range (`crates/gameplay/quests/src/quest/npc_rules/evaluate.rs:39-70`, `crates/gameplay/quests/src/quest/npc_rules/validate.rs:53-62`). Thus `-1` is the initial/unset value for these eleven AFB tracks, while other NPC programs can deliberately start tracks at zero.

Storage `between` guards use an inclusive interval `value..=maximum`; validation requires the upper bound and rejects an inverted range (`crates/gameplay/quests/src/quest/npc_rules.rs:82-92`, `crates/gameplay/quests/src/quest/npc_rules/validate.rs:33-37`). Source references accept a positive single line or an ascending `first-last` pair; validation checks that syntax, not that the referenced donor file's current contents match (`crates/gameplay/quests/src/quest/npc_rules/validate.rs:13-30`; test `crates/gameplay/quests/src/quest/npc_rules/tests.rs:484-493`).

The NPC catalogue/planner is mounted under `cfg(test)` only; it has no production NPC API, adapter, or writer (`crates/gameplay/quests/src/quest/npc_rules.rs:1-18`). The 13 JSONs are therefore source data and pure planner inputs, not proof that the NPC programs execute in game.

## A Father's Burden: Tereban

The donor source uses NPC topic `2` for all material confirmations and stores the selected material in a separate per-player `topic[playerId]` value. The authored Oteryn rules encode that selected material with topics `7201`–`7208`; those are program discriminators, not literal Crystal topic numbers (`Crystal:data-global/npc/tereban_functions.lua:161-170,185-200`; `content/dialogues/quest-rules/quest-batch3-a-fathers_burden.json:337,536,736,935,1134,1335,1534,1733`). Resource keyword lookup is exact after lowercasing, while `mission` and yes/no are `MsgContains` branches (`Crystal:data-global/npc/tereban_functions.lua:132-161,172-200`).

| Dialogue key / material flag | Item surrendered | Donor source for one required item |
|---|---:|---|
| `strong sinew`, Sinew 42954 | 11548 x1 | Use-item 11590 on Heoni's corpse yields 11548; separate corpse flag 42963 (`data-global/scripts/quests/fathers_burden/actions_corpse.lua:1-20`; storage IDs `data-global/lib/core/storages.lua:1703-1718`). |
| `exquisite wood`, Wood 42955 | 11547 x1 | Box 2469 at `(31987,31223,7)` (`data-global/startup/tables/container_quest.lua:282-288`). |
| `spectral cloth`, Cloth 42956 | 11546 x1 | Stone coffin 1983 at `(32122,31912,14)` (`data-global/startup/tables/container_quest.lua:584-589`). |
| `exquisite silk`, Silk 42957 | 11545 x1 | Container 11591 at `(33215,31413,10)` (`data-global/startup/tables/container_quest.lua:498-502`). |
| `magic crystal`, Crystal 42958 | 11552 x1 | Sarcophagus 1994 at `(33235,32803,12)` (`data-global/startup/tables/container_quest.lua:568-573`). |
| `mystic root`, Root 42959 | 11551 x1 | Box 2469 at `(32815,32502,8)` (`data-global/startup/tables/container_quest.lua:400-406`). |
| `old iron`, Iron 42960 | 11549 x1 | Box 2469 at `(32636,31985,14)` (`data-global/startup/tables/container_quest.lua:600-604`). |
| `flexible dragon scale`, Scale 42961 | 11550 x1 | Use-item 11589 on Glitterscale's corpse yields 11550; separate corpse flag 42962 (`data-global/scripts/quests/fathers_burden/actions_corpse.lua:1-20`; storage IDs `data-global/lib/core/storages.lua:1703-1718`). |

Crystal and Canary have the same two corpse mappings and one-time per-character corpse flags: the action checks the relevant flag, adds one material, and sets that flag to `1`; it has no active-quest guard (`Crystal:data-global/scripts/quests/fathers_burden/actions_corpse.lua:1-20`; `Canary:data-otservbr-global/scripts/quests/fathers_burden/actions_corpse.lua:1-20`). The six other items are donor map-container contents at fixed positions, not individualized reward claims; their presence in a target WorldBundle must be qualified separately (`Crystal:data-global/startup/tables/container_quest.lua:282-288,400-406,498-502,568-573,584-589,600-604`).

The quest tracks are `QuestLog=42951`, `Progress=42952`, `Status=42953`, and the eight material flags 42954–42961; corpse flags 42962–42963 are separate from the eleven NPC tracks (`Crystal:data-global/lib/core/storages.lua:1703-1718`). On topic-1 yes, the source writes all eleven in one acceptance path: QuestLog 1, Progress 0, Status 1, and each material flag 1, then topic resets (`Crystal:data-global/npc/tereban_functions.lua:172-184`; the 11 authored track declarations are at `content/dialogues/quest-rules/quest-batch3-a-fathers_burden.json:4-114`).

For each of the eight items, the source asks for one item; yes removes one, sets that material flag to 2, adds 1 to Progress, grants 2,500 XP, and clears the topic. A failed removeItem returns without changing the flag/progress/XP; no and other-message branches also do not deliver an item (`Crystal:data-global/npc/tereban_functions.lua:161-200`). The JSON records the same transfer effects and corresponding branches (`content/dialogues/quest-rules/quest-batch3-a-fathers_burden.json:337-1868`).

After Progress reaches 8 while Status is 1, `mission` sets Status 2, gives item 11701 x1 and 8,000 XP (`Crystal:data-global/npc/tereban_functions.lua:132-148`). The API quest page lists the full 28,000 XP / 8,000 gp / Old Cape reward (revid 903239); the NPC function explicitly calls `addItem(11701)` and `addExperience`, and has no separate `addMoney` operation in that branch (`Crystal:data-global/npc/tereban_functions.lua:139-145`). The donor XML places Tereban at centers `(10191,10085,6)` and `(33170,31807,6)`; the Wiki locates him in Edron above Depot, so only the target world's admitted placement can establish reachability (`Crystal:data-global/world/world-npc.xml:480-482,627-629`; Tereban revid 1116735).

The current planner allows at most eight changed tracks and refuses the AFB eleven-track fixture as `LimitExceeded`, leaving its input facts unchanged (`crates/gameplay/quests/src/quest/npc_rules.rs:25-30`; `crates/gameplay/quests/src/quest/npc_rules/tests.rs:387-402`). The accepted path is to keep this source transition whole and use a separately accepted atomic operation that can apply eleven track changes with session-fenced dispatch/recovery; no split or cap increase is implied (`docs/architecture/reviews/QUEST-FINISH-2026-11-06.md:28-29`).

## NPC fee boundary

The accepted Quest Gate 0 owner answer admits gold hand-ins to quest NPCs as this specific fee source: take backpack coins first, then the remainder from the bank through BANK-0 in the same transaction, refusing if funds are insufficient (`docs/architecture/reviews/OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md:422-438`). D178 defines a closed typed BURN cause per admitted source and forbids a generic cause, reason code, or free-form label; another source requires its own owner amendment (`docs/architecture/reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md:220-234`). The current Quest-Finish record says generic NPC cause/tag 18 lacks the executable helper, receipt/custody proof, SQL owner extension and canonical audit qualification; cause 15 remains Rotten Blood only (`docs/architecture/reviews/QUEST-FINISH-2026-11-06.md:30`). Do not treat the specific quest-fee answer as a generic fee authorization.

## Book text and USE transport

The manual says books, scrolls and letters show a generic description on Look and that reading/writing uses Use (`docs/reference/tibia-manual/controls.md:30-33`; classified `CIPSOFT_OFFICIAL` in `docs/architecture/reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md:64-67`). The WRITE-0 decision identifies the text window as opcode `0x96` (`docs/architecture/reviews/OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md:79-80`).

General `readable`, `writeable`, `allowdistread`, `maxtextlen`, and `writeonceitemid` item fields are DATA-ONLY for books/signs: promotion exists, but the parity audit finds no production general READ/USE text-window consumer, and the mail text policy has no live dispatch caller (`docs/architecture/reviews/OTERYN_GAME_CANARY_PARITY_INVENTORY_2026-10-25.md:252-257`). QUESTPC Q13 accepts durable authored-text grants while leaving ordinary carried-book READ/TextWindow out and LOOK unchanged (`docs/agents/evidence/chatgpt-quests-chest-coverage.md:43,153`). Accordingly, a book's authored text or a durable item text field does not itself open a client text window; the server/client route and bounded wire contract are still absent, with capabilities 289 and 360–369 plus commands 620–639 unused in this task (`docs/architecture/reviews/QUEST-FINISH-2026-11-06.md:31`).

## Implementable implications

- Preserve the 11-track AFB acceptance as one atomic change; the material flow is one item → one flag 2 → Progress +1 → 2,500 XP, followed by Status 2 / item 11701 / 8,000 XP at Progress 8.
- Interpret absent AFB tracks as `-1`; do not generalize that initial value to the full 230-track catalogue. The declared ranges and `between` guards are inclusive.
- Treat corpse and six container sources as donor behavior only until their live target-world placements/content are qualified.
- Do not introduce or reuse a generic NPC gold-fee cause; the quest-specific hand-in authorization has a distinct backpack-then-bank transaction contract.
- A general book read needs an actual server/client USE-to-TextWindow consumer; item text fields alone provide authored data, not the interaction.

## Run8 API refresh (2026-11-06)
The owner-provided honest-UA api.php helper now succeeds. Historical403/unavailable statements above describe prior attempts and are superseded for captured pages by `.questpc/wiki-r8/` (raw page text, revision metadata, SHA256 captures). Current inventory refreshed788 infobox pages; all373 pinned titles occur. Infobox presence is not a quest/access completion claim. Current Wiki Newhaven/Viola still selects51977→3572+800XP; Corruptor ordinary names resolve stale donor IDs as explicitly documented. Charge item pages establish item defaults, not charge quantities of specific chest grants; explicit donor reward count remains selected. SoulWar page/spoiler refreshed, implementation belongs questpc3 R..S in run8.
