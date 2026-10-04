# CREATURE-AI-1 packet: targeting, attacks, defences and the think budget

- Packet: `CREATURE-AI-1-PACKET-1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the scope split, the rulings and the packet
  below. CREATURE-AI-0 stays a candidate for everything else it decides; this packet implements its
  semantics §3, §4 and §7 only.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane decision queue of 2026-10-04 13:15, item 1 (CREATURE-AI-1 blocks
  SPAWN-1a, CREATURE-MOVE-1 and the first creatures that fight).
- Builds on: CREATURE-AI-0 (`OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md`)
  §3, §4, §7 and §10; ARCH-CORE-LOOP-PACKETS-2 (#1735) §0.2 and §0.3; SPAWN-1A-PACKET-1 (#1745)
  §1.5 and §1.6; ATTACK-0 §4; ATTACK-1a (#1737); DEATH-2 (#1742); AI-1 to AI-4 (merged); VIS-1
  (`movement/interest.rs`); GAME-ABILITY-01 (`ProposalSource::Ai`); SIM-DETERMINISM-01 §11 and §12.
- Amends, in this PR: the ARCH-CORE-LOOP-PACKETS-2 §0.3 writer list of
  `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (§0 here).
- Runtime, migration, registry, protocol and production authority: NONE. The packet needs its #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Order and shared files

CREATURE-AI-1 stays in the runtime lane of #1735 §0.2: after ATTACK-1b, before SPAWN-1a and
CREATURE-MOVE-1. It needs no wire change, no capability and no migration.

| File | Writers, in order |
|---|---|
| `apps/game-server/src/foundation/runtime_actor_carrier.rs` | DEATH-2, ATTACK-1b, CREATURE-AI-1, SPAWN-1a (#1735 §0.3, unchanged) |
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | ATTACK-WIRE-1, CHAT-1b-2b, ATTACK-1b, SPAWN-CONTENT-1, **CREATURE-AI-1**, SPAWN-1a; each writer edits only its own rows (this PR amends #1735 §0.3) |
| `apps/game-server/src/ability/creature_bite.rs` | ATTACK-1b, CREATURE-AI-1 |

## 1. Rulings

### 1.1 The split with CREATURE-MOVE-1

CREATURE-AI-1 decides **whom** a creature fights and **what** it casts. CREATURE-MOVE-1 decides
**how** it moves (CREATURE-AI-0 §5, the step timer, paths). Until CREATURE-MOVE-1 merges, the think
keeps the first slice's one cardinal step per think (CREATURE-AI-0 §5.1, last sentence), with the
goal set by this packet:

| Goal | Step until CREATURE-MOVE-1 |
|---|---|
| target, not fleeing | the first slice's greedy step toward the target, unchanged |
| fleeing (§4.5) | one greedy cardinal step that increases the Chebyshev distance from the target; none admits, it stands |
| no target, a perceived player, a profile with `movement.wander` | a wander step (§1.4) |
| anything else | it stands |

Walk back, leash, dance, keep-distance, the flee path and every path search are CREATURE-MOVE-1's.
A creature that goes idle away from its spawn cell idles where it stands until CREATURE-MOVE-1 adds
walk back.

### 1.2 The behaviour profile is one runtime value

- CREATURE-AI-1 adds `CreatureBehaviourProfile`: the projection of the validated v2 behaviour
  authoring (`ProjectV2BehaviorAuthoring`, `content/project/v2/creature.rs`) that the think
  reads: `targeting` (all fields), `movement.wander`, `attacks[]` and `defenses[]`. `summons` and
  `voices` are not projected here (MONSTER-SUMMON-1, a later voices decision).
- The carrier holds one profile per creature actor, given at admission. A creature without a profile
  is refused at admission (`PROFILE_MISSING`). It is never defaulted.
- The think dispatches on profile data, never on a creature key (CREATURE-AI-0 §3).
- **Source.** SPAWN-1a hands the authored `oteryn:behavior/rat-hostile` profile out with the spawn
  source (#1745 §1.6). Until SPAWN-1a merges, the AI-2 test path realizes D116, and the tests build
  the profile from the same values as #1745 §1.6. That profile is test data, not content, like
  `d116_definition()` (#1735 P2 4176975942). CREATURE-AI-1 adds no profile, health or bite literal
  outside tests.

### 1.3 The D115 constants leave the think path

As #1745 §1.6 requires:

| Constant | Replaced by |
|---|---|
| `D115_THINK_INTERVAL_MILLIS` 1,000 | the CREATURE-AI-0 §10 parity value 1,000 ms, renamed `CREATURE_THINK_INTERVAL_MILLIS` |
| `D115_PERCEPTION_RANGE_TILES` 7 | perception by the VIS-1 interest-area predicate (§2 item 2) |
| `D115_WANDER_CHANCE_PERCENT` 25, `D115_WANDER_RADIUS_TILES` 2 | `movement.wander` `{interval_ms, radius_tiles}` (§1.4) |
| the first slice's attack chance draw | the profile's `attacks[]` (§2 item 5) |

Pinned tests are rewritten against the new values. No `D115_*` symbol remains in `ai_think*`.

### 1.4 Wander until CREATURE-MOVE-1

A creature with no target, a perceived player and `movement.wander` adds 1,000 ms to its wander
ticks each think. When they reach `interval_ms`, they reset and it takes one cardinal step drawn
with `AI_WANDER`. The step stays within `radius_tiles` (Chebyshev) of its spawn cell, or of its
admission cell when it has no spawn point. A drawn step that leaves the radius, or does not admit,
is no step. There is no chance draw: the profile has none. CREATURE-MOVE-1 replaces only the step
mechanics.

### 1.5 Melee stays on ATTACK-1's swing

CREATURE-AI-0 §4.4: the entry whose ability is the creature's melee attack is not cast by the
think. ATTACK-1b's creature swing (the `AutoAttack` path through `creature_bite.rs`) swings it.
CREATURE-AI-1 changes three things in that swing:

- It swings only while the creature has a target (§2 item 4), and only at that target.
- Its interval, chance and magnitude are the melee entry's (`interval_ms`, `chance_ppm`,
  `magnitude`). They replace the fixture values of `CreatureBiteDefinition`.
- Its chance draw uses `AI_ATTACK` with the occurrence (creature, swing sequence, list `attacks`,
  entry index).

ATTACK-1b's rules stay as they are: re-entry protection, the protection zone, the in-fight
deadline and the death path. A profile with more than one melee entry is refused at admission
(`PROFILE_INVALID`).

### 1.6 Overrides are a slot, not a key

CREATURE-AI-1 builds the two override slots of CREATURE-AI-0 §4.3: a forced target until a
deadline, and a forced target distance 1 until a deadline (`RL-18`, the newer one replaces). It
also builds one owner-side setter that an Ability commit calls. No Ability `native_behavior` key
calls the setter in this packet. `monster_ai_override` is wired by the spell-lane child that
implements it. Tests drive the setter directly.

### 1.7 Rows

CREATURE-AI-1 registers these rows before implementation (CREATURE-AI-0 §10):

| Row | Value | Note |
|---|---|---|
| `CREATUREAI0-RL-05` | 1,024 thinks started per 50 ms window per channel | the rest stay due, in deadline order then `ExactActorRef` |
| `CREATUREAI0-RL-06` | 128 evaluation units per think | replaces `AI01-EVALUATION-WORK` 8 for creature thinks; max+1 ends the think idle with zero mutation |
| `CREATUREAI0-RL-07` | 24 Ability proposals per think (16 attacks, 8 defences) | admission refuses a profile above it |
| `CREATUREAI0-RL-18` | 2 overrides per creature | the newer replaces |

`AI01-PENDING-TIMERS-PER-ACTOR` stays 1. CREATURE-MOVE-1 replaces it with `RL-11` 2 when it adds
the step timer. The other CREATURE-AI-0 rows belong to these packets:

| Rows | Packet |
|---|---|
| `-01`, `-02`, `-03`, `-13` | SPAWN-CONTENT-1 |
| `-12`, `-14` | SPAWN-1a |
| `-04`, `-15`, `-16` | SPAWN-1b |
| `-09`, `-10`, `-11`, `-17` | CREATURE-MOVE-1 |
| `-08` | MONSTER-SUMMON-1 |
| `-19` | SUMMON-1 |

## 2. Packet

### 2.1 CREATURE-AI-1

```yaml
task_id: OTV2-20261004-creature-ai-1
decision: CREATURE-AI-0 §3, §4, §7, §10; this packet §1.1-§1.7
worker: oteryn-hard-worker
review: combat and determinism review (Codex, final frozen head)
branch: claude/creature-ai-1-20261004
base: main after ATTACK-1b merges (the carrier is serialized, #1735 §0.3)
owned_paths:
  - apps/game-server/src/ai_think.rs
  - apps/game-server/src/ai_think/**
  - apps/game-server/src/ai/{mod,perception,resolution,snapshot,tests}.rs
  - apps/game-server/src/ai/behaviour_profile.rs         # new: CreatureBehaviourProfile (§1.2)
  - apps/game-server/src/ai/targeting.rs                 # new: eligibility, search, change, strategies
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/channel_owner_creature_ai_tests.rs  # new
  - apps/game-server/src/foundation/mod.rs               # the test module line only
  - apps/game-server/src/ability/creature_bite.rs        # melee entry parameters, §1.5
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json        # CREATUREAI0-RL-05, -06, -07, -18
  - docs/agents/tasks/archive/OTV2-20261004-creature-ai-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - python tools/agents/validate_governance.py
  - python tools/repository/validate_repository_policy.py
  - git diff --check
```

It builds:

1. **Idle and wake** (CREATURE-AI-0 §4.1). A creature is idle when it perceives no player, has no
   target, and has no active condition (`runtime_actor_conditions.rs`). An idle creature has no
   pending think. A committed player move or admission wakes the creatures among the VIS-1
   interest candidates of that player (at most `MOVE-RL-09`, 1,024). Each woken creature gets one
   think, due at once, unless one is already pending. Walk back before idling is CREATURE-MOVE-1's
   (§1.1).
2. **Perception** (§4.2). A creature perceives a player when the player's interest area contains
   the creature: the `can_see` predicate of `movement/interest.rs`, under the floor rule. There is
   no second geometry. Candidates are taken in the MOVE-RL-11 canonical order from the creature,
   at most `AI01-PERCEPTION-CANDIDATES` (64), nearest first beyond it.
3. **Eligibility** (§4.2). A candidate is eligible when it is perceived, on the creature's floor,
   alive and attackable. It must not stand in a protection zone, and must not be under re-entry or
   login protection (`ReentryProtection`). A profile with `hostile` false or `can_target` false
   never targets. `sense_invisible` is read and unused until invisibility exists. Player summons
   are SUMMON-1's.
4. **Selection and change** (§4.3), in Canary order:
   - maintenance;
   - search when the creature has no target, or cannot attack the current one;
   - the timed change with `change_target`, drawn with `AI_TARGET_CHANGE`; the new search is
     random when `target_distance_tiles` ≤ 1 and nearest otherwise, drawn with `AI_TARGET_SEARCH`;
   - the strategy draw for a fleeing creature that cannot attack, by `strategy_weights` (nearest,
     lowest health, most damage dealt from the D132 contributor map, random).
   Ties break by Chebyshev distance, then the canonical order. "Cannot be reached" waits for
   CREATURE-MOVE-1's path result. Until then only "cannot be attacked from here" triggers a search.
5. **Attacks and defences** (§4.4). The think adds 1,000 ms to the attack and defence ticks and
   applies the due rule (`ticks ≥ i` and `ticks mod i < 1,000`). It draws `chance_ppm` with
   `AI_ATTACK` and `AI_DEFENCE` and checks `range_tiles` for attacks. The ticks reset when no entry
   waits. This wires the existing pure `ai_think/profile_schedule.rs`. Each passing non-melee entry
   becomes one GAME-ABILITY-01 intent with `ProposalSource::Ai`. Its issuer is the creature's
   `ExactActorRef`. Its occurrence is (creature, think sequence, list, entry index), at most 24
   (`RL-07`). Ability revalidates. A refusal, including an ability that GAME-ABILITY-01 cannot
   execute yet, changes nothing but the next think. The melee entry is the swing's (§1.5). This
   replaces the first slice's one action per think.
6. **Flee** (§4.5). A creature flees while its health is at or below `flee_health` and no override
   holds. It keeps attacking what is in range. Its step is the one in §1.1.
7. **Overrides**: §1.6.
8. **Budget** (§7). An owner window is 50 ms of owner semantic time. The owner starts at most
   `RL-05` thinks per window. Due thinks beyond that wait for the next window, in deadline order
   then `ExactActorRef`. Think timers keep `SKIP_TO_LATEST`. A think over `RL-06` units ends idle
   with zero mutation. Control, fencing, admission and player input never wait for a think.
9. **Randomness** (§4.6). The purposes are `AI_TARGET_SEARCH`, `AI_TARGET_CHANGE`, `AI_ATTACK`,
   `AI_DEFENCE` and `AI_WANDER`. Each is seeded by (creature `ExactActorRef`, think or swing
   sequence, entry index). A retried think never redraws. The first slice's `ai.think.*` purpose
   strings are retired with the D115 constants. `AI_DANCE` is CREATURE-MOVE-1's, and `AI_SUMMON`
   and `SUMMON_PLACE` are MONSTER-SUMMON-1's.

Acceptance:

- With ATTACK-1b on the base and the D116 rats realized through the AI-2 test path, each with the
  §1.2 test profile:
  - a player who walks into view wakes both rats;
  - the nearer rat targets the player and bites every 2,000 ms through the swing;
  - a rat at 5 health or less steps away from the player and still bites when adjacent;
  - when the player walks out of view, the rats go idle and stop costing thinks.
- A player in a protection zone, under re-entry protection or on another floor is never targeted.
  A target that becomes ineligible is dropped on the next think.
- A profile with `change_target` {interval 2,000 ms, chance 1,000,000 ppm} and two eligible players
  re-searches every second think. The results are identical on replay.
- Strategy weights of 100 on one strategy select that strategy's candidate. Ties break by
  Chebyshev distance, then identity.
- A profile with three attack entries and one defence entry emits exactly the due and drawn
  intents with the specified occurrence keys. A profile above `RL-07`, or with two melee entries,
  is refused at admission.
- 1,025 due thinks in one window start 1,024. The 1,025th starts in the next window. A think at
  `RL-06` + 1 units mutates nothing. Each row is tested at max and max+1.
- A forced target holds until its deadline. During it no search runs and the creature does not
  flee. The newer override replaces the older.
- A creature without a profile is refused `PROFILE_MISSING`.
- No `D115_*` symbol and no first-slice `ai.think.*` purpose remains (§1.3).

Not in scope: steps beyond §1.1, paths, walk back, leash, dance, keep-distance (CREATURE-MOVE-1);
spawn realization and respawn (SPAWN-1a); monster and player summons (MONSTER-SUMMON-1, SUMMON-1);
NPC thinks (NPC-BEHAVIOUR-0); invisibility; voices; the `monster_ai_override` key (§1.6); any wire
change.

## 3. Rejected options

- **CREATURE-AI-1 with the step timer and paths.** That would merge two hard reviews (AI
  determinism, movement and performance) into one batch above the size limit. CREATURE-AI-0's
  brief already splits them.
- **A default profile for creatures without one.** It would hide a missing content binding, and
  #1745 §1.6 rules "missing means refused".
- **Keeping the D115 constants until CREATURE-MOVE-1.** #1745 §1.6 gives their replacement to
  CREATURE-AI-1. The D115 wander chance has no source in the content.
- **Melee cast by the think.** CREATURE-AI-0 §4.4 and ATTACK-0 §4 give melee to the swing. Two
  paths would double-bite.

## 4. Decision test

- **Must decide now:** YES. CREATURE-AI-1 is the next packet in the runtime lane after ATTACK-1b,
  and SPAWN-1a and CREATURE-MOVE-1 wait on it.
- **Blocked work:** SPAWN-1a, CREATURE-MOVE-1, MONSTER-SUMMON-1 and the first creatures that fight.
- **Harder later:** yes. SPAWN-1a needs the profile seam (§1.2) to hand out the authored
  behaviour.
- **Superseding evidence:** a merged ATTACK-1b creature swing that already reads profile entries
  (§1.5 then shrinks to the target binding); a CREATURE-AI-0 acceptance review that changes §4.
- **Not decided:** everything in "Not in scope" above. CREATURE-AI-0 stays the semantic owner.
