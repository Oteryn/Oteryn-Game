# Geography source qualification

Crystal source revision `201189f871d23597b82caf88fd611454a2ddefb8`.
The retained B3 files determine the recognized footprints.

| Area | Evidence and decision |
| --- | --- |
| Dwacatra | Crystal `data-global/world/world-monster.xml` supplies the Cyclops at (32588, 31917, 14). Its B3 component has 2,696 land tiles, 451 lava boundary tiles, 8 water boundary tiles and no void boundary. The floor-13 pocket is separate. Main already restores the Explorer Society door at (32598, 31933, 15) through action 40654 in `quest-door-bindings.json`. |
| Redbone Castle | Crystal `data-global/world/world-npc.xml` supplies the Demon Skeleton at (32666, 31675, 15), confirmed by the TibiaWiki NPC page. The B3 component has 491 land tiles and 91 lava boundary tiles. Draconia's floor-7 footprint is separate. |
| Isle of Merriment | Fandom page 89930, revision 1082408, documents a temporary staff-operated Thais forcefield and a test-server raid arena. The pinned Crystal map and B3 have shallow water (4601) at the anchor and B3 has no land within the qualification radius. |
| Travora | The 15.30 subregion position establishes the 66-tile floor-7 footprint. Crystal places Captain Tiberius at (32056, 32368, 6). Adjacent clear deck (32057, 32368, 6) keeps arrivals within reach of the return captain. |
| Travora ticket and passages | Crystal `petros.lua` and `captain_sinbeard.lua` name Travora as a destination. The retained TibiaWiki BR NPC facts name Greyhound, Seahorse and Karith. The *Character World Transfer* transcript names Breezelda and the retained product manual supplies the 1,000-gold, 24-hour policy. Tiberius's Fandom revision 851604 specifies the originating port. The native ticket purchase and passage use successive confirmations. Inbound fares are covered by the ticket and the return always remains free for the remembered origin. |

## Draft surfaces

The B3 draft capture has seven areas and 18 unresolved markers. I compared the
base map and pinned player-recorded minimap with Crystal `world.otbm` (Git
blob `e61658ec0ab7b70ec35025ae969426d6fc57ae75`), its `maps.7z` fragments and
the `15.30/Thalassara_34010_32470_7_1530.otbm` fragment. The archive's Blue
Valley fragment already supplies the matching stacks in B3. No remaining
draft land tile had a replacement stack established by these sources.

| Area | Draft tiles | Markers | Qualification |
| --- | ---: | ---: | --- |
| Blue Valley | 10,165 | 2 | Both marker cells are absent from Crystal. Main already has the four donor on-use travel bindings. |
| Great Expedition Island | 6,745 | 0 | The TibiaWiki event page identifies 15.32 and the expedition construction progression. Crystal has water or absent tiles at the drafted surface. |
| Great Expedition Wharf | 1,644 | 8 | Crystal has water at all eight markers. The minimap gives positions and movement colors, while the event page describes shared construction. |
| Marapur–Thalassara | 14,226 | 0 | The standalone Thalassara fragment supplies existing geometry. Its land does not overlap the remaining draft stacks. |
| Nargor | 1,516 | 0 | Crystal supplies the existing cliff and cave cells. The missing upper-floor surfaces have no qualified replacement stacks. |
| Temple of Light | 6,301 | 8 | Crystal has water or no tile at the markers. TibiaWiki documents seasonal entry through Lucius. The minimap markers do not identify what connects the floors. |
| Upper Roshamuul | 901 | 0 | The missing floor-6 surfaces have no qualified replacement stacks. The wiki describes access via the world wall breach on floor 7. |
