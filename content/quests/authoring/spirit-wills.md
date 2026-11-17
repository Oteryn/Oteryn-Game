# Selected Spirit journey

Scope: `oteryn:quest.the_spirit_will_get_you_quest`, the selected Oteryn recipe in
`content/quests/definitions/quests-00200-00299.json`, rather than a claim of
historical source closure. Classification: `CHOSEN_OTERYN_RECIPE_ONLY` /
`OTERYN_AUTHORED_APPROXIMATION`.

A premium character accepts the Crone's request, discovers three separate faded
wills, and returns while retaining all three. The confession completes this
selected story and opens its conversation. It does not grant a Wayfarer outfit
or modify the independent Wayfarer addon service. An absent will refuses the
confession without consuming any other will or advancing progress.

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

Conservative placement assumption: the available source has no11544 map entry or
Spirit chest registration. The historical public walkthrough URLs returned
HTTP403/402. This selected recipe assigns one will to each of two existing box
piles and an adjacent chest in the same native world region as the Crone. The choice is an
Oteryn rule, not evidence that those objects historically held wills:

| Position (legacy floor) | Appearance | Native source stack |
| --- | --- | --- |
|33079,31158,8|2473 box|11002,2473,2473,2473|
|33083,31158,8|2469 box|11000,2469,2469|
|33083,31159,8|2472 chest|10999,2472|

All have house0, depth0, no UID/action/child attributes. Source:
`content/world/placements/region-z08-x129-y121.b3`, SHA256
`824f5682e85b1ac5e4ff9a22836a12b5cee6f3c07d285118f1091d54229348f4`.
No map bytes are changed. Any matching box in one pile shares its personal once
flag; another box in the same pile cannot mint another will. Each successful
source USE atomically commits its leaf MINT and discovery flag. A full inventory,
stale authority, or repeated source cannot advance that discovery.

NPC callbacks and world callbacks share the same owning quest tracks. The final
confession requires all three discovery flags and retained custody of at least
three11544 papers. The discovery flags prevent traded papers alone from skipping
the collection route. Premium is checked again for each fresh grant and final
conversation. Historical replay returns its stored outcome without minting or
refilling anything.

After completion, the `confession` conversation remains available from the
Crone through a read-only stage2 offer. It does not repeat the completion or
create a new source, item, outfit or Quest receipt. The bounded PG fixture
mounts the source NPC identity beside the native collection-room crop; the
production placement remains the separately pinned Crone placement above.
This qualifies quest bindings and custody, not a traversal of every intervening
world tile.
