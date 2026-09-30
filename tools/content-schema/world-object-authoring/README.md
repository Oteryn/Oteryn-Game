# WorldObject and Terrain authoring tooling (WO-1, WO-2)

This package implements the tooling half of the WO-0 decision
[`OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md`](../../../docs/architecture/reviews/OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md)
(owner decisions D93 and D94). It describes what a static Terrain or WorldObject definition is, validates one, and
censuses the pinned engine's routed ids.

The family key is a pure function of the Tibia Item key and the converter route (D93, A12 §4.6); no number is
allocated. `build_catalogue.py` (WO-2) writes the records to `content/world/terrain/` and `content/world/objects/`.
None of this is a placement, the runtime `LocalObject` overlay or runtime serialization.

| File | Purpose |
|---|---|
| `terrain.schema.json` | One Terrain record (WO-0 §4.2): `kind` (ground, border, wall, field, roof), an optional `behavior` marker (trash_holder, teleport), walkability, ground speed, projectile and sight blocking, floor change, automap, field type, client projection and provenance. It also holds the shared KNOWN/UNKNOWN wrappers. |
| `world-object.schema.json` | One WorldObject record (WO-0 §4.3): `kind` (object, door, ladder, bed, container_fixture, teleport, corpse, decoration), collision, movability, placement, floor change, fluid source, and kind-specific `bed`, `corpse` and `door` sections. |
| `routed-item-pointer.schema.json` | The WO-0 §4.1 typed `routed_to {family, key, revision}` pointer that a routed Item record carries. A bare key, a family other than Terrain or WorldObject, and a materializable Item are all rejected. |
| `world_objects.py` | The D93 key rule, the D94 exclusions, the record builders, the validator (`--validate RECORD...`) and the census. `--check` diffs an in-memory regeneration against the committed sample. `--records DIR` also writes every record, for local inspection only. |
| `build_catalogue.py` | WO-2: writes every record to `content/world/terrain/terrain-*.json` and `content/world/objects/objects-*.json` (500 per shard, ascending Tibia id), marks both directories `POPULATED` and rebuilds the census sample from the same run. `--check` fails on any byte difference, missing or extra shard. |
| `test_world_objects.py` | No-network tests. Routing goes through `engine_items.convert_item` over fabricated sources, using the real identity index. Run with `python test_world_objects.py`. |
| `samples/census-crystal-ff7ede5.json` | The committed census for the pinned Crystal revision. |

## Rules

- **Routing.** A record exists for every id where `engine_items.convert_item` reports `routed_non_item`, except for
  the D94 exclusions:
  - `WorldObject:appearance_placeholder_slot`;
  - `WorldObject:no_client_appearance`;
  - `Fluid:fluid_type_without_appearance`, which stays Item fluid.

  After D149 only the first still has ids (128).

  The route is never re-decided here.
- **Identity (D93, A12 §4.6).** `oteryn:item.tibia.i<id>` becomes `oteryn:terrain.tibia.i<id>` or
  `oteryn:world-object.tibia.i<id>`, keeping the same Tibia id. Nothing else is accepted.
- **Facts.**
  - A boolean `appearances.dat` flag is complete for an id that has an appearance, because an unset optional bool
    is false. It is therefore always KNOWN. An unset hook is `none`, and an unset height is elevation `0`.
  - An `items.xml` attribute is sparse. When it is absent the fact is UNKNOWN, never an engine default.
  - Without an appearance, every appearance fact is UNKNOWN.
  - Every KNOWN fact names its source field in `provenance.fields`.
- **Relations.** `rotateto`, bed parts, sleepers and transforms stay `{"source_item_id": n}`. Under A12 the source
  id is the Tibia id, so the target is determined; typed references come with the Rust family (WO-2b).
- **Corpses.** Decay target, duration and container capacity stay on the routed Item record (WO-0 §4.3). A
  WorldObject corpse only records the corpse and player-corpse flags.
- **Kind.** The first matching rule decides the kind. When no rule matches, the kind is UNKNOWN; it is never guessed.
  Owner decisions WO-2c (2026-09-30) resolved the WO-0 "contested routes":
  - 1a: a Terrain tile named as a roof, with no ground, border or wall flag, is kind `roof` (234 tiles);
  - 2a, 4a: a Terrain tile whose `items.xml` `type` is `trashholder` or `teleport` keeps its own kind and carries a
    typed `behavior` marker (`trash_holder`, `teleport`) for Interaction, which owns the behaviour;
  - 3a, 5a: magic fields route to Terrain `field` and fixed carpets to WorldObject `decoration` (converter routes
    `magic_field` and `fixed_carpet`, see the item-authoring README).

  65 Terrain tiles (mosaics, unbanked floors, leaves) still have no rule and stay UNKNOWN under
  `unknown_kind_examples`.

## Pinned Crystal census (`ff7ede5`)

- After D149, **21,330 records**: 8,548 Terrain and 12,782 WorldObject (128 placeholder slots excluded). This
  matches D94's "about 21.4k".
- Terrain: 3,761 border, 2,197 ground, 2,166 wall, 234 roof, 125 field, 65 UNKNOWN.
- WorldObject: 5,654 object, 3,358 corpse, 2,784 decoration, 728 door, 192 bed, 49 teleport, 17 ladder.

```sh
python world_objects.py --source <crystalserver checkout at ff7ede5>          # write the sample
python world_objects.py --source <checkout> --check                          # drift check
python world_objects.py --validate record.json                                # validate records
python build_catalogue.py --source <checkout>                                 # write the catalogues
python build_catalogue.py --source <checkout> --check                         # catalogue drift check
python test_world_objects.py
```
