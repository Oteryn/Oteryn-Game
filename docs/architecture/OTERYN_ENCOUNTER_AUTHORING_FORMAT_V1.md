# Oteryn Encounter Authoring Format v1

- Date: 2026-09-27
- DecisionStatus: CANDIDATE (D20 draft; owner decisions D26-D28 recorded in §10; the vocabulary is
  implemented offline before any runtime work)
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
4. Encounter state (counters, flags, timers, phase) lives with one encounter instance and is
   discarded on reset. It is never character state. By default an encounter runs as one isolated
   instance per party (D26): `WorldId + InstanceId` per `FND-ID-01_OWNER_ACCEPTED_BASELINE.md`
   ("instanced cooperative gameplay requires participants to be admitted into one common
   `WorldId + InstanceId`"), the "Instanced dungeon" row of `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`.
   Every encounter declares its scope explicitly ("Boss runtime ... must declare scope; no implicit
   default" in the same matrix).
5. Character and account effects are not executed by the encounter. The encounter emits a named
   outcome with its participants; the owning domain consumes it under its own contract (D27):
   boss cooldowns, reward eligibility and reward rooms belong to the reward domain ("Boss reward
   eligibility | Reward service/domain | Character/Account/World | Strong durable | Prevent repeated
   farming across channels" in the scope matrix); quest storages and mission steps belong to the
   quest domain; hazard levels to their progression domain. Their character writes stay
   session-generation fenced.
6. Canary defects are recorded, not reproduced blindly: where a script is broken (for example
   `GloothHorror` uses an undefined variable, `CracklerTransform` never runs its branch) the
   reference-date wiki decides the intended behaviour (D25).

## 3. Shape

```
Encounter
  identity                      Game key + revision
  display_name
  scope                         instance_per_party (default, D26) | channel_shared (explicit opt-in)
  entry                         admission anchor, party size limits, readiness (consumes the
                                shared activity-instance admission contract; not defined here)
  participants[]                role key -> CreatureRef
  phases[]                      ordered named phases; multi-form bosses move by `set_phase`
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
| `phase_entered(phase)` | stage bosses |
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
| `set_phase` | next or named phase (phase changes are triggers too: `phase_entered(name)`) |
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

## 9. Rules for the vocabulary (D28)

1. §4-6 is the closed v1 vocabulary. A mechanic outside it stays an unresolved import row with its
   evidence; it is brought to the owner instead of being approximated or scripted.
2. Rules run in their listed order; a trigger fires its rules once per occurrence; actions of one
   rule run in order. `prevent_death` is valid only in a `lethal_damage` rule.
3. Randomness (`chance_percent`, random positions) is drawn by the encounter instance, so a fight
   can be audited and replayed from its seed.
4. Health carried by `transform`/`spawn` is explicit (`keep_percent`, `keep_absolute`, `full`,
   percent); nothing is implied.
5. Anchors are typed (point or area) and must all be bound by the map project before admission;
   an unbound anchor blocks the encounter, never falls back to raw coordinates.
6. Validation mirrors the monster schema: JSON Schema plus a semantic validator and an import
   manifest in which every Canary event line is mapped, omitted with a reason, or unresolved.

## 10. Owner decisions

| # | Decision | Basis |
|---|---|---|
| D26 | Encounters are instanced per party by default (a separate copy of the arena for each party), unlike Canary's one shared arena; `channel_shared` is an explicit opt-in per encounter. | Owner request 2026-09-27; `WorldId + InstanceId` in FND-ID-01; "Instanced dungeon" in the scope matrix. |
| D27 | Encounters only emit named outcomes. Boss cooldowns, reward eligibility and reward rooms are consumed by the reward domain; quest steps by the quest domain. | Owner delegated the choice; the scope matrix already assigns boss reward eligibility to the reward domain. |
| D28 | §4-6 plus phases is the v1 vocabulary under the rules of §9; work starts with `FourthTaintBossesPrepareDeath` (15 bosses), then the largest death events. | Owner delegated the choice. |

Instance admission, party size and readiness are consumed from the shared activity-instance
admission contract (FND-ID-01 Party Finder consequences); this format does not define them.
