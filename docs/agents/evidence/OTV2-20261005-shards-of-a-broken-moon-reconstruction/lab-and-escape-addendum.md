# Laboratory and post-Magnolia escape addendum

Status: **EVIDENCE / VIDEO-VERIFIED SEMANTICS; EXACT PLACEMENT IDENTITIES STILL PARTIAL**

Walkthrough inspected visually from the actual video:
`https://www.youtube.com/watch?v=Zz-7UyufXbU`

The frame readback below supersedes the earlier assumption that the laboratory requires a separate manual colour-input puzzle.

## Laboratory: source + video result

Current quest reference places the laboratory sequence at:

- laboratory room entry: approximately `33881,32687,9`;
- Note Pinned on the Wall: detailed route `33881,32688,9`;
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
+ quest transition
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

The exact canonical placement/document binding of the laboratory note remains open.

## Post-Magnolia escape: video-verified order

The supplied walkthrough was also inspected visually across `22:10–23:10`.

Observed timeline:

- `22:10–22:17`: final Magnolia combat;
- approximately `22:21`: relocation into the white/icy prison chamber;
- `22:22–22:45`: object inspections/interactions inside the chamber;
- approximately `22:48`: a clear flame/fire effect appears on the north wall;
- immediately afterward: a second tool interaction occurs on the prepared wall;
- approximately `22:52`: the character moves through the newly opened passage;
- `22:53–22:58`: the character follows the narrow escape corridor;
- approximately `22:59`: relocation out of the prison route;
- `23:05–23:10`: completion dialogue begins and the quest-completion UI appears.

This visual order agrees with the current reference guide:

1. inspect skeleton -> obtain **Lit Torch**;
2. inspect icicles -> obtain **Icicle Chisel**;
3. use Lit Torch on the north wall;
4. use Icicle Chisel on the prepared wall;
5. cross the opening/collapsing passage;
6. follow the corridor to the Rope Spot / exit.

Known canonical Item:

- Icicle Chisel: `oteryn:item.tibia.i39578`.

The Lit Torch must use the existing Timed Item / Item owner semantics; do not create a quest-private torch implementation.

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

### What is still UNKNOWN

The broad previous blocker “escape sequence not re-observed” is now obsolete. The gameplay order is visually verified.

Only these placement/identity details remain open:

- exact skeleton coordinate / placement key;
- exact icicle coordinate / placement key;
- exact north-wall placement key / appearance id;
- exact Rope Spot coordinate / placement key;
- exact Lit Torch source appearance/alias used by the prison;
- whether the collapsing wall is represented by an existing LocalObject state, overlay transform or another accepted World Interaction representation.

Do not invent these coordinates or IDs from the video viewport alone.

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

Use exact map/source placement evidence to bind the prison skeleton, icicles, north wall and Rope Spot. The interaction order itself no longer needs re-observation.
