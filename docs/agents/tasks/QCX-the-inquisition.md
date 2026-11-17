# QCX38 · The Inquisition Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`. Quest `oteryn:quest.the_inquisition_quest`.

## Evidence

The direct Fandom MediaWiki API returned HTTP 402. The supplied qfacts MediaWiki wikitext is the Global authority for this audit. The repository's older structured capture corroborates the quest metadata. The Brazilian TibiaWiki API served the full walkthrough inline on the quest page. Its `/Spoiler` title was missing. Donor citations use Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, with source paths relative to that revision. [Global quest](https://tibia.fandom.com/?curid=25383&oldid=1211625) · [Global spoiler](https://tibia.fandom.com/?curid=34108&oldid=1092599) · [Brazilian walkthrough](https://www.tibiawiki.com.br/index.php?curid=3048&oldid=445826) · [Crystal quest scripts](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_inquisition_quest)

## Parity audit

| Part | Global and Crystal evidence | Main and QCX38 disposition |
| --- | --- | --- |
| Requirements | Free accounts can start and do the first three missions. Level 100 applies to the last mission only. 150+ is recommended. Demon Forge also requires Ungreez and at least one Pits of Inferno throne. Crystal checks questline ≥20, level ≥100 and any of seven throne values at (33192,31691,14). | Definition records premium and a generic minimum of 100 with an unparsed scope note. The Premium flag has no stage scope. Henricus's existing recruitment and guard callbacks do not enforce a level floor. The Forge entry has a blue portal (1949) in the B3 map and lacks the quest-specific conjunction with any-throne admission in the callback registries. |
| 1. Interrogation | Join Henricus, pick up the written field guide, interrogate Tim, Kulag, Grof, Miles and Walter in any order, report to Henricus. Crystal guide UID 6500 yields one book 2821. | Join, first mission, guard completion and report have NPC records. The guide is an authored variant held for written reward lowering, with no UID on the committed (32316,32260,8) chest. |
| 2. Eclipse | Henricus gives one flask 133. Uzon's Eclipse route leads to the cauldron. Use the flask, take one Witches' Grimoire 7874 and report with it. | Canonical Grimoire claim and chest geometry agree with Crystal. The chest at (32649,31932,1) has ground 4565 and chest 2472 at ordinal 1, with no UID. QCX38 binds donor UID 6270's original once claim to that native chest using the existing alias registry. The four B3 cauldron parts are appearances 2008–2011 at (32657–32658,31927–31928,1). They lack Crystal's startup AID 2000. Stage start, carpet route, cauldron use and report remain partial. The alias preserves the imported claim semantics, including its existing prerequisites. [Donor source](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2615-L2621) |
| 3. Vampire Hunt | Storkus accepts 20 Vampire Dust (5905) in installments. Three optional Garlic Necklaces (3083) yield the Weakened Count from the coffin (AID 2002), otherwise The Count. Return his ring to Storkus and report to Henricus. Donor dust progress uses 1 as the counter baseline and 21 as its terminal value. | Definition retains donor source progress. No complete active conversation, conditional coffin summon and ring hand-in was found. Storkus's repeatable vampire token rank XP belongs to a separate activity. |
| 4. The Haunted Ruin | One flask 133 from Henricus, Liberty Bay exorcism, optional Pirate Ghost kill, temporary door closure and report. | Source progress is retained. Active start, use and return need qualification. |
| 5. Essential Gathering | Henricus takes 20 Demonic Essences (6499). Demon Hunter base outfit is available afterward. | No active full hand-in was found. Outfit grants belong to the outfit lane. |
| 6. The Demon Ungreez | Enter the sealed room, kill Ungreez with most damage, report to Henricus. First addon follows via outfit dialogue. Crystal uses AID 1004 (5113/5114) and advances questline 18→19 for the most-damage player. | No active quest-specific door or most-damage credit was found. Crystal startup binds 1004 to the door at (33146,31639,11). Main's B3 door matches appearance 5113 and has no AID. Global describes a population-dependent 5–15 minute respawn and room access ending after report. Main supports qualified top-damage credit through the closed source-death registry. Ungreez is absent from that registry, and enabling it requires a shared Rust registration outside QCX38 scope. |
| 7. The Shadow Nexus | Reach the Forge, defeat Ushuriel, Zugurosh, Madareth, Latrivan and Golgordan, Annihilon, and Hellgorak. The brothers share a clear. Global gives two minutes to take each cleared exit, compared with Crystal's ten minutes. Timely entry unlocks the corresponding hub portal permanently. Hellgorak requires convinced Dreadbeasts. Purify the Nexus with holy water through its timed stages, then report. | Nineteen UID-only transport callbacks exist. The pinned donor has 31 entries (2150–2180), leaving 2150/2151/2153/2157/2159/2162/2164/2168/2170/2174/2176/2178 without active counterparts. A scan of the Forge placement regions did not resolve retained donor UIDs. Crystal's startup table identifies a portal of appearance 1949 at each of the 31 coordinates, and the committed B3 map has all 31 objects without those UIDs. The quest-specific ordered boss exits, shortcut unlocks, brother levers, Nexus cycle and final report remain partial. [Portal startup table](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/item.lua#L4033-L4157) |
| Final chest | Global names ten choices and allows one item. Crystal UID→item, count one each: 1300→8062, 1301→8090, 1302→8053, 1303→8060, 1304→8023, 1305→8096, 1306→8100, 1307→8102, 1308→8026, 1309→50261. Choices are Emerald Sword, Hellforged Axe, Obsidian Truncheon, Warsinger Bow, Royal Crossbow, Spellbook of Dark Mysteries, Master Archer's Armor, Fireborn Giant Armor, Robe of the Underworld and Merudri Nanbando. | Donor choice is retained as source interaction evidence. No active canonical final ten-way once claim was found. Crystal startup binds all ten chests to (Y32245,Z9), at X32330/32318/32328/32326/32314/32322/32324/32320/32316/32332 in UID order. Each committed tile has one untagged chest 2472 at ordinal 1. [Chest startup table](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L232-L281) The reward-room door at (32320,32258,9) has appearance 5104 and action 41699. The donor's choice warning writes a distinct room marker at three tiles. |
| Final access and achievements | High Inquisitor on report, Master of the Nexus on chest claim, Demonbane on both addons. Demon Forge access, the reward room and post-quest Inquisition blessing service are separate rewards. Global gives all five blessings together at a 10% surcharge. Global describes base after mission 5, first addon after Ungreez, final addon after reporting Shadow Nexus. Crystal also grants both addons at the chest. | No active end-to-end quest-specific report, choice and access chain was found. Main checked questline 25, the Crystal chest-claim state. QCX38 changes the purchase eligibility and supplemental refusal to unlock at the report and preserve eligibility in both subsequent states. Crystal assigns 23 on report and 24 on final outfit dialogue. Global unlocks the service at report. The donor addon grant at the chest differs from the Global timing. Outfit/addon records are outside QCX38. |

## BLOCKER

- BLOCKER Demon Forge timed boss exits shared boss death latch with Global's two-minute reset and a two-boss clear `data-global/scripts/quests/the_inquisition_quest/creaturescripts_inquisition_boss.lua:19-29`
- BLOCKER Vampire Hunt coffin conditional world-monster summon with three placed offerings `data-global/scripts/quests/the_inquisition_quest/actions_vampire_hunt.lua:10-26`
- BLOCKER Ungreez shared source-death registration for most-damage-only quest credit `data-global/scripts/quests/the_inquisition_quest/creaturescripts_inquisition_ungreez.lua:2-10`
- BLOCKER Demon Forge entrance any-of-seven Pits throne storage predicate composed with questline and level `data-global/scripts/quests/the_inquisition_quest/movements_entrance.lua:1-32`

- BLOCKER Haunted Ruin Pirate Ghost world summon composed with flask consumption and timed door reset `data-global/scripts/quests/others/actions_holy_water.lua:65-80`
- BLOCKER Shadow Nexus shared decaying target state and bounded completion budget `data-global/scripts/quests/others/actions_holy_water.lua:105-135`

## OPEN-FACTS

- Runtime qualification of all ten final reward item identities and acquisition of the exact Global quest XP, if any. The retained Global summary and pinned final-chest action list no fixed quest XP.
- Exact Global durations and completion capacity for the Shadow Nexus cycle. The walkthrough gives approximate values.

## PARTIAL

Guide execution, Eclipse start/use/report and travel, Vampire Hunt, Haunted Ruin, gathering, Ungreez, Forge entrance and progression, Shadow Nexus, end report, final choice and achievements remain open. Outfit and addon authoring is owned by the outfit lane.

## Verification

The Grimoire regression ran red through live USE dispatch with the original alias registry (36.04 s). Both commands returned `NothingToUse`. The fixture uses the exact committed chest position, appearance, ordinal and absence of UID in a cropped world.

PG via shard (pgsplit build files applied on `c8b9f843adcc0fa56c07d7487b33ec87fd5757af`) with only the blessing gate temporarily restored to its prior behavior returned RC 101. Grimoire passed in 17.04 s with its placement alias present. The blessing case failed in 22.83 s because the old gate rejected the report state before checking funds.

The run with the Rust eligibility change alone returned RC 101. Grimoire passed in 15.14 s. The blessing case reached the funded purchase, where the original migration 0315 receipt trigger still required state 25 (25.49 s). Migration 2897 updates only that quest-specific predicate to 23–25.

The quest authoring runner completed 44 check commands successfully. Pin refresh retained the original world digest `0676b1568f55a55fbe994f344a8e0c60debf766751d12849f56c19f60c93aae5`.

### Cases for the Server session

- Demon Forge entrance at level 100 after the Ungreez report, with any one Pits throne.
- Demon Forge boss exits, including the shared Latrivan and Golgordan clear, the two-minute exit window, retreat on expiry, and permanent shortcut unlock.
- Ungreez most-damage credit and the room's post-report access.
- Vampire Hunt coffin selection of The Count or The Weakened Count from the three necklace offerings.
- Haunted Ruin holy-water use, Pirate Ghost summon, and timed door reopening.
- Shadow Nexus shared wall decay and per-cycle completion capacity.

The local cases use two independent boundaries: a chest USE against the native placement alias and a blessing purchase given the durable final-report state. The blessing fixture seeds that state at the same boundary where the existing purchase tests seed the post-claim state.
