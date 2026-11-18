# QCX86 — The Isle of Evil Quest

Base `ccbd37016f33b984a98bdd45e422013cc138f877`. Quest `oteryn:quest.the_isle_of_evil_quest`.

Global sources are the [quest page, curid 36716 oldid 843974](https://tibia.fandom.com/?curid=36716&oldid=843974) and [spoiler, curid 36717 oldid 855959](https://tibia.fandom.com/?curid=36717&oldid=855959) from facts commit `460e1d8c2c0e88c438ae5e54910db00822dc0129`. Crystal fallback is [201189f871d23597b82caf88fd611454a2ddefb8](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global). The spoiler's Method permits asking about the Isle of Evil directly. Its transcript also describes the longer keyword sequence, which remains available.

| Part | Global and donor comparison with the base |
| --- | --- |
| Requirements | The imported definition already records level 35 and premium. Level 50 is recommended in the wiki. Rope and shovel or equivalents and Yalahar access are travel/equipment requirements. The level-35 Gate of Expertise belongs on the ship approach. At (32681,31992,14) the imported map has appearance 7727 without a level action. The donor door-06 binding candidate specifies appearance 1644 and action 42538 at the same cell. |
| Introduction and optional book | The live dialogue required Crystal's introductory topic chain (`npc/rapanaio.lua:56-188`). The direct keyword now opens the existing offer. The imported west door at (32678,31990,14) has appearance 7721 and action 42529. The quest record has no optional book claim. |
| 1. Fishing trip | One Mechanical Fishing Rod (9306), then one Mechanical Fish (9307), including a market fish. The two value exchanges were held. Main already implements success in the Yalahar sewer, with a nail (953) and spent water. The spoiler describes a failed attempt that can lose a nail and cause 35 energy damage plus a short skeleton appearance. Crystal retains the nail on failure and rolls for the shock separately (`scripts/actions/other/mechanical_fishing.lua:212-248`). The dispatch success path leaves the failure effects unbound. |
| 2. Fuel | Ten rum vials (2874, donor fluid subtype 13). The dialogue prompt and stage exist, and the hand-in is held. |
| 3. Something Sweet | One fruit juice vial (2874, donor fluid subtype 14) made by using a Juice Squeezer on fruit while carrying a vial. The dialogue prompt and stage exist, and the hand-in is held. |
| 4. Technomancer | Scutty lends one Worm Punisher (9234); the south door's import identity is 7723/action 42534 at (32576,31923,12). Six Carrion Worm hits with the hammer heated for ten seconds earn Worm Whacker. A drilling worm hit resets the hit count and incurs 500 gold. Scutty then issues one Machine Crate (9390), which Rapanaio takes. At base the NPC grants and hand-in were held. The minigame and fee remain unbound. |
| 5. Nautical Map | The Mortimer dialogue and its progress write already exist. The beach anchor (32531,31628,7) contains buried terrain 615. The wiki's box anchor (32576,31572,10) contains box 2473 without unique or action metadata. No native quest claim binds that box to the map. Crystal’s reward row 6340 places the box at erroneous floor 110, while its separate row 9253 places the bare box at floor 10 (`startup/tables/chest.lua:3158-3165,3201-3205`). The subsequent hand-in of one map (9308) was held. |
| 6. Ship and access | Preparing departure already advances progress. The boat confirmation is held, including the same-action storage writes and departure to (32668,31457,6). The two donor ship-exit tiles (32671,31454/31455,6) have no Isle guard callback. The re-entry flag is already written when mission 7 starts. The donor's topic-9 outbound repeat is unreachable inside its topic 1–8 branch. A free S10 return exists on the keyword `kazordoon`; the spoiler specifies `back` then `yes` after the report. |
| 7. Castle and bosses | The upper teleport matches Crystal (`movements_teleport_isle.lua:3-15`). The trap lever is present at (32756,31469,6) as appearance 2772 with no action. Crystal's lever script uses action 57237, already used for a No Rest For The Wicked item in its startup table (`startup/tables/item.lua:5116-5120`). The spoiler requires killing Doctor Perhaps, Dirtbeard, Mephiles, Boogey, Monstor then Evil Mastermind. It closes earlier corpse claims on using the final corpse. Crystal rewards any one of each earlier corpse's four entries, grants one card (9391) from the final corpse and advances only after all six claims. It checks mission stage and per-corpse KV, without proving actual kill credit. The repo has no corresponding bound Isle corpse claims. |
| Finale | One Fan Doll (9389), King Tibianus Fan, 6,666 XP and quest completion. Crystal's King callback grants the doll and achievement but no XP. The global callback keeps the shown card and delivers all rewards with completion in one fenced write. The wiki lists no outfit reward. |

Crystal's corpse table binds the following physical IDs. Each successful claim awards one item from the listed row. The available global facts specify the rule and the final card by name. [Crystal corpse table, lines 4–34](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_isle_of_evil_quest/actions_corpse_bosses.lua#L4-L34)

| Boss | Crystal corpse | Crystal reward IDs |
| --- | ---: | --- |
| Doctor Perhaps | 9402 | 9399, 9383, 9372, 9373 |
| Dirtbeard | 9403 | 9374, 9401, 9375, 9382 |
| Mephiles | 9406 | 9376, 9387, 9400, 9377 |
| Boogey | 9407 | 9379, 9385, 9384, 9378 |
| Monstor | 9405 | 9381, 9386, 9380, 9396 |
| Evil Mastermind | 9404 | 9391 |

The checked `world-callbacks-held` files contain no Isle rows to promote. The shared NPC callback registry carries the seven Isle records described above. The loader also registers the exact predecessor definition so characters with an existing quest pin can use the new callbacks. The definition and mission binding artifacts still describe the source projection, whose claim lists are empty. The door selector inventory resolves actions 42529, 42534, 42536 and 42538 to `oteryn:quest-progress/quest/…`, while the dedicated quest writes `oteryn:quest-progress/crystalserver/quest/…`. The NPC tests verify the written flags. World door admission still needs the matching track binding. Mortimer, Scutty, and the three Rapanaio variants have NPC definitions and Crystal spawn evidence (`world/world-npc.xml:558-560,1441-1443,3011-3013,3185-3190)); the native placement family inspected here contains King Tibianus only among these NPCs.

## Blockers

- BLOCKER Mission 1 sewer failure atomic failed-cast nail/energy/temporary outfit effect `data-global/scripts/actions/other/mechanical_fishing.lua:222-237`
- BLOCKER Missions 2–3 NPC hand-in fluid-subtype-specific transactional intake `data-global/npc/rapanaio.lua:206-235`
- BLOCKER Mission 4 rotating worm targets and timed carried transformation bound to minigame progress `data-global/scripts/quests/the_isle_of_evil_quest/actions_heat_worm_punisher.lua:8-19`
- BLOCKER Mission 6 ship exit disjunctive read-only STEP predicate `data-global/scripts/quests/the_isle_of_evil_quest/movements_isle_enter.lua:11-26`
- BLOCKER Mission 7 corpse rewards six individual boss credits bound to direct corpse-use claims `data-global/scripts/quests/the_isle_of_evil_quest/actions_corpse_bosses.lua:36-72`

## OPEN-FACTS

The global facts do not state exact nail loss or shock odds, skeleton duration, premium check timing, the optional book's exact item/box identity, the map box's native unique ID, the trap's action ID, earlier corpse loot IDs/odds or the native return coordinate. Crystal IDs and storage offsets cited above are implementation evidence. The global facts give no Nautical Map weight. Native custody retains the existing canonical 8.20 oz, with Crystal and Canary XML at 8.19 oz. No extra global quantities are inferred.

## PARTIAL

Optional book claim, fuel and juice exchanges, worm minigame and reset fee, Nautical Map box, production NPC placement admission, door track bindings, level-35 ship gate and both travel flows, castle trap and boss corpse progression, and parity of the generated definition/claim projection.
