# WorldProject v2 native entry qualification — Amendment 02: entry-room revision 2

- Decision: `NATIVE_ENTRY_SOURCE_QUALIFICATION_V1`, amending Amendment 01
- Date: 2026-10-04
- Tracking: SPAWN-1A-PACKET-1, #1745 P1 4177087019 and P1 4177181274.
- Normative status: accepted when #1745 merges. Implemented by SPAWN-1a
  (`reviews/OTERYN_GAME_SPAWN1A_FIXTURE_SPAWN_ROOM_R2_DECISION_2026-10-04.md` §2.1). It applies to the
  revision-2 room. Until SPAWN-1a merges, Amendment 01 describes the running code and the revision-1 room.
- Authority: unchanged; production/live authority remains NONE.

Revision 2 of the native entry room (SPAWN-1A-PACKET-1 §1.1) adds the two D116 den cells and one spawn of
two rats. Amendment 01 §3 and §4 fixed three room cells and a singular spawn cell. In revision 2 they read
as follows; every other rule of Amendment 01 stands.

## 1. Overlay (§3)

| Field | Revision-2 contents |
| --- | --- |
| cells | Exactly five (placement_key, region_key, collision) records: start, east, north, `entry-den`, `entry-den-north` |
| spawn | key, creature exact typed reference, behavior exact typed reference, `cell_keys`, population_limit, recovery, multiplicity, eligibility_scope, `respawn_delay_ms`, `occupancy_retry_interval_ms` |

`cell_keys` is an ordered, non-empty, duplicate-free list. Its length equals population_limit and is at
most 2 (FirstProduction Amendment 04). Each entry names a Walkable overlay room cell that is not a proof
cell (start, east, north, the door). The singular `cell_key` field is refused in revision 2, and
`cell_keys` is refused in revision 1. The two spawn inputs are bounded as SPAWN-1A-PACKET-1 §1.1 states.

**Authoring overlays.** Amendment 01 refuses every authoring overlay. Revision 2 admits exactly two,
the hostile rat's (SPAWN-1A-PACKET-1 §1.6):

- one Behavior profile targeting `oteryn:behavior/rat-hostile`;
- one Creature profile targeting `oteryn:creature/rat`.

Both target `oteryn:rev/entry-r2`. Any other authoring overlay, and any authoring overlay in
revision 1, is refused. The required fields and their checks are those of SPAWN-1A-PACKET-1 §1.6.

## 2. Lowering (§4)

- The consumer selects exactly six Terrain placements: five map bijectively to the five room cells and
  one to the door cell.
- The added cells are `entry-den` (2,0,0) and `entry-den-north` (2,-1,0), both Walkable.
- The envelope becomes [0,3) × [-1,1), the World bounds (0, -1, 3, 1). It fits the six placed cells
  exactly.
- `cell_keys` lowers in order to `FirstProductionSpawn.cell_keys` and to the spawn-cell records of
  FirstProduction Amendment 04.
- The two authoring profiles are not lowered into the FirstProduction artifact. The qualified spawn
  carries the creature's `health`, `initial_health` and `speed`, and the behaviour profile, to the
  activated spawn source.

A missing, extra or duplicate placement refuses, as before. So does a `cell_keys` entry outside the five
room cells, and a mix of revision-1 and revision-2 shapes.
