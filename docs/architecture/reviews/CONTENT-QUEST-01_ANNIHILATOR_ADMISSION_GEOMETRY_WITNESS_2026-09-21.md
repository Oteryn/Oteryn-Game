# CONTENT-QUEST-01 r11 — Annihilator admission geometry witness

Status: **OTS_HYPOTHESIS_ONLY / PHYSICAL_ADMISSION_SOURCE_BINDING / PROPOSED_NONCANONICAL**
Date: 2026-09-21
Tracking: #707 / PR #709
Evidence head: `47482a0fca9bf3d3b7178fa8db222dd95e8aaaa2`
Workflow run: `35588149012`
Workflow job: `106296213683`

## Purpose

Close the physical staging/destination footprint for the selected Crystal Annihilator admission source and distinguish its rectangular spectator query from a canonical Oteryn EncounterZone.

## Exact evidence inputs

- Crystal: `zimbadev/crystalserver@ac447fef0935e6df52dc6b6376ae4c1534ecd73f`;
- `world.otbm` Git blob: `e95e8f7c7a95d1b634b49a5dea5a5dc76021406b`;
- map SHA-256: `09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb`;
- existing parser: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`;
- selected lever source blob: `62e92e8acd676bad545a501c8431c983dcd4edfa`;
- shared Lua helper blob containing `roomIsOccupied`: `6aeb1fdb7891a45b3bb344b3728ebb0268c91e52`;
- `Game.getSpectators` Lua binding blob: `9430fdfa35730da9a7c5a82c894864aeafd5595f`;
- spectator search implementation blob: `3d155a34f3c450bdaeab46a9f374e603bf60e696`.

The one-shot evidence job verified exact source/map/parser identities and streamed the map with the existing strict parser. Result: **SUCCESS**, `problems=[]`.

## Exact authored admission mapping

The selected lever source declares this ordered source mapping:

| Slot | Staging source coordinate | Destination source coordinate |
|---:|---|---|
| 1 | `(33225,31671,13)` | `(33222,31659,13)` |
| 2 | `(33224,31671,13)` | `(33221,31659,13)` |
| 3 | `(33223,31671,13)` | `(33220,31659,13)` |
| 4 | `(33222,31671,13)` | `(33219,31659,13)` |

Strict map readback proves all eight coordinates exist as static tile records.

### Staging tiles

All four staging records have:

```text
ground_server_id = 10145
flags            = 0
house_id         = null
top_level_items  = []
```

### Destination tiles

All four destination records have:

```text
ground_server_id = 10145
flags            = 8
house_id         = null
top_level_items  = []
```

The adjacent lever record at `(33226,31671,13)` is independently confirmed as ground `410`, item `2772`, flags `0`.

No semantic meaning is inferred here from numeric ground or tile-flag values. The important source fact is that staging and destination records are physically distinct even though they share ground server ID `10145`.

## `roomIsOccupied` is a spectator query, not a room definition

The selected lever calls:

`roomIsOccupied(centerDemonRoomPosition, true, 4, 4)`

with center `(33221,31659,13)`.

The shared helper implements this as:

`Game.getSpectators(center, false, onlyPlayers, rangeX, rangeX, rangeY, rangeY)`

and returns occupied when at least one spectator is returned.

The exact Lua/C++ bindings prove:

- `multifloor = false`;
- `onlyPlayers = true`;
- range arguments `4,4,4,4` become `-4..+4` in X and Y;
- the search is coordinate-range based, not path/reachability/collision-room membership.

Thus the source occupancy query footprint is a 9x9 single-floor rectangle (81 candidate coordinate positions).

## Exact static-map footprint of that query rectangle

The strict r11 parse found **81/81 tile records**, no missing positions.

Ground source-ID distribution:

```text
410   -> 17
728   -> 1
729   -> 1
730   -> 2
4427  -> 47
10145 -> 6
21477 -> 7
```

Tile flag distribution:

```text
0 -> 60
1 -> 1
8 -> 19
9 -> 1
```

The same rectangle also contains many non-ground static items, including source IDs `1270..1276`, `2113` and `5107` on various positions.

This heterogeneity is decisive: the `±4` spectator rectangle is a source query window over map space, not evidence that every coordinate belongs to one semantically uniform arena/EncounterZone.

## Source admission ordering and its disposition

The selected Crystal lever performs, conceptually:

1. read top creature on each of four staging tiles;
2. require a creature and call `getLevel()`;
3. test room occupancy;
4. create six monsters;
5. iterate staging tiles again, check `isPlayer()` during the teleport loop and teleport sequentially;
6. transform the lever.

Repository-wide/source readback also shows no Party check in this lever.

Notably, `getLevel()` is a Player API but the source first obtains generic top creatures and does not call `isPlayer()` before the monster-spawn step. This is a source implementation characteristic/hazard, not a native Oteryn admission contract.

Oteryn MUST NOT port this mutation ordering literally. Existing E04/FND/Instance boundaries remain authoritative:

- resolve an actual typed Player roster first;
- validate the whole fixed group and all admission predicates before accepted side effects;
- Party membership is a separate owner/policy and is not inferred merely because four people are admitted together;
- reserve/accept the admission as one bounded operation;
- foreign spawn/teleport proposals reconcile by owner result;
- fixed-group transfer defaults all-or-nothing unless an explicit accepted policy says otherwise;
- no partial teleport/spawn sequence is treated as successful Encounter admission.

## Exit source binding composed with r7

r7 proved the static exit item `1949` at `(33236,31655,13)`.

Additional exact source readback now records:

- `TeleportUnique[38017]` in `data-global/startup/tables/teleport.lua@2d798951e1d46b7198dacad57d792971d0045899`;
- source destination `(33213,31671,13)`;
- effect `CONST_ME_TELEPORT`;
- generic movement handler `data-global/scripts/movements/others/teleport.lua@c430e446311c87fbf3750f923a33970f9204e2ab` registers UIDs `38001..40000`, accepts Player creatures and teleports to the configured destination.

This is a qualified Crystal source return-anchor candidate. The physical destination tile `(33213,31671,13)` was not part of the r11 strict target set and is therefore not claimed physically proven by this witness. It also does not become Oteryn recovery fallback or Global target truth.

## Importer / Encounter consequence

Source lowering must keep separate:

```text
staging-slot identity/order
admitted roster
source destination slot/order
spectator occupancy query footprint
semantic EncounterZone / reservation scope
source exit destination
native safe-return/recovery policy
```

None may be silently derived from another.

Source coordinates remain importer provenance and migration inputs. Oteryn authored spatial identities/regions remain under Content/world/runtime owners and Reference geometry still requires accepted evidence.

## New required tests

- **T81 — exact staging/destination source binding:** every authored staging and destination coordinate required by an imported fixed-group admission must resolve to the expected qualified map source; missing/mismatched source geometry fails linking rather than synthesizing a seat/landing point.
- **T82 — spectator rectangle is not EncounterZone:** a legacy occupancy query window, even when every tile exists, cannot become a canonical semantic room/EncounterZone or collision boundary without explicit authored/evidence-backed lowering; heterogeneous tile/item state must be preserved.

Existing T46 and E04 already cover group admission/handoff atomicity, so r11 does not mint a redundant third test for the source's late `isPlayer()`/sequential side-effect ordering.

Required CONTENT-QUEST-01 corpus is now **82 cases**.

## Evidence classification

```yaml
annihilator_admission_geometry:
  source_classification: OTS_HYPOTHESIS_ONLY
  staging_tiles: PROVEN_4_OF_4_FOR_PINNED_MAP_BYTES
  destination_tiles: PROVEN_4_OF_4_FOR_PINNED_MAP_BYTES
  room_query_rectangle_tiles: PROVEN_81_OF_81_FOR_PINNED_MAP_BYTES
  room_query_semantic_encounter_zone: REJECTED_INFERENCE
  source_party_requirement: NOT_FOUND_IN_SELECTED_LEVER
  source_admission_ordering: OBSERVED_NONATOMIC_SOURCE_SEQUENCE
  source_exit_destination: SOURCE_CONFIGURED
  source_exit_destination_physical_tile: NOT_CHECKED_IN_R11
  global_target_admission_geometry: EVIDENCE_REQUIRED
```

This r11 witness composes with r1-r10 and adds no new runtime owner or production authority.
