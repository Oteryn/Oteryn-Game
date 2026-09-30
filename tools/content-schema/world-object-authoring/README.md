# WorldObject and Terrain authoring tooling (WO-1)

This package implements the tooling half of the WO-0 decision
[`OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md`](../../../docs/architecture/reviews/OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md)
(owner decisions D93 and D94). It describes what a static Terrain or WorldObject definition is, validates one, and
censuses the pinned engine's routed ids.

It does not mint identity. The D93 family key is a pure function of the frozen CW2-B1 Item key and the converter route.
It also writes nothing under `content/`: population and identity review are WO-2. It is not a placement, not the
runtime `LocalObject` overlay, and not runtime serialization.

| File | Purpose |
|---|---|
| `terrain.schema.json` | One Terrain record (WO-0 §4.2): `kind` (ground, border, wall, field), walkability, ground speed, projectile and sight blocking, floor change, automap, field type, client projection and provenance. It also holds the shared KNOWN/UNKNOWN wrappers. |
| `world-object.schema.json` | One WorldObject record (WO-0 §4.3): `kind` (object, door, ladder, bed, container_fixture, teleport, corpse, decoration), collision, movability, placement, floor change, fluid source, and kind-specific `bed`, `corpse` and `door` sections. |
| `routed-item-pointer.schema.json` | The WO-0 §4.1 typed `routed_to {family, key, revision}` pointer that a routed Item record carries. A bare key, a family other than Terrain or WorldObject, and a materializable Item are all rejected. |
| `world_objects.py` | The D93 key rule, the D94 exclusions, the record builders, the validator (`--validate RECORD...`) and the census. `--check` diffs an in-memory regeneration against the committed sample. `--records DIR` also writes every record, for local inspection only. |
| `test_world_objects.py` | No-network tests. Routing goes through `engine_items.convert_item` over fabricated sources, using the real identity index. Run with `python test_world_objects.py`. |
| `samples/census-crystal-ff7ede5.json` | The committed census for the pinned Crystal revision. |

## Rules

- **Routing.** A record exists for every id where `engine_items.convert_item` reports `routed_non_item`, except for
  the D94 exclusions:
  - `WorldObject:appearance_placeholder_slot`;
  - `WorldObject:no_client_appearance`;
  - `Fluid:fluid_type_without_appearance`, which stays Item fluid.

  The route is never re-decided here.
- **Identity (D93, A12 §4.6).** `oteryn:item.tibia.i<id>` becomes `oteryn:terrain.tibia.i<id>` or
  `oteryn:world-object.tibia.i<id>`, keeping the same Tibia id. Nothing else is accepted.
- **Facts.**
  - A boolean `appearances.dat` flag is complete for an id that has an appearance, because an unset optional bool
    is false. It is therefore always KNOWN. An unset hook is `none`, and an unset height is elevation `0`.
  - An `items.xml` attribute is sparse. When it is absent the fact is UNKNOWN, never an engine default.
  - Without an appearance, every appearance fact is UNKNOWN.
  - Every KNOWN fact names its source field in `provenance.fields`.
- **Relations.** `rotateto`, bed parts, sleepers and transforms stay `{"source_item_id": n}` until WO-2 resolves them
  to typed references. Source numbers are provenance, never identity.
- **Corpses.** Decay target, duration and container capacity stay on the routed Item record (WO-0 §4.3). A
  WorldObject corpse only records the corpse and player-corpse flags.
- **Kind.** The first matching rule decides the kind. When no rule matches, the kind is UNKNOWN; it is never guessed.
  Pinned Crystal has 330 Terrain tiles (roofs and floors without a ground or border flag) and 21 WorldObject magic
  fields with UNKNOWN kind. The census lists them under `unknown_kind_examples` and `type_outside_family` as the
  decision's "contested routes".

## Pinned Crystal census (`ff7ede5`)

- 25,846 routed ids, of which 4,476 are excluded, leaving **21,370 records**: 8,581 Terrain and 12,789 WorldObject.
  This matches D94's "about 21.4k".
- Terrain: 3,779 border, 2,197 ground, 2,171 wall, 104 field, 330 UNKNOWN.
- WorldObject: 5,653 object, 3,358 corpse, 2,771 decoration, 728 door, 192 bed, 49 teleport, 17 ladder, 21 UNKNOWN.

```sh
python world_objects.py --source <crystalserver checkout at ff7ede5>          # write the sample
python world_objects.py --source <checkout> --check                          # drift check
python world_objects.py --validate record.json                                # validate records
python test_world_objects.py
```
