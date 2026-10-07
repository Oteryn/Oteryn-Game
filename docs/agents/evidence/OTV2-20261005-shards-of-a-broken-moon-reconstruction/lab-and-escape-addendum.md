# Laboratory and post-Magnolia escape addendum

Status: **EVIDENCE; DONOR COORDINATES RESOLVED; ACTIVE WORLD BINDINGS PENDING**

Walkthrough inspected visually from the actual video:
`https://www.youtube.com/watch?v=Zz-7UyufXbU`

The frame readback below supersedes the earlier assumption that the laboratory requires a separate manual colour-input puzzle.

## Laboratory: source + video result

Current quest reference places the laboratory sequence at:

- laboratory room entry: approximately `33881,32687,9`;
- Note Pinned on the Wall: exact donor 6121 at `33881,32684,9` (older guide route coordinate is not the object placement);
- sealed door: `33874,32686,9`;
- Crystal Apparatus: `33874,32677,9`.

The exact Summer map pin contains these route cells.

### Laboratory note

“Note Pinned on the Wall” is an existing generic readable immovable object class, not a new portable Shards quest Item.

The missing native binding remains:

```text
exact placed readable object
+ exact Shards laboratory document text
+ inspect/read interaction
+ optional lore observation (not required for quest progress)
```

Do not mint a new portable `Laboratory note` Item by name.

### Crystal Plinth — manual colour input is NOT part of the observed player path

Known canonical object:

- Crystal Plinth: `oteryn:item.tibia.i54515` routed to WorldObject.

The supplied walkthrough was inspected frame-by-frame in the original-video interval `11:17–11:42`, with a 720p readback around the Plinth.

Observed sequence:

1. the character enters the laboratory;
2. an incorrect/irrelevant object use visibly refuses with `You cannot use this object.`;
3. the character uses the Crystal Plinth;
4. at approximately `11:25`, the game reports:
   > From the colours and arrangement of the crystal panels on the plinth you learn the access sequence for the shard room.
5. the Quest Tracker updates to state that the player learned which colour code opens the shard-room doors;
6. dense `0.5 s` frame inspection from `11:32` through `11:42` shows **no separate code UI, no manual colour entry and no panel-click sequence**;
7. the character simply leaves the Plinth area and crosses the sealed-door boundary at about `11:40–11:41`.

Therefore the executable Reference-facing behavior is:

```text
use Crystal Plinth
  -> lab_access_sequence_learned

sealed shard-room door
  requires lab_access_sequence_learned
  -> pass/open
```

The visible colour arrangement may vary as presentation, but the supplied Reference walkthrough does not expose a player-authored sequence value that needs durable Quest storage.

### Consequence for the previous UNKNOWN

The previous blocker:

```text
Laboratory colour-sequence input/storage/reset semantics UNKNOWN
```

is **CLOSED for executable Shards parity**.

Do not invent:

- a random/cyclic persisted colour sequence;
- a per-character sequence value;
- manual coloured-panel input;
- failure/reset logic for a non-observed input mechanism.

If a later stronger source proves server-internal colour generation that materially affects a client-visible result, it can be added under its owning World/Interaction contract. It is not required to reproduce the observed quest path.

### Final laboratory shape

```text
read laboratory note
  -> lab_note_read

use Crystal Plinth
  -> lab_access_sequence_learned

gate exact sealed door on lab_access_sequence_learned
  -> laboratory_inner_chamber_entered

use/inspect Crystal Apparatus
  -> crystal_apparatus_inspected
  -> laboratory_done
```

The donor object coordinate is exact; native placement_key and Document admission remain World/Document-owned. The note is optional lore and must not gate `laboratory_done`.

## Post-Magnolia escape: video-verified order

The supplied walkthrough was also inspected visually across `22:10–23:10`.

Observed timeline:

- `22:10–22:17`: final Magnolia combat;
- approximately `22:21`: relocation into the white/icy prison chamber;
- `22:22–22:39`: object inspections/interactions inside the chamber;
- approximately `22:40`: the character breaks one of the massive icicles; the client reports `With all your strength, you manage to break off one of the massive icicles.`;
- approximately `22:47.5`: the client reports `You have found a torch.`;
- approximately `22:48.0`: the player first tries the icicle/chisel on the north-wall crack **before preparing it**; the server rejects the attempt and explains that the crack must be widened by another method;
- approximately `22:48.5`: the player uses the torch on the crack; a clear fire effect appears on the wall;
- approximately `22:50.5`: the player uses the icicle/chisel again; an ice/crumbling effect appears and the wall opens;
- approximately `22:51–22:52`: the character moves through the newly opened passage;
- `22:53–22:58`: the character follows the narrow escape corridor;
- approximately `22:59`: relocation out of the prison route;
- `23:05–23:10`: completion dialogue begins and the quest-completion UI appears.

This visual order agrees with the current reference guide, while also proving an observable failure branch:

1. inspect icicles -> obtain **Icicle Chisel**;
2. inspect the skeleton/torch source -> obtain **Lit Torch**;
3. attempting the Icicle Chisel on the unprepared crack is rejected and does not open the wall;
4. use Lit Torch on the north-wall crack -> wall becomes prepared;
5. use Icicle Chisel on the prepared wall -> wall opens;
6. cross the opening/collapsing passage;
7. follow the corridor to the Rope Spot / exit.

The walkthrough happens to collect the chisel before the torch. The required ordering constraint is on the **wall actions** (torch before successful chisel), not on the order in which the two tools are collected.

Known Item evidence:

- Icicle Chisel: `oteryn:item.tibia.i39578`;
- pinned Crystal Summer item id `54610` is named **lit torch** and is resolved as `light_source` in Oteryn's donor census;
- the Oteryn donor key is currently `donor:crystalserver@00ce02a5:item/54610`; do not silently promote it to a canonical `oteryn:item.tibia.i54610` identity unless the Item owner admits that identity.

Lit Torch 54610 uses Item/Durability admission and ordinary light/use semantics. No timed/decay/54609 transform lifecycle is proved; do not inherit the older torch lifecycle.

### Safe native semantics

```text
inspect exact skeleton placement
  -> grant Lit Torch through Item/Durability owner
  -> prison_torch_obtained

inspect exact icicle placement
  -> grant Icicle Chisel i39578
  -> prison_chisel_obtained

use Lit Torch on exact north-wall target
  -> wall_heated/prepared

use Icicle Chisel on unprepared wall
  -> reject
  -> preserve quest/world state
  -> hint that the crack must first be widened

use Icicle Chisel on prepared wall
  -> wall opening state
  -> prison_wall_open

cross opening
  -> prison_escaped_from_chamber
  -> collapse/reclose behind character

use/reach Rope Spot
  -> relocation
  -> prison_escaped
```

### Exact donor positions and remaining native gaps

Fresh exact-map parse: `prison-source-readback-20261007.json`.

- skeleton 8606: `31920,31360,9`;
- icicles 6966: `31914,31364,9`;
- center north wall: `31920,31359,9`, actual ground 6869; stacked 4728, debris 6374, ice wall 6729 and overlays 6926/6928;
- left candidate carries debris 6378; right candidate has a different wall 6737. Retained video fire/crumbling lies on the skeleton's center axis;
- Rope Spot ground 386: `31923,31377,9`; hole 610 directly above at z=8; south-default generic destination `31923,31378,8` has ground 6684 and no items;
- Crystal and pinned Canary `Position:moveUpstairs` agree on decrementing z and preferring one tile south when walkable.

These close the static coordinate/stack questions. Active placement_key, dynamic walkability/occupancy, collapse/reclose representation and Item admission remain with World/Item owners. The generic rope destination is DERIVED, not a hosted movement test. No quest-private teleport is required.

### Qualified rope callback and completion boundary

`prison-source-readback-20261007.json` now pins the complete provider chain, including Git blob and raw-byte SHA-256 for both repositories. The common rope action registers tools 3003/646. Both pinned Global and custom pack handlers recognize ground 386 through `ropeSpots` and call `Position:moveUpstairs`. The Global variant uses `Tile:isRopeSpot`; the custom variant checks the ground table directly and additionally refuses relocation into a protection zone when the player is PZ-locked. The loaded pack matters; the custom guard is not a universal Global rule.

Neither successful rope-spot branch consumes the rope nor writes Shards progress. Both return true after calling teleport without checking its result. Consequently callback acceptance alone cannot set `prison_escaped`: the Oteryn adapter must consume the existing World-authoritative successful relocation occurrence and submit the existing `QuestTransitionRequest`. Do not copy the donor's unrelated ground-7762 tutorial storage handling into this quest.

The south-default destination is conditional. If that tile is unavailable or non-walkable, `moveUpstairs` searches other directions; static absence of stacked items does not prove current walkability. The request must reference the actual admitted relocation outcome, not an assumed south coordinate.

### Runtime witness protocol for the owner

Record exact content generation, placement key, tool custody, character, command reference, World relocation occurrence and Quest transition result for each case. These are proposed acceptance observations, not executed tests or new runtime contracts.

| Case | Required observation |
| --- | --- |
| Ground 386 with admitted rope, open route and valid destination | World reports the actual destination and successful relocation; only then can the existing transition writer record `prison_escaped`. |
| Destination refused or relocation fails | No escape fact, no quest progress and no consumed rope. A donor-style callback `true` is insufficient. |
| South-default cell unavailable | Capture the World-selected destination or refusal. Do not credit an assumed teleport to `31923,31378,8`. |
| Wrong object, wrong tool or unresolved active placement | Refuse before quest progress; preserve item custody and state. |
| Duplicate successful occurrence / retry after relog | Existing transition identity admits at most one escape fact for the same occurrence. No direct QuestState writes. |
| Rope used before the chamber opening is crossed | Confirm the physical route and accepted quest prerequisites; do not let a coordinate-only trigger bypass the escape micrograph. |
| Custom-pack PZ-locked reference case | Keep separate from Global source behavior; select an Oteryn rule only through the owning Movement/World contract. |

## Donor status

Pinned Crystal Summer does not provide a Shards quest script for either the laboratory access flow or the post-Magnolia escape.

Crystal supplies map/object/item evidence. Oteryn still needs native World/Interaction/Quest producer bindings.

## Runtime ownership

- laboratory note: WorldObject + Document + QUEST-TRIGGER-1;
- Crystal Plinth: WorldObject + QUEST-TRIGGER-1;
- sealed laboratory door: QUEST-GATE-1 + Door, gated by `lab_access_sequence_learned`;
- skeleton/icicles/wall/Rope Spot: WorldObject/Terrain + Item/Durability + QUEST-TRIGGER-1;
- wall state change: existing World Interaction owner;
- final progress: existing `QuestTransitionRequest` writer.

No direct QuestState mutation belongs in an object handler.

## Remaining verification target

Bind the resolved donor coordinates to the active served generation and qualify generic tool success -> QuestTransitionRequest, wrong-order refusal and collapse/reclose with relog/restart. Static map and retained video evidence do not prove those runtime effects.
