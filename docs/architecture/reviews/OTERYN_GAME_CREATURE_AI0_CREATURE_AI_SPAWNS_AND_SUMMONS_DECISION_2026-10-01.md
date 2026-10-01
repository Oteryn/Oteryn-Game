# CREATURE-AI-0 Creature AI, spawns and summons

- Decision: `CREATUREAI0-CREATURE-AI-SPAWNS-SUMMONS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (AI
  determinism and performance, movement, combat; protocol for §8.6) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner decisions of 2026-10-01 on #162: 4a (full Tibia creature behaviour in the
  base game, driven by creature data), 5a (player summons in the base game) and 6a (world object
  state is per channel and resets at the server save); the open `GAME-AI-01` items of the horizon
  and of gap register §11; the GAME-AI-01 cadence amendment that CONDITIONS-0 §4.3 requests.
- Builds on: GAME-AI-01 (gate accepted; contract candidate §7-§20 as accepted semantics); the
  bootstrap slice (owner-accepted); the first creature slice (D53-D57, D115, D116; AI-1 to AI-4
  merged); MOVE-RL-11 (D84-D87, D222); ATTACK-0 §4; CONDITIONS-0 §2 and §4; GAME-ABILITY-01
  (`ProposalSource::Ai`, the typed AI issuer of AI-4); PARTY-PVP-0 §5 and §7.1; D3 (D121, D132);
  DUR-03 A4 (D52); VSL-COMBAT-01 (death, loot, XP); BOSS-RAID-0 (branch `claude/arch-boss-raid-0`:
  boss spawns, raids); ADR-0021 (D188, D191, D194, `WorldReset`); the scope matrix row "Creature
  and spawn runtime"; SIM-DETERMINISM-01 §11 and §12; the monster authoring schema; the spell
  native behaviours candidate (S27 keys `acquire_summon` and `monster_ai_override`);
  DISCONNECT-REENTRY protection; owner rule 5905825574 (Tibia-faithful)
- Amends, each pending on acceptance of CREATURE-AI-0, in this PR: the first creature slice §4.4
  and §4.9 (a paragraph after §4.9); CONDITIONS-0 §4.3; the `GAME-AI-01` entry of the horizon;
  gap register §11.
- Runtime, migration, registry, protocol and production authority: NONE. Each child needs its own
  #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| SPAWN-CONTENT-1 | content lane | the spawn family of the World bundle from the reference map (§6.1): 51,896 sources and 83,286 points of Canary `otservbr-monster.xml`, bound to admitted creature definitions; validation of `CREATUREAI0-RL-01` to `-03` and `-13` | the ADR-0021 bundle compiler (MAP-OVERLAY-1, MAP-CUTOVER-1) |
| SPAWN-1 | hard, performance review | realization at activation in windows, respawn with player blocking, the spawn warning and the occupancy chain, the point link, the rows of §10; activation and memory measurement (§6) | AI-2 (merged); SPAWN-CONTENT-1; VIS-1 |
| CREATURE-AI-1 | hard (AI), combat and determinism review | idle and active, perception from the interest index, target selection and change, flee, attacks and defences as Ability proposals, AI overrides, the think budget (§3, §4, §7) | VIS-1; ATTACK-1; AI-4 (merged) |
| CREATURE-MOVE-1 | hard (movement), movement and performance review | the creature step timer (the CONDITIONS-0 §4.3 amendment), the path profile and its window budget, chase, keep-distance, dance, flee, walk back, leash return (§5, §7) | CREATURE-AI-1; SPEED-1 |
| MONSTER-SUMMON-1 | impl, combat review | monster summons from `behavior.summons` (§8.1) | CREATURE-AI-1 |
| SUMMON-1 | hard (combat), combat and security review | the player summon owner link, cap, targeting and following, removal, protection zone and floor rules, attribution (§8.2-§8.5); the runtime that the spell lane's `acquire_summon` children call | CREATURE-MOVE-1; ATTACK-1 |
| SUMMON-WIRE-1 | impl, protocol review | the summon relation field on VIS-2 creature entries (§8.6) | VIS-2; SUMMON-1 |

Order: SPAWN-CONTENT-1 and CREATURE-AI-1 first; SPAWN-1 can realize the map before
CREATURE-MOVE-1 lands (creatures then stand). Later, each with its own decision: creatures pushing
items and creatures (needs a DUR-03 Ground cause for items), voices and sounds, day and night
spawn periods (needs a world clock), the Boosted Creature and Improved Respawn Rate modifiers
(§6.4 hook), invisibility (a later CONDITIONS family), familiars (spell lane C.1), NPC movement,
the player chase mode (ATTACK-0), diagonal steps (Movement), and off-writer parallel pathfinding
(only on measured need, §7).

## 1. Question

How do the World's creatures live on a channel: which owner runs them, what they notice, whom they
attack, how they move, flee and return, how spawns fill and refill the map, how monsters and
players summon creatures, what survives a restart, and how all of it stays bounded and
deterministic without ever blocking the channel writer?

## 2. Facts

**PROVEN**

- Scope matrix: "Creature and spawn runtime": `ChannelRuntime`, Channel, authoritative
  immediate, independent copies, separate per channel. Owner 6a (2026-10-01) confirms world object
  state per channel, reset at the server save; ADR-0021 D191 gives each channel a volatile overlay
  that the planned reset discards; D194 activates a new bundle only at a planned reset.
- GAME-AI-01 accepted semantics: one current channel owner and generation per AI actor; staged
  all-or-nothing resolution; think work arrives as FND-03 owner timers, no fixed tick (§7);
  canonical target selection with a stable tie-break (§8); path results are proposals the owner
  revalidates (§10); controlled actors stay server-authoritative (§16); bounded queues and no
  overload that consumes control capacity (§20).
- The first creature slice (merged as AI-1 to AI-4): the owner timer lane; a think every 1,000 ms,
  perception 7 tiles, 25% wander in radius 2, respawn 60 s, an occupied cell retried 3 times every
  5 s (D115); one spawn of 2 rats (D116); the typed AI issuer through `ProposalSource::Ai`; one
  proposal per think (§4.4); rows `AI01-SPAWN-SOURCES-PER-SCOPE` 16, `AI01-SPAWN-POPULATION` 4,
  `AI01-SPAWN-PLACEMENT-CELLS` 4, `AI01-PENDING-TIMERS-PER-ACTOR` 1 (D57, §4.9: "a larger value
  needs a new owner decision"). Registry: `AI01-ACTIVE-ACTORS` 256, `AI01-EVALUATION-WORK` 8,
  `AI01-PERCEPTION-CANDIDATES` 64, `AI01-PATH-REQUESTS-PER-ACTOR` 2, `AI01-PATH-SEARCH-WORK`
  1,024, `AI01-ROUTE-STEPS` 128, `AI01-ROUTE-BYTES` 4,096.
- MOVE-RL-11: the interest area (18 × 14 in the Reference profile), the floor rule, the canonical
  order (floor distance, Chebyshev distance, identity; actors before items, D222),
  `MOVE-RL-09` 1,024 examined candidates.
- ATTACK-0 §4: a creature's melee is an `AutoAttack` on its own timer through the one pipeline;
  "target choice stays with GAME-AI-01"; a protection zone and re-entry protection refuse attacks.
  CONDITIONS-0 §4: step duration, × 2 near the target, creature base speed drawn at spawn
  (`MONSTER_SPEED_DRAW`), and a requested GAME-AI-01 cadence amendment (§4.3).
- Content (`content/creatures/definitions/`, 1,503 creatures; `content/behaviors/`, 2,605
  profiles): `targeting {hostile, can_target, target_distance_tiles, static_attack_chance_ppm,
  flee_health, sense_invisible, change_target {interval_ms, chance_ppm}, strategy_weights
  {nearest, health, damage, random}}`; `movement {walks_on_*, push_*, pushable, wander
  {interval_ms, radius_tiles}}`; `attacks[]` (at most 16 per profile), `defenses[]` (at most 8),
  each `{ability, interval_ms, chance_ppm, magnitude}`, attacks with `range_tiles`; `summons
  {max_summons (at most 16), entries[] (at most 5) {creature, interval_ms, chance_ppm, count (at
  most 10)}}` in 161 profiles; creature `summoning {summonable (105), convinceable (141),
  mana_cost}` and `spawn_eligibility {blocked_by_nearby_players (true for 5, false for 1,498),
  period (1 creature `Night`), ignore_period_underground}`; 403 profiles flee (`flee_health` > 0).
- DUR-03 A4 and D52: a death key per actor generation; a despawn or scope retirement creates no
  death. D7 (monster schema): summons drop no corpse or residue. D3: the loot right (D121) and at
  most 16 damage contributors per creature (D132).
- PARTY-PVP-0: shared experience gives "a summon's share first" (§5.1); party members and their
  summons are immune to each other (§5.2); "a summon's actions are its owner's" (§7.1); the PvP
  ledger includes summons (§8.3).
- Spell native behaviours (owner-accepted keys, S27): `acquire_summon` (Summon Creature, Convince
  Creature, Animate Dead; cap 2; mana from `summoning.mana_cost`; `summonable` or `convinceable`;
  a convince target must have no master) and `monster_ai_override` (`force_melee`,
  `target_caster`).
- BOSS-RAID-0 (open): boss spawns keep durable per-channel clocks (§5); raid creatures are
  channel-local actors of a raid run (§4.4); encounter creatures follow their encounter.

**CIPSOFT_OFFICIAL** (the Tibia manual, `docs/reference/tibia-manual/`)

- "Respawn is suppressed while a character remains near the spawn point; creatures repopulate
  once the area clears" (`world.md:16`).
- The Boosted Creature respawns faster (`starting.md:69`); one area per server save gets an
  improved respawn rate (`interface.md:42-43`).
- Summons carry an own or other summon marker; party and guild summons are protected like their
  owners (`combat.md:216-218`). In shared experience a summon takes its share first
  (`combat.md:245`). A fatal blow by a summon makes a PvP death (`characters.md:206`).
- Floors change by walking onto stairs, ramps and holes, and by using ladders (`controls.md:27`).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`, read-only)

- Think every 1,000 ms (`creature.hpp:47`). A monster notices creatures within 11 tiles in x and y
  under the floor rule (`map_const.hpp:12-15`; `creature.cpp:68-90`); it targets only a creature on
  its own floor, outside a protection zone, visible unless it senses invisibility, and not login
  protected (`monster.cpp:1434-1497`).
- Target change: every `changeTarget` interval, with its chance, a new search, random for a melee
  creature and nearest otherwise (`monster.cpp:2141-2199`); with no target or a target it cannot
  attack or reach, the nearest (`:1726-1741`); a fleeing creature that cannot attack its target
  draws a strategy by the weights (`:906-925`).
- Flee: health at or below `runonhealth`, not a summon, not challenged (`:1460-1462`); the flee
  path keeps up to 11 tiles away without clear sight (`:3828-3856`). Dance step with chance 1 −
  static attack chance (`:2530-2550`).
- Idle: no target and at its spawn position, idle; no target, away, and no player on screen, walk
  back (`:1521-1556`, `:2501-2528`). Leash: beyond 50 tiles in x or y or 2 floors of its spawn
  position, teleport back and idle (`:1623-1626`, `:3322-3344`; `config.lua.dist:610-612`).
- A monster never enters a protection-zone, floor-change or teleport tile, familiars excepted
  (`tile.cpp:717-719`): monsters do not follow across floors.
- Path: A* of at most 512 nodes (`astarnodes.hpp:37`), search distance 12
  (`creature.cpp:1038-1044`), costs 10 per step, +25 diagonal, +40 over a creature, +180 over a
  harmful field (`astarnodes.hpp:38-40`, `astarnodes.cpp:274-296`).
- Defences and summons each think by interval and chance; a monster summons only with a reachable
  target, not as a summon itself, below `maxSummons` and the entry count, at its tile or a
  shuffled free neighbour (`monster.cpp:2202-2271`; `map.cpp:603-630`).
- Summons attack their master's target, else follow the master within 2 tiles
  (`monster.cpp:1332-1345`, `:3837`). They are removed when the master is more than 30 tiles or 2
  floors away (`creature.cpp:454-499`, `teleportSummons = false`, `config.lua.dist:333`;
  `:1911-1924`), with their master's removal (`game.cpp:1607-1610`), and die with a monster master
  (`monster.cpp:3255-3261`). A creature that was ever a summon drops no loot and gives no
  experience (`creature.cpp:1339-1356`). A summon's experience share reaches its master halved,
  familiars excepted (`creature.cpp:1308-1320`).
- Respawn: a blockable spawn waits while a player is on screen and then a full interval again; a
  non-blockable one shows a teleport effect and spawns 3 × 1,400 ms later; placement is forced
  onto the cell (`spawn_monster.cpp:183-189`, `:270-301`, `:317-345`; `spawn_monster.hpp:97`).
- Reference map `data-otservbr-global/world/otservbr-monster.xml`: 51,896 sources, 83,286 points,
  at most 55 points per source, respawn 5 s to 54,784 s (median 90 s), one creature per tile.
- Convince Creature makes the monster a summon of the caster without leaving its spawn
  (`data/scripts/runes/convince_creature.lua`).

**UNKNOWN** (`PARITY_PENDING` until evidence): the Global perception range, summon experience
halving, respawn-blocking range and the share of blockable spawns (the manual and content
disagree, §13 R6).

## 3. Ownership and representation (CREATURE-AI-1)

- **One owner.** Every creature, spawned, raid, encounter, monster summon or player summon, is a
  creature actor of exactly one scope owner (`ChannelRuntime`, or `InstanceRuntime` for an
  instance) with its `ExactActorRef`. Only that owner mutates it. AI work only proposes: steps go
  to the Movement owner (first slice §4.5), effects to GAME-ABILITY-01. A creature never changes
  channel or scope.
- **Kinds and links.** A spawned creature links to its spawn point (§6); a raid creature to its
  raid run (BOSS-RAID-0 §4.4); an encounter creature to its encounter, whose rules override this
  decision where they say so; a summon to its owner's `ExactActorRef` (§8).
- **Representation.** A typed state machine in Rust over the creature's content profile (the
  existing `ai_think`), dispatching on data, never on a creature's identity. No scripts. Custom
  behaviour enters only through Ability `native_behavior` keys.
- **State.** Runtime only: target, target-change ticks, attack and defence ticks, summon links,
  overrides, adopted path, idle flag. Nothing here is durable (§6.5).

## 4. Perception and targeting (CREATURE-AI-1)

### 4.1 Idle and active

- A creature is **idle** when it perceives no player and no player summon, has no active
  condition, and is at its spawn cell, has no spawn point, or failed its walk back (§5.5). An idle
  creature has no pending think and costs nothing.
- **Wake.** When a committed move or admission puts a player or a player summon within a
  creature's perception (§4.2), the owner schedules that creature's think, due at once (at most
  one pending, `RL-11`). The wake set is the creatures among the candidates the VIS-1 interest
  query examines for the moved player or player summon, from its own position (at most
  `MOVE-RL-09`, 1,024 each), so a summon away from its owner still wakes the creatures it
  reaches.
- A summon is never idle while its owner is on the channel.

### 4.2 Perception

- A creature **perceives** a player or player summon when the creature lies inside the MOVE-RL-11
  interest area centred on that player or summon (§4.1 there, the Channel's size and the floor rule). So a creature
  notices exactly the players who can see it (architect ruling R1).
- Candidates are taken from the interest index in the MOVE-RL-11 canonical order measured from the
  creature, at most `AI01-PERCEPTION-CANDIDATES` (64), nearest first beyond it.
- **Eligible target:** perceived, on the creature's floor, alive, attackable, not in a protection
  zone, not under re-entry or login protection, not invisible unless the profile has
  `sense_invisible` (when invisibility exists), and for a summon the PvP rules (§8.3). A
  non-hostile profile (`hostile` false) or one with `can_target` false never targets.
- **Player-summon targets.** A player summon also targets creatures: its owner's target creature
  is eligible for it when that creature lies inside the MOVE-RL-11 interest area centred on the
  summon and meets the other conditions above (§8.3). Perception of creatures by creatures is used
  for nothing else.

### 4.3 Selection and change (Canary order)

1. **Maintenance.** A target that stops being eligible is dropped.
2. **Search** when there is no target, or the current one cannot be attacked from here or cannot
   be reached (its last path search failed): the nearest eligible candidate.
3. **Timed change.** If `change_target.interval_ms` is not 0, the think adds its 1,000 ms to the
   change ticks; when they reach the interval they reset, and a draw with `chance_ppm` decides a
   new search: random among the eligible candidates when `target_distance_tiles` ≤ 1, nearest
   otherwise.
4. **Fleeing without attack.** A fleeing creature that cannot attack its target draws a strategy
   from `strategy_weights` (nearest, lowest health, most damage dealt to it from its D132
   contributor map, random) and searches with it.
- Ties: Chebyshev distance, then the MOVE-RL-11 canonical order (identity last). Canary keeps list
  order; Oteryn uses the stable order (GAME-AI-01 §8).
- **Overrides** (`monster_ai_override`, S27): an Ability commit may set a forced target until a
  deadline and a forced target distance 1 until a deadline, at most one of each (`RL-18`, the
  newer replaces). While forced, no search or change runs and the creature does not flee.
  Summons ignore every override. Reward bosses ignore only the forced target distance
  (`force_melee`, `skip_reward_bosses`); a forced target (`target_caster`, Challenge) applies to
  them as to any non-summon monster that can target the caster (S27 §D.6.3).

### 4.4 Attacks and defences

- Each think, the attack ticks and the defence ticks add 1,000 ms. An entry with interval `i` is
  due on a think when the ticks are at least `i` and `ticks mod i` < 1,000 (Canary). A due entry
  draws its `chance_ppm`; an attack entry also needs its target within `range_tiles`. Ticks reset
  when no entry waits (Canary).
- Each passing entry becomes one typed GAME-ABILITY-01 intent with `ProposalSource::Ai`, issuer
  the creature's `ExactActorRef`, occurrence (creature, think sequence, list, entry index), which
  extends AI-4's (creature, think sequence). At most 24 per think (`RL-07`). Ability revalidates
  everything (range, line of sight, cooldown, protection zone, re-entry); a refusal changes
  nothing but the next think.
- **Melee.** The entry whose ability is the creature's melee attack is swung by ATTACK-1's
  `AutoAttack` timer (ATTACK-0 §4 rules) with that entry's interval, chance and magnitude, not by
  the think.
- This replaces the first slice's "at most one action per think" (§4.4 there).

### 4.5 Flee

- A creature **flees** while its health is at or below `flee_health`, it is not a summon and no
  override holds. It keeps attacking what is in range. Movement: §5.4.

### 4.6 Randomness

Every draw uses a SIM purpose seeded by (creature `ExactActorRef`, think sequence, entry index):
`AI_TARGET_SEARCH`, `AI_TARGET_CHANGE`, `AI_ATTACK`, `AI_DEFENCE`, `AI_SUMMON`, `AI_WANDER`,
`AI_DANCE`, and `SUMMON_PLACE` (§8.1). A retry of a think never redraws (SIM-DETERMINISM-01 §12).

## 5. Movement (CREATURE-MOVE-1)

### 5.1 Steps and cadence (the CONDITIONS-0 §4.3 amendment)

- The think (1,000 ms) chooses the movement goal. Steps run on a second timer family per creature,
  the **creature step**: `DEADLINE_STATE`, at most one pending, due after the step duration of
  CONDITIONS-0 §4.2. Each step is one Movement owner step of the creature's own `ExactActorRef`,
  revalidated like a player's (first slice §4.5).
- A refused step drops the adopted path; the creature waits for its next think (no immediate
  retry). Until SPEED-1 lands, a creature keeps the first slice's one step per think.

### 5.2 Floors, zones and doors

- A creature step never enters a protection-zone, floor-change or teleport tile (Canary). So a
  creature never changes floor by walking and never chases across floors: a target that goes up or
  down stairs, ladders or holes leaves the creature's floor and stops being eligible (architect
  ruling R2). Summons follow the same rule (§8.3).
- Paths read the channel's committed walkability at the think: base map plus overlay (closed doors,
  magic walls, items) with creatures as cost. Creatures never open doors. Each step is revalidated
  at execution, so no Interaction invalidation feed is needed: this closes `GAME-AI-XD-02` for
  creatures.
- A damaging field of a type the profile does not walk on (`walks_on_*` false) costs +180, as in
  Canary; the creature may still cross it.

### 5.3 Paths

- **Profile.** A* or any algorithm with identical results; neighbours north, east, south, west,
  and the four diagonals once Movement admits diagonal steps; costs 10 per step, +25 diagonal, +40
  over a creature, +180 over a harmful field; ties by lowest total, then lowest estimate, then
  neighbour order, then discovery order. A search is bounded by `AI01-PATH-SEARCH-WORK` (1,024
  expansions) and `AI01-ROUTE-STEPS` (128); search distance 12 tiles for a chase, the leash
  distance for a walk back.
- **When.** At most one request per think (`RL-10`): when the creature has no adopted path, its
  goal moved, or its last step was refused.
- **Staleness.** A pending request binds (creature `ExactActorRef`, bundle revision), not the
  think sequence: a later think that still needs a path updates the goal of the pending request in
  place and keeps its queue position (one pending request per creature), and the search uses the
  goal current when it runs. A request is dropped only when the creature's actor generation or the
  bundle revision changes, or a think no longer needs a path. An adopted path binds (creature
  `ExactActorRef`, goal, bundle revision); it is dropped when any of them changes, when the target
  changes, or when a step is refused.
- **Budget on the writer.** Searches run inside the channel owner's deterministic window budget
  (§7). A request over the budget waits for the next window in canonical order; meanwhile the
  creature follows its previous path if still valid, else holds position. Never a failure, never a
  wait by the writer (architect ruling R3).

### 5.4 Goals

- **Chase.** `target_distance_tiles` ≤ 1: a path to a tile adjacent to the target. Greater: keep
  that distance by local distance steps (closer when farther, away when nearer); a path is
  requested only when no tile in range is reachable by local steps.
- **Dance.** Adjacent to its target, with chance 1 − `static_attack_chance_ppm` per step, a side
  step that keeps adjacency.
- **Flee.** Local distance steps away from the target, up to 11 tiles (Canary), without clear
  sight; cornered, a side step.
- **Wander.** With no target but a perceived player: `movement.wander` (interval and radius around
  the spawn cell); a profile without it stands.
- **Summons:** §8.3.

### 5.5 Walk back and leash

- **Walk back.** No target, no perceived player, away from its spawn cell: a path to the cell. On
  arrival, idle. If the path search fails, idle in place until the next wake (ruling R5).
- **Leash.** A spawned creature beyond 50 tiles in x or y, or 2 floors, of its spawn cell is
  relocated by the Movement owner to the cell and goes idle; if the cell cannot admit it, it idles
  in place. Raid, encounter and summon creatures have no leash.

## 6. Spawns (SPAWN-CONTENT-1, SPAWN-1)

### 6.1 Definitions

- A **spawn source** is a key, a centre and its points; a **point** is a creature definition, a
  cell, a direction and a respawn delay in ms. Sources are a family of the World bundle (ADR-0021),
  so they change only at a planned reset (D194).
- Validation refuses a bundle above `RL-01` points or `RL-02` sources per World map, `RL-03`
  points per source, or a delay outside `RL-13`. A point whose cell cannot admit its creature
  (no ground, protection zone, floor change, teleport) is a compile diagnostic and is dropped from
  production bundles, listed in the parity report (the ADR-0021 §4.5 pattern).
- A point whose creature has a `bosstiary` block or `reward_boss` is a boss spawn and belongs to
  BOSS-RAID-0 §5. Encounter-bound creatures are never realized by a spawn (E3).
- The fixture World keeps D116 (one spawn, 2 rats).

### 6.2 Realization

- At channel activation the owner realizes every active point of the active bundle in canonical order
  (source key, point ordinal), at most `RL-14` per window, each as a fresh actor-local generation
  with its speed drawn (`MONSTER_SPEED_DRAW`), before the channel admits players (`RL-15`).
- One live or pending creature per point, ever.
- A point is **active** when its period is `All`. A point with another period is compiled and
  listed in the parity report but is outside the activation set and `RL-15`, and is never
  realized or respawned until a spawn-period decision binds it to the World clock (owner 7a,
  WORLD-INTERACTION-0; one creature today).

### 6.3 Respawn

The first slice's respawn family (§4.3 there) applies per point. When a point's creature is gone
(a committed death, or removal of a convinced summon, §8.2), one respawn occurrence is due one full
delay later. When it is due:

- **Blocked** (profile `blocked_by_nearby_players` true and a player's interest area contains the
  cell on its floor): the occurrence ends `BLOCKED` and one successor is due a full delay later
  (ruling R6).
- **Warning** (not blockable): a spawn effect at the cell, and the admission 4,200 ms later in the
  same occurrence (Canary 3 × 1,400 ms). Admission checks occupancy again; a cell occupied then
  enters the Occupied chain below (each retry repeats the warning), so the point always ends
  realized, `SKIPPED` with a successor, or `BLOCKED` with a successor.
- **Occupied:** the accepted chain, 3 retries every 5,000 ms (D115) unless the source states
  another interval, then `SKIPPED` and a successor a full delay later. It never displaces or
  stacks (Canary forces placement; the accepted §4.3 rule stays).
- **Period** other than `All`: the point is inactive (§6.2) and never respawns.
- At most one pending occurrence per point (`RL-12`).

### 6.4 Rate hooks

The delay is the content delay. The Boosted Creature and the Improved Respawn Rate area
(`starting.md:69`, `interface.md:42`) apply a factor at scheduling, defined by their own later
decisions; until then the factor is 1.

### 6.5 Restart, reset and durability

- Creature actors, respawn occurrences, summons, targets, paths and overrides are process-local.
- **Crash restart:** activation realizes every active point (§6.2) again (D55, `EphemeralScopeReset`).
- **Server save (`WorldReset`):** every creature actor of every channel ends with the overlay (no
  death, no loot); the channel activates again with the bundle of the reset and realizes every
  active point (§6.2; owner 6a, D191).
- Only these outlive a creature: committed deaths and their descendants (DUR-03 A4, D52), and the
  boss clocks and raid runs of BOSS-RAID-0.
- **Multiplicity:** ordinary spawns are value-producing with independent copies per channel (the
  scope matrix row), which is the GAME-AI-01 §15 classification. Boss rewards stay BOSS-RAID-0's.

## 7. Budgets and overload (CREATURE-AI-1, CREATURE-MOVE-1)

- An **owner window** is 50 ms of owner semantic time (the CONDITIONS-0 beat). Per window the
  owner starts at most `RL-05` thinks and `RL-09` path work units. Due thinks and path requests
  beyond them stay due and run in the next windows in deadline order, then `ExactActorRef`. Think
  timers keep `SKIP_TO_LATEST` (first slice §4.2): a late think runs once, never a burst.
- A think over `RL-06` work units ends idle for that think with zero mutation (bootstrap §7).
- **Amendment (pending on acceptance of RANGED-0; `reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md` §8).** Player chase
  searches use the same path profile with their own row `RANGED0-RL-03` (64 per window per channel),
  served in actor-id order after creature searches.
- Control, fencing, admission and player input never wait for AI work: AI budgets are separate and
  bounded (GAME-AI-01 §20). Player-visible creatures get no priority: the order is deterministic
  (SIM-DETERMINISM-01), so an overload shows as later thinks, the same on every replay.
- `RL-17` measures the cost. If the p99 is above it, `RL-05` and `RL-09` fall by a new decision;
  they are never raised to hide it.

## 8. Summons (MONSTER-SUMMON-1, SUMMON-1, SUMMON-WIRE-1)

### 8.1 Monster summons

- On a think, a creature that is not a summon, has a target with a path, and has fewer summons than
  `max_summons` checks each `summons` entry: due by the §4.4 tick rule, fewer than `count` of that
  creature among its summons, then a draw with `chance_ppm`. A success admits the creature on a
  free tile next to the caster, linked to the caster; no free tile, nothing.
- **Placement** (monster summons, Summon Creature): the caster's tile if it admits the creature,
  else one of its 8 neighbours that does, drawn uniformly with the purpose `SUMMON_PLACE` of the creating occurrence
  (Canary shuffles the 8, `map.cpp:603-630`).
- **Animate Dead** places its creature on the corpse tile only (S27); a corpse tile that does not
  admit it fails the acquisition with nothing consumed (§8.2). Convince does not place.
- Encounter summons (D45 `ability_cast`) stay with the encounter runtime.

### 8.2 Player summons

- **Creation** is the spell lane's (`acquire_summon`, S27), with that contract's checks per source:
  Summon Creature takes mana from `summoning.mana_cost` and requires `summonable`; Convince takes
  mana from `summoning.mana_cost`, requires `convinceable` and a masterless target; Animate Dead
  has `mana_source = none` and `require_flag = none`, and consumes the corpse and one rune charge.
  This runtime commits every effect of one acquisition (mana, corpse removal, rune charge, summon
  admission) as one all-or-nothing operation, checking the cap of 2 summons per character
  (`RL-19`) and the placement in it: if admission fails, nothing is consumed.
  Summon Creature admits a new creature placed as §8.1; Animate Dead admits one on the corpse tile (§8.1). Convince changes the owner of the existing creature, keeping its
  actor and health.
- A convinced spawned creature keeps its point: the point respawns only after it is gone (Canary;
  ruling R7).
- A creature that has ever been a summon never yields a corpse, loot, experience, Bestiary, Prey,
  task or Bosstiary credit when it dies. A lethal combat outcome still creates or recognizes its
  one replay-stable death key and death occurrence (VSL-COMBAT-01 §§7–8, DUR-03 A4), so lethal
  replay and actor-generation fencing stay exactly-once; that occurrence simply starts no
  descendant workflow. Only a removal (§8.4) creates no death key.

### 8.3 Behaviour

- **Target:** its owner's current target (the ATTACK-0 target of a player, the target of a
  monster), when eligible for the summon (§4.2, including a creature target); a player target only where PARTY-PVP-0 §7 lets the
  owner attack it, because the summon's actions are the owner's. Otherwise it follows its owner to
  within 2 tiles.
- It uses the attack, step and path rules above; it never flees, wanders, walks back or idles
  while its owner is on the channel.
- It cannot enter a protection zone or a floor-change or teleport tile, so it waits when its owner
  enters one or changes floor, and follows again when the owner is reachable.

### 8.4 Removal

A summon is removed, with no death, when its owner logs out, dies, transfers channel or leaves the
channel at the end of its in-fight deadline; when the owner is more than 30 tiles in x or y or 2
floors away (checked on every committed move of either); when its owner link is stale (owner
generation changed); and at `WorldReset`. A monster's summons are removed with their master
when the master is removed (a despawn, scope retirement or `WorldReset`: a removal, not a death,
no death key, nothing dropped). When a monster master dies (a committed lethal transition), each
of its summons dies too in the same owner step, through the summon death path of §8.2 (its own
death key and occurrence, no descendant workflow; Canary kills them, `§2`). A reconnect to the same GameSession keeps them. Summons are never saved and do
not return at login. Familiars are the spell lane's (C.1, its Q3).

### 8.5 Attribution

- **Experience by damage share** (VSL-COMBAT-01): a player summon's share goes to its owner,
  halved and rounded down (Canary, `PARITY_PENDING`, ruling R8). Shared experience follows
  PARTY-PVP-0 §5.1 unchanged.
- **Loot right and contributors** (D121, D132): a summon's damage counts as its owner's.
- **PvP:** the owner's action (PARTY-PVP-0 §7.1, §8.3). **Party immunity** covers summons (§5.2
  there).

### 8.6 Wire (SUMMON-WIRE-1)

Each VIS-2 creature entry gains a `summon` field: `NONE`, `OWN` or `OTHER`, from the observer's
side (`combat.md:217`), at most 1 byte, inside the 128 B entry bound of MOVE-RL-11 §4.4. It rides
VIS-2's schema revision if VIS-2 is not yet frozen, else a capability reserved on #162 at
allocation.

## 9. Interactions

- **ATTACK-0:** creature melee is ATTACK-1's (§4.4); target choice is this decision's.
- **CONDITIONS-0:** §5.1 is the cadence amendment §4.3 there requests; conditions keep a creature
  active (§4.1); speed and paralysis act through the step duration.
- **GAME-ABILITY-01:** every creature effect is a typed intent with `ProposalSource::Ai` (§4.4);
  overrides arrive as Ability commits (§4.3).
- **Party and PvP:** §8.3 and §8.5. **D3, DUR-03 A4, VSL-COMBAT-01:** unchanged; a lethal summon death
  creates or recognizes its one death key and occurrence, which starts no corpse, loot or credit
  workflow (§8.2); only a removal (§8.4) makes no death. **D54** (no player death from creatures) stays until the player death
  decision replaces it.
- **BOSS-RAID-0:** boss spawns, raid runs and encounters are not redecided; their creatures use
  §3-§5 unless their encounter overrides.

## 10. Rows (registered by each child before implementation)

| Row | Value | Failure or note |
|---|---|---|
| `CREATUREAI0-RL-01` spawn points per channel | 131,072 | bundle refused; reference map 83,286 |
| `CREATUREAI0-RL-02` spawn sources per channel | 65,536 | replaces `AI01-SPAWN-SOURCES-PER-SCOPE` 16; map 51,896 |
| `CREATUREAI0-RL-03` points per spawn source | 64 | replaces `AI01-SPAWN-POPULATION` 4 and `AI01-SPAWN-PLACEMENT-CELLS` 4; map 55 |
| `CREATUREAI0-RL-04` creature actors per channel, every kind | 262,144 | replaces `AI01-ACTIVE-ACTORS` 256; an admission beyond it is refused `CAPACITY_EXCEEDED` (a respawn ends `SKIPPED`, a summon is refused) |
| `CREATUREAI0-RL-05` thinks started per 50 ms window | 1,024 | the rest wait, §7 |
| `CREATUREAI0-RL-06` evaluation work per think | 128 units (one per candidate, entry or rule step) | replaces `AI01-EVALUATION-WORK` 8 here; max+1 ends the think idle |
| `CREATUREAI0-RL-07` Ability proposals per think | 24 (16 attacks, 8 defences) | content validation |
| `CREATUREAI0-RL-08` monster summons per creature | 16 (`max_summons`); 8 entries; count 16 | content validation |
| `CREATUREAI0-RL-09` path work per channel per 50 ms window | 65,536 units (64 searches of 1,024) | the rest wait, §5.3 |
| `CREATUREAI0-RL-10` path requests per creature per think | 1 | within `AI01-PATH-REQUESTS-PER-ACTOR` 2 |
| `CREATUREAI0-RL-11` pending AI timers per creature | 2 (one think, one step) | replaces `AI01-PENDING-TIMERS-PER-ACTOR` 1 |
| `CREATUREAI0-RL-12` pending respawn occurrences per channel | 131,072 (one per point) | |
| `CREATUREAI0-RL-13` respawn delay | 1,000 ms to 86,400,000 ms | content validation; map 5 s to 54,784 s |
| `CREATUREAI0-RL-14` realizations per 50 ms window at activation | 4,096 | |
| `CREATUREAI0-RL-15` activation with every active point (§6.2) realized | at most 30,000 ms at 131,072 points, measured by SPAWN-1 | above it, a new decision |
| `CREATUREAI0-RL-16` creature runtime memory per channel | at most 128 MiB at 131,072 idle creatures, measured by SPAWN-1 | above it, a new decision |
| `CREATUREAI0-RL-17` AI work (thinks and paths) per 50 ms window | p99 at most 15 ms on the reference host, measured by CREATURE-MOVE-1 | above it `RL-05` and `RL-09` fall |
| `CREATUREAI0-RL-18` AI overrides per creature | 2 (one forced target, one forced distance) | the newer replaces |
| `CREATUREAI0-RL-19` player summons per character | 2 | refused by `acquire_summon` |

Unchanged: `AI01-PERCEPTION-CANDIDATES` 64, `AI01-PATH-SEARCH-WORK` 1,024, `AI01-ROUTE-STEPS`
128, `AI01-ROUTE-BYTES` 4,096, `AI01-SPAWN-OCCUPANCY-RETRIES` 3, `MOVE-RL-09` 1,024.

Parity values (not resource limits): think 1,000 ms; leash 50 tiles or 2 floors; summon removal 30
tiles or 2 floors; summon follow distance 2; flee distance 11; chase search distance 12; path costs
10, +25, +40, +180; spawn warning 4,200 ms; occupancy retry 5,000 ms (D115).

Derived: at most 20,480 thinks and 1,280 full path searches per second per channel.

## 11. Rejected options

- **Canary's 11-tile perception box.** A creature would attack players who cannot see it; one
  MOVE-RL-11 relation serves view, wake and perception (R1).
- **Asynchronous pathfinding on worker threads.** Results would land at timing-dependent moments,
  which breaks SIM determinism; a bounded writer budget is deterministic and enough until measured
  otherwise (R3).
- **Durable creatures or respawn timers.** The spawn is `EphemeralScopeReset`; Tibia's server save
  resets the map; only boss clocks need durability (BOSS-RAID-0).
- **Keeping the D57 envelope (16 × 4).** The reference map has 83,286 points (R4).
- **A global fixed AI tick.** GAME-AI-01 §7 forbids it.
- **Summons that survive logout or return at login.** Tibia removes them.
- **Creatures following through stairs, ladders and holes.** Tibia monsters do not (R2).

## 12. Owner-rule applications

**Tibia parity kept:** thinks every second; noticing only players on screen and on the same floor;
target change by interval and chance; the strategy weights; fleeing at the profile's health;
dancing; keeping distance; no chase across floors; no entry into protection zones; walking back
and leashing to the spawn; idle creatures far from players; attacks and defences by interval and
chance; monster summons by interval, chance and count; respawn suppressed near players where the
creature is blockable; a respawn warning effect otherwise; player summons capped at 2, following
and attacking the owner's target, removed at logout, death or distance, giving no experience or
loot; summon damage counting for its owner; spawns per channel, reset at the server save.

**Declared differences:**
- Perception equals the MOVE-RL-11 area, not Canary's 11 tiles (R1, `PARITY_PENDING`).
- Ties break by stable identity, not list order (GAME-AI-01 §8).
- An occupied spawn cell retries and skips instead of stacking (accepted first slice §4.3).
- A creature whose walk back fails idles in place (R5).
- Summon experience halving is Canary's (R8, `PARITY_PENDING`).
- Not yet built, each with its own decision: pushing items and creatures, voices, day and night
  periods, respawn rate modifiers, invisibility, diagonal steps, familiars.

## 13. Architect rulings (owner rule 5905825574)

**R1. Perception.** a) The MOVE-RL-11 relation: a creature perceives the players that see it
(recommended: one index, fair, the area D84 chose for vision); b) Canary's 11-tile
box. **Ruled a)**, and confirmed by the owner (answer 4a, 2026-10-01, #162). It supersedes the
first slice's 7-tile content value (D115).

**R2. Floors.** Owner 4a lists "chasing players across floors". Global monsters do not use stairs,
ladders or holes; Canary forbids floor-change tiles to monsters, and the Tibia escape by changing
floor depends on it. a) Tibia: no floor change by walking; the target on another floor is lost
(recommended: owner rule 5905825574, Tibia-faithful); b) cross-floor chase as a declared
difference. **Ruled a)**, and confirmed by the owner (answer 2a, 2026-10-01, #162).

**R3. Where paths run.** a) On the writer inside a window budget, deterministic deferral
(recommended); b) worker threads with asynchronous adoption, nondeterministic. **Ruled a).**

**R4. Population rows.** D57 asks a new owner decision for larger values. Owner 4a (full behaviour
from data) with D188 (the full base map) and 6a (spawns per channel) is that decision for the
reference map; the numbers are bounds with headroom over the map (`RL-01` to `RL-04`). **Ruled**,
and the owner confirmed that 4a covers these bounds without a separate approval (answer 3a,
2026-10-01, #162).

**R5. Failed walk back.** a) Idle in place until the next wake (recommended: bounded); b) Canary's
retry every think forever. **Ruled a).**

**R6. Respawn blocking.** The manual suppresses respawn near players; the content (from Canary)
marks only 5 creatures blockable. a) Follow the content flag per creature, and the content lane
re-checks the flag against official and wiki sources (recommended: data-driven, one rule); b)
block every spawn. **Ruled a)**, `PARITY_PENDING`.

**R7. Convinced spawned creatures.** a) Keep the point until the creature is gone (Canary,
recommended: no extra state); b) free the point at once. **Ruled a).**

**R8. Summon experience.** a) Canary's half share to the owner (recommended: the only source); b)
full share. **Ruled a)**, `PARITY_PENDING`.

## 14. Owner questions

None in this document. Every choice is a Tibia-parity application or a bound that owner rule
5905825574 gives to the architect. R2 and R4 are reported to the control plane, which may put them
to the owner.

## 15. Decision test

- **Must decide now:** YES. Owner 4a, 5a and 6a; ATTACK-1, SPEED-1, the spell summon children and
  the full map all wait on creature behaviour, and the first slice's rows cannot hold the map.
- **Minimum sufficient:** one state machine over content data; the existing timer lane, Movement
  and Ability owners; one more timer family (the step); one path profile on the writer; one link
  per creature; one wire field. No scripts, no worker pool, no durable creature state.
- **Superseding evidence:** official Global perception, respawn blocking or summon experience
  rules; a measured `RL-15`, `RL-16` or `RL-17` above its bound; a map above `RL-01`.
- **Deliberately not decided:** bosses, raids and encounters (BOSS-RAID-0), pushing, voices,
  periods, respawn modifiers, invisibility, familiars, NPCs, the player chase mode, diagonal steps.

## 16. Before-freeze checklist

1. **Contract amendments:** first creature slice §4.4 and §4.9; CONDITIONS-0 §4.3; the horizon
   `GAME-AI-01` entry; gap register §11; each pending on acceptance of CREATURE-AI-0.
2. **Serialization:** one channel owner per creature; steps through Movement, effects through
   Ability; summon creation in the spell's owner mutation; no new durable write.
3. **Restart:** everything here is runtime; activation and `WorldReset` realize every active point (§6.2) again.
4. **Typed references:** `ExactActorRef` for creatures, owners and issuers; occurrence (creature,
   think sequence, list, entry); spawn point (source key, ordinal).
5. **Wire:** one `summon` field on VIS-2 creature entries (§8.6).
6. **Split work:** per 50 ms window at most 1,024 thinks of 128 units and 65,536 path units;
   at most 262,144 creatures per channel.
