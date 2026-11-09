# M01b chosen entry approaches

These are explicit Oteryn visit points, not claims that a wiki Area marker is
an enterable tile or that reaching an entry completes its quest.

| Quest | Pinned marker | Chosen visit | Native start |
| --- | --- | --- | --- |
| Fire Axe Quest | Edron Dragon Lair: 33098,31701,-7 | 33098,31703,-7; ground4408 | west33097,31703,-7; ground4406+border1103 |
| Dragon Tower Quest | Shadowthorn: 33075,32170,-7 | north33075,32169,-7; ground408 | west33074,32169,-7; ground408 |
| Berserk Potion Quest | Helheim: 32478,31179,-7 | east32479,31179,-7; ground6593 | east32480,31179,-7; ground6590 |

The source marker positions and exact page/revision bindings remain unchanged
in `content/world/areas/hunting-places/hunting-places-00000-00444.json`.
Each selected point and start was decoded from the retained B3 region. The
Fire Axe offset is explicitly two tiles; the other two offsets are one tile.
The resolver pins the original position, floor and declared distance.

The original Fire Axe attempt failed with `StartNotEnterable` after
55.015954915s. Its start contained Item1855. A WorldObject declaration of
nonblocking decoration does not prove that this Item's native solidity is
known. The runtime's conservative collision rule remains unchanged.

Public source checked:
https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data/items/items.xml
contains `1854..1855` named `stones`, `1856` named `stone`, and `4457..4502`
named `mountain`. Those names do not establish native nonblocking flags.
The original Edron Area source is:
https://tibia.fandom.com/index.php?curid=19176&oldid=1114456

These callbacks grant visit counters only. Premium/level requirements remain
in force. No Item, XP, teleport or whole-quest completion is granted. Native
PostgreSQL qualification is recorded separately in the lane status ledger.

## Ferumbras arrival

Source: https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/ferumbras_ascension/movements_entrance.lua

The pinned entrance script rejects players below level 150. Its action 24837
sends an uncompleted player to (33275, 32390, 8), while the completed branch
uses floor 9. The chosen first-cycle EXPLORE approach is the clear eastern
neighbour (33276, 32390, 8), appearance 410. The retained B3 map has ground
409 at both the script destination and the next eastern starting tile
(33277, 32390, 8). This binds a visit on the initial entrance floor; it does
not replace the portal or grant seven-boss completion/access to later floors.
The canonical recipe's Current Premium requirement and level 150 are retained.

## Heart of Destruction Zao breach

Source: https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/heart_of_destruction/movements_teleport.lua

Action 48069 is explicitly labelled "from farmine" and lands at
(32117,31355,13); action 48068 goes back to Farmine. The chosen EXPLORE
approach is its clear eastern neighbour (32118,31355,13), appearance23054.
The retained map has ground23113 at the source landing and ground23049 at
the next eastern starting tile (32119,31355,13). The chosen recipe's level150
and Current Premium gates stay in place. Visiting this landing does not
replace the portal, grant boss access or manufacture any later-stage credit.

## Grave Danger chosen twelve-visit stage

Conservative chosen rule: visit seven sanctification-grave approaches and five
tomb antechambers once each. Visiting grants neither sanctification nor boss
credit. The five antechamber anchors use the source boss queue positions before
lever activation. The source movement script checks boss cooldowns, with no
chosen-stage prerequisite; this binding does not replace its portals or timers.

Source movement: https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/grave_danger_quest/movements_enter_tps.lua

| Visit | Source position (native floor) | Chosen entry (native floor) | Appearance |
|---|---|---|---|
| dark-cathedral | 32644, 32394, 8 | 32645, 32394, 8 | 410 |
| femor-hills | 32542, 31846, 6 | 32542, 31845, 6 | 11417 |
| ankrahmun | 33376, 32806, 8 | 33376, 32805, 8 | 418 |
| vengoth | 32959, 31534, 7 | 32960, 31534, 7 | 7062 |
| orclands | 32776, 31817, 8 | 32777, 31817, 8 | 4409 |
| ice-islands | 32012, 31558, 7 | 32012, 31557, 7 | 436 |
| kilmaresh | 33813, 31624, 9 | 33813, 31628, 9 | 469 |
| duke-krule | 33455, 31493, 13 | 33456, 31493, 13 | 429 |
| count-vlarkorth | 33455, 31413, 13 | 33456, 31413, 13 | 429 |
| earl-osam | 33516, 31444, 13 | 33517, 31444, 13 | 429 |
| lord-azaram | 33422, 31493, 13 | 33423, 31493, 13 | 429 |
| baeloc-nictros | 33424, 31413, 13 | 33425, 31413, 13 | 429 |

Sources checked (pinned Canary):

- https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/grave_danger_quest/actions_grave_sanctify.lua
- https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/grave_danger_quest/actions_duke_krule.lua
- https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/grave_danger_quest/actions_count_vlarkorth_.lua
- https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/grave_danger_quest/actions_earl_osam.lua
- https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/grave_danger_quest/actions_lord_azaram.lua
- https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/grave_danger_quest/actions_baeloc_nictros.lua

All entry and eastern starting tiles were checked against the retained B3
placements. Kilmaresh's immediate grave ring contains retained tomb/pillar
objects: use its clear southern outer approach four tiles away, with ground469.
This explicit bounded exception changes no source placements or collision rules.
All other chosen approaches are adjacent on the original source floor.
Canonical level250 and Current Premium requirements remain mandatory.
The later Cobra Bastion EXPLORE stage is separate and remains unwired here.
