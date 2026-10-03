# CONDITIONS-0 Actor conditions

- Decision: `CONDITIONS0-ACTOR-CONDITIONS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (combat,
  determinism and protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane's request (#162 5913838260: CONDITIONS-0 next) and the charm lane's
  first priority (5913203231 question 2, architect view a in 5913269950): paralysis, haste and
  cleanse, which also serve spell conditions (S8) and monster attacks
- Builds on: GAME-ABILITY-01 (owner-accepted: the cooldown, charge and condition lifecycle
  baseline, `ConditionDefinition` versus `ConditionInstance`, typed transitions through the Effect
  Plan and commit, explicit conflict policy, immunity, suppression and dispel layers, deterministic
  ticks) and its whole-gate candidate (clauses 5 and 6); SIM-DETERMINISM-01 (named RNG purposes);
  SPELL-D2 (runtime-actor-local vitals); SPELL-D8 H-1 (#1360: durable remaining time, receipt,
  save at lease release, reset at death); ITEM-USE-0 §6.1 (merged: `FoodRegeneration`); the spell
  authoring schema S17; the monster authoring schema §8; GAME-AI-01 D115 (think every 1,000 ms);
  `charm_effects.rs` (`CharmMissingSystem`); DISCONNECT-REENTRY protection (PvE only); ATTACK-0
  (in-fight deadline, domain 10, channel transfer; candidate #1347); owner rule 5905825574
  (Global parity)
- Fills what GAME-ABILITY-01 leaves open (its "not decided here" list): the first condition
  families, conflict keys, values, tick cadence, dispel selection, persistence and the client
  surface.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| COND-1 | hard (combat), combat and determinism review | the `ConditionInstance` store on the runtime actor, transitions, the tick scheduler, the families of §3, provenance (§3.2), dispel (§5), lifecycle (§6) | this decision |
| SPEED-1 | impl, movement review | effective speed, the step-speed table and step pacing for players and creatures (§4) | COND-1; the GAME-AI-01 step cadence (§4.3) |
| COND-WIRE-1 | impl, protocol review | the vitals revision with condition icons and the mana shield, and `speed` on VIS-2 entities (§7) | COND-1; SPEED-1; VIS-2 |
| COND-DUR-1 | hard (persistence), persistence review | the durable remaining food time (§6.3), in the SPELL-D8 H-1 pattern | COND-1; FOOD-REGEN-1 |
| COND-CONTENT-1 | content lane | `ConditionDefinition` records for the spells, runes, potions, food, fields and monster attacks that use §3 families | this decision |

FOOD-REGEN-1 (ITEM-USE-0) builds its definition on COND-1. After COND-1, the charm lane wires
Cleanse and the player-side cases to it. The speed charms (Cripple, Numb, Adrenaline Burst) stay
failing closed (`CharmMissingSystem::ParalysisCondition` / `HasteCondition`) until SPEED-1 and,
for creature targets, the GAME-AI-01 step cadence amendment (§4.3) have landed; before that a speed
condition would change nothing. Their speed values come from one `ConditionDefinition` per charm
(`charm.cripple`, `charm.numb`, `charm.adrenaline_burst`, COND-CONTENT-1); the charm variant keeps
only its `duration_ms` (`charm_effects.rs:153-158`) and names the definition by key.

## 1. Question

How do conditions (poison, burning, haste, paralysis, regeneration and the rest) exist, tick, end
and show?

## 2. Facts

**PROVEN**

- GAME-ABILITY-01 (accepted): a condition is a typed `ConditionInstance` of a versioned
  `ConditionDefinition`; every creation, refresh, removal and tick goes through the Effect Plan and
  the commit; conflict policies are explicit; immunity, resistance, suppression and dispel are
  distinct; ticks are deterministic and bounded. Its whole-gate candidate requires
  `RUN_EACH_BOUNDED` catch-up for anything that cannot be skipped (clause 5), explicit
  continuation at every exercised boundary (clause 6), and an explicit rule for an absent source.
- No condition code exists; `charm_effects.rs` fails closed for paralysis, haste and cleanse.
- Movement has no speed: `step_cardinal` has no step duration and cardinal steps only;
  `StepDisposition` is `Moved`, `Blocked`, `Rejected` (`world_spatial.rs:37-41`).
- Creatures think once per 1,000 ms and take at most one step per think (D115, `ai_think.rs`).
- Vitals are runtime-actor-local (SPELL-D2). `ACTOR_VITALS` is at most 32 bytes and rejects unknown
  fields (`actor_spell.rs:37`); its worst case is 27 bytes (SPELL-D8).
- ITEM-USE-0 §6.1 (merged) defines `FoodRegeneration` with its own key, `FULL` at 1,200 s, no gain
  in a protection zone, coexisting with Recovery.
- SPELL-D8 H-1 (#1360) made the remaining forced Serene time durable: a bounded column, a receipt,
  the save at Character lease release, and a reset at death.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`, OTClient)

- Condition types (`creatures_definitions.hpp:112-150`); damage conditions tick every
  `tickInterval`, default 2,000 ms, never faster than 1,000 ms (`condition.hpp:297-342`,
  `condition.cpp:1786`); fields tick every 5-10 s.
- Speed: one condition; `(a, b)` gives the **new speed** `a × (base − 40) + b` for each end of a
  range, the delta is the draw minus the base, and paralysis never takes speed below 40
  (`condition.cpp:2468-2472, 2546-2552, 2598-2606`); haste and paralysis replace each other
  (`creature.cpp:1216-1222`).
- Player base speed = vocation base (110) + level − 1 (`player.cpp:7331-7339`); effective speed is
  clamped to at least 10 (`creature.cpp:1643`); monster speed is drawn once between half and all of
  its content speed (`monsters.cpp:153-154`).
- Step duration: `floor(1000 × ground speed / step speed)` with
  `step speed = floor(857.36 × ln(speed + 261.29) − 4795.01 + 0.5)` (`creature.hpp:76-78,
  1061-1067`), rounded up to `SERVER_BEAT` (50 ms); × 3 for a diagonal step; × 2 for a monster
  whose target is near (`creature.cpp:1607-1627`).
- Damage over time: a new condition replaces the current one only when its total is strictly
  greater, an undrawn range counting as its midpoint (`condition.cpp:1852-1863, 2082-2093`); a
  field always replaces (`item_parse.cpp:704-706`); a replacement keeps the tick timing, deals one
  tick at once unless delayed, and takes the new owner; standing on a field of the same element
  does not use ticks up (`creature.cpp:1232-1255`); the owner is looked up at each tick and the
  damage has no attacker when the owner is gone; each tick counts as an attack on the target
  (`creature.cpp:1007`); offensive charms do not fire on ticks (`game.cpp:8760-8762`); in a
  protection zone the tick is refused and used up (`combat.cpp:460-462`).
- Mana shield: capacity `min(max mana, 300 + 7.6 × level + 7 × magic level)` (`magic_shield.lua`),
  depleted by damage after block, mitigation and defensive charms and before health; ends at
  capacity 0 or mana 0; a recast resets it; undefined damage bypasses it (`game.cpp:8566-8625`).
- Light: replaces when the new duration is at least the remaining one; the level decays over the
  duration (`condition.cpp:543-557, 2795-2806`).
- Cleanse removes a random negative condition type (`combat.cpp:1049`), gives 11 s immunity to that
  type (`player.cpp:1897-1910`), and cannot remove drowning.
- Timed conditions from default or combat sources are saved at logout (`condition.cpp:475-489`,
  `iologindata_save_player.cpp:279-291`).

## 3. Families, keys and policies (COND-1)

Each `ConditionDefinition` names one family, one conflict key and its values. At most one instance
exists per conflict key.

| Family | Conflict key | Policy on a new application |
|---|---|---|
| `SPEED` (haste, strong haste, paralysis) | `speed` | replace: the newer wins, so haste ends paralysis and the reverse |
| `DAMAGE_OVER_TIME` (poison, fire, energy, bleeding, drown, freezing, dazzled, cursed) | the element | §3.1 |
| `FOOD_REGENERATION` | `food_regeneration` | ITEM-USE-0 §6.1 as merged |
| `RECOVERY` (S17 regeneration spells) | `recovery` | refresh the duration |
| `MANA_SHIELD` | `mana_shield` | refresh the duration and reset the capacity (§3.3) |
| `LIGHT` | `light` | replace when the new duration is at least the remaining one; the level decays linearly over the duration |

- **Values** come from content (COND-CONTENT-1), never from code: tick interval, damage per tick or
  the total and its decay, the speed range `(a_min, b_min, a_max, b_max)`, durations, light level,
  the mana shield capacity formula.
- **Coefficients** are rationals in thousandths (`a = 1300` means 1.3); every product is computed in
  i64 and truncated toward zero once, at the end of each formula.
- **Speed.** The target speed is drawn uniformly in `[a_min × (base − 40) + b_min, a_max × (base −
  40) + b_max]`; the delta is target − base, fixed when applied; a paralysis target is at least 40,
  and a paralysis on an actor whose base speed is below 40 leaves its speed unchanged (delta 0).
- Later families (drunk, invisible, outfit, skill boosts, fear, root) come with their own decisions.
- **Amendment (pending on acceptance of EQUIP-0; `reviews/OTERYN_GAME_EQUIP0_EQUIPMENT_EFFECTS_DECISION_2026-10-01.md` §4).** An active item's `SUPPRESS` refuses admission of an
  instance with the named conflict key and removes an existing one. Skill boosts from equipment are
  derived reads (EQUIP-0 §3), not a condition family.

### 3.1 Damage over time

- A new application replaces the current instance of the element only when its remaining total
  damage is **strictly greater**; an undrawn total counts as the midpoint of its range. A field's
  application always replaces.
- A replacement keeps the current tick timing, deals one tick at once unless the definition is
  `delayed`, and takes the new source as its provenance.
- While the target stands on a field of the same element, its ticks are dealt but not used up.

### 3.2 Tick provenance

- Each tick is one GAME-ABILITY-01 occurrence and one Effect Plan, committed like any damage:
  resistances, the mana shield (§3.3), death. Offensive charms do not fire on ticks; defensive ones
  apply as to any damage.
- Provenance is frozen at application: source actor id, source kind, definition key and revision.
  At each tick the source is looked up; when it is gone (dead, despawned, logged out, on another
  channel), the tick is dealt with no attacker. Kill and loot credit follow the existing damage
  record (the DEATH and ATTACK-0 owners); an absent-source tick adds no entry to it.
- A tick is an attack on the target: it refreshes the target's in-fight deadline (ATTACK-0).
- In a protection zone a damage tick is refused and used up.

### 3.3 Mana shield

- The instance carries `capacity` (content formula, at most maximum mana) and `remaining`.
- Stage: after block, mitigation and defensive charms, before health, as a typed companion
  consequence of the damage. Damage up to `min(remaining, mana)` goes to mana; the rest to health.
  Damage of an undefined element bypasses it.
- It ends when `remaining` or mana reaches 0, or at its duration.

### 3.4 Determinism

- **Draws** use named SIM RNG purposes: `COND_SPEED_DRAW`, `COND_DOT_TOTAL_DRAW`,
  `COND_CLEANSE_PICK`, and `MONSTER_SPEED_DRAW` at spawn (SPEED-1 with the AI-2 spawn owner).
- **Tick order.** Ticks due in one simulation tick run in the key `(due tick, actor id, instance
  sequence)`, where the instance sequence is a per-actor monotonic u32 assigned at creation.
- **Overload.** At most `COND0-RL-03` damage ticks per actor per simulation tick; later ones run in
  the next simulation ticks in the same order (`RUN_EACH_BOUNDED`); none is dropped.
- **Instance limit.** One instance per conflict key gives at most 13 today, under `COND0-RL-01`
  (16); a family that would pass it needs an amendment, and a transition past it is refused.

## 4. Speed and steps (SPEED-1)

### 4.1 Effective speed

- Player base speed = 110 + level − 1 (vocation data); creature base speed is drawn at spawn (§2).
- Effective speed = base + the `SPEED` delta + worn-equipment speed (from the item definitions,
  summed by the equipment owner), clamped to `[10, 65,535]`.

### 4.2 Step duration

- **Step-speed table.** SPEED-1 generates, once and offline, a u16 table from the clamped speed
  range (10..65,535, where the formula is positive) to Canary's step speed, and checks it in as content with its digest; the runtime and the client read
  the table and never evaluate `ln`.
- Step duration = `floor(1000 × ground speed / step speed)` ms (ground speed from the tile's ground
  item definition, default 150), rounded up to a multiple of 50 ms (`SERVER_BEAT`); × 2 for a
  monster whose target is within its near range; × 3 for a diagonal step, when diagonal steps exist.
- Parity values, not resource limits: 1,200 s food, × 3 diagonal, × 2 near target, 50 ms beat.

### 4.3 Pacing

- **Players.** One step request may wait in a one-step buffer until the previous step's duration
  has passed; it then runs. A second early request is refused with a new disposition `TOO_EARLY`,
  registered behind capability `PACED_MOVEMENT_V1`; a session without it gets `Rejected`.
  **Amendment (pending on acceptance of RANGED-0; `reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md` §8).** A chase step is
  a step request from a server source on the same pacing clock and buffer; a client step request
  cancels a pending chase step instead of being refused as early.
- **Creatures.** GAME-AI-01 keeps its 1,000 ms think for decisions; SPEED-1 needs its movement
  cadence changed so that a creature takes each step of its chosen path at its step duration
  between thinks (Canary's walk events). This is requested as a GAME-AI-01 amendment, carried by
  SPEED-1's allocation; until it is accepted, creature speed has no effect.
  **Amendment (pending on acceptance of CREATURE-AI-0; `reviews/OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md`
  §5.1).** CREATURE-AI-0 is that amendment: a creature step timer (`DEADLINE_STATE`, one pending)
  runs each step of the adopted path at the step duration of §4.2, built by CREATURE-MOVE-1 after
  SPEED-1. SPEED-1 no longer carries it.
- SPEED-1 re-measures the existing movement tests against pacing.

## 5. Dispel (COND-1)

- Dispel is a typed action with a closed selector: one conflict key (`exana pox` removes poison),
  or the tag `negative` (damage over time and paralysis; later drunk and similar). A selector names
  how many it removes (1 or all).
- **Cleanse** (charm): the candidates are the target's `negative` instances except drowning, in
  instance-sequence order. With none, nothing happens and no draw is made; with one, it is removed
  without a draw; with two or more, one uniform draw through `COND_CLEANSE_PICK` picks the index.
  The target is then immune to that conflict key for 11 s, which refuses new applications at
  admission.
- **Cleanse immunity** is per-actor state, not an instance: `{conflict key, remaining time}`, at
  most one per key. It crosses reconnects and channel transfers with the instances (§6.2) and ends
  at death and at a fresh admission.
- Removal is forward-only: committed ticks stay committed.

## 6. Lifecycle (COND-1)

### 6.1 Rules

- **Death** removes every instance.
- **Re-entry protection** (PvE only, the owner decision): during the 4 s, new creature attacks do
  not apply conditions to the protected player, and the player's own offensive actions apply none;
  running ticks keep running, and fields and PvP are unaffected.
- **Immunity:** a creature's content lists its condition immunities; an immune target refuses the
  transition at admission.
- **In-fight** stays with ATTACK-0.

### 6.2 Boundaries

- **Reconnect and post-grace recovery** keep the instances with the runtime actor.
- **Channel transfer** carries every instance in the transfer continuation: definition, provenance,
  remaining duration, the offset to the next tick, the instance sequence, capacity and remaining
  values, plus each Cleanse immunity with its remaining time. The target channel resumes them; no
  tick is dealt twice or skipped, and the source channel deals none after the handover.
- **Fresh admission** starts with no instances, except §6.3.

### 6.3 Persistence

- **V1:** only the remaining `FOOD_REGENERATION` time is durable (COND-DUR-1): a column capped at
  1,200 s, written with a receipt at the Character lease-release save, reset at death, exactly as
  SPELL-D8 H-1 did for the forced Serene time. A fresh admission restores it as a new instance.
- Other timed conditions are not kept at logout. Canary keeps them; this is a declared
  `PARITY_PENDING` gap until DUR-02 durable vitals, when they follow the same pattern.

## 7. Client surface (COND-WIRE-1)

- **Own actor.** A new `ACTOR_VITALS` revision, behind capability `CONDITIONS_V1`, adds
  `condition_icons` (a 32-bit mask of the condition bits only: poison, burn, energy, drowning,
  freezing, dazzled, cursed, bleeding, haste, paralyze, mana shield, hungry) and the mana shield's
  `remaining` and `capacity`. Its cap is 48 bytes (worst case about 43: the 27-byte revision, a
  5-byte mask field and two 5-byte values with tags), acknowledged on #162 at allocation; the old
  revision stays at 32 bytes for sessions without the capability.
- In-fight is shown from ATTACK-0 domain 10; the protection-zone block belongs to PARTY-PVP-0 and the
  protection zone to tile state (MAP-WIRE-1). Neither is in this mask.
- **Other actors.** The VIS-2 `WORLD_SPATIAL_ENTITIES` entry gains `speed` (u16) behind the same
  capability; the client times walk animations from it, the tile's ground speed and the step-speed
  table (§4.2).

## 8. Rows

| Row | Value |
|---|---|
| `COND0-RL-01` instances per actor | 16 |
| `COND0-RL-02` minimum tick interval | 1,000 ms |
| `COND0-RL-03` damage ticks per actor per simulation tick | 4 |

## 9. Rejected options

- **All conditions durable now.** Vitals are not durable yet; only food time, whose loss players
  notice at every logout, becomes durable.
- **A separate timer per condition.** GAME-AI-01 forbids private timers; ticks use the simulation
  scheduler.
- **Values in code.** Content owns values; code owns families and policies.
- **Keeping haste and paralysis both.** Canary and Tibia keep one speed condition.
- **Evaluating `ln` at runtime.** Floating results differ across platforms; the table is exact.
- **Oldest-first cleanse.** Canary and Tibia pick at random.

## 10. Decision test

- **Must decide now:** YES. Charms, spells, monster attacks and food need it, and movement needs
  speed.
- **Minimum sufficient:** six families, closed conflict keys, one dispel selector, runtime
  instances except food time, one vitals revision and one entity field.
- **Superseding evidence:** official tick or refresh rules; DUR-02 durable vitals.
- **Deliberately not decided:** drunk, invisible, outfit, skill boosts, fear, root; diagonal steps;
  condition timers in the client.

## 11. Before-freeze checklist

1. **Contract amendments:** none now; COND-WIRE-1 amends the vitals and spatial wire contracts,
   SPEED-1 carries the GAME-AI-01 cadence amendment.
2. **Serialization:** transitions and ticks commit on the owning channel runtime in the §3.4 order.
3. **Restart:** runtime instances end with the actor; food time is durable (§6.3).
4. **Typed references:** condition definition key and revision, source provenance, actor ids,
   instance sequence.
5. **Wire:** §7.
