# CREATURE-MOVE-1 packet: creature steps, paths and movement goals

- Packet: `CREATURE-MOVE-1-PACKET-1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings and the packet below.
  CREATURE-AI-0 stays a candidate for everything else it decides. This packet implements its
  movement semantics (§5) and the path part of its budgets (§7).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane decision queue of 2026-10-04, item 2. CREATURE-MOVE-1 blocks:
  - creatures that walk at their own speed;
  - CHASE-1 (RANGED-0 §8);
  - NPC-ACTOR-1 (NPC-BEHAVIOUR-0);
  - SUMMON-1.
- Builds on:
  - CREATURE-AI-0 (`OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md`)
    §5, §7, §10 and §13 (R2, R3, R5);
  - CONDITIONS-0 §4.1-§4.3;
  - SPEED-1 (#1715, `movement/speed.rs`);
  - CREATURE-AI-1-PACKET-1 (#1763) §1.1, §1.4 and §1.7;
  - SPAWN-1A-PACKET-1 (#1745) §1.6;
  - ARCH-CORE-LOOP-PACKETS-2 (#1735) §0.2 and §0.3;
  - the first slice §4.5 (`movement.rs` `step_cardinal`);
  - `foundation/owner_timer.rs` (`DEADLINE_STATE`);
  - SIM-DETERMINISM-01.
- Amends, in this PR:
  - CONDITIONS-0 §2 and §3.4, and CREATURE-AI-0 §2 and §6.2: the creature speed draw (§1.2 here);
  - the #1735 §0.3 writer lists (§0 here).
- Runtime, migration, registry, protocol and production authority: NONE. The packet needs its #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Order and shared files

CREATURE-MOVE-1 is the last packet of the #1735 §0.2 runtime lane. It comes after CREATURE-AI-1 and
SPAWN-1a. It needs no wire change, no capability and no migration. Steps still commit through
`step_cardinal`, as the first slice's steps do, so observers see them the same way.

This PR adds CREATURE-MOVE-1 at the end of two writer lists:

| File | Writers, in order |
|---|---|
| `apps/game-server/src/foundation/runtime_actor_carrier.rs` | DEATH-2, ATTACK-1b, CREATURE-AI-1, SPAWN-1a, **CREATURE-MOVE-1** |
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | the #1735 §0.3 list as CREATURE-AI-1-PACKET-1 §0 amends it, then **CREATURE-MOVE-1**; each writer edits only its own rows |

`ai_think.rs` and `ai_think/**` are written by CREATURE-AI-1, then CREATURE-MOVE-1.

## 1. Rulings

### 1.1 The step timer replaces the step per think

- The think chooses the movement **goal**: chase, keep distance, dance, flee, wander, walk back or
  stand (§1.5). It never steps.
- Each creature has a **creature step** timer: `DEADLINE_STATE`, at most one pending, due after the
  step duration (§1.3). A creature has at most two pending AI timers, one think and one step
  (`RL-11`). This replaces `AI01-PENDING-TIMERS-PER-ACTOR` 1.
- A firing step:
  1. takes the next step of the adopted path, or the local step its goal yields;
  2. submits it through `step_cardinal` with the creature's own `ExactActorRef`, revalidated like a
     player's;
  3. re-arms the timer at the next duration while the goal still yields a step.
- A refused step drops the adopted path. The creature waits for its next think. It never retries at
  once.
- The CREATURE-AI-1 §1.1 greedy step per think and its §1.4 wander step are removed. Their goals
  remain, with the mechanics below.
- An idle creature has no pending step. Going idle cancels the step timer.

### 1.2 Creature speed: no draw

- **Superseding evidence.** CONDITIONS-0 §2 says that a monster's speed is drawn at spawn
  between half and all of its content speed. It cites `monsters.cpp:153-154`. At Canary `04b83b51`
  those lines are the `speed` spell's `speedChange` multiplier, not a spawn draw. The actual sources:
  - `Monster::Monster` sets `baseSpeed = mType->getBaseSpeed()` (`monster.cpp:100`).
  - `registerMonsterType.speed` passes `monster.speed` through unchanged
    (`register_monster_type.lua:131-133`).
  - The rat file has `monster.speed = 67`, the value SPAWN-1a carries (#1745 §1.6).
- **Ruled:** a creature's base speed is its content `speed`. Nothing is drawn. `MONSTER_SPEED_DRAW`
  is retired: CONDITIONS-0 §3.4 and CREATURE-AI-0 §2 and §6.2 carry this amendment, and no code
  defines the purpose.
- The effective speed is the base speed plus the active `SPEED` condition delta, read through
  `actor_active_speed_delta`. Creatures have no equipment term.
  - A base speed of 0 never steps. Canary stops walking at speed 0 (`creature.cpp:1655-1657`), and
    content uses 0 for immobile creatures.
  - A base speed above 0 is clamped to `[10, 65,535]` (CONDITIONS-0 §4.1).

### 1.3 Step duration

- The step duration is `StepSpeedTable::step_duration(speed, ground speed of the destination
  tile)`. It uses the SPEED-1 ground speed source, which stays at 150 until MAP-CLIENT-1.
- **The ×2 near the target** follows Canary exactly:
  - The creature keeps a near counter from 0 to 2.
  - Each chase or keep-distance evaluation (§1.5) adds 1 (up to 2) when the target is within 1 tile
    in x and in y. Otherwise it takes 1 away (down to 0). Source: `Monster::getDistanceStep`,
    `monster.cpp:2651-2658`.
  - The duration doubles while the counter is at least 1, the creature is not fleeing and it is not
    a summon. Source: `Monster::isTargetNearby`, `monster.cpp:3128-3130`, and
    `Creature::getStepDuration`, `creature.cpp:1621`.
  - The counter is runtime state. It resets at admission.
- Diagonal steps do not exist (`CardinalStep`), so the ×3 never applies (§3).

### 1.4 Paths

The CREATURE-AI-0 §5.3 profile applies unchanged, over four cardinal neighbours:

| Element | Value |
|---|---|
| neighbour order | north, east, south, west |
| step cost | 10 per step |
| creature on the tile | +40 |
| tie order | lowest total, then lowest estimate, then neighbour order, then discovery order |
| expansions per search | at most 1,024 (`AI01-PATH-SEARCH-WORK`) |
| route length | at most 128 steps (`AI01-ROUTE-STEPS`, `AI01-ROUTE-BYTES`) |
| search distance | 12 tiles for a chase; the 50-tile leash distance for a walk back |

- **Walkability** is the Movement owner's own step-admission predicate, read without mutation:
  - the base map;
  - the overlay (closed doors, walls, blocking items);
  - protection-zone, floor-change and teleport tiles, which are never entered (R2).

  There is no second walkability table. Creatures never open doors.
- **Harmful fields.** The +180 field cost and `walks_on_*` wait for a field owner, because no
  runtime field exists yet. The cost function takes a field term that is 0 until then, and a test
  pins the +180 with a stub term.
- **Requests and staleness.** The rules of §5.3 apply:
  - at most one request per think (`RL-10`), and one pending per creature, updated in place;
  - a request binds (creature `ExactActorRef`, bundle revision);
  - an adopted path binds (creature `ExactActorRef`, goal, bundle revision);
  - a path is dropped when the target changes or a step is refused.
- **Budget on the writer** (R3). Searches run on the channel owner inside `RL-09`: 65,536 work
  units per 50 ms window, one unit per expansion. Order:
  1. pending creature searches, by deadline, then `ExactActorRef`;
  2. player chase searches (`RANGED0-RL-03`), when CHASE-1 lands.

  A request over the budget waits for the next window. Meanwhile the creature keeps its previous
  path if it is still valid, or holds position. A search never fails because of the budget.
- **Reached.** CREATURE-AI-1 §2 item 4 used "cannot be attacked from here" until now. A failed
  chase search is now "cannot be reached", which triggers a target search on the next think.
- The bootstrap `ai/path_proposal.rs` stays the immutable route carrier. Search results are handed
  over as `PathProposal`, so its revalidation is reused.

### 1.5 Goals (§5.4, §5.5)

| Goal | When | Mechanics |
|---|---|---|
| **chase** | target, `target_distance_tiles` ≤ 1, not fleeing | a path to a tile adjacent to the target. Adjacent with a clear step, it holds position |
| **keep distance** | target, `target_distance_tiles` > 1, not fleeing | Canary `getDistanceStep`: one local step closer when farther, away when nearer. A path is requested only when no tile at the distance can be reached by local steps, or sight is not clear |
| **dance** | adjacent to its target, chasing | before each step, a draw with `AI_DANCE`. With chance 1 − `static_attack_chance_ppm`, a side step that keeps adjacency. None admits: it holds |
| **flee** | CREATURE-AI-1 §2 item 6 | a local step that increases the distance, while within 11 tiles of the target (sight not required). Cornered, a side step. None admits: it holds. The near ×2 never applies |
| **wander** | no target, a perceived player, `movement.wander` | after each `interval_ms`, one step drawn with `AI_WANDER` (CREATURE-AI-1 §1.4) within `radius_tiles`, through the step timer |
| **walk back** | no target, no perceived player, away from its spawn cell | a path to the cell (search distance 50). On arrival, idle. A failed search idles in place until the next wake (R5) |
| **stand** | anything else, speed 0 | no step |

**Leash.** A spawned creature that is more than 50 tiles in x or y, or 2 floors, from its spawn
cell is relocated at the think to the cell, then goes idle. If the cell cannot admit it, it idles in
place.

The Movement owner has no relocation entry yet. CREATURE-MOVE-1 adds `relocate_creature` to
`movement.rs`:
- it moves the creature's own `ExactActorRef` to one cell;
- it admits the cell with the same predicate as a step, and refuses a player actor;
- it commits through the same owner commit path, as one position change.

No other caller is added. Creatures with no spawn point (the AI-2 test path,
summons) have no leash and no walk back.

### 1.6 Rows

CREATURE-MOVE-1 registers these rows before implementation (CREATURE-AI-0 §10):

| Row | Value | Note |
|---|---|---|
| `CREATUREAI0-RL-09` | 65,536 path work units per channel per 50 ms window | the rest wait, §1.4 |
| `CREATUREAI0-RL-10` | 1 path request per creature per think | within `AI01-PATH-REQUESTS-PER-ACTOR` 2 |
| `CREATUREAI0-RL-11` | 2 pending AI timers per creature (one think, one step) | replaces `AI01-PENDING-TIMERS-PER-ACTOR` 1 |
| `CREATUREAI0-RL-17` | AI work (thinks and paths) per 50 ms window: p99 at most 15 ms | measured, §1.7 |

### 1.7 The `RL-17` measurement

No reference host is defined, so the measurement is evidence, not a CI gate.

- An ignored test (`#[ignore]`, run on demand) fills one window:
  - 1,024 due thinks (`RL-05`);
  - 64 full 1,024-expansion searches (`RL-09`) on a 128 × 128 open floor with obstacles.
- It reports the p99 window time over 200 windows.
- The worker records the result, the host's CPU model and the command in the task record.
- Above 15 ms, the packet still merges. The control plane gets a `QUESTION` for the new decision
  that lowers `RL-05` and `RL-09` (CREATURE-AI-0 §7). The rows are never raised to hide it.

## 2. Packet

### 2.1 CREATURE-MOVE-1

```yaml
task_id: OTV2-20261004-creature-move-1
decision: CREATURE-AI-0 §5, §7, §10, §13 R2/R3/R5; this packet §1.1-§1.7
worker: oteryn-hard-worker
review: movement, determinism and performance review (Codex, final frozen head)
branch: claude/creature-move-1-20261004
base: main after SPAWN-1a merges (the carrier is serialized, §0)
owned_paths:
  - apps/game-server/src/ai_think.rs
  - apps/game-server/src/ai_think/**
  - apps/game-server/src/ai/{mod,path_proposal,snapshot,tests}.rs
  - apps/game-server/src/ai/path_search.rs              # new: the §1.4 profile and budget
  - apps/game-server/src/ai/movement_goal.rs            # new: §1.5 goals, near counter
  - apps/game-server/src/movement.rs                    # read-only step-admission predicate; relocate_creature (§1.5)
  - apps/game-server/src/movement/speed.rs              # creature_step_duration, §1.2-§1.3
  - apps/game-server/src/foundation/owner_timer.rs      # the creature step family entry
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/channel_owner_creature_move_tests.rs  # new
  - apps/game-server/src/foundation/mod.rs              # the test module line only
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json        # CREATUREAI0-RL-09, -10, -11, -17
  - docs/agents/tasks/archive/OTV2-20261004-creature-move-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - python tools/agents/validate_governance.py
  - python tools/repository/validate_repository_policy.py
  - git diff --check
```

Acceptance:

- **Step pacing.**
  - A rat with speed 67 chasing over ground speed 150 steps at the SPEED-1 duration for 67.
  - Within 1 tile of the target, the duration doubles, with the counter rules of §1.3. Fleeing, it
    does not double.
  - A `SPEED` condition changes the next duration. A base speed of 0 never steps.
- **Timers.** A creature never has more than one think and one step pending. A third timer is
  refused (`RL-11` at max and max+1). Idle cancels the step.
- **Paths.**
  - Around a wall, the path matches a reference expected route, including ties.
  - A search over 1,024 expansions fails as "cannot be reached", and the next think re-searches the
    target.
  - Protection-zone, floor-change and teleport tiles are never entered, and closed doors are never
    opened.
  - A target that goes downstairs is dropped (R2).
- **Budget.**
  - 65 full searches due in one window run 64. The 65th runs in the next window, in deadline then
    `ExactActorRef` order.
  - The waiting creature keeps its valid path or holds position.
  - A second request in one think is refused (`RL-10` at max and max+1).
- **Goals.**
  - With a 1,000,000 ppm static chance, a creature never dances. With 0 ppm, every adjacent step is
    a side step drawn with `AI_DANCE`.
  - Keep distance 4 holds Chebyshev 4 on an open floor.
  - Flee stops at 11 tiles.
  - Wander stays within its radius.
- **Walk back and leash.**
  - Walk back reaches the spawn cell and idles.
  - A failed walk back idles in place.
  - Relocating a creature 51 tiles away returns it to the cell (the leash), or leaves it idle in
    place when the cell is occupied.
- **Refusals.** A refused step drops the path and makes no immediate retry.
- **Replay.** Identical results on replay: same draws, same routes, same order.
- **Cleanup.**
  - No greedy step per think remains in `ai_think*`.
  - No `MONSTER_SPEED_DRAW` symbol exists.
- **`RL-17`.** The measurement is recorded (§1.7).

Not in scope:

- diagonal steps;
- harmful fields beyond the stub term;
- door opening;
- creatures pushing creatures or items;
- player chase (CHASE-1);
- summon following (SUMMON-1);
- NPC walking (NPC-ACTOR-1);
- any wire change.

## 3. Rejected options

- **Implementing the speed draw anyway.** No Canary source draws a spawn speed, and the content
  values match `baseSpeed` directly. A draw would make every creature slower than Tibia without
  evidence.
- **Diagonal neighbours now.** Movement admits only `CardinalStep`. Diagonals need their own
  Movement decision. That decision adds ×3 and +25 together.
- **A second walkability table for paths.** It could disagree with step admission. Reusing the
  predicate keeps one truth, and each step is revalidated anyway.
- **Searching on worker threads.** R3 rules on-writer searches with deterministic deferral.
- **A CI gate on the 15 ms p99.** No reference host is defined. A gate on shared runners would
  flake; that is the measurement evidence §1.7 records.

## 4. Decision test

- **Must decide now:** the speed draw has no Canary source (§1.2). The near ×2 needed an exact rule
  (§1.3). The path walkability source and the field cost had no owner before this packet (§1.4).
- **Blocked work:** CREATURE-MOVE-1, then CHASE-1, NPC-ACTOR-1 and SUMMON-1.
- **Harder later:** creature speed lives in the carrier from SPAWN-1a. A draw added and later
  removed would change every pinned creature test twice.
- **Superseding evidence:** `monster.cpp:100`, `register_monster_type.lua:131-133`,
  `creature.cpp:1612-1625`, `monster.cpp:2651-2658` and `monster.cpp:3128-3130`, all at Canary
  `04b83b51`.
- **Not decided here:** diagonal movement, field owners, door use by creatures, creature pushing,
  summon follow (SUMMON-1).
