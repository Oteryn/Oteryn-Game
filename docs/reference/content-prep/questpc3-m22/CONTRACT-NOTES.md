# M22 — captured carried target quantities

Pinned donor: Crystal `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`.

| Fact | Source | SHA-256 |
| --- | --- | --- |
| Star Herb 3736 transforms antlers 10297 into antler talisman 22008, consuming one herb | [actions_star_herb.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/the_tainted_soul/actions_star_herb.lua), lines 8–13 | `f3ffe07cf0538c6a072cc0145e340ea9bf1a74a83601e3e7c42c6a90d03f13e4` |
| Holy Water 133 transforms antler talisman 22008 into sacred talisman 22009, consuming one flask | [actions_holy_water.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/scripts/quests/others/actions_holy_water.lua), lines 41–55, 141 | `464b7766160ab76a99d28ff1e87c8e1d870caffff69c7757171c50511c5fcffa` |
| A same-type transformation captures the original subtype/count before changing the item ID | [game.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/src/game/game.cpp), lines 3097–3148 | `4b26d0c5168e0907ae9044b2587a37ba647f6b6651a94e02f1c12a2e3af58d45` |
| Donor get/set subtype also exposes the count for ordinary nonstackable objects | [item.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/src/items/item.cpp), lines 505–514, 537–546 | `3484af408e0b25a7204a03725318b6e0c55811cef4922bbfd54af33f4b6661f5` |

Reused quantity core: `qm-qm22` commit `28a4c6b1f`, collected by QENGINE2
as `0bfca2195`. The existing rule captures the full target debit. A stackable
replacement keeps that count; a nonstackable replacement represents one
transformed object with quantity one. This is an explicit Oteryn representation
adaptation, not permission to mint a nonstackable Item with a stacked quantity.

The legacy quantity-blind API must refuse a capture-enabled conversion even
when a placeholder count of one was supplied. Only the quantity-aware durable
owner may use the captured debit. Capacity and native output admission must
succeed before either input changes; replay must use the original receipt.

The next runtime slice still needs to persist the captured target debit,
compose the existing Item mutation guards, and register the Tainted Souls
recipes. The quantity core alone does not qualify the quest end to end.
