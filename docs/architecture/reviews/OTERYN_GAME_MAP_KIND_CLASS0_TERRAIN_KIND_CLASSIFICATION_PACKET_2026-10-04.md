# MAP-KIND-CLASS-0: classification of the 50 Terrain records with UNKNOWN kind

Status: architect ruling and implementation packet, accepted when this merges.
Date: 2026-10-04. Base: `main` `42f0a0f8`.
Owner of the ruling: Sol Supervising Architect. Owner of the follow-up: the content lane, through
the control plane.

## 1. Problem

MAP-BUNDLE-2 (#1751) found 50 palette records routed to Terrain whose `kind` is UNKNOWN.
`tools/world-bundle-compiler/src/resolve.rs` (`terrain_of`) stops on an UNKNOWN kind, so the real
compile stops until they are classified. MAP-CUTOVER-1 cannot be allocated before that.

All 50 records share the same facts:

- Route `{owner: Terrain, reason: primarytype_world_object}`. The items.xml `primarytype` is
  `artificial tiles` or `natural tiles`, which `WORLD_OBJECT_PRIMARYTYPES`
  (`tools/content-schema/item-authoring/engine_items.py`) sends to Terrain.
- Not one of them carries `flags.bank`, `flags.fullbank`, `flags.clip` or `flags.bottom` in the
  pinned appearances (`content/assets/files/appearances-2dfa943b….dat`, read with
  `engine_items.load_appearance_objects`). None has a `magicfield` type or a name containing
  "roof".
- So no `terrain_kind` rule in `tools/content-schema/world-object-authoring/world_objects.py`
  applies, and `kind` stays UNKNOWN. That is correct: an unknown fact is never defaulted (WO-0 §4.3).

65 Terrain records in total have UNKNOWN kind. 50 of them are in the palette.

## 2. Evidence: these are not ground, border, wall, field or roof

- **Ground.** WO-0 §4.2 takes `ground_speed` from the bank waypoints, and format v2
  (`docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md` §12) needs a speed for every ground, at least
  1 when it is walkable. None of the 50 has a bank, so none has a speed. 29 of them are also
  `unpass`, which a ground does not carry.
- **Border, wall.** These need `clip` and `bottom`, and none of the 50 has either.
- **Field.** None has a field type, and none has the `fields` primarytype.
- **Roof.** None has "roof" in its name, so the WO-2c 1a rule does not apply.

In the client these appearances have no bank and no clip, bottom or top flag. They are drawn in
the common item layer of a tile, above the ground and its borders. They are tile overlays, such as
wooden floor planks, mosaics, leaves or sand drifts. They are not the tile's ground. They are
`unmove` world props, which WO-0 §4.3 gives to WorldObject.

## 3. Ruling

**R1.** An id whose route is Terrain by `primarytype_world_object`, and for which `terrain_kind`
returns `None`, routes to WorldObject instead. Its reason is `tile_primarytype_overlay`, and its
WorldObject kind comes from the existing `world_object_kind` rule:

- `type teleport` gives `teleport`;
- otherwise `flags.unpass` gives `object`;
- otherwise the kind is `decoration`.

The rule covers all 65 such records, not only the 50 in the palette. It leaves every Terrain record
that already has a kind unchanged.

**R2.** Placement palette keys stay unchanged. The palette uses Item keys
(`oteryn:item.tibia.iNNN`), and each one resolves through its routed pointer, so a reroute changes
the pointer target and not the placements.

**R3.** The bundle gets `terrain: null` for these ids (format §12, WorldObject route). Format v2
and the compiler are unchanged.

**R4.** Ids 628 and 878 carry `type teleport` and become WorldObject `teleport`. Their destination
and behaviour stay with Interaction and `Transition.Teleport`. This packet does not decide them.

### 3.1 Per-record classification

| Source ids | Name | Appearance flags | WorldObject kind |
|---|---|---|---|
| 571, 572 | wooden floor | unmove, automap 129 | decoration |
| 589, 590, 591, 592 | wooden floor | unpass, unmove | object |
| 628, 878 | wooden floor (`type teleport`) | unmove | teleport |
| 12651 | branches | unmove | decoration |
| 18400–18405 | sand | unpass, unmove, unsight | object |
| 18566–18577 | teal leaves | unpass, unmove | object |
| 20652 | parquet floor | unmove, light 4/138 | decoration |
| 20727 | parquet floor | unpass, unmove, automap 186 | object |
| 30655–30660, 30670–30673 | mosaic | unmove | decoration |
| 31298, 31311 | black marble floor | unpass, unmove | object |
| 31299, 31313 | stone tile | unpass, unmove | object |
| 31300, 31312 | white marble floor | unpass, unmove | object |
| 31317, 31319 | stone floor | unmove | decoration |
| 36081–36083 | mosaic | unmove | decoration |

The totals are 19 decoration, 29 object and 2 teleport, which makes 50. The `collision`,
`blocks_sight`, `automap` and `client_projection` facts carry over unchanged from the records'
current provenance. No record has an explicit hold.

## 4. Follow-up packet: MAP-KIND-CLASS-1

- Mode: ordinary implementation (`oteryn-impl-worker`), content lane.
- Owned paths:
  - `tools/content-schema/item-authoring/engine_items.py` and its tests;
  - `tools/content-schema/world-object-authoring/` and its tests;
  - regenerated `content/world/terrain/` and `content/world/objects/` (regenerate with the
    repository tools, never by hand);
  - the routed item pointers;
  - the task record.
- Change: implement R1 in the route, with a test for each of the three kinds and a test that
  every record already carrying a Terrain kind keeps its route.
- Acceptance:
  1. The real compile reports zero Terrain records with UNKNOWN kind.
  2. Palette parity moves from WorldObject-routed 7719 to 7769, and every Terrain kind count
     (ground 2016, border 3453, wall 1819, roof 198, field 102) is unchanged.
  3. Item keys and palette keys are unchanged.
- Stop condition: if a placement of one of these ids is the first item of its tile (no ground
  below it), the worker stops and reports that id instead of rerouting it. That id then gets an
  explicit hold, and this ruling is amended.
- Review: no protocol, persistence or authority change, so ordinary review applies.
- Order: MAP-KIND-CLASS-1 comes before MAP-CUTOVER-1.

## 5. Rejected options

- **Ground with an assumed speed.** This defaults an unknown fact, which WO-0 forbids, and it makes
  an `unpass` tile a ground.
- **A new Terrain kind `overlay`.** This needs a format §12 and compiler change for facts that
  WorldObject `decoration` and `object` already express.
- **Hold all 50.** This keeps the real compile blocked when the evidence is conclusive.
- **Per-id manual mapping.** A per-id table drifts. The flag rule is reproducible and covers the
  15 records outside the palette too.

## 6. Decision test

1. **Must this be decided now?** Yes. The real compile stops on these records, and MAP-CUTOVER-1
   waits on that.
2. **What work is blocked?** MAP-CUTOVER-1, and the real compile with a parity run.
3. **Is it harder later?** Yes. Once placements are cut over, a route change means regenerating
   and re-verifying a live bundle.
4. **What evidence would supersede this?** A Reference proof that one of these ids is the
   ground of its tile, or a placement that triggers the stop condition in §4.
5. **What is not decided?** Teleport destinations and behaviour (Interaction), light behaviour,
   and any change to format v2.
