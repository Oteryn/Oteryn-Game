# QCX41 — The Pits of Inferno Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`. Quest `oteryn:quest.the_pits_of_inferno_quest`, definition shard 00000–00099, record 89.

Global evidence comes from the November 17, 2026 wikitext capture at qfacts commit `460e1d8c2c0e88c438ae5e54910db00822dc0129`. [TibiaWiki quest, curid 12220 oldid 1103332](https://tibia.fandom.com/?curid=12220&oldid=1103332) · [TibiaWiki spoiler, curid 12226 oldid 1154024](https://tibia.fandom.com/?curid=12226&oldid=1154024)

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` supplies the legacy selectors and item IDs. The donor checkout was absent from `docs/reference` in this environment, so the audit used files from the pinned upstream revision and cross-checked the imported projections. Donor-only details below are identified as such. [Crystal chest table](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2170)

## Reward and entry-item claims

The project placement tiles dropped the startup UIDs. The four B3 region digests match the placement index and the eleven target tiles have no overlapping imported claim. The reward binder requires the source UID or an explicit native placement alias. The patch adds 11 aliases for verified untagged ordinal-1 placements. The original UID, coordinates, appearance, claim identity, counts, contents and once-per-character policy are retained. The three weapon placements share one claim identity.

| Global reward or prerequisite | Imported item IDs and counts | Crystal UID | Native binding |
| --- | --- | --- | --- |
| Crystal Key 3700 | 2971 ×1, key number 3700 | 5024 | Alias at 32842, 32225, 8 |
| Holy Tible | 2836 ×1, donor book text | 6202 | Alias at 32828, 32340, 7 |
| Pirate Backpack | 5926 ×1 containing Ceremonial Ankh 6561 ×1, Death Ring 6299 ×1, Life Ring 3052 ×1, Orichalcum Pearl 5021 ×3, White Pearl 3026 ×5, Platinum Coin 3035 ×11, Soul Orb 5944 ×2, Ultimate Healing Rune 3160 ×3, Sudden Death Rune 3155 ×2, Blank Rune 3147 ×1, Great Mana Potion 238 ×1 | 6204 | Alias at 32832, 32277, 10 |
| Backpack of Holding | 3253 ×1, silver emblems Ultimate Healing 11603, Sudden Death 11609, Explosion 11607 and Heavy Magic Missile 11605 ×1 each | 6205 | Alias at 32826, 32232, 11 |
| Platinum Coins | 3035 ×100 | 6206 | Alias at 32824, 32232, 11 |
| Frozen Starlight | 3249 ×1 | 6207 | Alias at 32819, 32232, 11 |
| Stuffed Dragon | 5791 ×1 | 6208 | Alias at 32814, 32232, 11 |
| Soft Boots | 6529 ×1 | 6209 | Alias at 32812, 32232, 11 |
| Arcane Staff, Avenger or Arbalest | Choice of 3341, 6527 or 5803 ×1 | 6210–6212 | Aliases at x 32804, 32806, 32808, y 32229, z 11 |
| Donor riddle book | 2816 ×1, donor book text corroborated by the spoiler riddle | 6203 | Untagged corpse at 32854, 32325, 11, ordinal 4. No alias in this slice. The capture corroborates the riddle text, but does not identify the donor corpse as the book’s Global pickup. |

The wiki specifies one charge per Sudden Death Rune. The imported reward grants two rune items. The active i3155 profile consumes one item per cast, yielding two casts. Its historical three-charge definition field is not the runtime consumption mode. Ruthless, Fountain of Life, the hub shortcut and the Inquisition access condition have separate owners from the reward chests. The wiki states no quest XP or outfit grant.

## Requirements, route and NPC comparison

| Section | Difference from the captured Global route |
| --- | --- |
| Definition and quest log | The level 80, Premium and quest log flag agree. The imported kind is reward-only despite the quest log flag. The inactive approximation lists Rope and Shovel, omitting Pick, Holy Tible, blood, oil, Key 3700, Destroy Field Rune and the five vocation paths. Its five stages omit the route puzzles, forms and individual throne gates, place the hub unlock after the thrones and choose Avenger in the reward intent. |
| Goshnar and shrine | The donor blood action moves players on the tile south of the grave. The repository has a PoI pick route, plus chosen level-80/Premium approach callbacks. The wiki requires blood at Goshnar, then Premium and a carried Holy Tible at the shrine, then a level-30 expertise door to the east. Crystal maps action 1030 to appearance 5129 at (32795, 32327, 10), found without that action in the committed placement. The donor shrine checks item 2836 without consuming it and registers UIDs 2000 and 2001. The chosen native callback at (32791, 32331, 10) is on a donor destination, so it cannot substitute for those triggers. |
| Dragon Lair | The oil lever, shared bridge reset and Pick-to-cool-lava puzzle lack active PoI callbacks. Crystal supplies a ten-minute oil reset. The spoiler gives no bridge reset duration. |
| Five Ways | No active PoI vocation damage or firewall sequence. Crystal has 30 fire action IDs with six damage tiers per vocation. Global describes 11 fields on the original paths and 13 on the Monk path, mixed damage on wrong paths and distinct final-field Stone Skin Amulet behavior. The five end switches do not require killing their room creatures. |
| Fifteen levers and spa | Missing ordered switch cycle, final switch, 15-minute rock reset, emergency exit and lever shortcut, and party-triggered spa ladder. The later fifteen riddle levers are decoys. |
| Maze | No active PoI teleport maze, pick exit, timed rock lever or ladder reset. The donor teleport table lists AIDs 28810–28834. Those tags were absent from the corresponding project regions. |
| Hub shortcut and Inquisition | Global grants shortcut access on first **entry** to any throne room and Demon Forge access after one spirit absorption. Crystal writes shortcut markers via AIDs 8816–8817 and also grants hub access on throne absorption. The imported definition lists both donor gates as source references. Neither route has matching active PoI progress writers/gates in the audited content. Crystal startup door records are at `data-global/startup/tables/door_quest.lua:665–674`. The floor 10 expertise door bindings are at `data-global/startup/tables/door_level.lua:27–51` and `data-global/startup/tables/door_level.lua:160–169`. |
| Thrones and reward room | Missing spirit credit/repeat ejection, seven energy barriers, level-80 reward approach, trap corridor and Key 3700 door binding. Crystal maps the level-80 action 1080 to appearance 5102 at (32834, 32280, 10), found without that action in the committed placement. Crystal uses UIDs 2080–2086 and 2090–2096 for the thrones and barriers. These tags were absent from the throne regions scanned. The donor Key 3700 door is item 1628 at (32827, 32246, 10), confirmed as a native untagged door without a startup binding at `data-global/startup/tables/door_key.lua:115–117`. The seven named bosses are rare encounters, with throne credit tied to stepping rather than kills. |
| Pumin and Sage | The four bureaucrat dialogues retain basic presentation, with no active form chain. The captured sequence is 356 request → 145 request → 411 request → permission for 287 → 287 → 411 → 145 → 356 → access. The Sage imported dialogue has the help prompt and no branch, and lacks the yes teleport to the Slime Cave. No PoI quest NPC callback/offer records were found. Crystal places the Sage and Demonguard at `data-global/world/world-npc.xml:2864–2871` and the four bureaucrats at lines 3305–3315. The Sage travel branch is `data-global/npc/a_ghostly_sage.lua:53–55`. The spoiler also describes a burn response from Demonguard. |
| Bazir | Missing maze lever branches, post-maze tile teleports, tapestry and mirror transfer, and carpet exit. |
| Achievements | Ruthless is listed as a quest reward. The optional Well of Life heals once and grants Fountain of Life. Neither is granted by an active PoI interaction. |

There were no PoI-specific active or held files in the two world-callback directories. The two chosen entries are in `content/quests/shared-access-callbacks.json`. Missing UID/AID placements and missing runtime owners are separate issues. A donor destination does not establish a Global trigger tile.

## BLOCKER

Donor paths below are at Crystal 201189f8.

- BLOCKER Shrine of Wonders shared STEP admission combining a non-consuming carried item, live Premium and return to origin data-global/scripts/quests/the_pits_of_inferno_quest/movements_holy_tible_tile.lua:14
- BLOCKER Goshnar shared blood-on-target party-tile relocation with PvP stack rules data-global/scripts/actions/other/fluids.lua:157
- BLOCKER Hall of the Five Ways shared per-vocation mixed-combat STEP admission and final-field equipment semantics data-global/scripts/quests/the_pits_of_inferno_quest/movements_fire_tile.lua:47
- BLOCKER optional Well of Life atomic once-only full heal and achievement on use data-global/scripts/quests/the_pits_of_inferno_quest/actions_fountain.lua:3
- BLOCKER rare PoI bosses random periodic world-level spawn selection data-global/scripts/quests/the_pits_of_inferno_quest/globalevents_pits_of_inferno_bosses.lua:12

## OPEN-FACTS

Global capture does not specify numeric source selectors, exact tiles for most effects, the oil reset interval, boss spawn schedule/destinations, the collectible riddle-book source, the exact Ruthless trigger, explicit UH-rune charge state, or any XP or outfit award. Obtaining mapper image geometry or named item numeric IDs from source records is separate from the captured wikitext.

## PARTIAL

Entry item retrieval and treasure placement binding are the implemented slice. The riddle-book placement, level and Premium gate relocation, Goshnar, oil/lava, Five Ways, lever and spa cycles, maze, shortcut, all throne progress/barriers, Pumin NPC dialogue, Sage exit, Key 3700 door, achievements and Inquisition handoff remain pending. No engine mechanics were added.

## Verification

- PG via shard (pgsplit build files applied on `a17ef1e6f631cc8eb9724e79794b631fdcca24ed`). The pre-fix case initialized PostgreSQL and failed at the production binder assertion for the first untagged source placement. The filtered run finished in 23.80 s with 1 failure.
- The test reuses the production reward dispatch and custody fixture with the exact untagged ordinal-1 occurrence for each selected placement. The four committed region digests and source tile ordinals were checked separately.

- PG via shard (pgsplit build files applied on `659d11c506a440d85d1b93f1e3c6bd92ddb247d9`). 1 passed and 0 failed. All 11 placement scenarios ran. Total duration 173.46 s.
- Quest authoring passed all 44 check commands and formatting passed. Pin refresh reproduced the committed World digest.

| PostgreSQL placement case | Duration |
| --- | ---: |
| Frozen Starlight | 17.55 s |
| Platinum Coins | 13.09 s |
| Soft Boots | 13.55 s |
| Arcane Staff | 12.99 s |
| Avenger | 14.26 s |
| Arbalest | 13.72 s |
| Stuffed Dragon | 15.22 s |
| Key 3700 | 14.40 s |
| Holy Tible | 13.44 s |
| Pirate Backpack | 13.89 s |
| Backpack of Holding | 12.60 s |

The same filter without `OTERYN_TEST_POSTGRES_ADMIN_URL` returned `Ok(())` with 1 passed, 0 failed and 0.00 s elapsed. The database evidence is the routed run above. Dispatch Clippy passed with `--all-features --lib -- -D warnings`. World pin-check passed with a temporary output directory, which the CLI requires.
