# QCX35 Cults of Tibia Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`, quest `oteryn:quest.cults_of_tibia_quest`.

## Reference evidence

The requested MediaWiki API returned HTTP 402, `{"message":"Please contact the site owner for access."}`. The audit used the Global MediaWiki wikitext captured on 2026-11-17 and supplied in `qcx-facts/cults-of-tibia.md`, the Portuguese API revision as a cross-check, and the Crystal pin `201189f871d23597b82caf88fd611454a2ddefb8` for implementation details. [Global quest](https://tibia.fandom.com/index.php?curid=81510&oldid=1098561) · [Global spoiler](https://tibia.fandom.com/index.php?curid=81513&oldid=1162736) · [Portuguese cross-check](https://www.tibiawiki.com.br/index.php?curid=40964&oldid=423919)

## Requirements, access and rewards

| Area | Reference and main comparison |
| --- | --- |
| Requirements | Global lists level 8, recommended 150 and premium. The definition uses level 8 and premium. Portuguese lists recommended 200+. The Global spoiler lists Feyrist access via Threatened Dreams, Key 3520 for Humans, rope or equivalent, Disintegrate Runes for Life, travel money and 20,000 gp. Prosperity spends 10,000 gp each at Gareth and Iwar. Most cult missions support direct starts at the hideout. Gerimor starts the seven independent assignments using mission, exact cult name and live confirmation. Feyrist is a travel prerequisite in the sources. Crystal Gerimor has no separate Feyrist-storage check. The base has the NPC definition and omits Gerimor from the world NPC placement shard. The retained placement candidate matches the pinned Crystal world NPC spawn at 33450,32246,7 (`data-global/world/world-npc.xml:1993-1995`). |
| Assignment settlements | The Gerimor rules offer and start all seven assignments. They omit the later mission reports and XP. Crystal records 25,000 each for Minotaurs, Orcs and Humans and 50,000 each for Prosperity, Barkless, Life and Misguided. Those sum to 275,000 before the 50,000 finale, consistent with Global’s 325,000. Crystal `data-global/npc/gerimor.lua:256-340,392-405`. |
| Finale access | Gerimor requires all seven reported stages before setting FinalBoss.Mission and FinalBoss.AccessDoor to 1. The runtime offer currently covers assignment starts only. The donor-derived door bindings include the final access track, but the NPC does not write it. Crystal `data-global/npc/gerimor.lua:356-379`. |
| Finale reward | After FinalBoss.Mission reaches 2, Gerimor offers the reward and asks for two free slots. The confirmed reward is 50,000 XP, Corruption Contained, one Mystery Box (26186) and one base-vocation crown. Druid 26187, knight 26188, paladin 26189, sorcerer 26190, monk 50313. Native Gerimor has no final offer or claim. Crystal `data-global/npc/gerimor.lua:380-386,424-447`. No outfit award appears in the compared finale sources. |
| Existing claim | The quest definition links a separate once-per-character chest claim for five Petrified Screams (10420), chest appearance 2472, UID 6313 at 32739,31426,8. The Fandom structured finale lists the box and vocation crowns as separate rewards. The chest gives no finale credit. Native B3 has the item at that cell without a raw UID. The existing qualified native claim resolves the placement independently (the retained `qlane-meta/questpc7/restart4-story-reward-pg-4.qualified.json` records successful source dispatch). |

## Steps, callbacks and placements

The callback packet has 18 runtime definitions (13 Barkless symbols, Life oasis and four teleports) and the separate Essence lever supplies one additional callback. The held directory contains no Cults packet. The prior donor evidence has 127 source rows. The table below records the remaining progression differences by route. The donor evidence breaks them down by exact selector, source line and reachability in [the Cults disposition](../evidence/quests-4-20261102/cults_of_tibia.json).

| Route | Difference |
| --- | --- |
| Barkless | Native code handles ten distinct symbols and does not increment on reuse. Source-action placement qualification is incomplete. Tigo initiation, the tar/sulphur/ice sequence, the final object and boss-access write remain outstanding. Crystal startup assigns 5535 to eleven symbols and 5536 to the two final-object slots. The current callback catalogue contains thirteen 5535 candidates, including the two 5536 cells, 32740,31494,9 and 32741,31494,9. Both current B3 slots lack the AID (`data-global/startup/tables/item.lua:1351-1375`). Leiden’s transformations and later boss/settlement route remain outstanding. |
| Minotaurs | Jamesfrancis’s assignment, 50 eligible kills, subsequent doors, boss credit and report are outside the mounted start rules. |
| Prosperity | Gareth patronage, ransom and investigation, the letter, Iwar and Angelo handoffs, magnifier crate and painting, and the Denominator puzzle (five consecutive correct answers) are outside the mounted start rules. Global specifies no boss fight for this route. |
| Life | The oasis callback preserves the donor’s exact AID 5560 and Mission 2→3 guard. Pinned startup assigns the AID along x=33479, y=32264–32290, z=10 across the listed sand and stone types. The corresponding B3 tiles contain those objects without the action attribute (`data-global/startup/tables/tile.lua:269-304` and `item.lua:1392-1395`). Angelo’s cave access and handoffs, analysis, counteragent, conditional river route and Sandking sequence are outstanding. |
| Misguided | Noozer’s access, equipping and charging the amulet, five exorcisms, barriers, shadow revelation and boss settlement remain outstanding. Global requires ten kills per charge, a five-minute charged use window and damage-based exorcism credit with a shared-experience party exception. |
| Orcs | Both teleport destinations and the two appearance-10842 selectors agree with Crystal. Orc, Orc Warlord and Orc Shaman disguise gates retain access after the first crossing in Global. Those gates, the Machine and Armored/Unarmored Voidborn phases and boss settlement are outstanding. Both teleports omitted the donor landing cue. |
| Humans | Carlin exit agrees geographically with Crystal. The timed torch, soul absorption (ten green and ten blue, 20 seconds to collect each) and associated barriers remain outstanding. Main has the Essence lever callback, phase owner and a Humans quest credit binding. The pinned lever at 33095,31942,15 is present in B3 as 8911, without AID 5500 (`data-global/startup/tables/lever.lua:27-31`). Carlin exit and the Essence exit omitted the donor landing cue. |
| Final | Global confirms up to five players per cult boss and up to ten for the finale, with 20-hour re-entry for both. The seven-report transition, access door write, final encounter population and credit, and reward delivery remain outstanding. Global uses Guilt from Dark Souls to open and sustain the Corruptor of Souls damage window, then spawns the Source of Corruption. |

The four teleport records now use the existing single magic-effect field for effect 11 at their exact landing coordinates. The source cue is `CONST_ME_TELEPORT`. The Cults packet generated the runtime registry, and the repository refresh function regenerated the callback proof. [Carlin donor lines 3–14](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/cults_of_tibia/movements_movement-cults-of-carlin-teleport.lua#L3-L14) · [Essence donor lines 3–14](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/cults_of_tibia/movements_teleport_from_essence_of_malice.lua#L3-L14) · [Orc donor lines 9–21](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/cults_of_tibia/movements_orc_teleport.lua#L9-L21)

The NPC placement generator now admits Gerimor’s retained source placement at 33450,32246,7. The PostgreSQL case uses the generated placement index, installs that source contact and drives the live Gerimor assignment dialogue and durable write. [Crystal NPC spawn](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/world/world-npc.xml#L1993-L1995)

## Validation

- Baseline PostgreSQL via the local shard, with the base callback and placement records restored for the run. Both tests failed as expected (74.59 s total). The teleports reached their correct landings without a cue (Carlin 42.29 s, Essence 12.78 s, Orc entry 8.42 s, Orc exit 8.78 s). Gerimor failed at the missing placement after initializing the database (1.06 s).
- PG via shard (pgsplit build files applied on `000a051a51b7e0b65d2669d7d5b1cba46edb7d9e`), 2 passed, 0 failed, 77.45 s total. Carlin exit 19.09 s, Essence exit 10.31 s, Orc entry 10.48 s, Orc exit 11.29 s, Gerimor 25.31 s. The run without the PG URL returned success for both route guards in 0.00 s.
- World pin refresh and pin-check passed. The compiled World contains 104 NPC definitions with placements, up from 103 on the base. The NPC placement generator replay also passed.
- Dispatch library Clippy passed. The quest-authoring suite stopped at the base quest-tree drift in three definition shards and their index. The Cults callback generator check passed independently.

## Blockers and remaining work

The entries below identify missing Cults-specific runtime or map ownership. Main has some shared mechanics and the Essence exception, so these entries apply to the named paths.

```text
BLOCKER Minotaurs quest kill counting data-global/scripts/quests/cults_of_tibia/creaturescripts_minotaur_task_count.lua:1
BLOCKER Barkless timed trials data-global/scripts/quests/cults_of_tibia/movements_tar.lua:2
BLOCKER Barkless purification loss-free simulated death data-global/scripts/quests/cults_of_tibia/movements_ice_death.lua:1
BLOCKER Barkless Leiden damage conversion and Wine Cask spawn sequence data-global/scripts/quests/cults_of_tibia/creaturescripts_splash.lua:3
BLOCKER Barkless final object AID 5536 native placement and ordered spatial feedback data-global/scripts/quests/cults_of_tibia/actions_last_object.lua:18
BLOCKER Life oasis native AID 5560 placement qualification data-global/scripts/quests/cults_of_tibia/movements_check_oasis.lua:3
BLOCKER Life river conditional cooldown route data-global/scripts/quests/cults_of_tibia/movements_river_teleport.lua:9
BLOCKER Prosperity per-player digit puzzle and timeout data-global/scripts/quests/cults_of_tibia/actions_lever_mota.lua:8
BLOCKER Prosperity Denominator arithmetic dialogue data-global/npc/denominator.lua:64
BLOCKER Misguided equipped amulet charge and kill events data-global/scripts/quests/cults_of_tibia/creaturescripts_glowing_rubbish_amulet.lua:3
BLOCKER Misguided Hermit geyser and Freed Soul phase ownership data-global/scripts/quests/cults_of_tibia/movements_geyser.lua:1
BLOCKER Humans timed torch passage data-global/scripts/quests/cults_of_tibia/actions_torch.lua:7
BLOCKER Humans vortex spawning and item attributes data-global/scripts/quests/cults_of_tibia/creaturescripts_carlin_vortex_spawn.lua:1
BLOCKER Orcs outfit-gated thresholds data-global/scripts/quests/cults_of_tibia/movements_looktype.lua:12
BLOCKER Essence and other cult bosses per-character 20-hour callback cooldown data-global/scripts/quests/cults_of_tibia/movements_boss_timer.lua:13
BLOCKER Gerimor XP reports and final access/reward persisted NPC programme predecessor registration data-global/npc/gerimor.lua:356-447
BLOCKER Finale Guilt vulnerability effects data-global/scripts/quests/cults_of_tibia/actions_guilt.lua:3
BLOCKER Finale boss phase spawning and credit data-global/scripts/quests/cults_of_tibia/creaturescripts_bosses_mission_cults.lua:9
```

**OPEN-FACTS** Enforced minimum party size, exact live map bindings for missing source actions, and the timing of the Destroyed Pillar visual relative to its guardian’s death. Global establishes the maximum party sizes and cooldown.

**PARTIAL** Gerimor’s seven XP reports, final entry and claim, upstream quest NPC and item-use steps, startup action binding regeneration, and remaining Cults encounter owners. The generated Gerimor programme is checked against the quest-specific generator and a source custody pin. The current authored programme covers assignment starts. Updating the generated rules changes the NPC programme marker in the Cults definition hash. The shared predecessor registration currently supports A Father’s Burden and Marlin Trophy only. Characters with the previous Gerimor programme hash would otherwise fail current quest writes with RevisionMismatch. A compatible predecessor registration requires a shared engine change.
