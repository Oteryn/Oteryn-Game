# M21: captured map container sources

The engine retains nested bundle entries and their original attributes. Nested
addresses follow the tile's top-level ordinals in preorder; existing top-level
addresses remain unchanged. The bundle digest and reset epoch remain part of
every source handle and durable MINT receipt. Invalid depth or address overflow
refuses the complete affected source tree.

Opening a source creates no item. Taking a leaf uses the existing shared map
MINT and custody transfer. Its original source address is hidden by the durable
receipt, including after a cache rebuild. Numbered keys retain the captured
action number through the immutable `game_item_keys` owner and typed MINT audit.

Checked source references (2026-11-08):

* [Outlaw Camp, immutable TibiaWiki BR revision](https://www.tibiawiki.com.br/index.php?curid=4537&oldid=428680):
  keys 3301/3302/3303 in dead trees at (32609,32244,7), (32617,32250,7),
  (32651,32244,7). The retained world currently contains the trees but lacks
  their child keys. This engine change alone does not restore those placements.
* [Crystal Item definitions](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml):
  copper key 2970, silver key 2969; barrel 2519/2523 capacity 25;
  skeleton decoration 6525. Dead tree 3634 has no container capacity in this XML.
* [Shards of Ancient Winters, immutable TibiaWiki BR revision](https://www.tibiawiki.com.br/index.php?oldid=416716):
  the Skeleton Decoration reward is the Tyrsung frost giant room, not the
  Chakoya igloo. No barrel placement is inferred from the nearby Chakoya barrels.

The Small Ruby already has a physical source and production reset tests in
`postgres_tests/l6_world_pickups.rs`. Personal quest completion is distinct from
the shared source receipt; this change does not mint a second personal ruby.

## First tested engine slice

The source-view regression failed before implementation (52.397030 s).
The real PostgreSQL source-view and nested-key pickup cases passed at
`92e99f6c8` with the engine changes: 58.948554 s and 58.148529 s.
The latter checks carried key number 3301, one shared source receipt,
a stale second pickup, an empty reread and rejection of a different reset epoch.
The publication merges qmech `4bed43770`; it changes neither this engine slice
nor its fixture or the backpack/key policies. World pin-check passed after that merge.
This is an engine checkpoint. Restoring source placements and wiring quest
completion are subsequent work; the quests are not declared Native-ready here.

## Fixed source views without a portable Item policy

The pinned Item index contains dead tree 3634, while the native portable Item
provider has no record for it. A read-only captured source therefore derives
its exposed slots from its captured children when custody facts are absent.
Portable unknowns and empty noncontainers still fail closed. Known capacities
are bounded by the existing 20-entry view window; excess children refuse the
view instead of being truncated. The standalone Rust planner regression failed
for the missing-policy case before the fix and all three cases pass after it.
