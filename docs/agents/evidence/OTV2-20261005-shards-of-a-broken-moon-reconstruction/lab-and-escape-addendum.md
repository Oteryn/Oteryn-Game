# Laboratory and post-Magnolia escape addendum

Status: **EVIDENCE / PARTIAL SEMANTICS ONLY**

This addendum closes what can be closed from current public quest references and the pinned Crystal Summer donor without inventing missing puzzle mechanics.

## Laboratory: exact known facts

Current quest reference places the laboratory sequence at:

- laboratory room entry: approximately `33881,32687,9`;
- Note Pinned on the Wall: reference guide places the note immediately at the laboratory entrance, with the detailed route using `33881,32688,9`;
- sealed door: `33874,32686,9`;
- Crystal Apparatus: `33874,32677,9`.

The exact Summer map pin already contains these route cells.

### The note is not a new Shards quest Item

“Note Pinned on the Wall” is an existing generic readable immovable object class used elsewhere in Tibia. The current authored recipe’s label `Laboratory note` should therefore not mint a new quest-item identity by name.

The missing Shards binding is:

```text
exact placed readable object
+ exact laboratory text/document
+ inspect/read interaction
+ quest transition
```

If the current canonical WorldObject/Document catalogues do not already bind that exact placement, the implementation should bind the placement to the existing generic readable-object identity and the Shards-specific document text rather than creating a duplicate portable Item.

## Crystal Plinth puzzle

The current quest reference explicitly states:

- the note explains that access to the lunar-fragment chamber depends on a **specific crystal-colour sequence**;
- that sequence **changes**;
- using the Crystal Plinth reveals the currently active colour order;
- the player then passes the sealed door and proceeds to the Crystal Apparatus.

Known canonical object:

- Crystal Plinth: `oteryn:item.tibia.i54515` routed to WorldObject.

Pinned Crystal Summer and current Canary source search show no quest script for:

- reading the changing sequence;
- selecting/inputting crystal colours;
- validating the sequence;
- failure/reset semantics;
- opening the sealed door from that puzzle.

Therefore the following details remain **UNKNOWN** and must not be invented:

1. where the active sequence is stored;
2. whether the sequence is per-character, per-channel, per-room or global;
3. whether it is random, cyclic or derived from world state;
4. which exact placed objects accept colour input;
5. how many inputs are required;
6. what happens on a wrong input;
7. whether using the Plinth itself is sufficient to authorize the sealed door, or whether the player must actively reproduce the shown sequence.

The current 16-stage recipe’s `s8 kind=explore count=3` is therefore insufficient for Reference-parity activation.

### Required implementation shape after observation closes the UNKNOWNs

```text
read laboratory note
  -> lab_note_read

use Crystal Plinth
  -> reveal current sequence (presentation-only observation)
  -> plinth_sequence_observed

[UNKNOWN exact input mechanism]
  -> lab_sequence_solved

gate sealed door on lab_sequence_solved
  -> laboratory_inner_chamber_entered

use Crystal Apparatus
  -> crystal_apparatus_inspected
  -> laboratory_done
```

Do not encode a guessed colour-input mechanism from the phrase “sequence” alone.

## Post-Magnolia escape: source consensus

After the permanent Magnolia kill, the character is moved into a separate/collapsed chamber and must improvise an exit.

Current TibiaWiki reference gives the following durable semantic order:

1. examine the skeleton -> obtain **Lit Torch**;
2. examine the icicles -> obtain **Icicle Chisel**;
3. use the lit torch and icicle chisel on the **north wall** / crumbling passage;
4. the wall opens enough to squeeze through and collapses behind the character;
5. follow the corridor to a **Rope Spot** and leave.

It records the resulting message after the chisel breaks the remaining wall:

> You use the icicle to knock down the rest of the crumbling wall.

This quote is kept short only to identify the observable transition.

A second independent walkthrough guide agrees on the functional route and describes:

- icicles in the south-west portion of the room;
- a nearby skeleton containing/providing the torch;
- torch on the northern wall, then chisel on the affected wall section.

Some secondary prose describes “use one on the other and then on the wall”. That wording is not strong enough to promote an item-on-item combination transaction.

### Safe native semantics

The implementation can safely model the source consensus as:

```text
inspect exact skeleton placement
  -> Item/Durability grant Lit Torch
  -> prison_torch_obtained

inspect exact icicle placement
  -> Item/Durability grant Icicle Chisel i39578
  -> prison_chisel_obtained

use Lit Torch on exact north-wall target
  [both acquisition facts]
  -> wall_heated/prepared

use Icicle Chisel on the prepared wall
  -> wall opening state
  -> prison_wall_open

step/cross exact opening
  -> prison_escaped_from_chamber
  -> collapse/reclose behind player

reach/use Rope Spot
  -> relocate to exterior route
  -> prison_escaped
```

This ordering matches the strongest current guide description while remaining compatible with the supplied video’s visible escape sequence.

### What remains placement-UNKNOWN

Until the exact video frames or current map placement evidence are read again, do not invent:

- skeleton coordinate;
- icicle coordinate;
- north-wall placement key/item id;
- Rope Spot coordinate;
- whether the wall change is a LocalObject state transition, an overlay transform, or a relocation trigger;
- exact Lit Torch source appearance if the room uses a presentation alias.

The item identities alone are not enough to bind the world interaction.

## Donor status

Pinned Crystal Summer does not provide a Shards quest script for the laboratory puzzle or the escape sequence.

Therefore neither mechanic may be described as “copied from Crystal”. Crystal supplies map/object/item evidence; Oteryn still needs native Interaction/Quest producers and exact placement bindings.

## Runtime ownership

- note/plinth/sealed door: WorldObject + Document + QUEST-TRIGGER-1 / QUEST-GATE-1;
- any colour-sequence state: requires architect/owner decision after observation establishes the real semantics;
- skeleton/icicles/wall/Rope Spot: WorldObject/Terrain + Item/Durability + QUEST-TRIGGER-1;
- wall overlay/local-state change: existing World Interaction owner;
- final progress: existing `QuestTransitionRequest` writer.

No direct QuestState mutation should be implemented inside an object handler.

## Re-verification still required

When the supplied video can be inspected again at full relevant frames, verify:

1. whether the player inputs a colour sequence or merely uses the Plinth and passes;
2. exact laboratory interactive placements;
3. exact skeleton/icicle/wall/Rope Spot positions;
4. exact order of Torch vs Chisel use;
5. whether any tool is consumed/transformed.

Until then those details remain explicitly UNKNOWN.
