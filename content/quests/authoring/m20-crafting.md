# M20 donor scope

Base: `b7da07f2b0d38836b60018a2ecd20c4149fb7ff9`.
Donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`
(`OTS_HYPOTHESIS_ONLY`, selected by the task owner).

The base already contains the first Cursed Crystal carried recipe in
`content/quests/carried-recipes.json`, the final recipe's two operand orders in
`content/quests/npc-callbacks/m20-cursed-crystal-recipes.json`, and the existing
captured-ingredient craft receipt extension in migration 2030. These definitions
and the receipt are retained. No second crafting system or migration is added.

Molten Wax uses the existing carried quest-tool receipt and exact self-target
proof. Item 9106 consumes one unit, raises Oneeyedjoe to 2 only from values below
2, and requires the user's own current entity inside x=31938..31946,
y=32932..32940, floor 10. This is the strict distance-under-five predicate around
(31942,32936,10) in donor
`data-global/scripts/quests/the_cursed_crystal/actions_MedusaOil.lua:29-43`.

## Donor blocker

The complete donor Git tree is untruncated (9,718 entries). Its extracted Lua
sources contain no Nutrient Solution, Bottled Yolk, Fresh Hydra Herbs,
Anti-Mutagen, conservation-jar, fish-flake blender or goldfish-feeding action for
the two requested quests. The only Cradle quest script is
`data-global/scripts/quests/cradle_of_monsters/the_monster_fight.lua`; it defines
the boss encounter (`:1-39`), not the laboratory recipes. Scatterbrained's sole
quest-specific declaration is the empty
`TheScatterbrainedSorcerer = {}` in `data-global/lib/core/storages.lua:1587`.
Fish flakes are item 9109 (`data/items/items.xml:24458`), but item 9109 has no
reference in any donor Lua file; the missing blend/feed logic is not merely
hidden behind numeric item IDs.

Consequently, exact donor parity cannot establish the missing recipes' item
identities, quantities, station selectors or collection consequences. Cradle
s8/s9, Scatterbrained s3/s4, multi-output recipe qualification and their container
stage consequences remain blocked until a pinned implementation source is
provided or another behavior source is authorized. No substitute data is authored.

## Native Item admission blocker

The final Cursed Crystal mix's native route also remains closed. Its source
crystal dust (21507) and output Medusa's Ointment (21506) have no native Item
profiles in `content/items/definitions/spell-native-profiles.json` or its Item
profile supplements. Only the first mixture (21505) is admitted there. The
attempted real final-mix caller reaches `Unavailable(InvalidStoredState)` when
the existing reward MINT owner resolves the unadmitted output. No admission is
fabricated and no Item guard is weakened. Item profiles and their registration
are outside this lane's owned paths; the temporary caller opening was removed.

The source's Molten Wax message and yellow-smoke effect (`actions_MedusaOil.lua:40-41`)
are not emitted by the existing carried callback adapter. This delivery proves
the item debit, self identity, area and source storage consequence, not full
visual feedback parity or a complete quest route.
