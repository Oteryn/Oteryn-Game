# QCX43 Dangerous Depths Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Global evidence was captured on the qfacts branch at `460e1d8c2c0e88c438ae5e54910db00822dc0129`. [Quest page](https://tibia.fandom.com/?curid=83368&oldid=889707) · [Spoiler](https://tibia.fandom.com/?curid=83369&oldid=1105447)

Donor evidence is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. The donor paths below are relative to `data-global/scripts/quests/dangerous_depth/` unless otherwise specified.

| Area | Global and base comparison |
| --- | --- |
| Requirements | The definition matches premium, level 100 and quest-log visibility. The recommended level is 200. The seven tasks are independent on Global. Base NPC dispatch exposes Ordnance admission and the Measurements hand-in. The rest of the task menu is absent. |
| Non-runtime recipe | The chosen recipe orders the NPCs, tasks and bosses into one sequence, marks it once-only and aggregates three Surprise Jars with the Scourge achievement. Global allows daily independent tasks and has separate points, access windows, three pump achievements and the combined achievement, and three Morgathla reward tiers. The recipe is held for Native bindings. |
| Task rewards | All tasks award one faction point and one Surprise Jar. The bonus awards one more of each. Crystal identifies the jar as 27654 (`data-global/npc/gnomus.lua:94,153,159`). The Measurements hand-in already grants one and records a 72,000-second deadline. Ordnance lacks the hand-in, and both cooldown paths treat positive deadlines as permanent. No quest-specific XP or outfit appears in the snapshot. |
| Gnomes | Ordnance has no live escort. Crystal expects five rescued gnomes and three crawlers (`data-global/npc/gnomus.lua:143-147`). Global describes one gnome at the spawn and up to three crawlers, without stating the task's total escort quota. Measurements needs five distinct lava spots and a trignometre from the Old Machine. Charting needs three of six structures, with the bonus at six. Neither field path nor the chart hand-in is active. Crystal dispenses items 27308 and 27307 in `actions_gnome_items.lua:6,14`. |
| Task hazards | Global specifies 500 fire damage from walkable lava in the Gnome and Scout areas and 1,500 physical damage from gnome-proximity rockfalls. The quest registry has no active hazard callbacks for either. Crystal registers lava item 22675 and rockfall action 57240 (`movements_lava_damage.lua:3-11`, `movements_gnome_avalanche.lua:45`). |
| Dwarves | Home Improvement needs 20 Lost Exiles, with nearby homes destroyed or avoided, and three captured dwarves for bonus. Subterranean Life counts 50 Diremaws and Deepworms across areas with no bonus. Base lacks the complete NPC and field loops. |
| Scouts | Growth needs three of five distinct sponge sites, with the bonus at five. Barrel detonation takes five seconds and can deal 1,000 fire damage. Pesticide needs 20 Diremaw corpses and applies after ten seconds, with a chance to spawn a Diremaw. Base lacks the complete NPC and field loops. |
| Faction entry | Each Suspicious Device trades for one point in the chosen faction. Each faction requires 15 points, or 10 after all three Bigfoot boss defeats. Base has three energy **exit** callbacks and no matching entry callbacks. Crystal uses three access flags for the discount (`movements_energy_entrance.lua:18-24,32-35,42-45`). Its six registered tile coordinates are at lines 1-7. The floor-14 placement shard tiles already contain item 22675 at all six donor coordinates. The floor-14/15 shards have none of the audited UIDs/AIDs listed below, and no Dangerous Depths callback is present in the held directory. |
| Funnel and Warzone access | Each player destroys one funnel per faction. An unused grant persists until first entry, then grants eight hours, permits re-entry and never ejects a player on expiration. Base lacks the funnel and timed-entry callbacks. |
| Geodes and bosses | Mining yields Heavy Crystal Fragments, with size-dependent chances. Global requires 30 team fragments, an eight-second portal, no lever or team cap, and a four-hour boss cooldown. Crystal instead starts ten hours at entry (`movements_boss_entrance.lua:2,36-47`). Each boss drops five fragments per eligible player and fragments have the stated 24-hour lifetime with depot storage. The base loot entries follow the donor counts for item 27713 (Baron 1, Count 1–7, Duke 1). They already map each mallet part to the correct boss. Base lacks the quest callback chain for mining, geodes and boss entry. |
| Encounter behavior | Baron hunger phases and organic matter, Count pipes and slime healing, Duke invulnerability and five Fiery Hearts, and Morgathla beetle waves and crystal vulnerability are separate from the imported creature records. The base quest callbacks supply none of these transitions. Global names the Duke in the Warzone 6 body despite the spoiler’s Baron heading. |
| Post-boss pumps | Global grants Buried the Baron, His Days are Counted and Duked It Out through the respective pumps. All three bosses and pumps grant access to Powerful Featherweight, and the spoiler associates Death in the Depths with all three boss defeats. Base has no active pump callbacks. The dispatch imbuement predicate also accepts the broad Dangerous Depths questline. |
| Morgathla | Global requires the Head, Handle and Pommel from Baron, Count and Duke, respectively. Gnominimus assembles one Strange Mallet for the party. The Gong consumes it and opens a 14-second portal into four encounter stages. The quest definition has an empty claims list. No active quest reward claim surfaced for any of the three reward rooms. No Gnominimus script or Gong callback appears in the Crystal snapshot. |
| Morgathla rewards | Chest grants Scourge of Scarabs. Left bag gives 5 Platinum Coins, 1 supply package, 1 Rusted Armor and 1 Piggy Bank. Center (three players) gives 50 Platinum Coins, 2 packages, 1 Slightly Rusted Armor and 1 Piggy Bank. Right (one player) gives 2 Crystal Coins, 4 packages, 1 Slightly Rusted Armor, 1 Slightly Rusted Legs and 1 Piggy Bank. |

## Donor item and map bindings

- Crystal `data/items/items.xml:52160-52279,52770-52778` maps Gnomish Pesticides to 27498, Heavy Crystal Fragment to 27509 (a second same-name item, 27713, appears at line 52997), Strange Mallet to 27523, Head/Handle/Pommel to 27524/27525/27526, Suspicious Device to 27653 and Surprise Jar to 27654. The geode action takes 27509 at `actions_using_crystals.lua:530`, while the boss loot uses 27713. Crystal device dialogue instead consumes ID 30888 (`data-global/npc/gnomus.lua:256-290`), which has no exact item entry in that revision’s XML.
- Crystal `data/items/items.xml:6618,6835,6867,23686,23691,23706,33637` maps Piggy Bank to 2995, Platinum Coin to 3035, Crystal Coin to 3043, Rusted Armor to 8895, Slightly Rusted Armor to 8896, Slightly Rusted Legs to 8899 and Gnomish Supply Package to 15698. Two items share the Bag name (235 and 2853 at lines 394 and 5951). The reward chest source must resolve its exact bag binding.
- Crystal registers chest and tool UIDs 57234–57236, Warzone and task AIDs 57230, 57231, 57233, 57234, 57240–57243, 57300–57303 and crystal AIDs 57350–57367. Decoding `content/world/placements/region-z14-*.b3` and `region-z15-*.b3` with the repository placement codec found zero objects carrying those attributes. Crystal creates several dynamically, including the rubble (`creaturescripts_lost_exile_task.lua:69-76`) and temporary boss portal (`actions_using_crystals.lua:437-447`).

## Blockers

```
BLOCKER Ordnance and Home Improvement bonus escort/flee/despawn quest credit movements_gnome_ordnance.lua:6; movements_gnomes_ordnance_end.lua:1; movements_lost_exiles.lua:1
BLOCKER Home Improvement and Subterranean Life per-task mixed-monster kill-credit wiring creaturescripts_lost_exile_task.lua:1-76
BLOCKER Growth and Pesticide attributed delayed barrel blast and corpse-age validation actions_lever_barrel.lua:98-145; actions_gnomish_pesticide.lua:6-38
BLOCKER Faction entry compound alternative guard for faction points and three boss defeats movements_energy_entrance.lua:18-49
BLOCKER Suspicious Device exchange dialogue-owned numeric payment and faction selection data-global/npc/gnomus.lua:241-295
BLOCKER Warzone funnel access timed per-player admission with faction point debit actions_crude_lava_pump.lua:15-61; movements_warzone_entrance.lua:8-54
BLOCKER Warzone boss entry and pumps team portal occupancy, boss cooldown and eligible kill credit actions_using_crystals.lua:442-518; movements_boss_entrance.lua:36-47; creaturescripts_bosses_mission_depths.lua:15-26
BLOCKER Powerful Featherweight quest-specific completion signal for all three bosses and pumps actions_crude_lava_pump_achievements.lua:34-58
```

## OPEN-FACTS

Global snapshot does not specify numeric item/achievement/placement IDs, exact gnome rescue quota, escort timeout, exact mining and mallet drop probabilities, the collectible fragment’s numeric variant, reward-room player selection, or the cooldown start rule when a team leaves without killing the boss. The spoiler gives conflicting point debit timing at funnel destruction and window expiration. No explicit quest XP or outfit reward is listed.

## PARTIAL

Faction entry and device hand-ins, task field objectives, the repeat Measurements offer and remaining NPC paths, boss access and pumps, encounter credit, Powerful Featherweight, and Morgathla assembly, encounter and claims remain open.

## Ordnance cooldown correction

The Gnomus generator now compares the stored deadline with the dispatch transaction clock. The active cooldown remains in effect while the deadline is in the future. A deadline reached or passed enables the existing repeat offer. The 20-hour interval comes from the Global spoiler. [Daily tasks](https://tibia.fandom.com/?curid=83369&oldid=1105447#Daily_Tasks)

The PostgreSQL fixture seeds the two states that normally follow escort completion. The real Gnomus dispatch must refuse the future deadline, offer the repeat mission at the expired deadline and accept confirmation. On the audit commit `146089d500fb55d786b74810643d72cebf8e908a` with the regression case present locally, PG via shard failed after 80.92 s at the expired-deadline assertion. It returned the cooldown reply.

PG via shard (pgsplit build files applied on `36b7df326128d94ad0c6a458417140b5a9ae646e`) passed the one Ordnance case in 100.86 s. The same filter without the PostgreSQL URL returned `Ok(())` in 0.00 s.

Quest authoring passed all 44 checks, including 269 schema cases. The Gnomus generator passed its three focused tests. World pin refresh reproduced the existing digest and pin-check passed for one world. Formatting passed.

Dispatch library Clippy with `-D warnings` passed.
