# Reliable Ram native map target

The retained region `content/world/placements/region-z07-x124-y124.b3`
contains tile 31926/31896/7 with appearances 4515, 5485 and 53439,
all at depth zero. Ram appearance 53439 retains source ordinal 2.
The official client 15.30 artifact admitted by
`imports/official/client-assets/15.30/manifest.json` contains appearance
53439 with exactly `unmove` and `unpass` flags and no automap color.
The producer verifies the complete artifact digest and admitted appearance
membership; its validator compares every generated field with that source.

Crystal's action selects this appearance at these coordinates:
https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/targuna/actions_reliable_ram.lua

The Item binding inventory has no identity for 53439. The Q1b resolver
contract in `tools/world-bundle-compiler/src/resolve.rs` permits direct
Terrain keys but reaches WorldObjects through Items. Therefore this bounded
appearance uses the existing appearance-only Terrain route. Its source
`unpass` flag selects the same wall collision classification used by the
Sugar room qualifier. It grants no pickup, inventory, Item alias or script
capability; the registered Targuna callback remains the quest action owner.
The physical turnips remain separately qualified by SRV7.
