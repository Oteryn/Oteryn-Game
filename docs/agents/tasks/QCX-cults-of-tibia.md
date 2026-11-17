# QCX35 Cults of Tibia Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`, quest `oteryn:quest.cults_of_tibia_quest`.

## Reference evidence

The requested MediaWiki API returned HTTP 402, `{"message":"Please contact the site owner for access."}`. The audit used the retained Global structured facts in `tools/content-schema/quest-authoring/wiki_quest_facts.json`, the Portuguese API revision as a cross-check, and the Crystal pin `201189f871d23597b82caf88fd611454a2ddefb8` for implementation details. [Global quest](https://tibia.fandom.com/index.php?curid=81510&oldid=1098561) · [Global spoiler](https://tibia.fandom.com/index.php?curid=81513&oldid=1162736) · [Portuguese cross-check](https://www.tibiawiki.com.br/index.php?curid=40964&oldid=423919)

## Requirements, access and rewards

| Area | Reference and main comparison |
| --- | --- |
| Requirements | Retained Global lists level 8, recommended 150 and premium. The definition uses level 8 and premium. Portuguese lists recommended 200+. Global spoiler lists Feyrist access via Threatened Dreams, Key 3520, a rope alternative and a Disintegrate Rune. Gerimor starts the seven independent assignments using mission, exact cult name and live confirmation. Feyrist is a travel prerequisite in the sources. Crystal Gerimor has no separate Feyrist-storage check. |
| Assignment settlements | The Gerimor rules offer and start all seven assignments. They omit the later mission reports and XP. Crystal records 25,000 each for Minotaurs, Orcs and Humans and 50,000 each for Prosperity, Barkless, Life and Misguided. Those sum to 275,000 before the 50,000 finale, consistent with Global’s 325,000. Crystal `data-global/npc/gerimor.lua:256-340,392-405`. |
| Finale access | Gerimor requires all seven reported stages before setting FinalBoss.Mission and FinalBoss.AccessDoor to 1. The runtime offer currently covers assignment starts only. The donor-derived door bindings include the final access track, but the NPC does not write it. Crystal `data-global/npc/gerimor.lua:356-379`. |
| Finale reward | After FinalBoss.Mission reaches 2, Gerimor offers the reward and asks for two free slots. The confirmed reward is 50,000 XP, Corruption Contained, one Mystery Box (26186) and one base-vocation crown. Druid 26187, knight 26188, paladin 26189, sorcerer 26190, monk 50313. Native Gerimor has no final offer or claim. Crystal `data-global/npc/gerimor.lua:380-386,424-447`. No outfit award appears in the compared finale sources. |
| Existing claim | The quest definition links a separate once-per-character chest claim for five Petrified Screams (10420), chest appearance 2472, UID 6313 at 32739,31426,8. The Fandom structured finale lists the box and vocation crowns as separate rewards. The chest gives no finale credit. |

## Steps, callbacks and placements

The callback packet has 18 runtime definitions (13 Barkless symbols, Life oasis and four teleports) and the separate Essence lever supplies one additional callback. The held directory contains no Cults packet. The prior donor evidence has 127 source rows. The table below records the remaining progression differences by route. The donor evidence breaks them down by exact selector, source line and reachability in [the Cults disposition](../evidence/quests-4-20261102/cults_of_tibia.json).

| Route | Difference |
| --- | --- |
| Barkless | Native code handles ten distinct symbols and does not increment on reuse. Source-action placement qualification is incomplete. Tigo initiation, the tar/sulphur/ice sequence, the final object and boss-access write remain outstanding. AID 5536 belongs to the final object. Leiden’s transformations and later boss/settlement route remain outstanding. |
| Minotaurs | Jamesfrancis’s assignment, 50 eligible kills, subsequent doors, boss credit and report are outside the mounted start rules. |
| Prosperity | Gareth patronage, ransom and investigation, the letter, Iwar and Angelo handoffs, magnifier crate and painting, digit puzzle, Denominator chat and boss settlement are outside the mounted start rules. |
| Life | The oasis callback preserves the donor’s exact AID 5560 and Mission 2→3 guard. Its production placement remains unqualified. Angelo’s cave access and handoffs, analysis, counteragent, conditional river route and Sandking sequence are outstanding. |
| Misguided | Noozer’s access, equipping and charging the amulet, exorcisms, barriers, shadow revelation and boss settlement remain outstanding. |
| Orcs | Both teleport destinations and the two appearance-10842 selectors agree with Crystal. Outfit gates, idol mechanics and boss settlement are outstanding. Both teleports omitted the donor landing cue. |
| Humans | Carlin exit agrees geographically with Crystal. Vortex/soul absorption and associated puzzles remain outstanding. Main has the Essence lever, phase owner and a Humans quest credit binding. Carlin exit and the Essence exit omitted the donor landing cue. |
| Final | The seven-report transition, access door write, final encounter population and credit, and reward delivery remain outstanding. |

The four teleport records now use the existing single magic-effect field for effect 11 at their exact landing coordinates. The source cue is `CONST_ME_TELEPORT`. The Cults packet generated the runtime registry, and the repository refresh function regenerated the callback proof. [Carlin donor lines 3–14](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/cults_of_tibia/movements_movement-cults-of-carlin-teleport.lua#L3-L14) · [Essence donor lines 3–14](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/cults_of_tibia/movements_teleport_from_essence_of_malice.lua#L3-L14) · [Orc donor lines 9–21](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/cults_of_tibia/movements_orc_teleport.lua#L9-L21)

## Blockers and remaining work

The entries below identify missing Cults-specific runtime or map ownership. Main has some shared mechanics and the Essence exception, so these entries apply to the named paths.

```text
BLOCKER Minotaurs quest kill counting data-global/scripts/quests/cults_of_tibia/creaturescripts_minotaur_task_count.lua:1
BLOCKER Barkless timed trials data-global/scripts/quests/cults_of_tibia/movements_tar.lua:2
BLOCKER Barkless final object AID 5536 native placement and ordered spatial feedback data-global/scripts/quests/cults_of_tibia/actions_last_object.lua:18
BLOCKER Life oasis native AID 5560 placement qualification data-global/scripts/quests/cults_of_tibia/movements_check_oasis.lua:3
BLOCKER Life river conditional cooldown route data-global/scripts/quests/cults_of_tibia/movements_river_teleport.lua:9
BLOCKER Prosperity Denominator arithmetic dialogue data-global/npc/denominator.lua:64
BLOCKER Misguided equipped amulet charge and kill events data-global/scripts/quests/cults_of_tibia/creaturescripts_glowing_rubbish_amulet.lua:3
BLOCKER Humans vortex spawning and item attributes data-global/scripts/quests/cults_of_tibia/creaturescripts_carlin_vortex_spawn.lua:1
BLOCKER Orcs outfit-gated thresholds data-global/scripts/quests/cults_of_tibia/movements_looktype.lua:12
BLOCKER Finale boss phase spawning and credit data-global/scripts/quests/cults_of_tibia/creaturescripts_bosses_mission_cults.lua:9
```

**OPEN-FACTS** Fresh Global revision bodies, live finale party minimum and cooldown, and exact live map bindings for missing source actions. The Portuguese party information and Crystal's dialogue about a group twice as large do not by themselves establish an enforced minimum.

**PARTIAL** Gerimor’s seven XP reports, final entry and claim, upstream quest NPC and item-use steps, placement qualification, and remaining Cults encounter owners. The generated Gerimor programme is checked against the quest-specific generator and a source custody pin. The current authored programme covers assignment starts.
