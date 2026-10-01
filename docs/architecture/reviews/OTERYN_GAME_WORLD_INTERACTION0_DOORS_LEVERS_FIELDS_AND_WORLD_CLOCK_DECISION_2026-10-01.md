# WORLD-INTERACTION-0 Doors, levers, fields and the World clock

- Decision: `WORLD-INTERACTION0-DOORS-LEVERS-FIELDS-AND-CLOCK-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  security, persistence, combat and determinism) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the `GAME-INTERACTION-01` horizon gate (`REQUIRED_FOR_ALPHA`) and gap register §12, with
  the §9 parts on pushing, floor transitions, stairs and teleports; the owner answers **6a** and
  **7a** (2026-10-01, #162) under owner rule 5905825574 (Tibia-faithful); the deferrals of
  RUNE-USE-0 (`RUNEUSE0-C7`, R5: Magic Wall, Wild Growth, fields that affect players),
  QUEST-GATE-0 (KEY-DOOR-0, the lever gate), ITEM-USE-0 (tools on the world, keys) and
  ITEM-MOVE-WIRE-1 §9 (Ground to Ground, map items)
- Builds on: ADR-0021 (§4.4 per-channel overlay, §4.6 FloorChange and Transition.Teleport, §4.7
  planned reset, D191); WO-0 (§4.3 `door` and `floor_change`, §4.4 LocalObject relation) and D216
  (`behavior: teleport`, unbanked magic field to Terrain `field`); the relocation and world-object
  owners proposal (D37, D38 W1-W3, §7 `revert_after`); QUEST-GATE-0 (§3 gates, §4 triggers);
  QUEST-STATE-0 (§7 predicates); D39 (the `USE` edge); ITEM-USE-0 (§3 fields 2 and 4, §5
  cooldown keys); RUNE-USE-0 (§11 field overlay, `RUNEUSE0-RL-05` and `-06`, the position arm);
  CONDITIONS-0 (§3.1 damage over time, the `LIGHT` family); PARTY-PVP-0 (§6.1 `pvp_type`, §7 legality,
  §8.1 PZ block); PREMIUM-ACTIVATION (§4.5 Premium-area entry) and PREMIUM-DELIVERY-0;
  HOUSE-RUNTIME-0 (house doors, unchanged); MAP-WIRE-1 (`map_item_handle`, `TILE_SET`);
  ITEM-MOVE-WIRE-0 and -1 (handles, command 9, drop reach and limits); BAGS-0 (§4.3 tree move,
  §9 Ground counter); CHAR-POSITION-0 (§3.3 fallback); MOVE-RL-11 (VIS-2); SIM-DETERMINISM-01
  §15; GAME-ITEM-01 §4; DUR-03 §7, §32, §39.1; the scope matrix; owner rule 5905825574
- Amends, each pending on acceptance of WORLD-INTERACTION-0, in this PR: the GAME-INTERACTION-01
  horizon section; the scope matrix (world-object state, the World clock); RUNE-USE-0 §11;
  QUEST-GATE-0 §3.1; PARTY-PVP-0 §7.2 rule 6 (fields and secure mode); DUR-03 §39.1 (the Ground
  move shape); GAME-ITEM-01 §4 (the key capability)
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| WORLDINT-WIRE-1 | impl, protocol review | capability `WORLD_INTERACTION_V1`: the `map_item` arm of USE field 4, command 9 Ground and map-item sources with the `GROUND` destination, command `PUSH_INTENT`, the new dispositions (§10.1) | MAP-WIRE-2; ITEM-USE-WIRE-1; ITEM-EQUIP-WIRE-1 |
| WORLDINT-USE-1 | hard, security review | the interaction gate: reach, floor, PZ rules, the `world_action` and `world_ex_action` cooldown keys, the per-tick cap; USE and USE-WITH on map items through LocalObject transitions (§3) | WORLDINT-WIRE-1; MAP-OVERLAY-1; WO-3 |
| DOOR-1 | hard, security review | normal and key doors as LocalObject state machines, the doorway rule, the Premium and vocation gate predicates (§4) | WORLDINT-USE-1; QUEST-GATE-1 |
| KEY-1 | hard, persistence review | the key capability on ItemInstance (`key_number`), its MINT through a RewardClaim, a key on a key door (§4.4) | DOOR-1; CHEST-1 (merged) |
| LEVER-1 | hard | levers, switches and pressure plates as `USE`, `ON_ENTER` and `ON_LEAVE` triggers with the closed child set (§5) | QUEST-TRIGGER-1; WORLDINT-USE-1 |
| FLOOR-1 | impl, movement review | walk-on floor changes, ladders and grates, rope, shovel, pick and machete targets, climbing, teleports, the landing rule (§6) | MAP-LOAD-1; WORLDINT-USE-1 |
| PUSH-1 | impl, movement and combat review | pushing creatures and players (§7.1) | WORLDINT-WIRE-1; the VSL-MOVE-01 movement owner; ATTACK-1 |
| GROUND-MOVE-1 | hard, persistence review | Ground to Ground moves of durable items and overlay moves of movable base items (§7.2) | ITEM-MOVE-2b; BAGS-GROUND-1 for trees |
| WORLDINT-ADMIT-1 | hard, protocol and security review with the FND-04 owner | the FND-04 admission rule that `RUNEUSE0-C7` asks for (§8.6) | FIELD-WIRE-1 |
| FIELD-2 | hard (combat), combat review | fields that affect players (map-authored, player-made by world type, creature-made), Magic Wall and Wild Growth in movement, pathing and the projectile query, their destruction (§8) | FIELD-1; WORLDINT-ADMIT-1; PVP-RT-1 |
| HAZARD-1 | impl, combat review | the `environment_damage` trigger child for map traps and damaging tiles (§8.5) | LEVER-1; COND-1 |
| CLOCK-1 | impl, determinism and protocol review | the World clock epoch, the clock and light functions, the `world_time` predicate, capability `WORLD_LIGHT_V1` with domain `WORLD_CLOCK` and the VIS-2 `light` field (§11) | VIS-2; COND-1 |
| WORLDINT-CONTENT-1 | content lane | LocalObject definitions for doors, levers, piles, holes, grass and torch bearers; tool kinds on Item definitions; key numbers on reward claims; Magic Wall and Wild Growth definitions; from TibiaWiki first, Canary tables as fallback (§12) | WO-2; WO-3; QUEST-CONTENT-2 |

Later, each with its own decision: lighting and burning carried torches (with the timed-item
decision that also covers soft boots and rings), the player-placed Trap item, digging yields
(worms, gold, a scarab), harvesting with a scythe, writing on map objects, beds, boss levers and
rooms (`SCOPE_HANDOFF`), the six native-behaviour runes of RUNE-USE-0 §9 (Destroy Field included),
items lost in water.

## 1. Question

Who may use which world object, and how: doors of every kind, levers and switches, teleports,
fields and walls, ladders, holes and tools, pushing; what of it is per channel and what is durable,
how it resets, what limits it, which commands carry it; and what time it is in the World?

## 2. Facts

**PROVEN**

- Owner answer **6a** (2026-10-01, #162): doors, levers, switches, fields (fire, poison and energy
  fields, Magic Wall, Wild Growth), traps and all other world-object state are **per channel**:
  each channel is a copy of the world map and behaves like its own Tibia server, and resets as
  Tibia does at server save. Quest progress stays durable per character (QUEST-STATE-0).
- Owner answer **7a** (2026-10-01, #162): **one Tibia game clock per World**, identical on every
  channel, 1 game hour = 2.5 real minutes, derived deterministically from a World epoch with no
  cross-channel traffic, driving day and night light; light sources are torches and light spells
  (CONDITIONS-0 `LIGHT`).
- ADR-0021 §4.4: each channel has its own overlay; moved or used map objects, doors, levers and
  removed map items are overlay state, never written to PostgreSQL, discarded at the planned reset;
  no overlay snapshot or journal, so a channel restart starts from the base. Player items on the
  Ground stay DUR-03 custody, survive a crash and are retired at the planned reset (D191).
- D38 (W1-W3): the scope runtime owns the overlay; `TRANSFORM`, `CREATE`, `REMOVE`, `RETAG` on a
  pre-authored anchor; state is scope-ephemeral; anything durable is quest state; pick-up-able
  objects are always DUR-03. §7 (owner-accepted direction): `revert_after` is the scope runtime's
  own later operation on its progression, cleared on scope restart. D37: relocation within the
  scope, rejected if not committed in its tick.
- QUEST-GATE-0: quest and level gates checked at `USE` and at every step onto the door; pass at USE
  opens and moves the character through; a failed step is pushed back; the door closes when the
  last creature leaves; `door_key` and the lever `shared_lock` gate are not lowered there; `USE`,
  `ON_ENTER` and `ON_LEAVE` triggers with quest, relocation, overlay and presentation children;
  `QUESTGATE0-RL-03` (10 firings per character per second).
- Quest format §3.1: 184 quest-progress gates, 38 key gates (24 keys from chests), 14 level gates,
  1 lever gate; a key door's lock is world-object state of its channel.
- RUNE-USE-0 §11: a field is a runtime world object in a per-channel field overlay, not an item;
  one per tile (`RUNEUSE0-RL-05`), 20,000 per channel (`RUNEUSE0-RL-06`); a player-made field
  affects creatures only; Magic Wall, Wild Growth and creature-made fields wait for a decision
  and an FND-04 admission rule (`RUNEUSE0-C7`, R5); fields travel as VIS-2 entities under
  `WORLD_SPATIAL_FIELDS`.
- MAP-WIRE-1: actionable base items carry a per-session `map_item_handle` bound to
  (`overlay_incarnation`, position, `placement_key`, `tile_revision`); `map_item {handle}` is a USE
  target and a command 9 source; an overlay change is described by `TILE_SET`; the protection
  zone is never on the wire.
- ITEM-MOVE-WIRE-1 §5: drop within 15 tiles, same floor, line of sight; pickup at Chebyshev 1;
  `ITEMMOVE1-RL-01` 10 loose items per tile, `ITEMMOVE1-RL-02` 20,000 per channel; §9 defers Ground
  to Ground and map items.
- PARTY-PVP-0: `pvp_type` is a World ruleset field; every PvP rule, fields included, runs in the
  GAME-ABILITY-01 legality stage; the PZ block bars stepping onto a protection-zone tile.
- PREMIUM-ACTIVATION §4.5: entering a Premium area is refused unless Premium is current at that
  moment.
- GAME-ITEM-01 §4: an instance carries authoritative state only for capabilities its ItemType
  declares; no free-form attribute bag (§5).
- SIM-DETERMINISM-01 §15: authoritative formulas consume normalized time facts, never a hidden
  clock read; replay uses the recorded facts.

**CIPSOFT_OFFICIAL** (the Tibia manual, `docs/reference/tibia-manual/`)

- Floors: stairs, ramps and holes by walking; a rope spot needs a rope or Magic Rope; sewer grates
  and ladders by "Use"; some transitions by climbing items (`controls.md:27`).
- Pushing: drag a creature; it happens after a short delay, only if the path is free and the target
  did not move; pushing in combat skips the pusher's next attack; many strong monsters cannot be
  pushed (`controls.md:41-46`). A character can be pushed inside a PZ but not out of it; in a PZ
  characters may share a tile (`combat.md:85`, `:89`).
- Jungle grass needs a machete; a loose stone pile opens with a shovel; secret holes with a pick;
  descend by moving onto a hole; ascend with a rope, also roping up a character below
  (`world.md:73-86`). "Use with" for ropes and shovels (`controls.md:50`).
- Harmful fields damage anyone including the caster on Hardcore, the caster only for the first 5 s
  on Retro Hardcore, and not their caster on Optional (`combat.md:153`, `:159`, `:170`). Secure mode
  does not prevent skull marks from indirect damage such as fire fields (`interface.md:101`).
  Field runes cause a PZ block without a hit (`combat.md:213`). A magic wall can trap a victim
  (`combat.md:146`).
- Items dropped into water are lost (`world.md:79`).

**TIBIAWIKI** (fetched 2026-10-01)

- *Time*: 2.5 s = 1 Tibian minute, 2.5 min = 1 Tibian hour, 1 hour = 24 Tibian hours; day is 6:00 to
  18:00 Tibian, which is hh:15 to hh:45 real time, so Tibian midnight falls on each full real hour.
- *Key*: a key used on a door leaves it unlocked until the next reset or until the key locks it
  again; keys have a base form and a key number.
- *Magic Wall Rune*: lasts 16-24 s, blocks one tile and all ammunition, spells and runes; on
  Optional PvP a player walking over it destroys it. *Wild Growth Rune*: 30-60 s, blocks movement
  only; a machete destroys it on the other world types; walking destroys it on Optional PvP.
- *Gate of Expertise*: opens at the required level ("Only the worthy may pass."); creatures cannot
  be lured or pushed through. *Door (Sealed)*: the quest door.
- *Jungle Grass*: cut grass grows back after 5 minutes. *Loose Stone Pile*: a shovel opens it to a
  hole; monsters cannot walk into a hole. *Rope Spot*: creatures cannot be roped.
- *Light* (spell): 3-tile radius, 6 min 10 s. *Lever*: usable levers open doors, teleports or
  holes.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`, read-only)

- Use reach: same floor and Chebyshev 1 (`src/lua/creature/actions.cpp:179-191`); far use 7 × 5
  with line of sight (`:201-219`). `timeBetweenActions = 200`, `timeBetweenExActions = 1000`,
  `pushDelay = 1000` (`config.lua.dist:408-417`).
- Push: pusher adjacent; target must be pushable; destination within the creature's throw range;
  no creature on it; a creature in a PZ cannot be pushed out (`src/game/game.cpp:1796-1895`).
  Monsters never enter PZ, floor-change or teleport tiles (`src/items/tile.cpp:717-719`); a PZ-locked
  player cannot step into a PZ (`:832-846`); on Optional PvP a player walking onto the safe Magic
  Wall or Wild Growth removes it (`:864-872`).
- Climbing: a player on a tile with height 3 steps up a floor; down symmetrically, never
  diagonally (`src/game/game.cpp:1909-1950`). Floor-change destinations follow the lower tile's
  direction flags (`src/items/tile.cpp:1013-1070`).
- Teleports and scripted relocations use `FLAG_NOLIMIT`, ignoring blocking creatures and the PZ
  lock (`src/game/game.cpp:3540-3590`, `src/items/tile.cpp:690-697`); a teleport moves creatures
  and items and refuses a destination loop (`src/game/movement/teleport.cpp:51-110`).
- Doors: a normal door toggles and does not close on a creature; a key door is locked, closed or
  open; a key whose action id matches unlocks it to open and locks a closed or open one; action ids
  101 and 1001 never unlock ("It is locked.", "The key does not match.")
  (`data/scripts/actions/doors/custom_door.lua:12-33`, `key_door.lua:28-78`). Quest and level doors
  push back a step and close behind (`data/scripts/movements/closing_door.lua:20-56`).
- Tools: rope on a rope spot moves the user upstairs, south first then the other directions; rope
  on a hole pulls up a player or a movable item from below (`data-otservbr-global/scripts/lib/
  register_actions.lua:319-349`, `data/libs/functions/position.lua:22-50`); shovel opens a pile and
  moves the digger down at once, refusing a PZ-locked digger over a PZ (`register_actions.lua:401-410`);
  machete cuts jungle grass and removes Wild Growth (`:838-856`).
- Traps: a closed trap deals damage on step-in and transforms, reverting on step-out, never in a
  PZ (`data/scripts/movements/trap.lua:1-60`).
- Light: day 250, night 40, color 215; a 10 s light tick advances 4 Tibian minutes and moves the
  level by 7 during sunrise (from 06:00) and sunset (from 17:30); the clock starts from the real
  minute of the hour (`src/game/game.hpp:66`, `:916-934`; `src/game/game.cpp:883-886`,
  `:9402-9456`, `:9479-9491`). A player's item light is its brightest equipped item
  (`src/creatures/players/player.cpp:6284-6305`). A lit torch decays while carried
  (`data/items/items.xml:6312-6328`).

## 3. The interaction model (WORLDINT-USE-1)

### 3.1 Who and what

- **Actor.** A player character with a live GameSession on the channel (or instance) that owns the
  object. Creatures never use doors, levers or tools; monsters never enter protection-zone,
  floor-change or teleport tiles (Canary evidence), so they never follow through a door, down a
  hole or into a teleport. House doors and house tiles stay with HOUSE-RUNTIME-0.
- **Targets and acts:**
  - `USE` on a map item (doors, levers, ladders, sewer grates, torch bearers): USE-WIRE-V1 with the
    MAP-WIRE-1 `map_item {handle}` target;
  - `USE-WITH` a carried tool or key on a map item: the used item in field 2 or 5 (ITEM-USE-0), the
    target in the new `map_item` arm of field 4 (§10.1);
  - a step onto or off a tile (walk, floor change, push): the `ON_ENTER` and `ON_LEAVE` edges;
  - a push of a creature (§7.1) and a move of a Ground or movable base item (§7.2).
- **Behaviour is data.** A map item does something only through its LocalObject definition (states,
  per-state collision, transitions, an optional `tool_target`, an optional gate, an optional
  trigger) or its FloorChange or Transition.Teleport record (ADR-0021 §4.6). A placement with none
  answers `NOTHING_TO_USE`. There is no script engine and no per-object code.

### 3.2 Reach

- `USE` and `USE-WITH` on a map item, the source of a push and the source of a Ground move need the
  actor on the same floor at Chebyshev distance at most `WORLDINT0-RL-01` (1), standing on the tile
  included. At distance 1 there is no line-of-sight test (Canary). A farther target is `TOO_FAR`;
  a target on another floor is `TOO_FAR`. The client walks; the server never walks the player.
- Look has no reach (unchanged).

### 3.3 Protection zones

- Doors, levers, ladders, ropes and tools work inside a protection zone.
- **The PZ block** (PARTY-PVP-0 §8.1) bars entering a PZ tile by every movement this decision
  defines: a walk-on floor change, a push, climbing, and every relocation landing (teleport,
  ladder, grate, rope, gate pass-through). A refused walk or push is `PZ_BLOCKED` and nothing moves;
  a refused landing leaves the actor where it was (§6.6). Architect ruling R1 (§14).
- A creature standing in a PZ cannot be pushed out of it (§7.1).
- No field or wall is created on a PZ tile (RUNE-USE-0 §11.2, unchanged).

### 3.4 Exhaustion

- Typed cooldown keys owned by the ChannelRuntime (GAME-ABILITY-01), started at admission of the
  act, kept after a refused or failed act:
  - `world_action`: `WORLDINT0-RL-02` (200 ms), for `USE` on a map item;
  - `world_ex_action`: `WORLDINT0-RL-03` (1,000 ms), for `USE-WITH` on a map item;
  - `push`: `WORLDINT0-RL-04` (1,000 ms).
- An act during its cooldown is `EXHAUSTED`; nothing is queued (as ITEM-USE-0 §5). These keys are
  separate from ITEM-USE-0's `food` and `potion` keys (architect ruling R4, §14).
- **Channel cap.** At most `WORLDINT0-RL-08` (512) world interactions commit per channel per
  simulation tick, in the owner lane's input order; a further one in that tick is `EXHAUSTED` and
  changes nothing (D37 R2: no queue).
- Trigger firings stay bounded by `QUESTGATE0-RL-03`, checked before the root commits.

### 3.5 Ownership, fences and one commit

- Every act runs in the owning scope runtime's lane (D37, D38). It is admitted against the handle's
  `overlay_incarnation` and `tile_revision`, the actor's session generation and position revision,
  and the content generation; any mismatch is `STALE`. The overlay change, any relocation and the
  `TILE_SET` publish in the same tick; a request not committed in its tick is rejected, except a
  delayed push (§7.1).
- **Delayed push.** A `PUSH_INTENT` is admitted in its arrival tick: that tick counts it against
  the per-tick action cap, starts the `push` cooldown and arms one owner-lane timer due one push
  delay later. The step commits in the tick the timer fires, which re-runs every fence above and
  every §7.1 rule on the live state; in that tick due push timers run in order of (due tick, pusher
  `ExactActorRef`), before new input, and are not counted against the cap again. A channel restart
  drops a pending push with no step.
- A world-object act writes nothing durable. A key, a tool and a moved durable item are touched only
  through DUR-03 shapes (§4.4, §7.2); nothing here mints or burns.
- Replay: an act carries its CommandRef; a replay in the same overlay incarnation returns the first
  outcome; after a channel restart the old handle is `STALE` and no volatile child (overlay,
  relocation, presentation) repeats.
- Durable trigger children are the exception. A lever, switch or plate firing whose root plan
  admitted a durable child (a quest transition, QUEST-GATE-0 §4) keeps that plan's occurrence-based
  recovery: after a crash or channel restart the unstarted durable children are recovered and
  committed exactly once from the occurrence, even though the overlay children are not repeated.

## 4. Doors (DOOR-1, KEY-1)

### 4.1 Kinds

| Kind | Source | Rule |
|---|---|---|
| Normal door | WO-0 `door` without gate or key | `USE` opens a closed door and closes an open one |
| Key door | quest format `door_key` gate, `shared_lock` | §4.3 |
| Quest door (sealed) | QUEST-GATE-0 §3 | unchanged |
| Level door, Gate of Expertise | QUEST-GATE-0 §3 (`level >= n`) | unchanged |
| Premium door | a gate with the `premium` predicate (§4.5) | QUEST-GATE-0 §3.2 mechanics |
| Vocation door | a gate with the `vocation_in` predicate (§4.5) | QUEST-GATE-0 §3.2 mechanics |
| House door | HOUSE-RUNTIME-0 | unchanged |

A door whose source behaviour is still `unresolved_semantics` (QuestDoorUnique and bare-number
quest doors, quest format §3.1) stays sealed.

### 4.2 State and collision

- A door is a LocalObject (WO-0 §4.4) with the states `closed` and `open`, and `locked` for a key
  door; each state presents its WorldObject key (WO-3). `closed` and `locked` block movement,
  projectiles and sight; `open` is walkable and blocks neither.
- State is per channel overlay (owner 6a, D38 W2): every player of the channel sees the same door;
  a door opened on one channel is closed on another.
- **Doorway rule.** A door does not close while a creature stands in its doorway (`NOT_POSSIBLE`).
  A door tile does not accept a dropped or moved item (`BLOCKED`), so closing never meets a loose
  item (architect ruling R3, §14).
- **Closing behind.** Quest, level, Premium and vocation doors close when the last creature leaves
  (QUEST-GATE-0 §3.2). Normal and key doors stay as left until someone uses them or the reset.

### 4.3 Key doors

- States `locked`, `closed`, `open`; the initial state comes from the placement (normally `locked`).
- `USE` on `locked`: "It is locked." (`LOCKED`), nothing changes. `USE` on `closed` opens it; `USE`
  on `open` closes it (doorway rule).
- `USE-WITH` a key on the door: when the key's `key_number` equals the gate's key binding, a
  `locked` door becomes `open`, and a `closed` or `open` door becomes `locked` (doorway rule for an
  occupied open door); otherwise "The key does not match." (`KEY_MISMATCH`). Bindings 101 and 1001
  never unlock (`WORLDINT0-RL-10`).
- The lock is shared by everyone on the channel and resets at the planned reset (TibiaWiki *Key*).
  Using a key never changes or spends the key.

### 4.4 Keys

- A **key capability** (amends GAME-ITEM-01 §4): an ItemType that declares it gives each instance
  one immutable typed `key_number` in `1..65,535` (`WORLDINT0-RL-10`). It is set at MINT and never
  changes; no other path writes it. It is part of the item's state digest, so trade bindings and
  replays see it. Keys are not stackable.
- A key reaches a character only through a `RewardClaim` MINT (the CHEST-1 path, D40) whose content
  names the key definition and number (the quest format's `key_from_claims`). KEY-1 adds the number
  to that MINT's typed evidence; every TRANSFER carries it unchanged (DUR-03 §39.1 amendment).
- A map-authored key on the base map is never materializable (ADR-0021 §4.4: it carries an `action`
  binding), so keys enter the economy only through claims.

### 4.5 Premium and vocation gates

- The QUEST-GATE-0 §3.1 predicate set gains `premium` and `vocation_in {set}` (amendment pointer).
  `premium` reads the PREMIUM-ACTIVATION §4.5 Premium-area surface (`REQUIRE_CURRENT` at use and at
  each step); until PREM-3 is live, the PREMIUM-DELIVERY-0 rule applies (any class other than
  `CURRENT_AUTHORITY` reads as Free), so the gate seals. `vocation_in` reads the character's A13
  vocation. Both are read-only and write nothing (QUEST-GATE-0 §3.2).

## 5. Levers, switches and pressure plates (LEVER-1)

- **A lever or switch** is a LocalObject with two or more states and one `USE` trigger; **a
  pressure plate** is a tile or object with `ON_ENTER` and `ON_LEAVE` triggers. Both are D36
  interaction definitions with QUEST-GATE-0 §3.1 read-only conditions.
- **Closed child set** (bounded content-defined actions), at most `WORLDINT0-RL-07` (16) children
  per firing, in the successor's canonical order (QUEST-GATE-0 §4):
  - overlay operations on named anchors of the same scope: `TRANSFORM`, `CREATE`, `REMOVE`, `RETAG`
    (D38), each optionally with `revert_after` (proposal §7): doors, walls, bridges, holes,
    teleports, the lever's own state;
  - a relocation of the triggering actor to an anchor or its previous tile (D37);
  - a quest transition (QUEST-GATE-0 §4);
  - presentation (a magic effect or a text);
  - `environment_damage` (§8.5).
  Any other script behaviour stays `unresolved_semantics` (bespoke) and the trigger is held. A
  `USE` on a lever that does nothing is a plain `TRANSFORM` of the lever.
- **Shared state.** A lever's position and everything it changed are per channel and visible to
  everyone there, as in Tibia; the quest format's lever gate (`shared_lock`, the Katana door) is a
  door whose `open`/`closed` state a lever's `TRANSFORM` children set.
- **Reverts.** Pending `revert_after` records per channel are bounded by `WORLDINT0-RL-09`
  (20,000, alarm at 80%); an act whose firing would schedule past it is refused before commit. A
  revert never schedules another (proposal §7).
- **Boss levers** (room checks, `SCOPE_HANDOFF`) are not in this decision.

## 6. Floor changes, tools and teleports (FLOOR-1)

### 6.1 Walk-on floor changes

- Stairs, ramps and open holes change floor when stepped on. The destination is compiled into the
  FloorChange record from the WO-0 and Terrain `floor_change` facts by the Canary destination rules
  (§2), not searched at runtime. Players only; monsters never step on them.
- An item moved onto an open hole falls to the hole's destination (§7.2).

### 6.2 Use to change floor

- A ladder (up) and a sewer grate (down) change floor by `USE` within reach; their destination is a
  FloorChange record with `trigger: USE`.

### 6.3 Tools on the map

- Tool kinds are an Item fact (`tool_kind`: `rope`, `shovel`, `pick`, `machete`); a multi-tool lists
  several. A target map item declares `tool_target {kind, transition, revert_after}`; a tool whose
  kind it does not list is `NOTHING_TO_USE`.
- **Rope** on a rope spot: the user relocates upstairs to the first walkable tile in the fixed order
  south, north, east, west, then the diagonals (Canary `moveUpstairs`), at most `WORLDINT0-RL-12`
  (8) candidates; none walkable is `NOT_POSSIBLE`. **Rope on an open hole:** the topmost player on
  the tile below is relocated up the same way, or else the topmost movable item is moved up;
  creatures are never roped (TibiaWiki).
- **Roped item.** A bounded cross-floor exception to the §7.2 reach and same-floor rules: the source
  is the tile directly below the hole (one floor down), the destination is the first tile around
  the rope spot that accepts items, in the same fixed order on the user's floor. A durable Ground
  item moves by the one-item Ground→Ground TRANSFER of §7.2 (same channel scope, tile limit and
  locks), a movable base item by the §7.2 overlay move; no candidate accepting it is
  `NOT_POSSIBLE`.
- **Shovel** on a loose stone pile, **pick** on a cracked or secret floor: `TRANSFORM` to an open
  hole with `revert_after` from content. The digger is not moved: it walks onto the hole
  (TibiaWiki; Canary moves it at once, declared, §14).
- **Machete** on jungle grass: `TRANSFORM` to cut grass, reverting after 300 s (TibiaWiki); on Wild
  Growth: removes it, except on Optional worlds (§8.3).
- Digging yields, harvesting and other value-producing tool uses are excluded: they would mint.

### 6.4 Climbing

- A player, stepping orthogonally from a tile whose stack height is at least `WORLDINT0-RL-13`
  (3 elevated items), climbs to the tile one floor up in that direction when it has ground and no
  blocking-solid item; it climbs down the same way (Canary `game.cpp:1909-1950`). Monsters do not
  climb.

### 6.5 Teleports (D216)

- A WorldObject with `behavior: teleport` and its Transition.Teleport record: stepping on it
  relocates the creature or player to the destination (D37); an item moved onto it lands at the
  destination (§7.2). Monsters never enter it. Gated teleports keep QUEST-GATE-0 §3.4, and a
  destination in a Premium area keeps PREMIUM-ACTIVATION §4.5.
- The compiler refuses a cycle of teleport destinations (Canary `checkInfinityLoop`). A landing on
  another teleport stays there (§6.6).

### 6.6 The landing rule

- Every relocation landing (§6.2-§6.5, gate pass-through, rope) accepts a tile already holding
  creatures (they share it, as Tibia's teleports do) and refuses a tile without ground or with a
  blocking-solid item, or a PZ tile for an actor under a PZ block (§3.3). A refused landing is
  `REJECTED_BLOCKED` (D37): the actor stays where it was.
- A landing is one hop (`WORLDINT0-RL-14`): it never fires a floor change, a teleport or a trigger
  of the tile it lands on (QUEST-GATE-0 §4: a relocation child is never a trigger root).

## 7. Pushing and moving things (PUSH-1, GROUND-MOVE-1)

### 7.1 Creatures and players

- `PUSH_INTENT {target, to}`: the pusher stands at Chebyshev 1 from the target, same floor; `to` is
  a tile at Chebyshev 1 from the target's tile, same floor (`WORLDINT0-RL-01`).
- Refused when: the target is a monster without the content flag `pushable`, or an NPC
  (`NOT_MOVABLE`); `to` holds a creature, blocks, or has no ground (`NO_ROOM`); the target is on a
  PZ tile and `to` is not (`NOT_POSSIBLE`); `to` is a PZ tile and the target has a PZ block
  (`PZ_BLOCKED`); the target moved since the client saw it (`STALE`); the `push` cooldown runs
  or a push of this pusher is already pending (`EXHAUSTED`).
- **Push delay** (server-side, the manual; Canary `pushDelay`): an admitted push executes one push
  delay (`WORLDINT0-RL-04`, 1,000 ms) later, not at once, under the §3.5 delayed-push rule. At execution every rule above is checked
  again on the live state, including the pusher's position and reach, and a failure ends it with
  that outcome and no step. At most one pending push per pusher; a client cannot shorten the
  delay.
- The pushed step is an ordinary step of the target, with its step duration: a floor change, an
  open hole or a teleport on `to` applies (pushing down a hole is Tibia), and its `ON_ENTER` and
  `ON_LEAVE` triggers fire with the push command as root (QUEST-GATE-0 §4). Gated tiles push back
  an unqualified target.
- A push by a character in combat delays its next auto-attack by one attack interval (the manual;
  ATTACK-1 applies it).
- Monsters that push creatures or items (content flags) stay with GAME-AI-01.

### 7.2 Items on the Ground and movable map items

- **Reach.** Except for a roped item (§6.3), the source is within `WORLDINT0-RL-01`; the destination is on the same floor, within
  `WORLDINT0-RL-05` on each axis (15 for a pickupable item, 2 for a movable item that is not
  pickupable, Canary `item.hpp:327-329`), in line of sight, and accepts items. A destination that is
  an open hole or a teleport resolves to its destination tile before commit.
- **Movable base items** (map-authored, movable, not materialized: crates, chairs): an overlay move
  that keeps the entry's one `placement_key` (ADR-0021 §4.4), hides it at its origin and places it
  at the destination, at most `WORLDINT0-RL-06` (10) moved base items on a tile. Volatile: it
  returns home at the reset or a channel restart. Picking one up is the ADR-0021 MINT, unchanged.
- **Durable Ground items** (drops, materialized map items, trees): one TRANSFER from Ground
  `{channel, tile A}` to Ground `{channel, tile B}` in the same channel scope (DUR-03 §32 fence),
  with the `ITEMMOVE1-RL-01` tile limit at B and a real row lock on both tile rows, taken after the
  item rows in ascending tile key order (composition rule 4). A tree moves by its root (BAGS-0
  §4.3); the Ground counter does not change. Rows in §12. A dropped item keeps its D191 reset.
- **Water.** A tile that does not accept items (water included) refuses them (`BLOCKED`); Global
  destroys them, a sink this decision does not add (declared, §14).

## 8. Fields, walls and hazards (FIELD-2, HAZARD-1, WORLDINT-ADMIT-1)

### 8.1 Kinds and sources

- **Damage fields** (fire, energy, poison; walls and bombs of them) and **blocking walls** (Magic
  Wall, Wild Growth).
- Sources: **map-authored** (Terrain `field` records, D216; base items, permanent unless their
  content decays them); **player-made** (RUNE-USE-0 §11); **creature-made** (a monster spell's
  `create_item` effect, enabled by FIELD-2). All live in the channel's field overlay except
  map-authored ones, which are base entries of the map.

### 8.2 Damage to players

- Stepping on or standing on a damage field applies its condition through CONDITIONS-0 §3.1.
- **Map-authored and creature-made** fields hurt every player (PvE): no PvP legality stage.
- **Player-made** fields hurt players through the PARTY-PVP-0 §7.2 legality stage at each
  application, with two field rules (amendment pointer): the caster's secure mode does not filter a
  field's damage (the manual: fields give incidental skulls); the caster itself is hurt only on
  `HARDCORE` (the manual: Optional spares the caster as the exception to Hardcore; Open
  `PARITY_PENDING`). A field tick in a PZ is refused (CONDITIONS-0 §3.2).

### 8.3 Blocking walls

- Magic Wall blocks movement, projectiles and sight; Wild Growth blocks movement only. Movement and
  pathing of players and monsters and the projectile and sight query read the field overlay.
- Creation refuses a tile with a creature, a floor-change or teleport tile and a PZ tile (RUNE-USE-0
  D.6.1, §11.2); the duration is drawn from content within `WORLDINT0-RL-11` (16-24 s, 30-60 s)
  under the RNG purpose `field_duration`.
- **Destruction:** on an `OPTIONAL` World a player stepping onto a Magic Wall or Wild Growth removes
  it and steps; on every other type a machete removes Wild Growth (§6.3); monsters that destroy
  walls use SW-2 `remove_items`. Each removal is a `leave` delta (RUNE-USE-0 §11.4).
- Walls and creature-made fields count against `RUNEUSE0-RL-05` (1 per tile) and `RUNEUSE0-RL-06`
  (20,000 per channel).

### 8.4 Monsters and fields

- Monster field avoidance and immunity follow GAME-AI-01 and the monster content (unchanged).

### 8.5 Traps and damaging tiles (HAZARD-1)

- A map trap (a closed spike trap and its kin) is an `ON_ENTER` trigger with an `environment_damage
  {element, min, max}` child and a `TRANSFORM`, and an `ON_LEAVE` trigger with the reverse
  `TRANSFORM` (Canary `trap.lua`). The damage is one GAME-ABILITY-01 occurrence with no attacker,
  drawn under the RNG purpose `trap_damage`, at most `WORLDINT0-RL-15` (1,000) per hit; never on a
  PZ tile; PvE. A content value above the ceiling holds the trigger.
- The player-placed Trap item is a later decision (it transforms a durable Ground item).

### 8.6 The admission rule (WORLDINT-ADMIT-1)

- `RUNEUSE0-C7` stays until an FND-04 amendment, written by WORLDINT-ADMIT-1 with the FND-04 owner,
  is accepted and active: on a World where any field can hurt or block a player (every public
  World under this decision), gameplay admission and recovery refuse a session that did not
  negotiate `MAP_STATE_V1`, `WORLD_SPATIAL_FIELDS` and `WORLD_INTERACTION_V1`. Until then FIELD-2's
  player effects and blocking walls stay inactive, and fields affect creatures only (RUNE-USE-0).
  Map-authored fields reach a session through `MAP_TILES`; overlay fields through VIS-2.

## 9. Persistence, reset and determinism

### 9.1 What is volatile per channel, what is durable

| State | Scope | Lifetime |
|---|---|---|
| Door, lock and lever states; open holes, cut grass, transformed objects; moved base items; trap states; pending reverts | Channel overlay (instance overlay inside an instance) | Volatile: cleared at the planned reset and at a channel restart |
| Fields and walls of every source except map-authored | Channel field overlay | Volatile: expiry, reset or restart |
| Quest tracks, gate passes as read state | Character (QUEST-STATE-0) | Durable |
| Keys, tools, dropped and materialized Ground items | DUR-03 ItemInstance | Durable; Ground items retired at the planned reset (D191) |
| Character position | Character per World (CHAR-POSITION-0) | Durable |
| Cooldown keys, conditions | Runtime actor | Runtime (CONDITIONS-0 §6) |
| World clock epoch | World ruleset | Durable configuration |

### 9.2 Reset

- **Planned world reset** (Tibia's server save, ADR-0021 §4.7): every channel starts from the base
  with empty overlays: doors closed or locked as authored, levers home, holes closed, fields gone.
- **Channel restart** (a crash, owner 6a: each channel is its own server): that channel's overlay is
  lost and rebuilt from the base; other channels are untouched. Durable Ground items are rebuilt
  (ADR-0021). A character re-admitted onto a tile now blocked takes the CHAR-POSITION-0 §3.3
  fallback.
- **Channel transfer:** the character meets the target channel's own doors and fields; no
  world-object state travels.
- **Content revision:** the bundle and content change only at a planned reset (ADR-0021 §4.7), when
  every overlay is empty, so no world-object state needs migration.

### 9.3 Determinism

- Every act is an input with its CommandRef, applied in the owner lane's order (FND-03
  `RuntimeExecutionOrdinal`); outcomes depend only on committed overlay state, content, the input
  and named RNG purposes (`field_duration`, `trap_damage`). Reverts and expiries are timer inputs
  on the scope runtime's progression (proposal §7). The clock reads the recorded time fact (§11.2).

## 10. Wire

### 10.1 Capability `WORLD_INTERACTION_V1` (WORLDINT-WIRE-1)

- Requires `MAP_STATE_V1` and `ITEM_USE_V1`; its number is reserved on #162 at allocation.
- **USE** (command 2) keeps the MAP-WIRE-2 `map_item {handle}` target for doors, levers, ladders,
  grates and torch bearers. **Field 4 `use_with`** gains a `map_item {handle}` arm (a tool or key on
  a map item); its field number is assigned by WORLDINT-WIRE-1. The payload stays within 529 bytes
  (`WORLDINT0-RL-22`).
- **Command 9** gains, under the capability, the sources `map_item {handle}` (a movable base item)
  and a Ground item handle (domain 1), each with the `GROUND {position}` destination (§7.2).
- **New command `PUSH_INTENT`** (type number reserved at allocation): `{target {actor_id,
  generation}, to WorldTilePosition}`, at most `WORLDINT0-RL-21` (32 bytes), non-durable.
- **Dispositions**, under the capability, at most 4 bytes: `TOO_FAR`, `LOCKED`, `KEY_MISMATCH`,
  `SEALED` (with the gate's message id), `PZ_BLOCKED`, `NOT_MOVABLE`, `NO_ROOM`, `NOT_POSSIBLE`;
  the existing `EXHAUSTED`, `STALE`, `BLOCKED` and `NOTHING_TO_USE` are reused.
  **Amendment (pending on acceptance of OFFLINE-0; `reviews/OTERYN_GAME_OFFLINE0_STAMINA_AND_OFFLINE_TRAINING_DECISION_2026-10-01.md` §6).** Plus `OFFLINE_TRAINING`: a training statue accepted the
  activation and a graceful logout follows.
- **Without the capability** a USE on a map item keeps its MAP-WIRE-2 meaning with new outcomes
  reported as `REJECTED`; the field 4 map arm, the new command 9 sources and `PUSH_INTENT` are
  refused.
- **State on the wire:** every overlay change (door, lever, hole, moved base item) is a `TILE_SET`
  of its tile (MAP-WIRE-1 §7), describing a LocalObject entry by its current state's presentation;
  fields and walls are VIS-2 entities (RUNE-USE-0 §11.4). No new map message.

### 10.2 Capability `WORLD_LIGHT_V1` (CLOCK-1)

- Requires `WORLD_SPATIAL_ENTITIES`; its number and the domain id are reserved at allocation.
- **Domain `WORLD_CLOCK`**: `{tibian_minute u16, light_level u8, light_color u8}`, at most 9 bytes
  (three tagged varints). A snapshot at admission, reconnect and channel transfer; a delta when the
  light level changes and at each Tibian hour, at most one per `WORLDINT0-RL-18` (10,000 ms).
- **VIS-2 `light`**: `{level u8, color u8}` on player and creature entries, at most
  `WORLDINT0-RL-20` (6 bytes) per entry; an update at most once per 1,000 ms per entity. CLOCK-1
  amends MOVE-RL-11 at allocation, as FIELD-WIRE-1 does.
- Without the capability the client renders a fixed day light and no creature light.

## 11. The World clock and light (CLOCK-1)

### 11.1 One clock per World (owner 7a)

- The World ruleset gains `world_clock_epoch_utc_ms`, the same on every channel, bound into the
  ruleset revision and never changed after the World's first activation. Its default is a whole
  UTC hour, so Tibian midnight falls on each full real hour, as in Global (TibiaWiki *Time*).

### 11.2 The clock

- `tibian_minute(t) = floor((t − epoch) / 2,500 ms) mod 1,440`, for `t ≥ epoch`
  (`WORLDINT0-RL-16`): 2.5 s per Tibian minute, 150 s per Tibian hour, one Tibian day per real hour.
- `t` is the scope runtime's normalized UTC time fact of the input or tick that reads it, recorded
  with it (SIM-DETERMINISM-01 §15); replay reuses it. No channel sends the clock to another;
  channels of a World agree within the deployment clock skew bound `WORLDINT0-RL-19` (1,000 ms,
  alarm above).

### 11.3 World light

- A pure integer function of the Tibian minute `m` (`WORLDINT0-RL-17`):
  - `360 <= m < 480` (sunrise): `min(250, 40 + 7 × (floor((m − 360) / 4) + 1))`;
  - `480 <= m < 1,050`: 250 (day);
  - `1,050 <= m < 1,170` (sunset): `max(40, 250 − 7 × (floor((m − 1,050) / 4) + 1))`;
  - otherwise 40 (night). Color 215.
- This is Canary's 10 s step of 7 over 30 steps, made a function of the epoch instead of a running
  counter (`PARITY_PENDING` against Global's exact curve).
- How a client darkens underground floors is presentation (MAP-CLIENT-1).

### 11.4 Light sources

- **Creature light:** the higher of the actor's `LIGHT` condition level (CONDITIONS-0, decaying
  linearly) and its brightest equipped item's light (Item facts; Canary `player.cpp:6284-6305`);
  a monster's from its content. Sent as VIS-2 `light`.
- **Map light sources** (wall torches, lamps) render from their appearance; a torch bearer toggles
  by a LocalObject `TRANSFORM` (per channel).
- **Carried torches:** lighting and burning one transforms and decays a durable carried item; it
  waits for the timed-item decision (deferral, not a difference).

### 11.5 Reading the clock in gameplay

- A read-only content predicate `world_time {from_minute, to_minute}` (and `is_day`: 06:00 to 18:00)
  for interaction conditions, NPC dialogue (the "time" answer) and spawn rules, evaluated on the
  input's recorded time fact.

## 12. Rows (registered by each child before implementation)

| Row | Value |
|---|---|
| `WORLDINT0-RL-01` reach of USE and USE-WITH on a map item, of a push, a push destination from the target, and of a Ground move source (a roped item excepted, §6.3) | Chebyshev 1, same floor |
| `WORLDINT0-RL-02` `world_action` cooldown | 200 ms |
| `WORLDINT0-RL-03` `world_ex_action` cooldown | 1,000 ms |
| `WORLDINT0-RL-04` `push` delay and cooldown | 1,000 ms |
| `WORLDINT0-RL-05` move range: pickupable item / movable non-pickupable base item | 15 / 2 tiles on each axis, same floor, line of sight |
| `WORLDINT0-RL-06` moved base items on one tile | 10 |
| `WORLDINT0-RL-07` children per trigger firing | 16 |
| `WORLDINT0-RL-08` world interactions committed per channel per simulation tick | 512; over it `EXHAUSTED` |
| `WORLDINT0-RL-09` pending `revert_after` records per channel | 20,000, alarm at 80%; refused before commit above it |
| `WORLDINT0-RL-10` key number | 1 to 65,535; 101 and 1,001 never unlock |
| `WORLDINT0-RL-11` Magic Wall / Wild Growth duration | 16,000-24,000 ms / 30,000-60,000 ms |
| `WORLDINT0-RL-12` rope landing candidates | 8, fixed order |
| `WORLDINT0-RL-13` climbing stack height | 3 elevated items |
| `WORLDINT0-RL-14` relocation hops per step | 1 |
| `WORLDINT0-RL-15` environment damage per hit | at most 1,000; above it the trigger is held |
| `WORLDINT0-RL-16` Tibian minute; Tibian day | 2,500 ms; 1,440 minutes |
| `WORLDINT0-RL-17` light: night, day, step, color, sunrise start, sunset start | 40, 250, 7 per 4 Tibian minutes, 215, 06:00, 17:30 |
| `WORLDINT0-RL-18` `WORLD_CLOCK` deltas per session; payload | at most 1 per 10,000 ms; at most 9 bytes |
| `WORLDINT0-RL-19` clock skew between channels of a World | at most 1,000 ms, alarm above |
| `WORLDINT0-RL-20` VIS-2 `light` field; updates per entity | at most 6 bytes; at most 1 per 1,000 ms |
| `WORLDINT0-RL-21` `PUSH_INTENT` payload; result | at most 32 bytes; at most 4 bytes |
| `WORLDINT0-RL-22` `USE_INTENT` payload with the `map_item` arm of field 4 | at most 529 bytes (unchanged), measured by WORLDINT-WIRE-1 |
| `DUR03-RL-01-GROUND-MOVE` touched items | 1 (a tree: `DUR03-RL-01-TREE-MOVE`, 500) |
| `DUR03-RL-02-GROUND-MOVE` location lines | 2 |
| `DUR03-RL-06-GROUND-MOVE` participants / work units | 1 / 3 (a tree: 1 / 502) |
| `DUR03-RL-07-KEY` MINT and TRANSFER payload | the existing one-item caps plus one `key_number` (at most 4 bytes) |

`QUESTGATE0-RL-03` (10), `RUNEUSE0-RL-05` (1), `RUNEUSE0-RL-06` (20,000), `ITEMMOVE1-RL-01` (10),
`ITEMMOVE1-RL-02` (20,000) and `MAP01-CHANNEL-OVERLAY-BYTES` are unchanged and bind every act here.

## 13. Rejected options

- **Durable world-object state** (door and lever rows). Owner 6a and D38 W2; a crash restarting a
  channel is Tibia's restart.
- **World-shared doors and levers across channels.** Owner 6a; it would need cross-channel writes
  in the authoritative path.
- **A script engine for levers and tools.** DUR-04's component host is the future path; the closed
  child set covers the transcribed shared mechanisms, and the rest stays bespoke.
- **Keys as one Item definition per key number.** Tibia keys are a base form plus a number; a
  definition per number multiplies the catalogue and breaks the A12 numbering.
- **A clock per channel, or a clock broadcast between channels.** Owner 7a.
- **Server-computed light per tile.** Light is presentation; the server sends the World light and
  creature light only.
- **Player-affecting fields before the admission rule.** A player could be hurt or blocked by a
  field its client cannot see (`RUNEUSE0-C7`).
- **Queueing exhausted acts** (Canary's push and use tasks). Unbounded per-actor queues; the client
  repeats.

## 14. Owner-rule applications (owner rule 5905825574)

**Global parity kept:** doors of every Tibia kind, opened and closed by anyone on the channel; key
doors locked and unlocked with numbered keys until the reset; quest, level, Premium and vocation
gates per character with push-back and closing behind; levers that move doors, walls, bridges and
teleports for everyone on the channel; ladders, grates, stairs, holes, ropes, shovels, picks and
machetes; climbing on stacked items; roping up another player; pushing creatures and players,
not out of a protection zone; pushing a player down a hole; throwing items into holes and
teleports; fields that burn, shock and poison players; Magic Wall and Wild Growth blocking and their
Optional-world and machete destruction; spike traps; one Tibian day per real hour with midnight on
the full hour; day and night light; light spells and lit equipment; every world object reset at
server save.

**Declared differences:**
- A relocation landing (teleport, ladder, grate, rope) on a PZ tile is refused under a PZ block
  (R1); Canary ignores the block for scripted relocations; Global unsourced (`PARITY_PENDING`).
- A shovel opens the hole and the digger walks onto it (TibiaWiki); Canary moves the digger at once.
- Items cannot be dropped into water (refused) instead of being destroyed: no new sink.
- A door tile refuses dropped items (R3, `PARITY_PENDING`).
- Map-item `USE` has its own 200 ms key, not shared with food (R4); a use or push during its
  cooldown is refused, not queued.
- NPCs cannot be pushed (`PARITY_PENDING`).
- Channels of a World may differ by up to 1 s of clock (`WORLDINT0-RL-19`).
- A channel crash resets only that channel's objects (owner 6a: each channel is its own server).
- The light curve is Canary's, as a function of the epoch (`PARITY_PENDING`).
- Deferred, not different: carried torches, the Trap item, digging yields.

**Architect rulings:**
- **R1. PZ block on landings.** a) Refuse every landing on a PZ tile under a PZ block
  (recommended: a block cannot be escaped by a ladder or teleport; fail closed); b) Canary: only
  walks and the shovel check it. **Ruled a).**
- **R2. Where the key number lives.** a) An immutable typed capability on the instance, set at MINT
  (recommended: Tibia's model, GAME-ITEM-01 §4 typed state, no attribute bag); b) one definition
  per number. **Ruled a).**
- **R3. Items on door tiles.** a) Refuse (recommended: closing never meets a loose item and needs no
  displacement rule); b) displace on close, a second relocation rule for items. **Ruled a).**
- **R4. Cooldown keys.** a) Own keys `world_action`, `world_ex_action`, `push` (recommended: no
  coupling with ITEM-USE-0's accepted keys); b) share Canary's one action gate with food. **Ruled
  a).**
- **R5. Channel cap.** a) 512 acts per channel per tick, refused above (recommended: bounds the
  overlay writer; D37 forbids queues); b) no cap. **Ruled a).**

## 15. Owner questions

None. Owner answers 6a and 7a fix scope and the clock; every other choice is a Tibia-parity
application or a bound under DUR-03 §28 and owner rule 5905825574.

## 16. Decision test

- **Must decide now:** YES. `GAME-INTERACTION-01` is required for Alpha; doors, levers, ladders and
  teleports gate most of the map and 204 quests; RUNE-USE-0, QUEST-GATE-0, ITEM-USE-0 and
  ITEM-MOVE-WIRE-1 each defer a part here.
- **Minimum sufficient:** reuses the D37 relocation, the D38 overlay, the QUEST-GATE-0 gates and
  triggers, the ITEM-USE-0 fields, the MAP-WIRE-1 handle and `TILE_SET`, the RUNE-USE-0 field
  overlay and the ADR-0021 reset; adds one closed child kind, one item capability, one DUR-03 move
  shape, one command, two capabilities, one domain, one entity field and a pure clock function.
- **Superseding evidence:** official reach, exhaust, push, field-caster or light-curve values;
  Global evidence on PZ-blocked teleports or items on door tiles.
- **Deliberately not decided:** carried torches and timed items, the Trap item, digging and
  harvest yields, writing, beds, boss levers and rooms, Destroy Field and the native-behaviour
  runes, items lost in water, the stairhop pacification (combat owner).

## 17. Before-freeze checklist

1. **Contract amendments:** the GAME-INTERACTION-01 horizon section; the scope matrix; RUNE-USE-0
   §11; QUEST-GATE-0 §3.1; PARTY-PVP-0 §7.2 rule 6; DUR-03 §39.1; GAME-ITEM-01 §4; each pending on
   acceptance of WORLD-INTERACTION-0. The FND-04 admission amendment is WORLDINT-ADMIT-1's.
   Capability, command, domain and field numbers are reserved at allocation; CLOCK-1 amends
   MOVE-RL-11 at allocation.
2. **Serialization:** one owner lane per channel or instance; acts in input order, capped per tick;
   overlay change, relocation and `TILE_SET` in one tick; a Ground move locks item rows, then both
   tile rows in tile key order (rule 4); replay by CommandRef.
3. **Restart:** world-object and field state volatile per channel; Ground items, keys, quest tracks
   and positions durable; overlays empty after every planned reset.
4. **Typed references:** `map_item_handle`, `placement_key`, LocalObject state keys, FloorChange
   and Transition.Teleport records, `key_number`, actor identities, `world_clock_epoch_utc_ms`.
5. **Wire:** §10, capabilities `WORLD_INTERACTION_V1` and `WORLD_LIGHT_V1`.
6. **Split work:** one act per command; at most 16 children per firing; one item (or one tree) per
   Ground move; at most 8 landing candidates; one hop per relocation.
