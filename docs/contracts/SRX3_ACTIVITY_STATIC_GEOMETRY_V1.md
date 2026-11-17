# Pinned activity static geometry

The SRX-3 source captures contain 161 appearance IDs with no admitted inventory
Item definition. All 162 are immovable, non-pickupable static source surfaces in the
pinned Crystal map. They are published as 38 ground Terrain definitions and 123
WorldObject definitions. No Item alias, inventory reference, MINT cause or pickup
right is created. The closed census and exact source policies are in
`content/world/source-static-definitions.json`.

The source is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`:
`data-global/world/world.otbm`, `data/items/appearances.dat` and
`data/items/items.xml`. Every capture producer run verifies all three Git blob
identities. `src/items/items.cpp:203-208` defines stack order; `:233-244` loads
solid, projectile, pickup and movement flags from the appearance flags. Every
published ID also has verified membership in the admitted official client 15.30
appearance artifact. Source policy supplies ground speed and collision; absent
inventory identity never supplies an inventory default.

`tools/content-schema/world-object-authoring/activity_static.py` publishes the
closed canonical shards, the static registry and the existing stable placement
palette slots. `build_catalogue.py` includes these qualified shards. The native
activity source producer chooses the actual static family and rejects every
other missing native Item identity. The source-room Content owner checks the
closed binding and actual active Content pair; all remaining Item bindings still
require the real native Item index.

The existing map codec has Item and Terrain transport families. A static
WorldObject uses map-only Terrain transport with kind `common`, no ground speed
and no walkability claim. Its canonical family remains WorldObject. Ground
Terrain keeps its actual source speed and walkability. Compact map definition IDs
are the sorted union of all canonical Terrain keys and the 123 closed static
WorldObject keys. The compiler and private source-room converter share that
exact generated index. These map IDs are never Item references.

A bounded callback context is sealed only after decoding the complete actual
bundle and comparing every captured tile, its full ordered stack, depth, source
flags, house, zones and protected attributes. Protected Item references must
still match the actual active native index; static references must match the
closed map-only index. Current occupancy, ground transformations and actor
identity are read again by the real map owner at callback execution. The proof
grants no entry, actor spawn, mutable Item ownership or callback rule.

A published-World proof additionally requires the complete actual bundle digest
to match the repository World pin. Its active issuer checks the current Content
pair and activation, World, map, coordinate frame and exact retained collision
index. The source-frame conversion comes from the World loader's explicit
`world-bundle-v3` legacy-z frame; runtime floor +z corresponds to source floor z
and native map floor −z. A copied frame contains conversion data only and grants
no rule or map authority. Soul War realm membership remains a separate pinned
Lua rectangle/exclusion rule; no nonexistent OTBM zone ID is invented.

Validation before the Bakragore extension: the 147-record static-census test failed with the missing
registry, then passed with all 147 non-Item records (0.093 s). Structural schema
validation passed for all 147 records (0.315 s); exact source-body semantic
validation passed for all 158 (1.771 s). The actual default 31 MB bundle context
test failed before context qualification (13.979 s). Final Rust, PostgreSQL and
client validation is recorded in the task report; these producer checks alone
do not establish an encounter play test. External wiki checks are wiki-pending.

The 158-record stage added eleven additional non-Item Bakragore decorations:
43497, 43498, 43507, 43508, 43983–43989. Crystal
`data-global/monster/quests/rotten_blood_quest/bakragore.lua:369-414` supplies the
four flame and four pillar coordinates and their transform/reset pairs. The
capture contains the genuine adjoining map cells and never fills missing tiles.
The source-map generator rejects all unknown definition IDs together before
writing its output. That stage contained 158 exact catalogue bodies and
8772 sorted map-only definition keys. At that stage, source-body/schema validation passed
for all 158 records in 2.175 s; the two census/substitution tests passed in
1.463 s, and the publisher's `--check` matched every emitted byte.

Bundles carrying these static surfaces require the manifest capability string
`source-static-surfaces-v1`. The compiler and private source converter emit it;
older readers must reject the bundle rather than interpreting a map-only
WorldObject as an ordinary nonblocking common surface. This adds no numeric
capability bit, command, protocol message or new binary format.

Vemiath's composed exit additionally requires the actual arrival decoration42671.
The exact pinned map tile `(34001,31820,15)` contains ground36344 and
WorldObject42671, with no invented neighboring tiles or inventory Item binding.
The closed registry now contains159 definitions (37 Terrain,122 WorldObject).
The actual missing-destination regression failed before publication (0.003s);
all three static census/substitution/destination tests passed afterward (1.472s).
The changed sorted transport index requires a repository-tool rebuild and pin
refresh of the served default bundle; the earlier bundle digest is not valid
proof of this159-record state.

Anomaly restoration additionally qualifies source ground23049 (Terrain, speed150,
solid/projectilefalse) from the pinned callback-anomaly capture policy. It is
a restore-only target, so it need not occur in the initial tile stack. The
closed registry is now160 (38 Terrain,122 WorldObject); the canonical Terrain
key already existed, so the sorted technical key enumeration stays unchanged.
The official publisher replaces its older unqualified Item-pointer projection
with protected source surface facts and grants no inventory Item identity.
The23049 record replaces its original canonical Terrain row in place through
the publisher; it is not duplicated in the appended37-row terrain shard.
All other499 records in its original shard remain unchanged. The160-entry
sealed registry and original technical Terrain ordinal are unchanged by this
canonical distribution correction.

## Native12 Ebb replacement maps

The three pinned replacement captures retain all12,355/34,392/18,449 tiles, including source outliers and ordered item attributes. Their sole additional static appearance33276 is an immovable, non-pickupable WorldObject with no collision or inventory Item alias. The official static publisher appends exactly one canonical definition; no previous33276 definition exists in the World corpus. The registry now contains161 records (38 Terrain,123 WorldObject); the original23049 Terrain ordinal remains unchanged. Source captures alone do not authorize a runtime map replacement; the World owner must bind the exact capture and current Map/Item state.


Outburst source-entry closure adds only static WorldObject20661 from its actual lever tile32207/31283/14. Its pinned policy is immovable/non-pickupable and blockSolid=true, with no inventory or dynamic Item alias. The five separate waiting tiles32207/31284..31288 and the literal return anchor32218/31375 remain walkable in the genuine map. The registered publisher now owns162 records (38 Terrain,124 WorldObject), and preserves all prior161 physical policies. The lever's blocking decoration is never relabelled walkable. The additional explicit23482 policy records the Lua onUseExtra query; the current map already has23483, so that first query takes Lua's nil branch.
