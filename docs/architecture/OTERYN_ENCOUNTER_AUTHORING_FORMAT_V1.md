# Oteryn Encounter Authoring Format v1

- Date: 2026-09-27
- DecisionStatus: CANDIDATE (D20 draft; owner decisions D26-D29 recorded in §10; the vocabulary is
  implemented offline before any runtime work)
- DeliveryStatus: OPEN (design draft only)
- ImplementationStatus: NOT_STARTED
- Programme: KAN-16 / #504
- Companion: `OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` (monsters stay reusable; encounters bind them)
- Machine artifacts: `tools/content-schema/encounter-authoring/` (schema, semantic validator,
  focused checks, Canary transcription; first sample `samples/soul_war_taint_zones/`)
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
  scope                         instance_per_party (default for boss fights, D26) | channel_shared
                                (explicit; zone rules over open hunting grounds such as Soul War)
  entry                         admission anchor, party size limits, readiness (consumes the
                                shared activity-instance admission contract; not defined here)
  participants[]                role key -> CreatureRef
  phases[]                      ordered named phases; multi-form bosses move by `set_phase`
  anchors[]                     named point or area (rectangle/zone) to be bound by the map project
  state
    counters[]                  name, initial integer
    flags[]                     name, initial boolean
    timers[]                    name, duration_ms, repeat
  rules[]                       trigger, optional delay_ms, conditions[], actions[] (in order)
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
| `health_crossed(role, percent or absolute health, downward)` | `onThink`/`onHealthChange` health checks; the percent may be fractional (D29) |
| `creature_spawned(role)` | `mType.onSpawn` (D29) |
| `ability_cast(role, AbilityRef)` | a monster spell script with fight effects (D29) |
| `damage_taken(role, source: player/any)` | `onHealthChange` per hit |
| `damage_accumulated(role, amount)` | `onHealthChange` damage counters |
| `timer_elapsed(timer)` | `addEvent` delays, `onThink` countdowns |
| `counter_reached(counter, value)` | global kill/stage counters |
| `area_entered(anchor, role or player)` / `area_left` | zone crossing (`izcandarThink`) |
| `phase_entered(phase)` | stage bosses |
| `encounter_started` / `encounter_reset` | lifecycle |

## 5. Conditions

`chance_percent`, `counter_compare(counter, op, value)`, `flag(name, value)`,
`creature_present(role, anchor or near(role, radius), present/absent)`, `in_anchor(role or killer, anchor)`,
`killer_is_player`, `has_master(role, value)` (Canary skips summoned copies of a boss),
`health_percent(role, op, value)` (fractional allowed),
`attacker_wears(ItemRef)` (the Asura counter items), `killer_progress(quest key, op, value)` - a
read-only view of the killer's quest progress published by the quest domain (the Soul War taints);
the encounter never writes it. `world_state(key, op, value)` (D29) is the same read-only view of a
value another domain publishes for the world or channel (a quest stage, a world counter, an item
buff such as the cobra flask).

## 6. Actions

| Action | Parameters |
|---|---|
| `spawn` | role or CreatureRef, count, at (`death_position`, anchor, `random_in(anchor)`, offset), owner (none, subject, or `death_master`: the master of the dying creature), health (`full`, `carry_over`, percent) |
| `remove` | role, or `all_in(anchor)` (monsters only; players are never removed) |
| `transform` | role -> next stage, CreatureRef or `random_of` several CreatureRefs (uniform); health `keep_percent`/`keep_absolute`/`full` |
| `heal` | role, amount, range or `full` |
| `damage` | subject, amount or range, damage type (D29) |
| `prevent_death` | only after `lethal_damage` |
| `damage_modifier` | role, multiplier (0 = immune), damage types, sources, duration or until reset; `component: primary` limits it to the primary part of a hit (D29) |
| `reflect_damage` | role, percent, damage types |
| `convert_damage_to_heal` | role, damage types, optional `component` |
| `teleport` | role or `players_in(anchor)`, to anchor |
| `map_item` | create/transform/remove ItemRef at an anchor or `at: death_position` (death and lethal damage triggers), `revert_after_ms`; a teleporter carries `destination` and optionally `revert_destination` anchors; a revert restores the original item with its original attributes unless `revert_destination` overrides the destination; optional `effect`; optional `interaction`: the key of interaction-domain content that defines what the item does when used or stepped on (D29) |
| `counter` / `flag` / `timer` | set, add, start, stop |
| `set_phase` | next or named phase (phase changes are triggers too: `phase_entered(name)`) |
| `cast` | AbilityRef at a role or anchor (death explosions) |
| `say` | role, text, mode |
| `drop_item` | ItemRef, chance, at role position |
| `emit_outcome` | named outcome for quests, cooldowns and rewards (§2.5), `credited`: `damage_contributors`, `killer` or `players_in_anchor(anchor)` |

## 7. Worked examples

- `FourthTaintBossesPrepareDeath` (15 monsters): despite its name these are the ordinary Soul War
  hunting monsters, not bosses (`soul_war_mechanics.lua` lines 66-84). It is a zone rule:
  `lethal_damage(any participant)` by a player + `killer_progress(fourth Soul War taint held)`
  (`Player:getTaintNameByNumber(4)`, a quest flag Canary resets at login) + the killing player
  stands in a Soul War hunting zone (`Player:getSoulWarZoneMonster`) + `chance_percent 10`
  (`math.random(1, 10) == 1`) -> `say`, `heal(full)`. `Game::combatChangeHealth` then still drains
  the lethal hit, capped at the health the creature had before the heal (`realDamage` is taken
  before `onPrepareDeath`), so it survives with maximum health minus that amount.
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
   rule run in order. `prevent_death` is valid only in a `lethal_damage` rule. A rule with
   `delay_ms` is scheduled once per trigger occurrence (like Canary `addEvent`): its conditions are
   evaluated and its actions run `delay_ms` later, and `death_position` is the position of that death.
   In a rule triggered by one creature (a death, lethal damage, damage, health, spawn or cast
   trigger), `transform`, `prevent_death`, `heal`, `damage`, `damage_modifier` and `say` naming the
   trigger's role act on that creature; `remove` of a role removes every creature of the role. A
   delayed action on a creature that no longer exists does nothing. `creature_spawned` fires when a
   creature of the role appears: placed by the map or a lever, spawned, or transformed into the role.
3. Randomness (`chance_percent`, random positions, `random_of`, `{min, max}` ranges for spawn
   counts, heal and damage amounts, rule delays and timer durations) is drawn uniformly by the
   encounter instance, so a fight can be audited and replayed from its seed.
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
| D28 | §4-6 plus phases is the v1 vocabulary under the rules of §9. Work starts with `FourthTaintBossesPrepareDeath` (15 Soul War hunting monsters, a `channel_shared` zone rule with a read-only quest-progress condition), then the largest death events. | Owner delegated the choice; the source shows the event is a zone rule, not a boss fight. |
| D29 | Vocabulary extensions: `creature_spawned` and `ability_cast` triggers; the `world_state` read-only condition; fractional or absolute health thresholds; `creature_present` near a role; `{min, max}` ranges; a `damage` action; `component: primary`; `map_item.interaction`. Acting on whatever stands on a fixed tile is not added: each case names its role from wiki or map evidence. Scripted movement is deferred. State shared by all parties belongs to the quest domain, which the encounter reads through `world_state`. | Owner accepted the proposal ("kontynuuj tak jak uważasz za optymalne", 2026-09-27). |

Instance admission, party size and readiness are consumed from the shared activity-instance
admission contract (FND-ID-01 Party Finder consequences); this format does not define them.

## 11. First transcription

`samples/soul_war_taint_zones/` transcribes `FourthTaintBossesPrepareDeath`: 15 participants read
from the monster files that register it, one `channel_shared` rule, every source line mapped in the
manifest, schema and semantic validation clean. The monster
converter now records that event as relocated for those 15 monsters; 6 of them had no other
blocker, so the population census rises from 1,377 to 1,383 fully resolved monsters.

Boss death events followed, one encounter per boss with a `damage_contributors` outcome whose
Canary storages are listed in the manifest `outcome_evidence` for the quest and reward domains:

| Event | Encounters | Covered monsters | Unresolved |
|---|---:|---:|---|
| `dreamCourtsDeath` | 7 | 9 | Alptramun's dream escalation: Canary never counts the dreams (the summons have a master and the script skips them) while the wiki lists all four; modelling it needs an ability-cast trigger (owner decision) |
| `ForgottenKnowledgeBossDeath` | 7 | 9 | Melting Frozen Horror acts on whatever creatures stand on two fixed tiles |
| `AscendantBossesDeath` | 9 | 8 | Ferumbras Mortal Shell resets quest-wide crystal storages and fixed crystal items (needs a quest-domain contract) |

The Ascension bosses open their room teleporter to the Godbreaker for 60 s (`map_item` with
destinations). A dead `elseif` branch of the thorn knight is recorded as an approved omission.
24 encounters validate, 21 manifests resolve fully, `verify_encounter_schema.py` 35/35; the census
rises from 1,383 to 1,389.

A third slice transcribed more boss events; where Canary's script is broken, the reference-date wiki
decides (D25):

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `CultsOfTibiaBossDeath` | 7 | 7 | The corruptor's zarcorix removal is dead code in Canary (`removeMonster` is an undefined global); the wiki says the zarcorix disappears, so it is removed. The Sandking's credit waits for the fight stage, whose rules are not transcribed yet (unresolved). |
| `DestroyedPillar`, `EssenceOfMaliceSpawnsDeath` | 1 | 10 | Canary spawns the Essence of Malice when the dying mini-boss stands alone, which happens after the first of five kills taken one at a time; the wiki says it spawns after all five, so a kill counter decides. |
| `WrathOfTheEmperorBossDeath`, `ZalamonDeath` | 5 | 8 | Statues unseal; Zalamon's forms follow one another. The shared-arena lock is replaced by the instance (D26). |
| `ghuloshDeath` | 1 | 2 | The Book of Death returns 12 s after Concentrated Death dies. |
| `GloothHorror`, `RathletonBossDeath` | 3 | 8 | `GloothHorror` reads an undefined `targetMonster` and spawns nothing; the wiki says each stage splits into two of the next, 16 in total. Teleporters open for 2 minutes and revert to their original destination. |
| `DepthWarzoneBossDeath` | 3 | 3 | Teleporters open for 20 minutes. |
| `AzerusDeath` | 1 | 2 | A teleporter at the death position (`map_item at: death_position`) and the arena cleared of monsters. |

`GlowingRubbishAmuletDeath` only advances Misguided mission counters and swaps a quest item, so it is
quest progress under D6, not an encounter. 44 encounters validate, 40 manifests resolve fully,
`verify_encounter_schema.py` 39/39; the census rises from 1,389 to 1,427.

A fourth slice:

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `gorzindelDeath`, `gorzindelHealth` | 1 | 6 | Gorzindel is immune until no stolen knowledge is left; each knowledge death checks one second later (`delay_ms`). The Stolen Tome of Portals stays unresolved: its portal sends each player to a different free room. |
| `HeartMinionDeath` | 2 | 8 | Minion and boss deaths update the World Devourer counters and the Rupture resonance state. |
| `ChargerSpawn` | 1 | 1 | Each dead charger returns after 6 s on one of ten tiles, once per death. |
| `AstralGlyphDeath`, `DragonEssenceDeath`, `DisgustingOozeDeath` | 3 | 3 | The ooze splits in two with a 10% chance, and the new oozes keep the dying ooze's master. |
| `FeroxaTransform` | 1 | 2 | Feroxa2 also carries the display name "Feroxa", so its branch runs; it becomes Feroxa3 or Feroxa4 at random. |

`facelessHealth` stays unresolved. The script adds the still-negative damage value back as health, so the hit is applied anyway, and Alptramun absorbs death damage completely before the event. The intended behaviour needs a wiki check.

50 encounters validate, 45 manifests resolve fully, `verify_encounter_schema.py` 45/45; the census rises
from 1,427 to 1,442.

A fifth slice uses the D29 vocabulary:

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `UrmahlulluChanges` | 1 | 4 | Canary changes forms at thresholds of one 512000-health scale and reverts a form after 60 s. The wiki gives each form its own health and describes five consecutive kills, so each form becomes the next on its lethal hit (D15, D25). |
| `dreamCourtsDeath` (Alptramun) | 1 | 5 | Unmastered dream deaths raise the dream counter in bands of nine; Alptramun's death resets it. The escalation spell that reads the counter is never cast by any Canary monster; the wiki says killed summons are replaced by stronger ones without numbers, so that ability stays unresolved. |
| `mType.onSpawn` (Splinters of Madness) | 1 | 2 | Each stage grows into the next after 120 s (`creature_spawned` + `delay_ms`). Canary never lets a grown splinter grow again; the wiki says they do (D25). The mighty splinter's absorption into the boss stays unresolved. |

Inline callbacks covered by an encounter manifest are relocated by the monster converter like events.
52 encounters validate, 46 manifests resolve fully, `verify_encounter_schema.py` 63/63; the census rises
from 1,453 to 1,463.
