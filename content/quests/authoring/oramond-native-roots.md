# Oramond: selected daily root route

This binds the existing chosen daily recipe's five marked plants, not the full
Crystal Oramond quest. Ask Chavis about **marked roots**. The same fenced NPC
receipt starts the route, advances its monotone personal epoch, resets its five
plant flags and root count, and stores the transaction clock. Report **marked bundle** to Chavis to consume the five roots and record
the personal hand-in. Another start is
refused until 86,400 seconds have elapsed. Each selected native plant grants one
Juicy Roots (21291) and advances its own flag and the shared count atomically.
Bag/capacity refusal changes neither progress nor its cycle-scoped MINT claim.

The five selectors are existing floor-7 native entries in
`content/world/placements/region-z07-x130-y124.b3`: 33534/31908 ordinal2 (21104),
33525/31909 ordinal2 (21104), 33520/31915 ordinal4 (21104),
33530/31921 ordinal3 (21105), 33523/31930 ordinal4 (21104).
Coordinates, ordinals and identities were decoded from the pinned region, not
invented or added to the map. Native floors use -7.

Source evidence:
https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/oramond/to_take_roots/actions_to_take_roots.lua
(lines 14/34 grant 21291, lines 24/44 increment personal count).

Explicit chosen-route adaptation: the existing recipe selects five roots and a
daily route. These five plants yield deterministically once per personal epoch;
Crystal's 50% random harvest and shared 120-second depleted-plant reset are not
implemented by this selected route. No global plant transform or world-progress
credit is claimed. Existing retained donor rules and holds remain unchanged.
Pollen, probing and sewer stage bindings remain separately owned; this component
does not assert that the whole selected Oramond journey is qualified.

Harvests also increment the existing source To Take Roots count, enabling its
Chavis five-root hand-in. The selected `marked bundle` receipt consumes exactly
five roots and resets that source count without minting a second reward.
Daily restart requires that source count to be zero after hand-in; it cannot
erase an unreported harvest or consume a second bundle.
