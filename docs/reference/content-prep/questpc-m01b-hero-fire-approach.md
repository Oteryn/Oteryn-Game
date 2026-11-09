# Hero of Rathleton: chosen fire-route survey

Checked 2026-11-09 against [Canary 04b83b51, movements_lava.lua](https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/data-otservbr-global/scripts/quests/hero_of_rathleton/movements_lava.lua).

The action-24866 movement checks access, the running encounter and seven charged machines before transporting the player to `Position(33552, 32049, 15)`. This is the first raging-fire room, rather than the Professor Maxxen boss room.

The chosen personal stage-3 survey selects the eastern approach `(33553,32049,-15)`, retained ground 20537. Its eastern starting tile `(33554,32049,-15)` retains ground 20533. Both were checked in `content/world/placements/region-z15-x131-y125.b3`; no source placements are changed.

The resolver preserves the chosen recipe's current Premium requirement, minimum level zero, and exact stage-1/stage-2 counts of one. Machine charging, portal eligibility and the encounter remain with their existing owners. This resolver is inactive pending its native STEP test; it does not establish full quest playability.
