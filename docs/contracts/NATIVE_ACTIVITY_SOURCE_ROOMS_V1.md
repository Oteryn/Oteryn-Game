# Native activity source rooms v1

SRX-3 supplies bounded, reproducible map captures for the Primal Ordeal,
Soulpit, Soul War, Rotten Blood, Brain Head and eleven fixed-global callback contexts. Source: Crystal
`201189f871d23597b82caf88fd611454a2ddefb8` (`OTS_HYPOTHESIS_ONLY`,
owner-requested donor reference; wiki-pending).

| Room | Script placement evidence | Captured map evidence |
| --- | --- | --- |
| Brain Head | `data-global/scripts/quests/feaster_of_souls/actions_portal_brain_head.lua:6-7,70-76`; `data-global/world/world-zones.xml:3` identifies zone1 | Actual OTBM zone1 has476 cells and bounds `(31941,32313)..(31966,32337),z10`; capture includes genuine neighboring cells and the literal return anchor |
| Primal | `data-global/scripts/actions/bosses_levers/the_primal_manace.lua:4` boss; `:8-12` five participants; `:15-16` arena bounds; `:20` return position | 537 actual tiles, including the source staging cells and return anchor |
| Soulpit | `data-global/lib/others/soulpit.lua:34-55` five participant/arrival pairs; `:138` obelisk; `:140` return position; `:178` arena bounds | 622 actual tiles, including the source staging cells and obelisk |
| Soul War: Spite | `data-global/lib/quests/soul_war.lua:296-329` boss, five participant/arrival pairs, arena and return anchor | 336 actual tiles |
| Soul War: Malice | `data-global/lib/quests/soul_war.lua:332-352` boss, five participant/arrival pairs, arena and return anchor | 390 actual tiles |
| Soul War: Greed | `data-global/lib/quests/soul_war.lua:355-391` boss, five participant/arrival pairs, arena and return anchor | 334 actual tiles |
| Soul War: Hatred | `data-global/lib/quests/soul_war.lua:394-414` boss, five participant/arrival pairs, arena and return anchor | 285 actual tiles |
| Soul War: Cruelty | `data-global/lib/quests/soul_war.lua:422-442` boss, five participant/arrival pairs, arena and return anchor | 336 actual tiles |
| Soul War: Megalomania | `data-global/lib/quests/soul_war.lua:450-470` boss, five participant/arrival pairs, arena and return anchor | 628 actual tiles, including source Blue-callback LOS rays |
| Rotten Blood: Murcion | `data-global/scripts/quests/rotten_blood_quest/rotten_murcion.lua:1-31` | 321 actual tiles |
| Rotten Blood: Ichgahal | `data-global/scripts/quests/rotten_blood_quest/rotten_ichgahal.lua:1-23` | 402 actual tiles |
| Rotten Blood: Chagorz | `data-global/scripts/quests/rotten_blood_quest/rotten_chagorz.lua:1-32` | 357 actual tiles |
| Rotten Blood: Vemiath | `data-global/scripts/quests/rotten_blood_quest/rotten_vemiath.lua:1-23` | 348 actual tiles |
| Rotten Blood: Bakragore | `data-global/scripts/quests/rotten_blood_quest/rotten_bakragore.lua:1-24`; `rotten_bakragore1.lua:1-24` second staging line; `monster/quests/rotten_blood_quest/bakragore.lua:369-414` essence flame/pillar anchors | 590 actual tiles |

| Callback: Lady Tenebris | `data-global/scripts/quests/forgotten_knowledge/actions_lady_tenebris.lua:3-4,35`; exact callback coordinate regions are pinned in the producer recipe | 625 actual tiles |
| Callback: Outburst | `data-global/scripts/quests/heart_of_destruction/actions_outburst_lever.lua:4-15`; exact callback coordinate regions are pinned in the producer recipe | 600 actual tiles |
| Callback: Professor Maxxen | `data-global/scripts/quests/hero_of_rathleton/movements_professor_maxxen.lua:5,24,84`; exact callback coordinate regions are pinned in the producer recipe | 210 actual tiles |
| Callback: Razzagorn | `data-global/scripts/quests/ferumbras_ascension/actions_razzagorn.lua:4,40`; exact callback coordinate regions are pinned in the producer recipe | 882 actual tiles |
| Callback: Shulgrax | `data-global/scripts/quests/ferumbras_ascension/actions_shulgrax.lua:11,25`; exact callback coordinate regions are pinned in the producer recipe | 552 actual tiles |
| Callback: Plagirath | `data-global/scripts/quests/ferumbras_ascension/actions_plagirath.lua:11,47`; exact callback coordinate regions are pinned in the producer recipe | 621 actual tiles |
| Callback: Ragiaz | `data-global/scripts/quests/ferumbras_ascension/actions_ragiaz.lua:3-4,25`; exact callback coordinate regions are pinned in the producer recipe | 552 actual tiles |
| Callback: Zamulosh | `data-global/scripts/quests/ferumbras_ascension/actions_zamulosh.lua:3-4,26`; exact callback coordinate regions are pinned in the producer recipe | 357 actual tiles |
| Callback: The Time Guardian | `data-global/scripts/quests/forgotten_knowledge/actions_the_time_guardian.lua:3,8-9,46`; exact callback coordinate regions are pinned in the producer recipe | 1619 actual tiles |
| Callback: Mazoran | `data-global/scripts/quests/ferumbras_ascension/actions_mazoran.lua:2-11,26`; exact callback coordinate regions are pinned in the producer recipe | 644 actual tiles |
| Callback: Anomaly | `data-global/scripts/quests/heart_of_destruction/actions_anomaly_lever.lua:3-15`; exact callback coordinate regions are pinned in the producer recipe | 702 actual tiles |

The binary source is `data-global/world/world.otbm`, Git blob
`e61658ec0ab7b70ec35025ae969426d6fc57ae75`, SHA-256
`c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85`.
Binary evidence addresses tiles by `(x,y,z)`, rather than text line numbers.
Each capture records the producer's exact closed `source_regions` rectangles.
Those regions prove where the source map was inspected, including absent tiles;
`source_bounds` is only their envelope. Gaps between a room, its staging cells
and its return anchor remain uninspected. Placement cannot treat those gaps as
missing donor tiles or silently choose another fallback cell. The producer's
`--all --source-regions-only` path scans the pinned OTBM once and verifies every
previous JSON field, tile and policy unchanged before writing any of the 25
metadata updates. No rectangle is filled with invented terrain.
`data/items/appearances.dat` and `data/items/items.xml` provide the exact
ground speed, collision, pathfinding, Item flags and placed Item attributes.

`tools/content-schema/native-gameplay/build_activity_source_world.py` verifies
all three pinned Git blobs before extracting any data. It reads no executable
donor code. Missing tiles remain absent. Unknown tile/Item attributes and house
tiles reject extraction. Ground and collision facts are recalculated by the
Content loader from the captured Item policies. Mutable source placements remain
explicit; an unmaterialized placement does not prove a free tile or clear LOS.

`PinnedActivitySourceRoom` selects an embedded capture, rather than accepting
caller-supplied map bytes or coordinates. `ActiveGeneration::qualify_activity_source_room`
reproves the active server/client artifact pair and resolves every captured Item
identity against its native Item index. The closed 162 non-inventory static
placements instead retain their actual Terrain/WorldObject families and exact
published source policy; see `SRX3_ACTIVITY_STATIC_GEOMETRY_V1.md`. The resulting room retains those content
digests and receives its own source-map/frame digests. Qualification issues no
Instance allocation, readiness, participant, session or teleport authority.

Source positions retain donor `x,y,z`; the read-only source projection uses
`(x,y,-z)`. The native bundle adapter materializes the same captured tiles through
the repository bundle codec. Its map coordinates use negative floors and its
runtime actor positions use the existing positive legacy floor convention.
The ordinary map snapshot (domain 17), item handles and activity scope barrier
(domain 99) remain the native client path. No capability, command or protocol
message number is allocated.

The activation owner retains the qualified rooms for the activity runtime's
lifetime. An encounter still requires its own complete source callback closure,
entry qualification, fenced durable handoff, current runtime and terminal owner.
Map qualification alone does not qualify an encounter.

Regeneration, using read-only reference files named by the producer:

```sh
python3 tools/content-schema/native-gameplay/build_activity_source_world.py --source-dir <reference-dir> --room primal
python3 tools/content-schema/native-gameplay/build_activity_source_world.py --source-dir <reference-dir> --room soulpit
```

Add `--check` to verify the generated artifacts without changing them. The
Thirteen source captures are generated data; their bytes and pins are
changed only through the producer. The producer's `ROOMS` table records every
exact bounded region; the matching `content/encounters/<room>/source-room.json`
records each captured coordinate and original Item attributes. For example:

```sh
python3 tools/content-schema/native-gameplay/build_activity_source_world.py --source-dir <reference-dir> --room soul-war-greed --check
python3 tools/content-schema/native-gameplay/build_activity_source_world.py --source-dir <reference-dir> --room rotten-blood-bakragore --check
```

Bakragore captures both source staging lines without selecting between their
68-hour and 20-hour cooldowns. Its source Item 3081 at `(33016,32393,15)`
retains OTBM attribute 22 (charges) equal to zero. Capture preserves that
attribute; it does not grant permission to discard it during materialization.

W11 White-ground target Item409 is a real pinned policy/binding in the Malice
and Megalomania captures, although no white tile is invented in their initial
placements (`soul_war.lua:1139-1140`). Megalomania additionally captures the
source terrain around `(33710,31634,14)`. The Blue Lua matrix has centre3 at
row7/column8; its actual symmetric cross reaches +/-8x and +/-7y. The wider
captured west/north margin is genuine source coverage, not callback reach or an
expanded encounter zone. This capture does not itself qualify the callback or
permit a synthetic map cell.

Brain Head's source callback uses the literal global tile `(31954,32325,10)`
(`data-global/scripts/spells/monster/heal_brain_head.lua:2`). Its source room
is generated with `--room brain-head`; matching raw coordinates on an unrelated
bundle do not qualify this anchor. The native callback must retain the exact
source capture/frame binding before reading the current top creature.

Mutable bundle facts are scoped to the actual Instance. The activation's source
capture may share immutable tiles, Item policies and presentation assets between
parties; its callback changes, movement effects and creature steps must not be
shared. A source role and matching static map digest alone do not prove that
mutable ownership. Production activity entry requires a fresh fact owner (with
its lifetime retained by the registered runtime); a shared activation-room Arc
is insufficient. Concurrent-Instance validation must cover ground/effect
isolation as well as equal immutable capture bytes.

Source activity Item policies also retain `source_top_order` from the pinned
appearance flags. Crystal `src/protobuf/appearances.proto:149-151` supplies clip,
bottom and top fields2/3/4; `src/items/items.cpp:203-208` establishes precedence:
clip1, otherwise top3, otherwise bottom2, otherwise0. This source-only metadata
supports native callbacks that distinguish top Item order. It does not change
bundle collision flags or the legacy source-world capture. The activity producer
regenerates all14 bounded captures; Content requires a value0..3 and the source
bundle converter rejects out-of-range values.


Bakragore's four essence flames and four pillar anchors are actual source tiles.
The capture additionally retains their genuine three-wide cross approaches, so
native movement/USE does not have to cross unknown cells. This changes captured
coverage, not the boss encounter zone or callback reach. The ten original/target
server IDs and corridor decoration43989 are map-only WorldObject references,
qualified through the closed static registry; they create no Item definition,
inventory admission or physical MINT authority. Source flame transformations
change collision with their visible state (solid inactive, nonsolid active);
both pillar states remain solid.


Vemiath retains both exits as source recipe evidence. The raw Lua exit is
`(33043,32344,15)` (`data-global/scripts/quests/rotten_blood_quest/rotten_vemiath.lua:18`,
Crystal201189f). Its actual OTBM stack includes Teleport34111 with destination
`(34001,31820,15)`. The latter tile is captured exactly and contains ground36344
and decoration42671. Neither the blocked raw Teleport tile nor any unknown tile
is declared walkable to qualify the room.

The effective return is the final destination, composed through the existing
World floor/Teleport resolver over the authenticated actual Bundle cells. The
source chain is `CreatureFunctions::luaCreatureTeleportTo` (`creature_functions.cpp:877-893`),
`Game::internalTeleport` (`game.cpp:3167-3191`), `Map::moveCreature`
(`map.cpp:444`), `Tile::postAddNotification` (`items/tile.cpp:1585-1590`) and
`Teleport::addThing` (`game/movement/teleport.cpp:76-108`). These are the pinned
Crystal201189f sources. The qualified template, literal native catalogue,
participant exit record, durable return receipt and native scope snapshot must
all use the same resolved `(34001,31820,15)` cell. A caller-supplied different
coordinate cannot qualify this recipe. Wiki checks remain wiki-pending.

Rotten centre portals use an internal physical Item source class3, separate
from Ability1 and Encounter2. This is a CreatureEvent cause with the actual
Vemiath/Chagorz/Bakragore Creature definition and the actual reserved onThink
occurrence; no substitute spell or encounter definition is introduced.
Crystal201189f `data-global/monster/quests/rotten_blood_quest/vemiath.lua:261-275`,
`chagorz.lua:251-265` and `bakragore.lua:269-283` create37000, set action50001,
and schedule a15s callback reading the CURRENT37000 at the fixed centre.
The Item has no fabricated ItemType decay. A retained current owner/Instance
and accepted activity pin proof gates the existing Item line/custody writer;
removing the original issuing actor does not invalidate a source addEvent.
The Rotten portal branch allows only those three Creature keys, source rules and Item37000,
with exact tagged Instance scope and same-transaction physical proofs.

The same class3 writer also has closed copied-death branches for
`minion_of_gaz_haragoth` and `corrupted_soul`. Their sealed inputs retain the
actual native death BEGIN, copied event and original actor incarnation. Their
Node/scope authority uses the existing physical scope lock and transaction;
it does not manufacture a Character session or a Player resource cost.
These branches accept either an exact Channel or an exact Instance namespace.
Gaz creates20121/action33542 and its60s callback looks up CURRENT20121.
Corrupted Soul creates23726 when none of23726–23728 exists and returns without
scheduling expiry. Otherwise it transforms the current23726→23727→23728 and
schedules a30s current-item lookup. An existing23728 stays unchanged and still
schedules that lookup. TransformGround preserves the same UUID,
quantity, custody and minted identity. The source Lua bodies are pinned to
`140e7db53054ac626d49e0b411395fe7b5e0a3d4bdb951de42be0853c77d6159`
and `1e03253a5fdbe294e57b0a7066b7ec232dfac8191a7b62665be633d71421c0fa`,
respectively. These branches do not grant a generic dead-actor writer or change
the Rotten branch's Instance and accepted-activity requirements.

Bakragore's Agony census is41 by23 tiles around `(33044,32398,15)`.
The literal `20,20,0,0` in Crystal201189f `bakragore.lua:443` does not select a
single row: `src/lua/functions/core/game/game_functions.cpp:212-230` passes the
arguments to `Spectators::find`, and `src/map/spectators.cpp:181-184` replaces
each zero with the default viewport extent. `src/map/map_const.hpp:20-23`
defines that Y extent as6+5=11. The pure owner checks X±20,Y±11 and the same
floor; this changes no Agony damage tier, stack cap or timing rule.

Ragiaz's hidden Bone Capsule is an actual map spawn, not a phase replacement
inferred from its appearance. Crystal201189f `data-global/world/world-monster.xml:182950-182952`
places it at `(33485,32333,14)`, radius1, with60s respawn. The source Gitblob is
`7949251d25d8db8f0b2ca65b67df76ac577a5caf`, SHA256
`f90e82203a112c9068d4ed123e3e5bb4c1b9743f2c2d3bcb12039ac8ebd9ad4e`.
The bounded `build_srx3_ragiaz_spawn.py` producer checks both hashes before
emitting its one source record. Qualification and registration must reuse the
native XML SpawnDefinition owner; the evidence alone grants no actor admission.
`src/creatures/monsters/spawns/spawn_monster.cpp:271-295` uses extended/nonforced
startup and nonextended/forced respawn; default direction isNorth (:91-99).

Smelly Cheese's delayed combat area retains the cast-time position. In pinned
Crystal201189f `data-global/scripts/spells/monster/smelly_cheese_berserk.lua:45-56`,
both delayed closures retain `var`; they re-resolve the live caster at execution.
`src/creatures/combat/spells.cpp:1293-1328` constructs that position variant
from the caster's original position, and `src/lua/functions/creatures/combat/combat_functions.cpp:213-215`
executes the retained variant position. Thus caster movement does not move the
500ms three-by-three Earth400..800 pulse or the700ms presentation pulse.
The player at each tile's actual current top is selected at execution, not at
cast time. The Lua SHA256 is
`59074420d45ea48551a8276d4ee6d082dcb6b48a7832ca301db2f88e12ab65e3`;
the two verified C++ file hashes are respectively
`06906bc067b56d89fc3f1d6c0d30fc58d405425d4d00e0127f49a14b6e01385d`
and `c1c580c5ef7ed713e51f4013fdbae49c351412257fc25e1bfa5cbca5ba9d2b95`.
Wiki checks remain wiki-pending.

Rotten Blood movement callbacks use the existing canonical position commit.
`CommittedMovementOccurrence` is immutable OLD/NEW data produced once on a
successful translation; turns, refused moves and failed commits produce none.
The closed Vemiath/Chagorz/Bakragore registration wraps that occurrence with its
actual native source body and scope/content fence. Source observers are selected
from the original CPP old/new spectator rectangle/union before later callbacks
can teleport the mover. A registered HP-zero NPC remains observable until its
canonical removal. The retained queue blocks later movement/native turns only
in the affected source Instance until the synchronous Item outcome is known.

Crystal `data-global/monster/quests/rotten_blood_quest/vemiath.lua:448-488`,
`chagorz.lua:377-417` and `bakragore.lua:633-706` qualify the three onMove rules.
Vemiath and Chagorz guard CURRENT top-down IDs43589/43590/43625; Bakragore guards
43589/43590/43297. Player footprints create43589, transform CURRENT43589 after1s,
transform CURRENT43590 after another3s, and remove the CURRENT final item after
another3s. The final ID is43297 for Vemiath/Bakragore and43625 for Chagorz.
Bakragore's own distinct movement creates43294 and transforms CURRENT43294 to
43295 after5s. Other creatures receive neither rule. These are retained Lua
callbacks, not invented ItemType decay. A known absent current item commits an
immutable zero-operation result and creates no subsequent nested timer.
The source maps contain none of those guard IDs statically; source qualification
refuses a future capture that changes that property. Dynamic stack reads use the
existing canonical top-down ordinal.

Internal physical Item operation7 is `TransformGround`. It extends the existing
single physical writer with the closed source edges above. The SAME Item UUID,
quantity1, Ground placement, frame, ordinal and minted identity survive; the
actual state revision advances once. There is no remove-and-MINT replacement,
Player transform grant, new Item store or protocol number. Existing operation1–6
and Portal CreatureEvent bytes are unchanged. Callback source class3 retains its
truthful Creature/onMove identity; its closed footprint rule has no action ID.
Unknown SQL outcomes retry the exact retained callback and first execution clock.
A nested timer starts from that known callback's retained execution, so retries
cannot resample a different current Item or shift its subsequent deadline.

The native attack scheduler retains its attacked creature in the proposal even
when a source spell has `needs_target=false`. The sealed Smelly adapter validates
that original proposal and its target intent, then captures the caster's VAR
position separately; it does not rewrite the proposal into a fictitious self
attack. Legacy draft-only validation retains its existing self-target contract.
Only retained delayed source work may observe an HP-zero, still-registered caster.
Source death completion makes `Creature(cid)` nil even when the physical slot is
retained for later settlement; new native casts still require a living caster.

The operation7 Item trigger probes a local candidate row with its successor
revision. It assigns that row to NEW only after the complete closed transform
proof succeeds. A failed probe preserves every incoming NEW field for the
original Item guard, including the ordinary Craft revision and quantity rules.

Smelly presentation uses the literal source constants: Crystal201189f
`src/utils/utils_definitions.hpp:72,76` defines HITBYPOISON17 and POISONAREA21
(SHA256 `03ebfc0da66b01cdef46dca4195c4f1a0132a0fce058d6c670cb401bcc35ae7d`).
Canary47df's same file:77,81 confirms both values
(SHA256 `bdd23ca8e86ab433bef04766f0c183d46fc2ca1796a81bab1d44db8d3fd685d1`).
They are existing native effect IDs, not newly allocated protocol messages.
