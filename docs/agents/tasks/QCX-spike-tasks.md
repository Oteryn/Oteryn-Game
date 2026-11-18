# qcx69 Spike Tasks Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Quest `oteryn:quest.spike_task`.

Global evidence is the fetched TibiaWiki wikitext from 2026-11-17, classified as TIBIAWIKI_STRUCTURED. [Quest, curid 65390 oldid 885773](https://tibia.fandom.com/?curid=65390&oldid=885773) and [Spoiler, curid 65391 oldid 1038961](https://tibia.fandom.com/?curid=65391&oldid=1038961).

Donor evidence is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, classified as OTS_HYPOTHESIS_ONLY. The checked-in Batch 4 packet preserves the three mission NPCs and the source disposition. I checked the donor portal and geomantic scripts and the Gnommander and Gnomux NPC files at that revision. [Crystal quest directory](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/the_spike_tasks)

## Differences

| Part | Global | Repository at base and disposition |
| --- | --- | --- |
| Quest definition | Premium, four independent 20-hour tasks in each of three brackets. Upper 25–49, middle 50–79, lower 80+. | Requirements are unset in the donor record. The disabled chosen recipe is a non-Premium, level-80, one-time campaign through all tiers with a synthetic completion access reward. Global has no such campaign or reward. The display name is “Spike Task.” |
| Kazordoon entry | Northern teleporter near Xelvar, with zero Teleport Crystals. | No AID4226 runtime callback or action on the retained “To: The Spike” object at (32623,31853,-11). The mapper point is (32623,31857,-11), near the sign rather than on its object. Crystal checks Premium and uses separate success and failure landings. See blocker below. |
| Return to Kazordoon | The Upper instructions direct the player through the large crystal teleporter. | The imported AID4227 callback has the Crystal return landing (32624,31855,-11), but the retained object 14973 at (32242,32597,-8), beside the “To: Kazordoon” sign in `region-z08-x125-y127.b3`, carries no action. Bound the generated callback to that position and appearance. No new item, cost, storage or level predicate. |
| Other division portals | The three brackets enter their respective divisions. | AID4228–4243 are present as callbacks, with no qualified map bindings in the Batch 4 evidence. Crystal records `level = 80` on outbound entries but its callback never reads that field. Exact per-portal Global requirements and anchors remain open. |
| Upper pacifiers | Gnomilly, seven fork uses, two-minute reuse per pacifier. | Seven uses appear in the recipe. No admitted use target or timed reuse callback. |
| Upper angry spirits | Dig until one Enraged Soul spawns, then kill it. Two-minute reuse per mound. Failures can yield nothing, a Ghost, Zombie or Demon Skeleton. | The chosen recipe calls for four mounds and the donor NPC checks progress 4. No admitted randomized mound or soul objective. |
| Upper tracking and kills | Ghost Detector gives diagonal directions and finds a potentially player-specific spot. Seven Demon Skeletons. | Chosen recipe describes tracking and counts seven kills. Neither has a live Spike objective binding. |
| Middle nests | Mutated Bat disguise, eight nests, about one to two minutes to repair. | The chosen recipe says three nests. Crystal checks eight and uses a two-minute disguise. AID56421 and nest use have no admitted binding. |
| Middle mushrooms | Four distinct mushrooms. Uniqueness is per character. | Chosen recipe already counts four. The imported action is evidence only. The world and per-character distinct-use binding is absent. |
| Middle geomantic charges and kills | Use the Lodestone on charged monoliths. First and second uses change the item, third finishes. Monoliths recharge in about one to two minutes and may teleport the player. Kill seven Crystalcrushers. | Chosen recipe counts three uses and seven kills but has no live binding. Crystal's separate crystal STEP may set charge state 1 to 2, and can combine this with the disguise. The facts do not resolve how that STEP relates to the three Global uses. |
| Lower parcels and agents | Receive four Gnomish Supplies, visit four of five outposts. Collect three of nine undercover reports through NPC greetings. | The chosen counts match. No admitted Spike grant, consume, distinct outpost or greeting callback. Crystal's Gnomargery gives item 19219 ×4 (`data-global/npc/gnomargery.lua:169`). |
| Lower temperature and kills | The hotspot is shared across characters and changes at server save. The thermometer melts on success. Kill seven Drillworms, including outside the Spike. | Chosen recipe describes the hotspot and counts seven kills. No admitted hotspot, consume or Spike Drillworm binding. Crystal gives item 19206 ×1 (`data-global/npc/gnomargery.lua:232`). |
| NPC offers and repeat turn-ins | Gnomilly, Gnombold and Gnomargery give the bracket assignments and reports. Gnomux replaces lost tools. Each completed task gives one fame. Upper pays 1000 XP, middle has 2000 XP explicitly listed on three tasks, lower pays 3500 XP. | Three mission NPCs have saved donor dialogue but no live Spike offers or callbacks. Crystal pays 2000 XP for geomantic charges as well (`data-global/npc/gnombold.lua:91-100`). No Spike reward claim exists. The separate repeatable cooldown and XP primitives exist, but require mission bindings. |
| Outfit, addons and achievement | Gnommander exposes fame in the quest log. 100 fame for the full outfit, with gold costs 1000/2000/3000 for base/first/second. Cave Completionist achievement. | No Spike fame, claim or achievement binding. Crystal subtracts 100 fame for **each** purchase (`data-global/npc/gnommander.lua:139-167`), which conflicts with Global's 100 total. The separate Cave Explorer Outfits quest has a level-80, 20-kill trial with those same gold prices. Its records belong to another quest. |

Checked the Quest definition, world callbacks and held directory, reward claims, NPC callbacks/offers and NPC service records, placement regions and Batch 4 source disposition. No Spike record was present in `world-callbacks-held`.

## Blockers

- BLOCKER Kazordoon entry stateless live Premium STEP with source refusal landing `data-global/scripts/quests/the_spike_tasks/movements_spike_teleport.lua:35`
- BLOCKER Upper angry spirits randomized mound spawn with a shared per-object reuse timer and soul objective binding `data-global/npc/gnomilly.lua:101`
- BLOCKER Middle crystal passage combined timed disguise and charge STEP branch `data-global/scripts/quests/the_spike_tasks/movements_geomantic_charges.lua:13`
- BLOCKER Upper and Middle NPC admission maximum-level predicate in the shared quest NPC callback owner `data-global/npc/gnomilly.lua:152` and `data-global/npc/gnombold.lua:150`

## OPEN-FACTS

- Global exact item and outfit IDs, NPC tool grant quantities except the four parcels, replacement tool prices, achievement identifier and exact award trigger. Donor IDs and prices are candidate mappings only.
- Individual Global fame thresholds or allocation across purchases and whether fame is spent. The total of 100 and gold prices are explicit. Exact Global geomantic reward XP is unstated.
- Exact Global identities, per-portal level checks, action and unique IDs, landing effects and positions beyond the return mapping corroborated by the retained sign and Crystal route. Precise nest and monolith reset time within the stated range, charge teleport odds and destination pool, mound outcomes' odds, detector selection rules and reliable server-save hotspot coordinates.
- Donor Gnomux candidates are 19214, 19205, 19219, 19207, 19203, 19206, 19204 (`data-global/npc/gnomux.lua:26-33`). The wiki names the equipment and does not establish the numeric IDs or prices.

## PARTIAL

Premium entry, division portal bindings, task offers and acceptance, world and kill objectives, replacement shop, repeat turn-ins and rewards, fame journal, outfit/addons and achievement. Quest definition and disabled chosen recipe differences remain listed above.
