# Siflind: paint custody at the mission boundary

Checked source:
https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/siflind.lua

At Questline 7, the mission conversation selects topic 3 (lines 65–67).
The first confirmation selects topic 4 (137–139). The second confirmation
adds one Item 7253, sets Questline 8 and Mission04 1, then returns to topic 0
(140–145). The registered callback keeps that final `yes`, topic and phase
boundary; it does not skip earlier missions or complete the whole quest.

The native profile uses the pinned imported XML and appearances through the
existing source importer: a nonstackable, movable, pickupable 1.80 oz vial,
legal in CharacterInventory or Ground. No scripted use effect is inferred.
The existing World callbacks continue to own target identity, distinct-seal
progress, temporary appearance and conditional final consumption.

The focused PG case starts from the explicit prior mission snapshot. It mints
and equips its backpack through the real Item owners, grants the real native
vial together with both progress writes, replays without another vial, then
uses three registered seal callbacks. The original vial stays carried for
two seals and is burned with removed custody on the third. This is not an E2E
qualification of prior missions, dialogue navigation or client map clicks.
