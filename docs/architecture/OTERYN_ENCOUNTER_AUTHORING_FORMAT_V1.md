# Oteryn Encounter Authoring Format v1

- Date: 2026-09-27
- DecisionStatus: PROPOSED (owner decision D20: draft for owner acceptance before any implementation)
- DeliveryStatus: OPEN (design draft only)
- ImplementationStatus: NOT_STARTED
- Programme: KAN-16 / #504
- Companion: `OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` (monsters stay reusable; encounters bind them)
- Evidence: `tools/content-schema/monster-authoring/samples/events-canary-47dfd51f.json`,
  `samples/population-canary-47dfd51f.json`

## 1. Problem

Canary attaches boss and arena logic to monsters through creature events (`onDeath`,
`onPrepareDeath`, `onHealthChange`, `onThink`), inline `mType` callbacks and spell scripts that
use fixed map positions or global counters. None of it is monster data: it is the logic of one
fight in one place. In the population census 168 of 1,656 monster files are blocked by such events
alone, and every remaining boss depends on them.

The 131 unresolved events (263 monster references) use four handlers: `onDeath` 168 references,
`onHealthChange` 55, `onThink` 36, `onPrepareDeath` 19. A keyword pass over their recorded effects
(approximate; an event can count in several rows) shows a small vocabulary:

| Primitive | Events | Monster references |
|---|---:|---:|
| spawn a creature (adds, next boss, respawn) | 67 | 141 |
| counters and flags (kills, stages, phases) | 27 | 75 |
| heal (full heal, heal back damage) | 38 | 66 |
| quest progress, boss cooldown, reward | 22 | 64 |
| remove creatures (self, adds, arena) | 25 | 62 |
| create/transform/remove a map item (teleporter, vortex, door) | 28 | 62 |
| timers and delays | 27 | 61 |
| transform into the next stage | 21 | 55 |
| teleport creatures or players | 28 | 48 |
| health thresholds | 21 | 29 |
| incoming damage modifiers (immune, x2, conditional) | 14 | 26 |
| random chance | 12 | 27 |
| area attack on death | 5 | 5 |
| reflect damage | 2 | 2 |

## 2. Principles

1. An Encounter is Game-owned content, like a monster; it references monsters and never copies them.
2. Data first: a fixed set of triggers, conditions and actions (§4-6); no embedded scripts. A
   mechanic outside the set stays unresolved in the import manifest, as monsters do.
3. No raw coordinates. Positions and areas are named anchors of the encounter; a map project binds
   anchors to world positions. Canary coordinates are kept as source evidence only.
4. Encounter state (counters, flags, timers, phase) lives with one encounter instance in one
   `WorldId`/`ChannelId` and is discarded on reset. It is never character state.
5. Character and account effects (quest storages, boss cooldowns, reward rooms, hazard levels) are
   not executed by the encounter. The encounter emits a named outcome; the owning system (quests,
   character persistence with session-generation fenced writes, rewards) consumes it under its own
   contract.
6. Canary defects are recorded, not reproduced blindly: where a script is broken (for example
   `GloothHorror` uses an undefined variable, `CracklerTransform` never runs its branch) the
   reference-date wiki decides the intended behaviour (D25).

## 3. Shape

```
Encounter
  identity                      Game key + revision
  display_name
  participants[]                role key -> CreatureRef, plus "stage" order for multi-form bosses
  anchors[]                     named point or area (rectangle/zone) to be bound by the map project
  state
    counters[]                  name, initial integer
    flags[]                     name, initial boolean
    timers[]                    name, duration_ms, repeat
  rules[]                       trigger, conditions[], actions[] (in order)
  lifecycle
    start                       first participant engaged | anchor entered
    reset                       no player in the arena anchor for reset_after_ms | explicit action
    outcomes[]                  named results (victory, defeat, stage_reached) for other systems
```

## 4. Triggers

| Trigger | Canary source |
|---|---|
| `creature_died(role)` | `onDeath` |
| `lethal_damage(role)` - may `prevent_death` | `onPrepareDeath` |
| `health_crossed(role, percent, downward)` | `onThink`/`onHealthChange` health checks |
| `damage_taken(role, source: player/any)` | `onHealthChange` per hit |
| `damage_accumulated(role, amount)` | `onHealthChange` damage counters |
| `timer_elapsed(timer)` | `addEvent` delays, `onThink` countdowns |
| `counter_reached(counter, value)` | global kill/stage counters |
| `area_entered(anchor, role or player)` / `area_left` | zone crossing (`izcandarThink`) |
| `encounter_started` / `encounter_reset` | lifecycle |

## 5. Conditions

`chance_percent`, `counter_compare(counter, op, value)`, `flag(name, value)`,
`creature_present(role, anchor or radius, present/absent)`, `health_percent(role, op, value)`,
`attacker_wears(ItemRef)` (the Asura counter items).

## 6. Actions

| Action | Parameters |
|---|---|
| `spawn` | role or CreatureRef, count, at (`death_position`, anchor, `random_in(anchor)`, offset), owner (none/caster), health (`full`, `carry_over`, percent) |
| `remove` | role, `all_in(anchor)`, self |
| `transform` | role -> next stage or CreatureRef; health `keep_percent`/`keep_absolute`/`full` |
| `heal` | role, amount or `full` |
| `prevent_death` | only after `lethal_damage` |
| `damage_modifier` | role, multiplier (0 = immune), damage types, sources, duration or until reset |
| `reflect_damage` | role, percent, damage types |
| `convert_damage_to_heal` | role, damage types |
| `teleport` | role or `players_in(anchor)`, to anchor |
| `map_item` | create/transform/remove ItemRef at anchor, `revert_after_ms` |
| `counter` / `flag` / `timer` | set, add, start, stop |
| `cast` | AbilityRef at a role or anchor (death explosions) |
| `say` | role, text, mode |
| `drop_item` | ItemRef, chance, at role position |
| `emit_outcome` | named outcome for quests, cooldowns and rewards (§2.5) |

## 7. Worked examples

- `FourthTaintBossesPrepareDeath` (15 monsters): `lethal_damage(boss)` + `chance_percent 10` ->
  `prevent_death`, `heal(boss, full)`.
- `UrmahlulluChanges`: `health_crossed(boss, N)` -> `transform(boss -> next stage, keep_absolute)`.
- `HeartBossDeath`: `creature_died(boss)` -> `map_item(transform vortex at anchor)`, and for the
  final boss `remove(all_in(arena))`.
- `AsurasMechanic`: `damage_taken(asura)` + not `attacker_wears(counter item)` ->
  `damage_modifier(asura, 0)` for that hit.
- `DepthWarzoneBossDeath`: `creature_died(boss)` -> `map_item(transform teleporter at anchor,
  revert_after_ms 1200000)`, `emit_outcome(boss_defeated)`.

## 8. Import approach

1. Transcribe each event once into an Encounter rule set, with source lines, as the spell patterns
   were; a probe or template match only where the behaviour is checked exactly.
2. Monsters that register the event get an `encounter_binding` manifest row instead of an
   unresolved `events` row once the encounter exists.
3. Canary positions become anchors with the source coordinates as evidence; binding them to the
   Oteryn map is a map-project step.
4. Quest storages, cooldowns and rewards become `emit_outcome` names plus a list for the quest and
   reward owners.

## 9. Open questions for the owner

1. Instance scope: one shared arena per channel (Canary behaviour) or an instanced arena per party.
2. Owner of boss cooldowns and reward rooms: quest system, character persistence or a boss-lock
   service.
3. Acceptance of the primitive set in §4-6 as the v1 vocabulary; anything else stays unresolved.
4. Order of work: the 15-monster `FourthTaintBossesPrepareDeath` and the largest death events first.
