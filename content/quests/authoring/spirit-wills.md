# Selected Spirit journey

Scope: `oteryn:quest.the_spirit_will_get_you_quest`, the selected Oteryn recipe in
`content/quests/definitions/quests-00200-00299.json`, rather than a claim of
historical source closure. Classification: `CHOSEN_OTERYN_RECIPE_ONLY` /
`OTERYN_AUTHORED_APPROXIMATION`.

A premium character accepts the Crone's request, discovers three separate faded
wills, and returns while retaining all three. The confession completes the quest
and opens both Wayfarer addon hand-ins. The cape and hat each require their own
items, and Wrath of the Emperor awards the base outfit. An absent will prevents
confession and leaves custody and progress unchanged.

The admitted Crone placement is `(33192,31044,6)` in the native NPC placement
shard. Pinned Crystal source:
https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/cranky_lizard_crone.lua
SHA256 `8ac41cc1e41785f8b3bf0d2d4b965673e73428511b5290fc327a2b2ea1ddaa3f`.
Pinned Canary counterpart:
https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/npc/cranky_lizard_crone.lua
SHA256 `fc53fabc5f0e29370c8de5bb929182daffd95bcb99ee5743e7bf3944b665a8b8`.
Neither donor supplies this Spirit branch. The conversation text and stage order
are selected Oteryn behavior, not reconstructed donor dialogue.

Item11544 is captured with the existing native profile importer from
`imports/canary/items-xml/items.xml:28483-28487` at Canary47dfd51f. The exact XML
SHA256 is `1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2`.
The paper weighs200 native units; its description says the writing has vanished.
No readable text, charges, timer, subtype or binding is inferred.

TibiaWiki's Spirit spoiler (2022-10-19) gives three mapper coordinates in
the Souleater Mountains. The B3 surfaces at those coordinates retain the
Crystal scenery. The World overlay restores only the quest furniture.

| Position (legacy floor) | Source object | Appearance |
| --- | --- | --- |
|33177,31021,5|Brown quest box|2473|
|33161,31056,4|Quest box|2469|
|33166,31056,6|Quest chest|2472|

Each successful source USE atomically commits its leaf MINT and discovery flag.
A full inventory, stale authority or repeated source leaves discovery unchanged.
The approach scenery at the second and third pickups uses Crystal's fixed,
walkable placement traits for Items 8703 and 10538.

NPC callbacks and world callbacks share the same owning quest tracks. The final
confession requires all three discovery flags and retained custody of at least
three11544 papers. The discovery flags prevent traded papers alone from skipping
the collection route. Premium is checked again for each fresh grant and final
conversation. Historical replay returns its stored outcome without minting or
refilling anything.

After completion, the Crone offers a read-only stage 2 `confession` conversation.
The PostgreSQL fixture exercises the three pickup locations and both Crone offers
as separate bounded map crops.

TibiaWiki lists 50 experience per character level. The fixed-amount NPC reward
field cannot represent that formula.
