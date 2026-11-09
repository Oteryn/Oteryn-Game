# Hero of Rathleton: chosen fire-route survey

Reference donor checked 2026-11-09: [Crystal 201189f8](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/hero_of_rathleton/movements_lava.lua) independently confirms destination `(33552,32049,15)` and action 24866. The retained canonical source was cross-checked against [Canary 04b83b51, movements_lava.lua](https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/hero_of_rathleton/movements_lava.lua).

The action-24866 movement checks access, the running encounter and seven charged machines before transporting the player to `Position(33552, 32049, 15)`. This is the first raging-fire room, rather than the Professor Maxxen boss room.

The chosen personal stage-3 survey selects the eastern approach `(33553,32049,-15)`, retained ground 20537. Its eastern starting tile `(33554,32049,-15)` retains ground 20533. Both were checked in `content/world/placements/region-z15-x131-y125.b3`; no source placements are changed.

The resolver preserves the chosen recipe's current Premium requirement, minimum level zero, and exact stage-1/stage-2 counts of one. Machine charging, portal eligibility and the encounter remain with their existing owners. The native STEP case passed in 116.711981393 seconds on branch source 7d7350fc6, with a real first survey and only the separate evidence-owner precondition arranged. It proves the chosen survey hook and its guards; full quest playability remains unverified.
