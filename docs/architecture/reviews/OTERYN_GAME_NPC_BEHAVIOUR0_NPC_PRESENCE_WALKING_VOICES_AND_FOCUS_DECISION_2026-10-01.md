# NPC-BEHAVIOUR-0 NPC presence, walking, voices and focus

- Decision: `NPC-BEHAVIOUR0-PRESENCE-WALKING-VOICES-AND-FOCUS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  movement and determinism) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the base-mechanics close-out plan (#162 5929069698, item 3). NPC-0 decided talk, trade
  and travel but not "NPC movement schedules"; CREATURE-AI-0 left "NPC movement"; the gap register
  §11 keeps "NPC behaviour stays unresolved". No decision says how an NPC exists in a channel, shows
  on the wire, walks, speaks its voices or faces its customer.
- Builds on: SIM-DETERMINISM-01 §10 and §12, ATTACK-0 §3, VSL-MOVE-01 §5 and §10, the NPC service
  boundary (§3: "NPC actor-local behavior -> GAME-AI role"), NPC-0 (§3.2
  placements, §4 conversation and talk range), the NPC authoring schema (movement, voices,
  placements) and its owner decisions D9 and D11, MOVE-RL-11 D85-D87 (visible actors), CREATURE-AI-0
  §4.1, §5.1 and §7 (perception, step timer, budgets), CONDITIONS-0 §4.2 (step duration), CHAT-0 §3
  (local speech), WORLD-INTERACTION-0 §7.1 (NPCs cannot be pushed), owner rule 5905825574.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| NPC-ACTOR-1 | impl, determinism review | NPC runtime actors per channel from the bundle placements (§3); the NPC think (§4); walking (§5); the turn toward the queue head (§7) | NPC-PLACE-1; CREATURE-MOVE-1; SPEED-1; NPC-WIRE-1 |
| NPC-TALK-2 | impl | the customer queue, the range check and the walk-away close in GAME-NPC-SERVICE (§7; the NPC-0 §4.1 amendment) | NPC-TALK-1 |
| NPC-VIS-1 | impl, protocol review | entity kind 5 `Npc` in `world_spatial_v1.proto` `EntityKind` (capability 6) before it is offered; VIS-3 depends on it (§3.2) | VIS-2 |
| NPC-VOICE-1 | impl | voices as local speech (§6) | CHAT-1; NPC-ACTOR-1 |
| NPC-CONTENT-2 | content lane | `walk_interval`, `walk_radius`, `floor_change`, base speed, voice cadence, chance and lines on NPC definitions, from the admitted sources (D9) | NPC-CONTENT-1 |

Tests: an NPC appears at its placement on every channel at start; walks only inside its radius and
never while talking; never walks onto a floor change or teleport; faces its newest customer; ends a
conversation when the customer leaves talk range; says a voice line only while a player perceives
it; the same seed replays the same walk and voices.

Later, each with its own decision: NPC schedules (day and night positions; none in the Reference
base), NPCs in instances, NPC sounds, scripted NPC behaviour (`script` keyword kind stays held).

## 1. Question

How does an NPC exist in a channel, how do players see it, and what does it do between
conversations?

## 2. Facts

**PROVEN**

- NPC service boundary §3: inside the current runtime owner, "NPC actor-local behavior -> GAME-AI
  role"; "NPC dialogue/service -> GAME-NPC-SERVICE role"; the runtime owner is the only writer.
- NPC-0 §3.2: NPC positions come from Canary/Crystal spawn data, compiled into the World Bundle; §4:
  `NPC_TALK_INTENT {npc_actor, text}`, a conversation per (NPC, character), closed on reconnect or
  transfer, `NPC0-RL-05` open conversations per NPC; CHAT-0 §3: a greeting `say` within the NPC's
  talk range (`CHAT0-RL-08`, 4 tiles) starts a conversation.
- NPC authoring schema §2: `movement (walk interval/radius, floor change)`, `voices (cadence + lines
  (text ref, yell))`, `placements (absolute position, direction, ...)`; D9 admits Tibia NPC text,
  voices included, as reference data; D11 holds NPCs removed from Global.
- MOVE-RL-11 §4.2 (D85): visible actors are "creatures and other players" with identity, kind,
  position, direction, appearance and health; `crates/protocol-oteryn/src/world_spatial_entities.rs`
  has kinds Player 1, Creature 2, Corpse 3, GroundItem 4; the registry marks capability 6
  `offered: false` until VIS-3.
- CREATURE-AI-0 §4.1: a creature is active only while a player perceives it (the MOVE-RL-11
  relation); §5.1: the creature step timer runs each step through the Movement owner; §7: per-window
  think and path budgets.
- WORLD-INTERACTION-0 §7.1: NPCs cannot be pushed (`PARITY_PENDING`).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`, `npc.cpp`)

- The think runs voices and walking only while a player spectator exists (`:707-737`).
- Walking (`:1080-1102`, `:1189-1237`): every `walkInterval` ms, if no conversation is open, one
  step in a shuffled order of north, west, east, south, to a tile inside the square of
  `walkRadius` around the spawn position that the NPC may enter, not a floor change or teleport
  (unless `floorChange`), not a raised tile.
- An NPC outside its spawn range is moved back to its spawn position and its conversations end
  (`:722-726`).
- Voices (`:1058-1078`): every `yellSpeedTicks` ms, with `yellChance` percent, one random line, said
  or yelled.
- Focus (`:1156-1183`, `:1268-1277`): a new customer is queued and the NPC turns to it; when a
  customer leaves talk range (4 tiles, `:535`) the conversation ends with the walk-away message and
  the NPC turns to the last remaining customer.
- `isPushable` is false by default (`:524`).

## 3. Presence (NPC-ACTOR-1, NPC-VIS-1)

### 3.1 Actors

- At channel start the channel owner creates one runtime actor for each admitted NPC placement of
  the World Bundle (NPC-0 §3.2), with its own `ExactActorRef` (runtime actor id and generation), on
  every channel of the World. Ids are allocated in canonical order of the placement key (NPC key,
  then position), never container order (SIM-DETERMINISM-01 §10). NPC-0's `npc_actor` in commands 7
  and 8 is this D85 identity {16-byte identity, generation} of a visible kind-5 entity; a stale
  generation fails closed. NPCs are runtime only: nothing durable, recreated at a channel restart
  or planned reset at their placements.
- An NPC is not a creature for combat: it cannot be targeted (ATTACK-0 refuses it as not a
  creature), damaged or pushed (a WORLD-INTERACTION-0 declared difference, Canary); it blocks
  movement like a creature.
- `NPCBEH0-RL-01`: at most 2,048 NPC actors per channel (the reference data admits about 1,110
  NPCs in the whole World).

### 3.2 On the wire

- Entity kind **5 `Npc`** joins `world_spatial_v1` (capability 6) as an actor entry (direction,
  appearance, health percentage 100). It is added before capability 6 is offered (VIS-3), so no
  released client sees an unknown kind (the decoder rejects an unknown kind). VIS-3 depends on
  NPC-VIS-1; if capability 6 were already offered, the kind would need its own capability. An NPC
  with an item look maps it to an `appearance_ref` as items do. Amended: MOVE-RL-11 §4.2 and §4.5.
- NPCs count in D87's 256-entity ceiling and rank with players and creatures before items (D222).
  Amended: MOVE-RL-11 §4.3.

## 4. The NPC think (NPC-ACTOR-1)

- One NPC think per 1,000 ms, in the GAME-AI role of the channel owner, on CREATURE-AI-0's think
  scheduling. It runs only while a player perceives the NPC (CREATURE-AI-0 §4.1 relation, Canary's
  spectator rule; NPCs join CREATURE-AI-0 §4.1's wake set); otherwise the NPC is idle, holds no
  pending think, and its walk and voice ticks do not accumulate.
- NPC thinks have their own row in CREATURE-AI-0 §7's window budget, `NPCBEH0-RL-02` (256 per window
  per channel), served after creature thinks in deadline order, then `ExactActorRef`; over it, a
  think waits. CREATURE-AI-0's cost measure (`RL-17`) covers NPC thinks; NPCs do not count in its
  creature population rows.
  Amended: CREATURE-AI-0 §7 (this PR).
- RNG purposes `NPC_WANDER` and `NPC_VOICE`, seeded by (NPC `ExactActorRef`, think sequence); a retry
  never redraws (SIM-DETERMINISM-01 §12).

## 5. Walking (NPC-ACTOR-1)

- Each think adds 1,000 ms to the walk ticks while no conversation is open (an open conversation
  resets them). At `walk_interval` (content, Canary default 2,000 ms; 0, or a base speed of 0, means
  the NPC never walks) the think draws a shuffled
  order of north, west, east, south (`NPC_WANDER`) and proposes the first step whose tile:
  - lies within the square of `walk_radius` around the placement (Chebyshev distance);
  - is not a floor change or teleport tile (content with `floor_change` set is held: none in the
    Reference base), holds no damaging field (Canary), has no ON_ENTER or ON_LEAVE trigger
    (WORLD-INTERACTION-0 §4.2, §8.5), does not cross a protection-zone boundary the placement does not
    already sit in, and is not raised;
  - the Movement owner would accept for the NPC's `ExactActorRef`.
- The step runs on the CREATURE-AI-0 §5.1 step timer at the NPC's step duration (CONDITIONS-0 §4.2,
  from its content base speed, Canary default 55); a refused step waits for the next interval.
  CREATURE-AI-0 §5.2's creature tile bans do not apply: the Movement owner checks the NPC actor
  class (shop NPCs stand in protection zones). Movement occurrences (VSL-MOVE-01 §5): a step is
  (NPC `ExactActorRef`, think sequence, `NPC_STEP`), a relocation (NPC ref, think sequence,
  `NPC_RELOCATE`), a turn (the opening conversation's CommandRef, `NPC_TURN`).
- **Out of range:** on every think, perceived or not, an NPC more than 50 tiles from its placement
  or 2 floors away (Canary `deSpawnRadius`, `deSpawnRange`) is moved back to its placement by one
  Movement owner relocation, asking GAME-NPC-SERVICE to close its conversations; if the placement
  cannot admit it, it stays in place (CREATURE-AI-0 §5.5's fallback).

## 6. Voices (NPC-VOICE-1)

- Each think adds 1,000 ms to the voice ticks. At the content cadence, with the content chance
  (`NPC_VOICE`), one line drawn uniformly is sent as local speech by the NPC's actor: `say` or `yell`
  per the line, with CHAT-0 §3's ranges and floor rules, to players with `CHAT_V1`, speaker name the
  NPC's display name, text within `CHAT0-RL-01`. NPC lines are exempt from the player yell cooldown,
  level rule, upper-casing and spam control (Canary upper-cases player yells only).
- Voices are admitted text under D9's two-source or single-source rule; a line with an unresolved
  placeholder, a held NPC or a line without admitted text says nothing.
  Amended: CHAT-0 §3 (NPC speakers, this PR).

## 7. Focus (NPC-TALK-2 and NPC-ACTOR-1)

- **Ownership.** The customer queue and every conversation close belong to GAME-NPC-SERVICE (the NPC
  service boundary §4 and §11: GAME-AI owns no dialogue state). GAME-AI reads only the queue head to
  turn the NPC, and asks GAME-NPC-SERVICE to close on a relocation.
- When a conversation opens (NPC-0 §4, or a CHAT-0 greeting), the character joins the end of the
  queue and the NPC turns to face it (a Movement owner turn, shown as the actor's direction).
- **Range check.** On every committed move or relocation of the customer or the NPC (Canary checks on
  each move), not on the budgeted think: a customer beyond talk range (4 tiles, Chebyshev, same
  floor, now NPC-0's own `NPC0-RL-07`) has its conversation closed with the NPC's walk-away line, or
  the generated farewell when the NPC has no Dialogue (NPC-0 §3.3); the NPC turns to the remaining
  head. A pending service invocation still resolves by replay (boundary §7).
- The queue holds at most `NPC0-RL-05` characters; it is runtime only and is cleared on channel
  restart, transfer and reconnect with the conversations (NPC-0 §4). Amended: NPC-0 §4.1.

## 8. Rejected options

- **NPCs as creatures in CREATURE-AI-0's targeting.** NPCs never fight or flee; one small think keeps
  combat code untouched.
- **Durable NPC positions.** Tibia resets NPCs at server save; positions are placement-derived.
- **Walking everywhere for everyone.** Thinks only while perceived (Canary) keep 1,110 NPCs free when
  no player is near.
- **Voices as presentation events.** They are speech; CHAT-0 already carries speech with ranges.

## 9. Architect rulings (owner rule 5905825574)

- **R1. Wire.** a) A new entity kind in capability 6 before it is offered (recommended: no unknown
  kind reaches a released client); b) a separate NPC domain. **Ruled a).**
- **R2. Activity.** a) Think only while perceived (recommended: Canary, bounded); b) always.
  **Ruled a).**
- **R3. Protection zones.** a) An NPC never walks across a protection-zone boundary away from its
  placement's zone state (recommended: shopkeepers stay in their shops, no NPC is stranded outside a
  protection zone); b) ignore zones. **Ruled a)**, `PARITY_PENDING` (Canary checks only
  `queryAdd`).

## 10. Owner questions

None. Every choice is a Tibia-parity application or a bound.

## 11. Decision test

- **Must decide now:** YES. Without it NPCs placed by NPC-0 are invisible, still and silent.
- **Minimum sufficient:** one entity kind, one think, one walk rule, one voice rule, one focus queue,
  nothing durable.
- **Superseding evidence:** an official source on NPC walk or voice timing; a measured NPC think cost
  above the window budget.
- **Deliberately not decided:** schedules, instances, sounds, scripted behaviour.

## 12. Before-freeze checklist

1. **Contract amendments:** MOVE-RL-11 §4.2, §4.3, §4.5 (kind 5, order, VIS-3 sequencing);
   CREATURE-AI-0 §4.1 and §7 (wake set, NPC think row); CHAT-0 §3 (NPC speakers); NPC-0 §4.1 (queue,
   range close, talk range). Applied in this PR.
2. **Serialization:** runtime only, inside the channel owner's tick; steps through the Movement
   owner.
3. **Restart:** nothing durable; NPCs are recreated at their placements.
4. **Typed references:** NPCs are `oteryn:npc.<slug>` definitions with runtime `ExactActorRef`s.
5. **Wire:** kind 5 inside capability 6 before it is offered; voices under `CHAT_V1`.
6. **Split work:** none.
