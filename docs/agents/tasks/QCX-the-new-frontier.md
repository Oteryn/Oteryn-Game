# QCX51 The New Frontier Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Global evidence comes from the 2026-11-17 qfacts capture at `460e1d8c2c0e88c438ae5e54910db00822dc0129`. The quest page is curid 41754, oldid 1058656. The spoiler is curid 41756, oldid 1099208. [Quest page](https://tibia.fandom.com/?curid=41754&oldid=1058656) · [Spoiler](https://tibia.fandom.com/?curid=41756&oldid=1099208)

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` supplies source bindings and item IDs. The [217-row import inventory](../evidence/quests-2-20261102/the_new_frontier.json) classified 212 rows as left out at import time. The [retained dialogue capture](../../../content/dialogues/quests-2/emperor-donor-dialogues.json) contains Ongulf and Chrak's source replies. Current main also ships the M39 Tome dialogue and chest callbacks and the Warmaster wardrobe.

## Comparison against base

| Part | Global and donor evidence | Base behavior and disposition |
| --- | --- | --- |
| Requirements and first passage | The quest page requires premium and says level 77 applies only to the last mission. The spoiler lists six boxes/parcels or Levitate overall, three climb props or Levitate for Mission 01, optional shovel and rope, travel gold and possible Black Bert access. Gurbasch quotes 200 gold in the transcript. Crystal's initial Gurbasch branch is 110 gp in `data-global/npc/gurbasch.lua:73-87`. | Definition retains the annotated level as unparsed and premium as data. The short chosen recipe has no equipment checks. Farmine first travel, stage routing, and the level gate still need integration. Equipment supports terrain traversal and should be treated as route alternatives. |
| 01 New Land | Ongulf's transcript opens with `mission`, offers long or short, and then starts exploration. Crystal uses `project` at `data-global/npc/ongulf.lua:67-111`. The right lever gives access and leaving the mountain credits exploration. | Added the opening, both story choices and the quest start. The native mountain step callbacks at (33082,31532–31533,7) already write mission01 and questline 1→2. The wiki route marker is (33085,31532,7), three tiles east of the donor trigger. Lever and sealed door admission, discovery feedback and Ongulf's report/next-mission handoff still need wiring. |
| 02 From Kazordoon With Love | Melfar supplies bait for three unordered trees. Global lists 4 enraged squirrels and 4 wolves plus a war wolf. Crystal spawns 3 enraged squirrels and 5 wolves plus a war wolf. The thief steals bait and Melfar charges 100 gp for a replacement. | The definition counts three uses. Imported storage transitions do not execute the encounters or the Melfar conversation and turn-in. The spoiler contradicts itself about the numbered thief tree. |
| 03 Strangers in the Night | Use vines to reach Lazaran, then request mission and peace. | Matching vine use callbacks exist at (33021,31536,4/6). Lazaran's conversation and Ongulf's acceptance/report are incomplete. |
| 04 The Mine Is Mine | The left lever opens the Stone Golem route and the Shard of Corruption must die. Crystal advances questline 11→12 for every qualifying damaging player. | Chosen recipe has one generic boss target and the progress import retains the death transition. No native Frontier death objective binds this source credit. Lever access and report remain open. |
| 05 Getting Things Busy | Six supporters including Leeland, six named gifts on failed negotiations, then Depot and Tavern. The spoiler describes uncertain keyword outcomes. | The chosen recipe counts five and omits Leeland. The mission-state import retains partial NPC/storage rows. Negotiation outcomes, gift retries, six-way completion and Farmine staging remain incomplete. |
| 06 Days of Doom | Curos offers a 120-second survival trial with the Mooh'Tah Master, then a report and Farmine Bank access. | The title and scalar state were imported. Curos's active M39 callback is for An Uneasy Alliance. Frontier's arena timer, return, report and Bank gate remain incomplete. |
| 07 Messengers of Peace | Sealed door leads to prison. The nearby west chest gives one Tome of Knowledge. | Mission 07–08 scalar transitions and Tome claim UID 6287 exist. The active M39 position/appearance callback grants one i10217. The sealed door and prison-entry transition are incomplete. |
| 08 An Offer You Can't Refuse | The bed supplies the note. Use the dragon decoration and speak with Ztiss, then leave for the Zao Steppe/Muggy Plains route. Farmine shops open and Cael accepts six Tomes. | The note/prison/escape states are partially represented without a full native route. M39 uses mission08=2 and defers Tome 7 until mission10=2. Farmine shops, Ztiss and the prison-to-steppe chain are incomplete. Existing later Snake Head callbacks are distinct from this prison exit. |
| 09 Mortal Combat | Zurak passage, one Tome near Chrak, two eligible players, 13 bosses with successive waves at 90 seconds, Tirecz reward-room exit, Warmaster wardrobe, then report to Chrak. | Tome UID 6288 has a live M39 position/appearance callback for one i10217. The Tirecz encounter definition models death/egress but does not deliver the paired timed arena. The wardrobe already grants base Warmaster at (33053,31020,7), appearance 5862, after questline 26. Added Chrak's report to advance 27→28 and open Mission 10. |
| 09 Reward room claims | Two gold ingots, one piggy bank and one each of bullseye, mastermind and berserk potions. Crystal `data-global/startup/tables/chest.lua:1324-1343` maps the three chests to UIDs 6097–6099. | Item plans already agree: i9058 ×2, i2995 ×1 and a bag i2854 containing i7439/i7440/i7443 ×1 each. The committed tiles contain ground plus appearance 2472 at ordinal 1 without stored UIDs. Added native aliases for the three claims, preserving once-only identities and nested contents. |
| 10 New Horizons | Ongulf gives 8,000 XP and permission to use the Farmine Magic Carpet. Cael accepts further Tomes. Crystal `data-global/npc/ongulf.lua:235-248` sets 29, mission10=2 and MagicCarpetDoor=1. | The definition already records 8,000 XP as a reward intent. Added the live guarded turn-in and carpet progress flag. The flag still needs the actual carpet door and travel endpoints. The existing Cael post-finale checks use mission10=2. |
| Achievement and extended access | Global describes southern and central Zao access. Crystal awards Champion of Chazorai upon Tirecz's death. | No live Frontier boss-to-achievement binding found. The fetched wiki pages do not establish the achievement grant. This lane excludes the qcx33 Mages Tower gate. |

## NPC placement admission

Ongulf and Chrak had catalogue records but were absent from the native NPC placement scope. The existing placement generator now admits the three Farmine stages for Ongulf at (33023,31542,10), (33024,31539,12) and (33024,31543,14), and Chrak at (33078,31022,2). The promotion candidates and Crystal XML agree. [Crystal NPC positions](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/world/world-npc.xml#L45-L46)

The committed native map carries arena door action 42722 at (33080,31019,2) and reward door action 42723 at (33061,31025,7). The arena admission and egress remain partial.

## Engine gaps

`BLOCKER M02 tree events staged target-item use with variant ambient spawn and stolen-bait recovery data-global/scripts/quests/the_new_frontier/action_beaver.lua:1-44`

`BLOCKER M04 Shard of Corruption stage advance for all damaging players data-global/scripts/quests/the_new_frontier/creaturescripts_shard_of_corruption_kill.lua:3-14`

`BLOCKER M05 negotiations per-character random keyword assignment in NPC transitions data-global/npc/ongulf.lua:181-195`

`BLOCKER M06 Curos trial occupied-arena admission with survival timer, progress and egress data-global/scripts/quests/the_new_frontier/movement_minotaur_boss.lua:1-41`

`BLOCKER M09 Mortal Combat paired arena admission with concurrent scheduled waves and reward egress data-global/scripts/quests/the_new_frontier/action_arena.lua:1-89 and data-global/scripts/quests/the_new_frontier/creaturescripts_tirecz_kill.lua:19-38`

The scalar kill counter, lease-expiry callbacks, and imported Tirecz encounter on main cover narrower behavior than the cited scripts.

## OPEN-FACTS

- Global thief-tree identity (the spoiler's summary and bullets disagree), each NPC's exact negotiation distribution, Champion of Chazorai grant conditions, and the extra Tome hand-in count and XP.
- Exact Global selectors and positions for the Farmine lifts, doors and carpet. The spoiler supplies route descriptions and some map markers. The final-mission level requirement needs confirmation of the exact action that enforces it.
- Global travel prices and discount conditions beyond Gurbasch's quoted 200 gp.

## PARTIAL

- Farmine travel and level/premium admission.
- Mission 01 lever, sealed door and report.
- Missions 02–08 progression, Farmine service gates and access to Zao.
- Mission 09 travel, arena, reward door and achievement.
- Mission 10 carpet door and travel binding.
- Quest-log recipe parity, including the sixth supporter and Missions 06–10.
- The underlying UID-based claims for the two Tome chests lack native aliases. Their positions and appearance agree with the active M39 World callbacks.

## Validation

The sharded PostgreSQL cases use the actual NPC talk owner and the native alias binding followed by wire USE. The finale fixture arranges prior arena victory and wardrobe claim, then drives Chrak and Ongulf. Entry cases exercise both story choices. The carpet assertion checks the durable permission flag.

Baseline PG on `91e10b53790e5498eb25e47c78cccc57b062e8a7` with the updated test fixture and local pgsplit build files executed six cases in 221.11 s. The three untagged chest bindings and both New Land choices reproduced the missing behavior. New Horizons stopped on a fixture progression-context mismatch before Chrak's handoff. The candidate fixture initializes progression through the active content policy.

PG via shard (pgsplit build files applied on `aac98b7b504a44cadc2d9b03aa759709d7e92e56`) passed all 6 cases in 323.57 s.

| Case | Duration |
| --- | ---: |
| Gold ingot chest | 43.67 s |
| Piggy bank chest | 46.30 s |
| Potion chest | 65.50 s |
| Chrak report and New Horizons turn-in | 49.31 s |
| New Land, long dialogue | 64.06 s |
| New Land, short dialogue | 54.61 s |

Without the admin URL, the same 6 tests returned `Ok(())` in 0.01 s. Those calls did not exercise PostgreSQL. Quest authoring passed 44 checks, formatting passed, and the World pin check passed for `oteryn` with digest `61c115a6de9f45cca0b797a025de51b4ce644d8d84be9e1d2da105540039a3a8`.

Dispatch library Clippy passed with all features and `-D warnings`.
