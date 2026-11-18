# QCX43 Dangerous Depths Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Global evidence is the [quest page](https://tibia.fandom.com/?curid=83368&oldid=889707) and [spoiler](https://tibia.fandom.com/?curid=83369&oldid=1105447), captured on the qfacts branch at `460e1d8c2c0e88c438ae5e54910db00822dc0129`. Donor evidence is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. The donor paths below are relative to `data-global/scripts/quests/dangerous_depth/` unless otherwise specified.

| Area | Global and base comparison |
| --- | --- |
| Requirements | The definition matches premium, level 100 and quest-log visibility. The recommended level is 200. The seven tasks are independent on Global. Base NPC dispatch exposes Ordnance admission and the Measurements hand-in rather than the complete task menu. |
| Task rewards | All tasks award one faction point and one Surprise Jar. The bonus awards one more of each. Crystal identifies the jar as 27654 (`data-global/npc/gnomus.lua:94,153,159`). The Measurements hand-in already grants one and records a 72,000-second deadline. Ordnance lacks the hand-in, and both cooldown paths treat positive deadlines as permanent. No quest-specific XP or outfit appears in the snapshot. |
| Gnomes | Ordnance has no live escort. Crystal expects five rescued gnomes and three crawlers (`data-global/npc/gnomus.lua:143-147`). Global describes one gnome at the spawn and up to three crawlers, without stating the task's total escort quota. Measurements needs five distinct lava spots and a trignometre from the Old Machine. Charting needs three of six structures, with the bonus at six. Neither field path nor the chart hand-in is active. Crystal dispenses items 27308 and 27307 in `actions_gnome_items.lua:6,14`. |
| Dwarves | Home Improvement needs 20 Lost Exiles, with nearby homes destroyed or avoided, and three captured dwarves for bonus. Subterranean Life counts 50 Diremaws and Deepworms across areas with no bonus. Base lacks the complete NPC and field loops. |
| Scouts | Growth needs three of five distinct sponge sites, with the bonus at five. Barrel detonation takes five seconds and can deal 1,000 fire damage. Pesticide needs 20 Diremaw corpses and applies after ten seconds, with a chance to spawn a Diremaw. Base lacks the complete NPC and field loops. |
| Faction entry | Each Suspicious Device trades for one point in the chosen faction. Each faction requires 15 points, or 10 after all three Bigfoot boss defeats. Base has three energy **exit** callbacks and no matching entry callbacks. Crystal uses three access flags for the discount (`movements_energy_entrance.lua:18-24,32-35,42-45`). Its six registered tile coordinates are at lines 1-7. No matching Dangerous Depths record surfaced in the authored placement or held callback JSON. |
| Funnel and Warzone access | Each player destroys one funnel per faction. An unused grant persists until first entry, then grants eight hours, permits re-entry and never ejects a player on expiration. Base lacks the funnel and timed-entry callbacks. |
| Geodes and bosses | Mining yields Heavy Crystal Fragments, with size-dependent chances. Global requires 30 team fragments, an eight-second portal, no lever or team cap, and a four-hour boss cooldown. Each boss drops five fragments per eligible player and fragments have the stated 24-hour lifetime with depot storage. Base lacks the quest callback chain for mining, geodes and boss entry. |
| Post-boss pumps | Global grants Buried the Baron, His Days are Counted and Duked It Out through the respective pumps. All three bosses and pumps grant access to Powerful Featherweight, and the spoiler associates Death in the Depths with all three boss defeats. Base has no active pump callbacks. The dispatch imbuement predicate also accepts the broad Dangerous Depths questline. |
| Morgathla | Global requires the Head, Handle and Pommel from Baron, Count and Duke, respectively. Gnominimus assembles one Strange Mallet for the party. The Gong consumes it and opens a 14-second portal into four encounter stages. The quest definition has an empty claims list. No active quest reward claim surfaced for any of the three reward rooms. |
| Morgathla rewards | Chest grants Scourge of Scarabs. Left bag gives 5 Platinum Coins, 1 supply package, 1 Rusted Armor and 1 Piggy Bank. Center (three players) gives 50 Platinum Coins, 2 packages, 1 Slightly Rusted Armor and 1 Piggy Bank. Right (one player) gives 2 Crystal Coins, 4 packages, 1 Slightly Rusted Armor, 1 Slightly Rusted Legs and 1 Piggy Bank. |

## Blockers

```
BLOCKER Ordnance and Home Improvement bonus escort/flee/despawn quest credit movements_gnome_ordnance.lua:6; movements_gnomes_ordnance_end.lua:1; movements_lost_exiles.lua:1
BLOCKER Home Improvement and Subterranean Life per-task mixed-monster kill-credit wiring creaturescripts_lost_exile_task.lua:1-76
BLOCKER Growth and Pesticide attributed delayed barrel blast and corpse-age validation actions_lever_barrel.lua:98-145; actions_gnomish_pesticide.lua:6-38
BLOCKER Warzone funnel access timed per-player admission with faction point debit actions_crude_lava_pump.lua:15-61; movements_warzone_entrance.lua:8-54
BLOCKER Warzone boss entry and pumps team portal occupancy, boss cooldown and eligible kill credit actions_using_crystals.lua:442-518; movements_boss_entrance.lua:32-45; creaturescripts_bosses_mission_depths.lua:1
```

## OPEN-FACTS

Global snapshot does not specify numeric item/achievement/placement IDs, exact gnome rescue quota, escort timeout, mining probabilities, reward-room player selection, or the cooldown start rule when a team leaves without killing the boss. The spoiler gives conflicting point debit timing at funnel destruction and window expiration. No explicit quest XP or outfit reward is listed.

## PARTIAL

Faction entry and device hand-ins, task field objectives and remaining NPC paths, boss access and pumps, encounter credit, Powerful Featherweight, and Morgathla assembly, encounter and claims remain open.
