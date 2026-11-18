# qcx59 Blood Brothers Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Quest `oteryn:quest.blood_brothers_quest`.

Global evidence is the supplied TibiaWiki quest snapshot (curid 30623, oldid 843374) and spoiler snapshot (curid 30703, oldid 1132612), both fetched 2026-11-17. Crystal 201189f871d23597b82caf88fd611454a2ddefb8 supplies technical IDs and code comparisons. [Quest](https://tibia.fandom.com/?curid=30623&oldid=843374) · [Spoiler](https://tibia.fandom.com/?curid=30703&oldid=1132612) · [Crystal quest directory](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/blood_brothers_quest)

## Audit against origin/main

| Part | Global | Oteryn and Crystal differences |
| --- | --- | --- |
| Requirements | Premium, level 40 for the route, 70+ recommended. Mission 5 passes a level-30 gate and one of two level-40 gates. The Inquisition Vampire Hunt skips Mission 1. Mission 9 requires a female participant, who may already have completed the quest. | Definition already records 40 and Premium. Its inactive seven-stage recipe abbreviates ten missions and replaces the female-participant requirement with a personal ritual. It does not encode the Inquisition branch. Exact enforcement at Julius is unspecified in the supplied facts. |
| 1. Gaining Trust | Worn Garlic Necklace, flour + holy water + garlic, oven to bake Garlic Bread, eat in front of Julius. | Julius's active QN3 necklace proof correctly requires the worn item (3083). The staged dialogue and state rows do not activate the initial offer, the Inquisition skip, the ingredient and oven chain, or the eat-in-front-of-Julius proof. Crystal's Julius uses a topic-based check and response after seeing the bread (141–166, 309–320). |
| 2. Bad Eggs | Holy water, garlic, flour and Baking Tray yield 12 cookies. Give one to each of Serafin, Lisander, Ortheus, Maris and Armenius, then report. | No live Blood Brothers recipe or five-cookie dispatch chain located. The inactive recipe combines this stage with Mission 1. Crystal's NPCs use one cookie (8199), individual suspicion tracks and Julius's confirmation. |
| 3. His True Face | Only Armenius is necessary. Spell can eject to sewers, leave the player in the tavern, or rarely reach his basement, where the achievement and encounter occur. Two squares east of the entrance provide another chance. | No live encounter or tile outcome. Crystal's Armenius stages other replies at the same NPC and uses three equally likely branches explicitly labelled as a guess (71–104). Global establishes rarity and the additional tile opportunity. |
| 4. The Dark Lands | Harlow takes the player from Yalahar to Vengoth. Map any four sites plus castle entrance. Eight marks earn up to 7,500 XP and count toward Castlemania. | Only the Vengoth-side return route is present in the travel family. No live outbound offer, map marking, report XP or achievement condition. Crystal marks once per location with item 8200. The primary report checks only the castle flag (Julius 347–363) and pays no XP. Its separate map keyword pays 7,500 at a counter of at least seven (551–569). Global requires five with the castle and graduated XP for extras. |
| 5. Into the Castle | Quest chest, soul charges the crystal for 24 hours, recharge after expiry, four eligible players with four charged crystals stand on four carvings. | Global chest map tile contains item 2472 without donor unique 58104. No live chest claim, Soul exchange, recharge or four-carving callback. Crystal dispenses 8453, trades it for 8225 and awards Ghostwhisperer (Soul 83–93), then waives the item check for attuned helpers. The existing repeat castle movement correctly requires the exact hidden-entrance track and preserves it. |
| 6. A Black History | Blue carpet, trapdoor, southeastern library, main diary, six optional pages at 1,000 XP each. | No live Blood Brothers bindings for the secret route, diary or pages. Crystal gives the manuscript (28483) and six distinct pages (641), then grants 1,000 on each ordered turn-in (Julius 603–619). The quest infobox separately lists five 1,000 XP rewards. |
| 7. Boreth | Three Fire Bug uses on the twines, room with Boreth and two Plaguethrowers, stake corpse and report dust. | No live Blood Brothers ritual, encounter or Julius hand-in. Crystal's Boreth death action clears Plaguethrowers and Julius consumes one dust (8717). |
| 8. Lersatio | Break two mirrors, room with Lersatio and two Vampires, stake corpse and report dust. | No live Blood Brothers mirror/encounter, timer or turn-in. Crystal restores each mirror after 30 seconds and spawns once the counter reaches two. Julius consumes one dust (8718). |
| 9. Marziel | Female participant steps in front of statue, moves away, uses vial of blood on the tile, and kills two brides. Fight Marziel and one Vampire, stake corpse and report dust. | No live Blood Brothers trigger, encounter or hand-in. Crystal's step-in checks sex, vial 2874 subtype 5, transforms the statue and spawns brides. It consumes the vial during step-in and its mission-complete guard excludes returning female helpers. Julius consumes one dust (8719). |
| 10. Arthei | A Fading Memory gives the tear, use it twice, 10 minutes to defeat Arthei and two Vampires, stake and report the dust for a Vampiric Crest. A sealed courtyard door leads to the optional Blood Goblet. | No live Blood Brothers tear, encounter, courtyard unlock or reward claim. Crystal's tear script consumes the tear and starts the encounter on its first valid use, with no deadline in that action. Julius exchanges one dust (8720) for one crest (9041). Crystal gives one Blood Goblet (8531) from pile UID 14052, setting a personal once-only marker. The committed global-target map has the pile (appearance 4285) at (32953, 31473, 4), one square north of the wiki marker, without a legacy unique. |
| Outfit and achievements | The crest is used for the Yalaharian addon. His True Face and Castlemania require distinct actions. Castlemania also includes 100 Vampire Lord Tokens from Vampire Hunter. | The crest-based Palimuth addon offer and its side-specific awards are already imported under In Service of Yalahar. The Blood Brothers acquisition path and quest achievement prerequisites remain missing. |

The baseline Quest definition has an empty claim list and a waiting native lowering. The corresponding mission catalogue has imported track and transition evidence. The full staged dialogue at `content/dialogues/quest-batches/quests-5/blood_brothers_quest.json` declares itself outside the runtime. The live Blood Brothers NPC rule is `content/dialogues/quest-rules/quest-batch3-qn3-blood_brothers.json`. No matching row was found in `content/quests/world-callbacks-held`, `content/interactions/reward_claims`, or the split quest NPC callback/offer directories. The castle entry in `content/quests/world-callbacks.json` is already live and donor-equivalent. The source crosswalk and B3 region distinguish the adjacent pile from the marker square. [Donor UID and turn-in code](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/blood_brothers_quest/actions_blood_goblet.lua#L3-L19) · [Retained UID placement](../../../tools/content-schema/quest-authoring/samples/chests/empty_containers.json#L1728-L1743)

## Blockers

`BLOCKER His True Face random NPC outcome with encounter spawn and separate step trigger data-global/npc/armenius.lua:87`

`BLOCKER Into the Castle atomic four-player step rite with live per-player charged-item eligibility data-global/scripts/quests/blood_brothers_quest/movements_unlock_entrace.lua:52`

`BLOCKER Boreth scoped on-death encounter-add cleanup data-global/scripts/quests/blood_brothers_quest/boreth_death.lua:3`

`BLOCKER Lersatio temporary per-mirror repair coupled to an area-wide boss start data-global/scripts/quests/blood_brothers_quest/actions_lersatio_mirrors.lua:38`

`BLOCKER Marziel sex and exact fluid custody folded into a step-triggered encounter start data-global/scripts/quests/blood_brothers_quest/movements_marziel_blood.lua:14`

`BLOCKER Arthei two-use statue ritual with area fan-out and a ten-minute encounter limit data-global/scripts/quests/blood_brothers_quest/actions_ghost_tear.lua:16`

## OPEN-FACTS

- Exact Armenius odds, alternate unlock method and exact east-tile selectors.
- Reconcile the 13,200 XP infobox breakdown with the spoiler's six optional 1,000 XP pages and the 7,500 XP all-eight-map total. Confirm the individual map-mark payouts and whether the initial diary turn-in pays separately.
- Global Harlow tariff and route confirmation topics, exact level/Premium enforcement outside the Research Centre gates, and whether the donor-only Ghostwhisperer grant belongs to the target.
- Exact trigger mapping for the Research Centre sealed door, the completed-helper recharge handoff and arena resets/cooldowns beyond the durations the spoiler gives.

## PARTIAL

Mission 1 start/skip and baking proof, Missions 2–4, crystal chest and charging, castle unlock, diary and page turn-ins, all four boss routes and dust turn-ins, Vampiric Crest delivery, courtyard gate, and achievement qualification. The portable reward slice can be exercised independently of its blocked approach.

## Courtyard reward correction

The source callback now binds the existing bare pile at (32953, 31473, 4), appearance 4285, ordinal 1. It grants one Blood Goblet (8531) and advances the existing personal `blood_goblet` track from 0 to 1 through the same durable transaction. The quest is still a partial route. The PG case starts inside the courtyard and exercises the reward binding, native inventory delivery and repeated USE. The wiki's marker square contains debris, so the case starts on adjacent pavement.

Baseline PG with the corrected fixture on `9517a72b6a7abe3882948da4086275fe23437666` returned `NothingToUse` on the literal USE where a reward commit was expected. One case failed in 19.25 s. The earlier fixture setup refusal was `StartNotEnterable` on the debris square.
