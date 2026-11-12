# Grizzly room count: SPKG2-FIX

The qualified Tromphonyte room contains **288 distinct tiles**, not 73.
This is the eight-neighbour component under the package's admitted collision
facts, not the count of every tile inside the donor's rectangular bounds.

## Pinned inputs

- Repository: `zimbadev/crystalserver`, revision
  `201189f871d23597b82caf88fd611454a2ddefb8` (the debris profile generator's pin).
- `data-global/world/world.otbm`: Git blob
  `e61658ec0ab7b70ec35025ae969426d6fc57ae75`, 53,471,153 bytes, SHA-256
  `c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85`.
- `data-global/scripts/quests/killing_in_the_name_of/movements_boss.lua:167–178`:
  player `(33111,31184,8)`, boss `(33120,31195,8)`, center `(33113,31188,8)`,
  ranges `(11,18)`. Thus the source rectangle is `[33102,31170,33124,31206,8]`.
- `data/items/items.xml:17891–17893`: IDs 6475–6487 are debris, primarytype rocks;
  SHA-256 `8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509`.
- Official 15.30 appearances SHA-256
  `2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2`:
  the eleven admitted IDs below lack `unpass`.
  `build_grizzly_debris_profiles.py` verifies this XML and appearance evidence.
  IDs 6484 and 6487 remain outside this package's closed admission set.

## How counted

Fetch the exact donor files above from `raw.githubusercontent.com`, verify their
digests, and stream the OTBM with
`tools/content-schema/world-authoring/otbm_reader.py:read_tiles`.
The reader visited 19,492,788 donor tiles with no unknown item or tile attributes.
Keep the source room rectangle plus a one-tile ring. For each coordinate, retain
only depth-zero items; nested container contents do not occupy floor tiles.

Run an eight-neighbour flood fill from the source player position, requiring the
source boss position to belong to it. Apply the same qualification rules as
`grizzly_rooms.py`: known walkable ground, known nonblocking Terrain borders and
WorldObjects, and `blocks_movement == false` for native Item fallback. Unknown
collision remains blocking. The eleven debris fallback facts are independently
checked against the pinned XML and official appearances above. Read positions
and item stacks from the donor OTBM, not the generated room file. Check the
one-tile ring to ensure the component does not reach the extraction boundary.

The resulting donor component counts are Hide **90**, Esmeralda **64**, and
Tromphonyte **288**, matching the regenerated repository placements. Within
Tromphonyte's component the top-level debris item counts are:

| Item ID | Occurrences |
| --- | ---: |
| 6475 | 48 |
| 6476 | 7 |
| 6477 | 10 |
| 6478 | 6 |
| 6479 | 10 |
| 6480 | 11 |
| 6481 | 12 |
| 6482 | 13 |
| 6483 | 16 |
| 6485 | 9 |
| 6486 | 11 |

These **153 item occurrences occupy 146 distinct tiles**; overlapping items
are counted once for room area. Another **142 tiles have no admitted debris**.
Thus `146 + 142 = 288`. No source placements or collision facts were changed
to obtain that count.

## Where 73 entered the test

`git log -S '16: 73' -- tools/content-schema/quest-authoring/test_grizzly_rooms.py`
identifies `e415b2b7de52a4faf1b4e9c183d738702cf9daf2`. That commit first asserted
73 and added `grizzly-debris-item-profiles.json` with **all eleven** profiles,
each `blocks_movement: false`, plus their registration hook. However, neither
that commit nor its parent materialized any of those profiles in
`content/items/definitions/spell-native-profiles.json`. Both retained generated
room counts `31/0/0` for bosses `2/6/16`. It also omitted the generated ground
9290 shard. The committed assertion therefore had no passing generated state.

A controlled replay on the donor tile stacks explains 73: allow only debris
IDs **6478 and 6483**, treating the other nine admitted types as unknown/blocking.
The reachable component then has **11 debris-bearing tiles + 62 others = 73**
(five occurrences of 6478 and six of 6483). Restoring all eleven yields 288.
This reconstructs the numerical result; Git does not establish that the author
actually used that partial admission. It would be incorrect to claim that only
two profiles existed in the historical commit.

The correction retains the exact count assertion and every other assertion.
It changes the expected value to the fully admitted, source-backed count.
