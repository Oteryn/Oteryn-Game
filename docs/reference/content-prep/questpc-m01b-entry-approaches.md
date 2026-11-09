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
