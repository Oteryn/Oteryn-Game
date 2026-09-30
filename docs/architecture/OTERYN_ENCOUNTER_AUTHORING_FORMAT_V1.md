# Oteryn Encounter Authoring Format v1

- Date: 2026-09-27
- DecisionStatus: CANDIDATE (D20 draft; owner decisions D26-D29 recorded in §10; the vocabulary is
  implemented offline before any runtime work; §12 holds the CW2 extensions, ACCEPTED 2026-09-29; §13 holds the
  Soul War extensions SW-3..6, ACCEPTED 2026-09-30)
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
  anchors[]                     named point or area (rectangle/zone) to be bound by the map project;
                                optional location (E2): a point {x, y, floor} or boxes of whole tiles
                                {x: [min, max], y: [min, max], floor}, each on one floor, in Canary map coordinates
  state
    counters[]                  name, initial integer
    flags[]                     name, initial boolean
    timers[]                    name, duration_ms, repeat
  rules[]                       trigger, optional delay_ms, conditions[], actions[] (in order)
  abilities[]                   optional area effects authored by the encounter (D34): square or circle
                                radius, damage type and range, affected players and named creatures,
                                optional visual effect; a `cast` of `encounter_ability` uses one
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
| `ability_cast(role, AbilityRef)` | a monster spell script with fight effects (D29); a spell whose script only summons is converted as an ability that points to its encounter, and the encounter does the summon (D45) |
| `damage_taken(role, source: player/any)` | `onHealthChange` per hit |
| `heal_received(role, source: player/any)` | `onHealthChange` per heal (Canary runs the handler for heals too) (D31) |
| `damage_accumulated(role, amount or percent)` | `onHealthChange` damage counters: fires each time one creature of the role has taken `amount` damage, or `percent` of its maximum health (the resolved creature definition's, after wiki adoption), since it appeared or since it last fired; the count then restarts at 0 and heals do not count (D34) |
| `timer_elapsed(timer)` | `addEvent` delays, `onThink` countdowns |
| `counter_reached(counter, value)` | global kill/stage counters |
| `area_entered(anchor, role or player)` / `area_left` | zone crossing (`izcandarThink`) |
| `phase_entered(phase)` | stage bosses |
| `item_used(role, ItemRef)` | an `Action` whose `onUse` targets a creature of the role; the item is used up (D34); an optional `base_vocation` lets only a player of that base vocation use it (a promoted vocation counts as its base) |
| `stepped_on(role, ItemRef)` | a `MoveEvent` stepin registered on the item: a creature of the role steps onto a tile holding it (D46) |
| `encounter_started` / `encounter_reset` | lifecycle |

## 5. Conditions

`chance_percent`, `counter_compare(counter, op, value)`, `flag(name, value)`,
`creature_present(role, anchor or near(role, radius, square or circle), present/absent)`, `in_anchor(role or killer, anchor)`,
`killer_is_player` (the killing damage is a player's own; damage from a player's summon or familiar is not; SW-4 adds an optional `value: false` for the opposite, §13.2), `has_master(role, value)` (Canary skips summoned copies of a boss),
`has_condition(role, conditions, present)` (D46: whether the role's creature has any of the listed conditions on itself:
poison, fire, energy, bleeding, drown, freezing, dazzled or cursed),
`summon_count(role, op, value)` (D45: how many live summons a creature of the role has; Shulgrax calls
more only while it has fewer than 8),
`health_percent(role, op, value)` (fractional allowed),
`attacker_wears(ItemRef)` (the Asura counter items), `killer_progress(quest key, op, value)` - a
read-only view of the killer's quest progress published by the quest domain (the Soul War taints);
the encounter never writes it. `world_state(key, op, value)` (D29) is the same read-only view of a
value another domain publishes for the world or channel (a quest stage, a world counter, an item
buff such as the cobra flask). `chance_from_amount(per)` (D34) holds with a chance of the amount of
the triggering change divided by `per`; it is allowed only in `damage_taken` and `heal_received`
rules.

## 6. Actions

| Action | Parameters |
|---|---|
| `spawn` | role or CreatureRef, count, at (`death_position`, `subject_position`, anchor, `random_in(anchor)`, `offset_tiles(n)`: a random free tile within n tiles of the subject, `closest_free_tile`: the free tile nearest the subject (D34), `relative(x, y)`: the tile at that offset from the subject on its floor, used even when occupied (D46; like `subject_position`, `offset_tiles` and `closest_free_tile` it needs a trigger fired by one creature), or `role_position(role)` optionally `otherwise: death_position` (D31)), owner (none, subject, or `death_master`: the master of the dying creature), health (`full`, `carry_over`, percent, or `remembered`: the health the spawned role had when it last left the fight, full the first time (D31)) |
| `spawn_per_player` | `players_in(anchor)`, `by_base_vocation` (a CreatureRef for each base vocation that gets one: knight, paladin, sorcerer, druid, monk), at, owner, health; each player in the area gets one creature of its base vocation, players of a vocation without an entry get none, and an optional `counter` is raised by the number spawned (D34) |
| `remove` | role, `all_in(anchor)` (monsters only; players are never removed; `keep_summons` spares monsters with a master), or `triggering`: only the creature that fired the rule (D31) |
| `transform` | role -> next stage, CreatureRef or `random_of` several CreatureRefs (uniform); health `keep_percent`/`keep_absolute`/`full` |
| `heal` | role, amount, range (may start at 0, D31) or `full` |
| `damage` | subject, amount or range, damage type (D29); type `none` is a direct health change that no resistance, buff or immunity changes and that credits no one (D31) |
| `prevent_death` | only after `lethal_damage` |
| `damage_modifier` | role, multiplier (0 = immune), damage types, sources, duration or until reset; `component: primary` limits it to the primary part of a hit (D29); the multiplier may be `timer_remaining(timer, floor)`: the percent of a non-repeating timer's duration still left when the action runs, at least `floor` (D34) |
| `shared_life` | role: every creature of the role present now shares one health; a change to one sets the others to the same health, and a lethal hit kills them all (D34) |
| `reflect_damage` | role, percent, damage types |
| `convert_damage_to_heal` | role, damage types, optional `component` |
| `teleport` | role or `players_in(anchor)`, to anchor |
| `map_item` | create/transform/remove ItemRef at an anchor or `at: death_position` (death and lethal damage triggers), `revert_after_ms`; a teleporter carries `destination` and optionally `revert_destination` anchors; a revert restores the original item with its original attributes unless `revert_destination` overrides the destination; optional `effect`; optional `interaction`: the key of interaction-domain content that defines what the item does when used or stepped on (D29) |
| `counter` / `flag` / `timer` | set, add, start, stop; a timer `add` of `ms` delays a running timer and does nothing to a stopped one (D34) |
| `set_phase` | next or named phase (phase changes are triggers too: `phase_entered(name)`) |
| `cast` | AbilityRef, or an `encounter_ability` of the encounter's own `abilities` (D34), at a position (death explosions) |
| `say` | role, killer or `spawned` (the creature of the preceding one-creature `spawn` in the same action list, D31), text, mode |
| `message` | text to every player in an anchor area (D31) |
| `one_of` | two or more weighted branches, each a list of actions; the encounter instance draws one (D31) |
| `drop_item` | ItemRef, chance, at role position |
| `attribute` | role, `outgoing_damage_percent` (extra percent on the primary damage the role deals to players) or `defense`, `add` a value or a counter's value, or `reset` to the creature type's value (D34) |
| `move_lock` | role, `locked`: a locked creature keeps fighting and casting but does not move (D34) |
| `emit_outcome` | named outcome for quests, cooldowns and rewards (§2.5), `credited`: `damage_contributors`, `killer`, `players_in_anchor(anchor)` or `party`: the party of the top damage contributor, wherever its members are (D31) |

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
   `health_crossed` fires when a creature's health falls from above the threshold to at or below it;
   a Canary check with a strict `<` differs only at the single health value on the threshold and is
   noted in the manifest. The conditions of a `damage_taken` or `heal_received` rule read the health
   before that change, as Canary's `onHealthChange` does. `role_position` is taken when the trigger
   fires, like `death_position`. A `near` area is a square (Canary spectator ranges) unless
   `shape: circle`, which is Canary's circle area of that radius (every tile with dx² + dy² ≤ r² + 1).
   In a rule triggered by one creature (a death, lethal damage, damage, health, spawn or cast
   trigger), `transform`, `prevent_death`, `heal`, `damage`, `damage_modifier` and `say` naming the
   trigger's role act on that creature; `remove` of a role removes every creature of the role. A
   delayed action on a creature that no longer exists does nothing. `creature_spawned` fires when a
   creature of the role appears: placed by the map or a lever, spawned, or transformed into the role.
3. Randomness (`chance_percent`, random positions, `random_of`, `{min, max}` ranges for spawn
   counts, heal and damage amounts, rule delays and timer durations) is drawn uniformly by the
   encounter instance, so a fight can be audited and replayed from its seed.
4. Health carried by `transform`/`spawn` is explicit (`keep_percent`, `keep_absolute`, `full`,
   percent, or `remembered` for a spawn into a named role); nothing is implied.
5. Anchors are typed (point or area) and must all be bound by the map project before the encounter is
   activated by a runtime; an unbound anchor blocks activation, never falls back to raw coordinates. Content
   admission into WorldProject/v2 takes the anchor's location in the project frame (E2,
   `OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1.md`); an anchor without a location blocks admission.
6. Validation mirrors the monster schema: JSON Schema plus a semantic validator and an import
   manifest in which every Canary event line is mapped, omitted with a reason, or unresolved.

## 10. Owner decisions

| # | Decision | Basis |
|---|---|---|
| D26 | Encounters are instanced per party by default (a separate copy of the arena for each party), unlike Canary's one shared arena; `channel_shared` is an explicit opt-in per encounter. | Owner request 2026-09-27; `WorldId + InstanceId` in FND-ID-01; "Instanced dungeon" in the scope matrix. |
| D27 | Encounters only emit named outcomes. Boss cooldowns, reward eligibility and reward rooms are consumed by the reward domain; quest steps by the quest domain. | Owner delegated the choice; the scope matrix already assigns boss reward eligibility to the reward domain. |
| D28 | §4-6 plus phases is the v1 vocabulary under the rules of §9. Work starts with `FourthTaintBossesPrepareDeath` (15 Soul War hunting monsters, a `channel_shared` zone rule with a read-only quest-progress condition), then the largest death events. | Owner delegated the choice; the source shows the event is a zone rule, not a boss fight. |
| D29 | Vocabulary extensions: `creature_spawned` and `ability_cast` triggers; the `world_state` read-only condition; fractional or absolute health thresholds; `creature_present` near a role; `{min, max}` ranges; a `damage` action; `component: primary`; `map_item.interaction`. Acting on whatever stands on a fixed tile is not added: each case names its role from wiki or map evidence. Scripted movement is deferred. State shared by all parties belongs to the quest domain, which the encounter reads through `world_state`. | Owner accepted the proposal ("kontynuuj tak jak uważasz za optymalne", 2026-09-27). |
| D30 | Crystal Server (`zimbadev/crystalserver`, a Canary fork) is consulted as a second donor wherever a Canary script is broken, ambiguous or unresolved. It is evidence only: Canary stays the transcription source and the reference-date wiki still decides (D25). | Owner request 2026-09-27 ("sprawdzać też crystal jako donor"). |
| D31 | Vocabulary additions, each added only for an event that needs it: `heal_received`; `message` to the players in an area; `remove triggering` and `keep_summons`; weighted `one_of` branches; `role_position` (with an optional `otherwise: death_position`); a `spawned` speaker; the `party` credit; circular `near` areas; heal ranges from 0; the untyped `none` damage; `remembered` spawn health (a boss that returns with the health it left with: Foreshock, Aftershock, Outburst); in a `heal_received` rule a `this_hit` `damage_modifier` scales that heal (`HealthForgotten` doubles heals as well as damage); a `non_player` source for `damage_taken` and `heal_received` (a change by another creature; one without an attacker is not included); an optional `slot` for `attacker_wears`, which with `killer_progress` also reads the healer in a `heal_received` rule (`AsurasMechanic`). Boss attribute changes and a stepped-on trigger are not added yet. | Owner consent 2026-09-27 ("jeśli kończenie zadania tego wymaga i wiesz co robisz, to masz zgodę"). |
| D34 | Seven more vocabulary additions, each added with the first event that needs it: a boss attribute change (the Hatred damage multiplier); damage scaled by elapsed time (King Zelos); shared life (the Magnor shards); a death explosion as an authored ability; a summon chosen by the vocation of the player; `move_lock`; and `chance_from_amount`. The fourteenth slice adds five of them: the time scaling, shared life, authored abilities, `move_lock` and `chance_from_amount`; it also fixes when `damage_accumulated` fires. The fifteenth adds the boss attribute change with the `item_used` trigger and the timer `add` it needs (the Sorrow of Burning Hatred). The seventeenth adds the per-vocation summon (`spawn_per_player` and the `item_used` `base_vocation`) with Count Vlarkorth. | Owner answer 2026-09-27 ("Wszystkie 7"). |
| D45 | A monster spell whose Canary script summons creatures stays an ability of the monster, but the ability points to an encounter (`encounter`) instead of listing effects. The encounter's `ability_cast` rule does the summon, with the counters, flags and timers the script keeps. The `summon_count` condition is added for it. | Owner answer 2026-09-28 ("Tak, przez encountery"). |
| D46 | Two vocabulary additions for the events that need them: a `stepped_on` trigger (a creature of a role steps onto a tile holding an item; the Heart of Destruction vortex) and a `has_condition` condition on a role's own conditions (the Soulcatcher's summon while poisoned or bleeding), with the `relative` position that event places its summon at. | Owner answer 2026-09-28 ("tak"). |

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

A sixth slice rechecked a research brief against the source. The brief was often wrong and was
corrected before transcription. Canary passes damage to `onHealthChange` as a negative value, so
handlers that test `primaryDamage > 0` only react to heals.

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `PythiusTheRottenDeath`, `TheRavagerDeath`, `TheShattererDeath`, `ThePrimalMenaceDeath`, `TireczDeath` | 5 | 5 | Kill outcomes (quest, reward room, achievement and hazard level for their domains); exit teleporters; Tirecz's arena is emptied. The Shatterer's chain and lever reset only prepares the shared room for the next group (D26). |
| `FirstDragonDeath` | 1 | 1 | The lair's players are credited, then teleported out. |
| `WhiteDeerDeath`, `WhiteDeerScoutsDeath` | 1 | 2 | A `channel_shared` hunting rule; one roll picks the enraged or the desperate deer, which a flag reproduces exactly. |
| `Splash`, `MakeshiftHomeDeath`, `OrganicMatterDeath`, `FerumbrasSoulSplinterDeath`, `FerumbrasEssenceImmortal` | 4 | 5 | The Wine Cask summons a liquor spirit and heals; the rubble with its escort interaction; the immortal essence. |
| `TentacleDeep` | 1 | 1 | Canary decides by the first spectator in its list. The wiki decides (D25): a dead tentacle regrows after 10 s while another lives, and the Deep Terror rises once all are dead. |
| `GazHaragothHeal`, `MinionGazDeath` | 1 | 2 | Below 12.5% Gaz'haragoth heals 300,000 seven seconds later; its regeneration condition heals 0 and only marks the wait. Minions leave a one-minute nightmare vortex. |
| `UsurperCommanderDeath`, `KesarImmortal` | 1 | 2 | The last of three usurper commanders brings Kesar, who is immortal, and Drume. |

These events stay unresolved; each needs one of these additions, which go to the owner under D28:

| Needed | Events |
|---|---|
| a message to the players in an anchor | `VersperothDeath`, `LionCommanderDeath`, `ParasiteDeath` |
| remove only the triggering creature (a role `remove` takes all of them) | `Evaporate`, `TheWelterEgg` |
| a uniform or weighted choice for `spawn` and `drop_item` | `PossessedTree`, `UglyMonsterDrop` |
| spawn at another role's position | `SoulWarAspectOfPowerDeath` (a new aspect appears at the boss) |
| rules on heals (Canary runs health-change handlers for heals too) | `SoulcatcherSummon`, `LeidenHeal` (the wiki: healing damages Leiden), `soul_heal` |
| a creature stepping on an item | `AngryPlantDeath`: its corpse turns the Unbeatable Dragon that steps on it into Somewhat Beatable |
| boss attribute changes (defence, element reflection, outgoing damage) | `SoulCageDeath` (Goshnar's Malice absorbs the cage), `NecromanticFocusDeath` |
| an authored Ability for a death explosion (spell work, P4) | `WormlingDeath` |
| per-player counters, a count of nearby creatures | `SomewhatBeatableDeath`, `SnailSlimeThink` |

`SoulCageHealthChange` reflects only heals by players; its manifest waits for the cage's death rule.
61 encounters validate, 55 manifests resolve fully; the census rises from 1,463 to 1,478.

A seventh slice uses D31 and D30:

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `lokathmorDeath`, `mazzinorDeath`, `mazzinorHealth` | 2 | 2 | Both death events check the name of a knowledge the boss never has, so no parchment or vortex ever appears. The wiki decides (D25): Dark Knowledge leaves the parchment, Wild Knowledge a one-minute vortex. `mazzinorHealth` re-applies each hit without an attacker, which would leave the reward boss without rewards; the wiki describes an ordinary fight, so it is not reproduced. |
| `LionCommanderDeath` | 1 | 1 | The last lion commander loses the skirmish: the room is emptied (summons stay), and the players get a message and leave. |
| `Evaporate` | 1 | 1 | Leiden in the spirit's radius-3 circle loses 3,000-6,000 untyped health; that arms Leiden's death to bring Ravenous Hunger (`SpawnBoss`); only this spirit disappears. |
| `TheWelterEgg`, `PossessedTree`, `UglyMonsterDrop` | 3 | 3 | An egg hatches after 10 s; a tree unleashes one of four unbound creatures and regrows after 60 s; the Ugly Monster drops one of eleven items, weighted, on 1% of its health changes. |
| `SoulcatcherSummon`, `DeathPriestShargonDeath`, `SnailSlimeDeath`, `SnailSlimeThink` | 3 | 3 | Heals count as health changes; Shargon credits the top damage dealer's party; `SnailSlimeThink` checks a name its slime never has. |

Ghulosh stays unresolved: Crystal's lever starts the stage counter that Canary never starts. Neither
server brings the Deathgaze back to Ghulosh, and the wiki describes that without numbers.
`SoulWarAspectOfPowerDeath` stays unresolved too. Canary starts the blue Annihilation form after
every vulnerable phase, while the wiki says it happens once per fight and does not say what starts
it.
67 encounters validate, 61 manifests resolve fully, `verify_encounter_schema.py` 84/84; the census
rises from 1,478 to 1,486.

An eighth slice:

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `CarlinVortexDeath` | 1 | 3 | A `channel_shared` rule: a dying cultist leaves one of two soul remains (`one_of`) for one minute, with the mission interaction of action id 5580. |
| `DeathDragon` | 1 | 1 | The guard reads an undefined global, so Canary never runs the body; the wiki says Ragiaz's death dragons respawn immediately (D25), so the body is transcribed: a new dragon after 1 s and Ragiaz's line. |

The Crystal comparison (D30) of the remaining events found 68 identical scripts. Three differ only
cosmetically. Four differ in behaviour: nil guards and a Monk's Apparition branch in Mirror Image,
none of which changes a transcription. Sixteen mechanics were removed in Crystal, most of them in
Grave Danger. Crystal's lever for Ghulosh is the one fix that bears on a transcription.
68 encounters validate, 62 manifests resolve fully; the census rises from 1,486 to 1,490.

A ninth slice transcribes the five Heart of Destruction boss rooms and the World Devourer's death. Crystal carries the
same eleven scripts.

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `HeartBossDeath` | 6 | 7 | The room vortex opens (action id 14325, 14354 for the World Devourer) and every player in the room is credited (D27); the World Devourer's room is emptied. Eradicator2 is also named "Eradicator". |
| `AnomalyTransform`, `ChargedAnomalyDeath` | 1 | 2 | At 75, 50, 25 and 5% the anomaly gives way to a charged anomaly; its death brings the anomaly back at that health. |
| `RuptureResonance`, `RuptureHeal` | 1 | 1 | Five resonance waves at 80, 60, 40, 25 and 10%; while a damage resonance lives, each player hit heals the rupture by 5,000-10,000. |
| `ForeshockTransform`, `AftershockTransform`, `ShocksDeath` | 1 | 2 | The shocks swap at every threshold, each coming back with the health it left with; the aftershock's death brings Realityquake. The wiki spawns Realityquake after both are defeated, so a dead foreshock does not return (D25). |
| `EradicatorTransform` | 1 | 2 | Timers: 74 s as Eradicator, then 9 s as Eradicator2 with four sparks, with the same health. |
| `OutburstCharge`, `ChargingOutDeath` | 1 | 2 | At 80, 60, 40 and 20% the outburst gives way to a charging outburst; killing it brings the outburst back with the health it left with. |

A threshold checked by `onThink` is a `health_crossed` rule, plus a `creature_spawned` rule when the boss can come back
already below the threshold of its stage. Charged Anomaly, Charging Outburst and the World Devourer keep one unresolved
spell script each. 72 encounters validate, 66 manifests resolve fully, `verify_encounter_schema.py` 87/87; the census
rises from 1,490 to 1,498.

A tenth slice completes the Forgotten Knowledge fights. Crystal carries the same scripts; its Lloyd script only adds nil
guards.

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `HealthForgotten` | 2 | 4 | Without a shadow tentacle (Lady Tenebris) or a possessed tree (the Thorn Knight forms) within 7 tiles, the primary part of every health change is doubled, heals included. |
| `ThornKnightDeath` | 1 | 2 | The mounted knight becomes the shielded knight and a thorn steed, then the enraged knight. |
| `LloydPrepareDeath`, `EnergyPrismDeath`, `EnergyPrismHealthChange` | 1 | 5 | Four times Lloyd survives, stands between the prisms at full health and makes the next prism killable for 10 s; a prism heals 10,000 on every change while he is away from the centre. The wiki's Lloyd page describes the same five kills. |

Lady Tenebris and the Mounted Thorn Knight keep unresolved spell rows. 72 encounters validate, 66 manifests resolve
fully, `verify_encounter_schema.py` 88/88; the census rises from 1,498 to 1,505.

An eleventh slice. Crystal carries the same scripts.

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `facelessHealth` | 2 | 2 | Primary death damage heals Alptramun, primary earth damage heals Plagueroot. |
| `AsurasMechanic` | 3 | 3 | Only a prepared player hurts or heals an Asura: one of twelve red armours in the armour slot (The Diamond Blossom), silver chimes in the right hand (The Blazing Rose), an active fragrance (The Lily of Night, read-only quest progress). Changes by other creatures are zeroed. The wiki describes none of it. |
| `GreedMonsterDeath` | 1 | 5 | Each greed creature returns to its tile 10 s after its death; greedbeast deaths are counted for Goshnar's Greed. |

`BurningChangeForm` changes a boss attribute (the Hatred damage multiplier), which D31 does not add. `izcandarThink` and the
King Zelos events stay for a later slice. 76 encounters validate, 70 manifests resolve fully, `verify_encounter_schema.py`
91/91; the census rises from 1,505 to 1,511.

A twelfth slice. Crystal carries the same scripts.

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `DisruptionTransform`, `ChargedDisruptionTransform` | 1 | 2 | Counted thinks: a disruption becomes a charged disruption 12-13 s after it appears, a charged one becomes overcharged after 18-19 s (a rule delay range, D29). |
| `CracklerTransform`, `DepolarizedTransform` | 1 | 2 | On every think (a repeating 1 s timer) cracklers turn depolarized while the room is polarized and back when it is not, keeping their health. The polarization is written by the vortex tiles (`movements_vortex_crackler.lua`), a stepped-on mechanic transcribed with the room. |
| `ReplicaServantDeath` | 1 | 2 | A `channel_shared` outcome: the quest domain counts the servants for the world and for the top damage dealer, and opens the teleport at five of each. |

The King Zelos events stay unresolved: King Zelos's damage scales with the time the knights took, the Magnor shards share
one life, and two deaths explode as authored areas. `UglyMonsterSpawn` draws its chance from the damage dealt. 78 encounters
validate, 72 manifests resolve fully; the census rises from 1,511 to 1,517.

A thirteenth slice transcribes `mType.onSpawn` callbacks as `creature_spawned` rules (D29):

| Monster | Encounter | Notes |
|---|---|---|
| Iron Servant Replica | `replica_servants` | 70% become a diamond or golden replica, depending on the mechanism world values the quest domain publishes. |
| Cobra Assassin, Cobra Scout, Cobra Vizier | `cobra_bastion` (`channel_shared`) | While the cobra flask works (a world value), each appears with 75% of its health: untyped damage of a quarter of its health. |
| Lion Commander, Usurper Commander | `drume` | Each arrives with five summons drawn uniformly from its summon list. |

Crystal's monster files carry none of these callbacks; Canary is transcribed (D30). 79 encounters validate, 73 manifests
resolve fully; the census rises from 1,517 to 1,523.

A fourteenth slice uses five of the D34 additions:

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `UglyMonsterSpawn`, `UglyMonsterCleanup`, `UglyMonsterDeath` | 1 (`channel_shared`) | 3 | A hit on Gaffir or Guard Captain Quaid calls the Ugly Monster with a chance of the damage per million (`chance_from_amount`), once per boss and while no ugly monster is out; it leaves after 60 s unless killed first. |
| `BossHealthCheck` | 1 | 2 | Sir Baeloc waits in place (`move_lock`) while Sir Nictros fights. At 85% Nictros steps back to his post and stands still, and Baeloc comes in; at 85% Baeloc calls his brother back and both fight. |
| `zelos_damage`, `zelos_init`, `rewar_the_bloody`, `fetter_death`, `blood_death`, `magnor_death`, `shard_death`, `nargol_death` | 1 | 7 | King Zelos takes the percent of an 800 s ritual timer still left when the four knights are done, at least 1% (`timer_remaining`). Canary registers `zelos_init` only on King Zelos and never records the start, so its King Zelos takes normal damage; the wiki (D25) describes the ritual. Every 12,500 damage Rewar calls one to three fetters and turns immune until the last dies. The four shards of Magnor share one life (`shared_life`) and each explodes when it dies; the vampiric bloods explode with drown damage that hurts players and The Red Knight (authored `abilities`). Canary registers the blood explosion on Rewar; the wiki gives it to the vampiric bloods. Nargol's regenerating mass brings him back after 30 s unless it is killed. |

81 encounters validate, 75 manifests resolve fully; the census rises from 1,523 to 1,533.

A fifteenth slice adds the last D34 addition used by Soul War, the boss attribute:

| Event | Encounters | Covered monsters | Notes |
|---|---:|---:|---|
| `BurningChangeForm`, `GoshnarsHatredBuff`, `mType.onSpawn`, `mType.onDisappear` (Goshnar's Hatred) | 1 | 5 | The campfire takes its next form every 45 s (the Blaze 46 s), one timer per form. Each new Ashes raises the hatred by 10: Goshnar's Hatred deals 10% more to players, and every player hit adds the hatred to its defense (`attribute`). A Sorrow used on the fire (`item_used`) delays the next form by 10 s (timer `add`). The fire is removed when the boss dies. |
| `mType.onSpawn` (Mighty Splinter of Madness), `GoshnarsHatredBuff` (Goshnar's Megalomania) | 1 | 4 | A mighty splinter still in the room after 120 s is absorbed and raises the madness by 5; Canary's callback fails on an undefined global, and the wiki decides (D25). Every player hit adds the madness to Megalomania's defense. Canary's outgoing branch never applies to Megalomania. |

82 encounters validate, 77 manifests resolve fully; the census rises from 1,533 to 1,539.

A sixteenth slice moves the arena summon spells into their encounters (D45). Each spell stays an
ability of its monster and points to the encounter; the encounter's `ability_cast` rule summons:

| Spell | Encounter | Covered monsters | Notes |
|---|---|---:|---|
| `razzagorn summon` | `razzagorn` | 1 | Four Demons appear anywhere in the arena, with no master. Canary only: the reference-date wiki lists Eruption of Destruction and no Demons, so this needs checking (D25). |
| `shulgrax summon` | `shulgrax` | 1 | While Shulgrax has fewer than 8 summons (`summon_count`), four Sin Devourers as his summons and four Damned Souls with no master; the wiki names both in his room. |
| `rage summon`, `destruction summon` | `world_devourer` | 2 | The Rage calls a Frenzy and The Destruction a Disruption next to itself (`offset_tiles` 1) while fewer than 3 have been called; The Destruction waits 15 s between calls (a flag cleared by a timer). The wiki names both minions. |

The Hunger (a counter the vortex lowers when Greed steps on it) and the Glooth Generator (a
per-creature 14 s timer) stay unresolved. 82 encounters validate, 77 manifests resolve fully,
`verify_encounter_schema.py` 116/116; the census rises from 1,548 to 1,552. Like the other encounter-covered monsters,
the four wait in the creature staging until encounters are admitted.

A seventeenth slice uses the last D34 addition, the per-vocation summon:

| Event | Encounter | Covered monsters | Notes |
|---|---|---:|---|
| `count_vlarkorth_transform`, the Good Remains actions | `count_vlarkorth` | 1 | Every 15% of the boss's maximum health taken as damage each player in the room gets one dark creature of its base vocation, raising a shield; each good remains used on the boss by that vocation lowers it. The reference-date wiki (Fandom rev 1140872) decides where Canary differs (D25): two waves, no damage while the shield holds (Canary only stops counting), and a Dark Merudri with its remains (item 50311) for a Monk, which Canary lacks (D44). |

83 encounters validate, 78 manifests resolve fully, `verify_encounter_schema.py` 128/128; the census rises from 1,552 to
1,553. Count Vlarkorth waits in the creature staging like the other encounter-covered monsters.

An eighteenth slice moves the remaining summon spells that the vocabulary already expresses into their bosses' encounters
(D45):

| Spell | Encounter | Notes |
|---|---|---|
| `devourer summon` | `world_devourer` | While fewer than three minions are out (the existing `devourer_summons` counter, which Frenzy and Disruption deaths lower), one of Greed, Frenzy or Disruption next to the World Devourer; the wiki names all three in the fight. |
| `plagirath summon` | `plagirath` | Plagirath fills its summons up to four Disgusting Oozes anywhere in its room (one rule per number already out); the wiki places them with Plagirath. |
| `tenebris summon` | `lady_tenebris` | A Shadow Fiend anywhere in the room, which says "The shadow fiend revives!"; the wiki places it with Lady Tenebris. |
| `thorn summon` | `the_enraged_thorn_knight` | A Thorn Minion within three tiles of the Mounted Thorn Knight. The wiki places Thorn Minions in the Thorn Knights' dungeon but does not name the summon: Canary evidence, to be verified. |

83 encounters validate, 78 manifests resolve fully; the census rises from 1,553 to 1,555 (World Devourer and Mounted Thorn
Knight). Plagirath and Lady Tenebris stay blocked by `plagirath bog` and `tenebris ultimate`; The Hunger waits for the
vortex that lowers its summon counter, and Soulcatcher for a condition on the caster's own poison or bleeding.

A nineteenth slice adds D46:

| Event | Encounter | Notes |
|---|---|---|
| `hunger summon`, `movements_vortex_hunger` | `world_devourer` | While fewer than three Greeds are out and 15 s after the last, The Hunger calls a Greed next to itself. The Hunger or the World Devourer stepping on a closed vortex opens its vortex tile (it stays open: the vortex has no decay); a Greed stepping on an open vortex disappears and lowers both summon counters. The wiki describes walking the boss over the teleport and then the Greed. |
| `soulcatcher summon` | `soulcatcher` | While the Soulcatcher is poisoned or bleeding, a Corrupted Soul on the tile north of it. The wiki gives no abilities for the Soulcatcher: Canary evidence, to be verified. |

83 encounters validate, 78 manifests resolve fully, `verify_encounter_schema.py` 139/139; the census rises from 1,555 to
1,557 (The Hunger and Soulcatcher).

A twentieth slice adds the anchor locations of E2 (`OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1.md`, owner answer
2026-09-28 "tak"). A point is `{x, y, floor}` and an area is a list of boxes of whole tiles, each on one floor; the validator
checks that a point has a point and an area has boxes, and that no box starts after it ends. Two corners on different
floors fail. `canary_encounters.py` reads each location
from the Canary coordinates the anchor already describes: a point, a rectangle between two corners, a square of a
radius around a tile, or a list of single tiles. It fails on any other form. The five Essence of Malice spots are read
from the positions in the lever script. 141 of the 142 anchors are located. The Soul War taint zones stay without a
location: they subtract safe areas and include the Goshnar boss rooms, so that encounter is not admitted yet.

83 encounters validate, 78 manifests resolve fully, `verify_encounter_schema.py` 148/148. No manifest changes, so the
census is unchanged.

## 12. Extensions for the remaining unresolved rows (CW2)

- Status: **ACCEPTED** 2026-09-29 (owner batch 2 on #162, comments 5884513528 and 5884527699). The owner questions
  are answered in §12.6. CW2-1..4 join the closed v1 vocabulary of §9 rule 1: §4-6 are read with these widenings.
- Tasks: design `OTV2-20260928-cw2-encounter-vocabulary-extensions`; implementation
  `OTV2-20260929-cw2-encounter-vocabulary-impl` (schema, validator and focused checks, §12.5 step 1).
- Scope: Alptramun, Gorzindel, Melting Frozen Horror and The Sandking. Ferumbras Mortal Shell waits for a
  quest-domain contract and is not covered here.
- Evidence: CrystalServer (`zimbadev/crystalserver` at `ff7ede593c69d4c658b382c97443e8155926924a`) is the donor
  checked under D30. Its `data-global/` pack carries the Canary scripts. Its `data-crystal/` pack registers no events
  on these bosses and holds none of their fight scripts, so it adds nothing. Paths below are relative to that clone;
  the Canary transcription source stays the manifest source.
- Rule: where the Canary behaviour is clear it is followed; a deviation or a product choice is an owner question
  (§12.6), never decided here. The four manifest rows stay `unresolved_semantics` until their transcription slice
  (§12.5 step 2).

| Boss | Unresolved mechanic | Resolution |
|---|---|---|
| Alptramun | the `alptramun summon` escalation | no extension: global replacement by the next tier on death, with `creature_died` and `spawn` (Q1) |
| Gorzindel | the Stolen Tome of Portals portal: each player to the next free knowledge room for 10 s | CW2-1: `teleport` and `in_anchor` of the `triggering` creature |
| Melting Frozen Horror | death actions on "the top creature" of two fixed tiles | no extension: the lever script names both roles; the Solid form is always removed (Q4) |
| The Sandking | the stage counter the credit reads, and the fight that advances it | CW2-2: `stepped_on` a role's `corpse_of`; CW2-3: `map_item remove triggering`; CW2-4: `random_in` with `free: true` |

All four extensions widen a parameter of an existing term. No trigger, condition or action kind is added, so the
counts in E1 of `OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1.md` (17 triggers, 14 conditions, 25 actions) stay
the same.

### 12.1 Alptramun: no extension (global replacement, Q1)

1. **Mechanic.** `alptramun summon`: dreams whose type rises with the `dreams_killed` counter.
2. **Canary behaviour.**
   - `data-global/scripts/spells/monster/alptramun_summon.lua:15-47`: while the caster has fewer than 5 summons, it
     summons 1-4 dreams as its own summons (line 45) next to itself.
   - The dream type follows the counter: unpleasant at 0-9, horrible at 10-18, nightmarish at 19-27, mind-wrecking
     at 28-36, and one of the four at random above 36 (lines 25-35).
   - The spell is registered (line 53), but no monster casts it: Alptramun's attacks and defenses do not list it
     (`data-global/monster/quests/the_dream_courts/bosses/alptramun.lua:120-132`).
   - The counter skips every creature with a master
     (`data-global/scripts/quests/the_dream_courts/creaturescripts_dream_courts_death.lua:68-70`), so dreams from
     the spell would never raise it (lines 107-114).
   - The only dreams it counts are the two unmastered unpleasant dreams the lever places
     (`data-global/scripts/quests/the_dream_courts/actions_dreamscar_levers.lua:22-25`). As shipped, the fight never
     escalates.
3. **Owner answer (Q1): follow global, not Canary.**
   - A killed dream is replaced at once by the next tier: unpleasant, then horrible, then nightmarish, then
     mind-wrecking. The tier is capped at 4, so a killed mind-wrecking dream is replaced by another mind-wrecking
     dream.
   - The initial dreams are the two unmastered unpleasant dreams the lever places (`actions_dreamscar_levers.lua:22-25`,
     entry placement), which matches the wiki's "initially appear with several summons".
   - The dreams' own healing stays the Canary monster data: horrible 20-30, nightmarish 120-190, mind-wrecking
     130-205 (the `COMBAT_HEALING` defenses of their monster files). It is monster content, not encounter content.
4. **Extension.** None. The replacement is one `creature_died` rule per tier with a `spawn` at `death_position`,
   `owner: none` (like the lever's dreams, so the next death fires again) and `health: full`. It has no delay. The
   `has_master` condition keeps Canary's skip of summoned copies. The spell, its `ability_cast` form (D45) and the
   `dreams_killed` counter are not used: nothing casts the spell, and nothing reads the counter once it is gone.
   ```json
   {"key": "unpleasant_dream_replaced", "trigger": {"kind": "creature_died", "role": "unpleasant_dream"},
    "conditions": [{"kind": "has_master", "role": "unpleasant_dream", "value": false}],
    "actions": [{"kind": "spawn", "role": "horrible_dream",
                 "creature": {"family": "Creature", "key": "canary:creature/horrible_dream", "revision": "canary-47dfd51f"},
                 "count": 1, "at": "death_position", "owner": "none", "health": "full"}]}
   ```
   The horrible and nightmarish rules follow the same pattern, and the mind-wrecking rule spawns a mind-wrecking
   dream.
5. **Validation and tests.** None for the vocabulary: the four rules validate against the current schema with the
   sample's catalog. The transcription records `alptramun_summon.lua` 1-59 as `approved_omission` (never cast;
   superseded by the global replacement) and replaces the Canary counter rules with the four replacement rules.
6. **Out of scope.** Alptramun's other spells, `facelessHealth` (already mapped) and the lever's day-of-week boss
   rotation (entry, not encounter).

### 12.2 Gorzindel: CW2-1, `teleport` and `in_anchor` of the `triggering` creature

1. **Mechanic.** The portal left by the Stolen Tome of Portals sends each player who steps in to the first free
   knowledge room, and brings that player back after 10 s.
2. **Canary behaviour.**
   - The dying tome leaves item 1949 with action id 4952 on its tile. After 10 s the tome returns there and the item
     is removed (`data-global/scripts/quests/the_secret_library_quest/library_area/creaturescripts_gorzindel.lua:38-50`).
   - The tome never moves (speed 0, not pushable:
     `data-global/monster/quests/the_secret_library/stolen_tome_of_portals.lua:14,35`). Its tile is its lever
     placement (32688, 32715, 10) (`data-global/scripts/actions/bosses_levers/gorzindel.lua:27`). Whenever no portal
     exists, the tome stands on that tile.
   - The portal (`movements_gorzindel.lua:13-38`, action id 4952) acts on players only (lines 14-16). It sends the
     player to the first of five rooms still marked open, in index order (lines 1-7, 20-23), and marks it busy. 10 s
     later the player, if still online, goes to the middle (32687, 32719, 10) and the room reopens (lines 24-30).
   - The five rooms are the tiles of the five stolen knowledges (`gorzindel.lua:22-26`).
   - **Owner answer (Q2): yes.** The rooms are per instance and reopen after 10 s in every case. The delayed return
     teleports only players still in the instance, which fixes Canary's server-wide room leak.
3. **Extension CW2-1.** `teleport.who` also takes `{"triggering": true}`: the creature or player that fired the
   rule. This extends the D31 `remove triggering`. It is valid in a rule fired by one creature (§9 rule 2) and in
   `area_entered`/`area_left`, which also fire per creature.
   - CW2-1 also lets the existing `in_anchor` condition take `{"triggering": true}` as its subject, in the same
     rules. It holds only while that creature or player is still in this encounter instance and stands in the
     anchor. It is false for one that has left the instance, even if it is still online elsewhere. This is the
     membership check the delayed return needs, and no new condition is added. `triggering` is not added to the
     shared `subject` of `say`, `heal` and the other actions.
   - The first-free-room choice needs no new term. Rules run in order and each reads the flags the earlier ones set,
     as `white_deer` already does with `deer_variant_chosen`.
   - The portal tile is a one-tile area anchor. The tome cannot leave that tile and occupies it whenever no portal
     exists, so a player can only enter it through the portal.
   - Declined: `stepped_on` with a player subject on item 1949. The same room has another step-in exit (action id
     4950 at (32687, 32726, 10), `movements_timers.lua:8,36`), and the scripts do not say which item carries it, so
     an item match could fire on the wrong tile.
   - Declined: `map_item.interaction` (D29). It would move the per-instance room state into the interaction domain.
4. **Authored JSON** (room 1 of 5 and the return; flags `portal_assigned` and `room_1_busy`…`room_5_busy`,
   non-repeating 10,000 ms timers `room_1_hold`…`room_5_hold`):
   ```json
   [{"key": "portal_step_starts", "trigger": {"kind": "area_entered", "anchor": "portal_tile", "who": "player"},
     "conditions": [], "actions": [{"kind": "flag", "flag": "portal_assigned", "value": false}]},
    {"key": "portal_to_room_1", "trigger": {"kind": "area_entered", "anchor": "portal_tile", "who": "player"},
     "conditions": [{"kind": "flag", "flag": "portal_assigned", "value": false},
                    {"kind": "flag", "flag": "room_1_busy", "value": false}],
     "actions": [{"kind": "teleport", "who": {"triggering": true}, "to": "knowledge_room_1"},
                 {"kind": "flag", "flag": "room_1_busy", "value": true},
                 {"kind": "flag", "flag": "portal_assigned", "value": true},
                 {"kind": "timer", "timer": "room_1_hold", "operation": "start"}]},
    {"key": "portal_return", "trigger": {"kind": "area_entered", "anchor": "portal_tile", "who": "player"},
     "delay_ms": 10000,
     "conditions": [{"kind": "in_anchor", "subject": {"triggering": true}, "anchor": "knowledge_range"}],
     "actions": [{"kind": "teleport", "who": {"triggering": true}, "to": "library_middle"}]},
    {"key": "room_1_reopens", "trigger": {"kind": "timer_elapsed", "timer": "room_1_hold"},
     "conditions": [], "actions": [{"kind": "flag", "flag": "room_1_busy", "value": false}]}]
   ```
   The tome's death rule creates item 1949 `at: death_position` with `revert_after_ms` 10000, and a `delay_ms` 10000
   rule on the same death spawns the tome at `death_position`. Both use existing terms.
   - The delayed return is gated on `in_anchor` of the triggering player in `knowledge_range`, the sample's existing
     anchor for the main room and the five knowledge rooms. The condition is evaluated 10 s later (§9 rule 2). A
     player who has left the instance, whether still online or not, is not pulled back.
   - The rooms reopen at 10 s through their own timers, whatever happened to the player.
   - The lever admits five players (`gorzindel.lua:7-13`), and a player in a room cannot reach the portal, so every
     step finds a free room. If a larger party ever finds all rooms busy, that player only moves to the middle
     after 10 s.
5. **Validation and tests.**
   - Schema: `teleport.who` gains the `triggering` branch. The `in_anchor` subject gains it too, and only there.
   - Semantic: `teleport triggering` and `in_anchor` of `triggering` fail in a rule whose trigger is neither fired
     by one creature nor `area_entered`/`area_left` (for example `timer_elapsed`). `remove triggering` keeps its current rule, and it
     also fails when the trigger names `who: player`, because players are never removed.
   - `verify_encounter_schema.py`: one positive check (the Gorzindel portal) and negative checks for
     `teleport triggering` and `in_anchor` of `triggering` in a `timer_elapsed` rule, and for `remove triggering`
     with `who: player`.
   - Rust (E1): the typed `teleport` and `in_anchor` mirrors gain the variant and the same check, with a focused positive and negative
     test in `content_world_project_v2_encounter_admission.rs`.
   - Transcription: anchors `portal_tile`, `knowledge_room_1`…`5` and `library_middle` located from the lever and
     movement scripts (E2); every line of `creaturescripts_gorzindel.lua` 38-50 and `movements_gorzindel.lua` 1-38
     mapped.
6. **Out of scope.** The lever, its cooldown and player positions (entry contract); the step-in exits of
   `movements_timers.lua` (interaction domain); the defects in Q2, which the owner answer fixes.

### 12.3 Melting Frozen Horror: no extension (the lever names the roles)

1. **Mechanic.** On the boss's death the top creature on (32269, 31084, 14) is replaced by a baby dragon and the top
   creature on (32267, 31071, 14) is removed.
2. **Canary behaviour.**
   - Death actions: `data-global/scripts/quests/forgotten_knowledge/creaturescripts_bosses_kill.lua:37-48`.
   - `MeltingDeath` (`creaturescripts_melting_death.lua:7-19`) is registered with it
     (`data-global/monster/quests/forgotten_knowledge/bosses/melting_frozen_horror.lua:16-19`) and replaces the
     creature on the egg tile once more. The net result is still one baby dragon.
   - The lever names both tiles' creatures (`actions_frozen_horror.lua:7-12, 64-67`): the dragon egg on
     (32269, 31084, 14), the Melting Frozen Horror parked on (32267, 31071, 14), and the Solid Frozen Horror in the
     room.
   - A fully healed egg swaps the two horrors for 20 s (`creaturescripts_dragon_egg.lua:24-43`): the solid horror goes
     to the parking tile and the melting horror takes its place. `revertHorror` (lines 1-22) swaps them back.
   - So the melting horror dies in the room while the parking tile holds the solid horror. The egg has speed 0
     (`dragon_egg.lua:14`).
3. **Extension.** None. D29 already rules that acting on whatever stands on a fixed tile is not added and that each
   case names its role from evidence. The lever gives the roles `dragon_egg` and `solid_frozen_horror`, as
   `actions_gorzindel.lua` gives Gorzindel's knowledges. `role_position`, `remove` of a role and `creature_present`
   express the rest. **The row is resolvable now.**
   - **Owner answer (Q4): follow global.** Forgotten Knowledge has one form-changing boss (TibiaWiki spoiler), so the
     death of the Melting form always removes the Solid form. This deviates from Canary, which removes the top creature
     of the parking tile. The removal is a rule of its own, with no condition.
4. **Authored JSON** (the arena anchor is the lever's cleanup area (32264, 31070)-(32284, 31104, 14),
   `actions_frozen_horror.lua:86`):
   ```json
   [{"key": "melting_frozen_horror_hatches_the_egg",
     "trigger": {"kind": "creature_died", "role": "melting_frozen_horror"},
     "conditions": [{"kind": "creature_present", "role": "dragon_egg", "anchor": "horror_arena", "present": true}],
     "actions": [{"kind": "remove", "role": "dragon_egg"},
                 {"kind": "spawn", "creature": {"family": "Creature", "key": "canary:creature/baby_dragon", "revision": "canary-47dfd51f"},
                  "count": 1, "at": {"role_position": "dragon_egg"}, "owner": "none", "health": "full"}]},
    {"key": "melting_frozen_horror_removes_the_solid_form",
     "trigger": {"kind": "creature_died", "role": "melting_frozen_horror"},
     "conditions": [], "actions": [{"kind": "remove", "role": "solid_frozen_horror"}]}]
   ```
   `role_position` is taken when the trigger fires (§9 rule 2), so the dragon appears on the egg's tile after the egg
   is removed. If damage over time kills the parked horror, Canary removes the dying horror itself and leaves the
   solid horror in the room; under Q4 the second rule removes the solid horror in every case.
5. **Validation and tests.** No vocabulary change. The transcription adds the two participants, the anchor and the
   two rules, maps `creaturescripts_bosses_kill.lua` 37-48 and `creaturescripts_melting_death.lua` 1-21, and adds
   `MeltingDeath` to the manifest `covers`. `validate_encounter.py` must pass with the catalog.
6. **Out of scope.** The rest of the fight is the dragon egg's `DragonEggHealthChange` and `DragonEggPrepareDeath`
   (fire heals the egg, a full egg swaps the horrors). These events block the Dragon Egg creature in the census, and
   under E4 this encounter is admitted only with it. That transcription is a later slice, and this design does not
   claim the vocabulary covers it.

### 12.4 The Sandking: CW2-2 `stepped_on corpse_of`, CW2-3 `map_item remove triggering`, CW2-4 `free` random tiles

1. **Mechanic.** The credit reads the fight stage (at least 5). The stages come from the Sandking's
   vanish-and-brood cycle, during which a Sandking heals by walking over sand brood corpses.
2. **Canary behaviour** (`data-global/scripts/quests/cults_of_tibia/`):
   - The lever places "the sandking fake" and sets stage 1 (`actions_bosses_levers.lua:480-481`). The fake's monster
     name is "The Sandking" (`data-global/monster/quests/cults_of_tibia/bosses/the_sandking_fake.lua:4`), so every
     name check below matches it too.
   - `creaturescripts_sandking.lua:58-75`: while the stage is at most 3, a fake below 95% says its line and leaves.
     Four sand vortices appear (lines 64-73), and ten sand broods follow, one every 5 s, on random tiles of
     (33092-33105, 31853-31865, 15) (lines 38-48).
   - Lines 1-37: 5 s after the tenth brood, and then every 5 s, the script checks
     (33087-33109, 31848-31871, 15). Once no brood is left, the vortices go, a new fake appears at (33099, 31859, 15)
     and says its line, and the stage rises by one.
   - Lines 76-97: at stage 4 a fake below 50% becomes three fakes at half health that share one life.
   - Lines 121-126: their death brings the real Sandking at half health, and stage 5 follows 2 s later. The credit
     needs stage 5 (`creaturescripts_bosses_mission_cults.lua:7,24-26`).
   - A dying brood marks its corpse (`creaturescripts_sandking.lua:109-119`). A creature named "The Sandking" that
     steps on it removes the corpse and heals 100-1000 (`movements_sandking.lua:3-16`).
   - The corpse is item 6023 (`sand_brood.lua:19`), which larvae and parasites share. It decays to 4191 after 10 s
     (`data/items/items.xml:16561-16566`). The decay changes the id of the same item (`src/game/game.cpp:3097`,
     `3148`), so the mark survives it.
   - `SandHealth` (`creaturescripts_sandking.lua:132-152`) never acts. Heals return early, and damage arrives
     negative, so its `> 0` branches never run (the sixth-slice finding).
3. **Extensions.**
   - **CW2-2:** `stepped_on` takes exactly one of `item` and `corpse_of` (a role). `corpse_of` matches a tile
     holding the corpse a creature of that role left in this encounter instance, at any decay stage. An item id
     cannot say this: it is shared with other monsters and changes as the corpse decays.
   - **CW2-3:** `map_item` with `operation: remove` takes `triggering: true` in place of `item` and `anchor`/`at`. It
     removes the item that fired the `stepped_on` rule.
   - **CW2-4:** the `random_in` position takes an optional `free: true`, which draws uniformly among the tiles of the
     area that a creature can be placed on now. If no such tile exists, that spawn creates nothing and the remaining
     actions still run. Without `free`, or with `free: false`, `random_in` keeps its current semantics exactly:
     existing uses such as `razzagorn`, whose manifest records that a failed random creation ends the cast, are
     untouched. `free` is used only for the Sandking broods (Q3 accepted).
   - Everything else uses existing terms:
     - `health_crossed` (a think check, as in the ninth slice);
     - the `stage` and `broods_left` counters, and repeating 5 s timers `brood_wave` and `brood_check`;
     - `spawn`, `creature_present` absent, and `remove` of a role;
     - the `spawned` speaker and `shared_life` (D34);
     - a flag, so the three shared deaths bring one real Sandking;
     - a `delay_ms` 2000 rule for stage 5.
   - The rule that ends the brood calls is listed before the rule that calls a brood. It reads the counter before
     that rule lowers it, which reproduces Canary's call chain.
4. **Authored JSON** (one of the three stepping roles `sandking_fake`, `sandking_split`, `the_sandking`):
   ```json
   {"key": "sandking_eats_a_brood_corpse",
    "trigger": {"kind": "stepped_on", "role": "sandking_fake", "corpse_of": "sand_brood"},
    "conditions": [],
    "actions": [{"kind": "heal", "subject": {"role": "sandking_fake"}, "amount": {"min": 100, "max": 1000}},
                {"kind": "map_item", "operation": "remove", "triggering": true}]}
   ```
   A stage rule, with existing terms only:
   ```json
   {"key": "sandking_vanishes", "trigger": {"kind": "health_crossed", "role": "sandking_fake", "percent": 95},
    "conditions": [{"kind": "counter_compare", "counter": "stage", "op": "<=", "value": 3}],
    "actions": [{"kind": "say", "subject": {"role": "sandking_fake"}, "text": "THE SANDKING VANISHES INTO THE SAND AND HIS BROOD EMERGES!", "mode": "say"},
                {"kind": "remove", "triggering": true},
                {"kind": "spawn", "creature": {"family": "Creature", "key": "canary:creature/sand_vortex", "revision": "canary-47dfd51f"},
                 "role": "sand_vortex", "count": 1, "at": {"anchor": "vortex_1"}, "owner": "none", "health": "full"},
                {"kind": "spawn", "creature": {"family": "Creature", "key": "canary:creature/sand_brood", "revision": "canary-47dfd51f"},
                 "role": "sand_brood", "count": 1, "at": {"random_in": "brood_area", "free": true}, "owner": "none", "health": "full"},
                {"kind": "counter", "counter": "broods_left", "operation": "set", "value": 9},
                {"kind": "timer", "timer": "brood_wave", "operation": "start"}]}
   ```
   Vortices 2-4 are three more `spawn` actions. `stage` starts at 1, the value the lever writes. The sample's credit
   condition (`stage >= 5`) is unchanged.
5. **Validation and tests.**
   - Schema: `stepped_on` takes exactly one of `item` and `corpse_of`, `map_item` gains `triggering`, and the
     `random_in` position gains an optional boolean `free`.
   - Semantic: `corpse_of` names a known role. `map_item triggering` is valid only with `operation: remove` and in a
     `stepped_on` rule, and it forbids `item`, `into`, `anchor`, `at`, `destination`, `revert_*`, `effect` and
     `interaction`.
   - `verify_encounter_schema.py`: a positive check (the corpse rule) and negative checks for both and for neither of
     `item` and `corpse_of`, an unknown `corpse_of` role, `triggering` with `create` or `transform`, and `triggering`
     outside `stepped_on`. Every existing sample that uses `random_in` validates unchanged.
   - Rust (E1): the typed `stepped_on`, `map_item` and position mirrors gain the same variants and checks, with focused tests.
   - Transcription: every line of `creaturescripts_sandking.lua`, `movements_sandking.lua` and lever lines 480-481 is
     mapped or omitted with a reason. The anchors are located from the script coordinates (E2), and the
     `the_sandking` manifest row turns `mapped`.
6. **Out of scope.**
   - The lever, its 60-minute kick and the boss cooldown (entry and reward domains).
   - `SandHealth`, transcribed as Canary's no-op (`approved_omission`). Its intended reflection is a D25 wiki check,
     and `reflect_damage` already expresses it if the wiki confirms it.
   - The defect in Q3, which CW2-4 fixes.

### 12.5 Implementation order after acceptance

1. Schema, `validate_encounter.py` and `verify_encounter_schema.py` for CW2-1..4. **Done** in
   `OTV2-20260929-cw2-encounter-vocabulary-impl`; every existing sample validates unchanged.
2. Transcriptions with the owner answers: Alptramun (Q1), Gorzindel (CW2-1, Q2), Melting Frozen Horror (Q4) and The
   Sandking (CW2-2..4, Q3). The samples are the output of `canary_encounters.py` and feed the monster census, so this
   slice owns that tool, the four samples and the census outputs.
3. The Rust typed profile and its tests (slice 3 of the admission design).
4. Restaging under E4. Melting Frozen Horror then stays deferred behind the dragon egg.

Steps 2-4 are **done** in `OTV2-20260929-cw2-encounter-transcriptions`: 82 of 83 manifests resolve (Ferumbras Mortal
Shell remains), the census resolves 1560 monsters (was 1557), and the restage admits Alptramun, Gorzindel and The
Sandking (61 encounters, 1476 creatures). Melting Frozen Horror waits for the Dragon Egg.

Each step is its own owned slice.

### 12.6 Owner questions and answers (deviations and product choices only)

The owner answered in batch 2 on #162 (2026-09-29), where these questions are numbered Q4, Q5, Q6 and Q8. The
answers deviate from the proposed default for Q1 and Q4.

| # | Question | Canary behaviour | Proposed default | Owner answer |
|---|---|---|---|---|
| Q1 | Alptramun's escalation: follow Canary, or adopt the wiki? | The spell is never cast, and a cast would not escalate because the counter skips summons (§12.1). The reference-date wiki says killed summons are replaced by stronger ones, without numbers. | Follow Canary: record the spell as `approved_omission`. Adopting the wiki needs owner-chosen numbers: the cast interval and chance, and whether summoned dreams count. | Batch Q4: follow global. A killed dream is replaced at once by the next tier, capped at tier 4; the healing values are Canary's; the initial dreams come from Canary (§12.1). |
| Q2 | Gorzindel's portal defects: free each room after 10 s in every case, and return only players still in the instance? | The room table is server-wide. A room stays busy forever if its player is gone when the 10 s end, and a player who died and respawned is still pulled back to the middle (`movements_gorzindel.lua:24-30`). | Yes. The room state is per instance (D26), and the return acts only on a player still in the fight (§9 rule 2). | Batch Q5: yes. The return teleports only players still in the instance (CW2-1 `in_anchor` of `triggering`). |
| Q3 | The Sandking's brood calls: spawn each brood on a free tile of the area? | A brood is created on an unchecked random tile. If that fails, the call chain stops and the Sandking never returns (`creaturescripts_sandking.lua:43-44`). | Yes. Only the brood spawns use `random_in` with `free: true` (CW2-4); every existing `random_in` keeps its semantics. | Batch Q6: yes, as proposed. |
| Q4 | Melting Frozen Horror: when damage over time kills the parked melting horror, remove the Solid Frozen Horror anyway? | The death script removes the top creature of the parking tile, which is then the dying melting horror itself. The solid horror stays in the room, and `revertHorror` finds no melting horror to swap back (`creaturescripts_bosses_kill.lua:44-47`, `creaturescripts_dragon_egg.lua:1-22`). The egg still hatches and the kill is still credited. | Follow Canary. Gate the removal with the existing `creature_present(solid_frozen_horror, parking_tile, present)` on a one-tile anchor, in a rule of its own. The fight is already credited, and the leftover solid horror goes when the instance resets (D26). Removing it everywhere would be a deviation with no evidence. | Batch Q8: follow global. The Melting death always removes the Solid form (one form-changing boss, TibiaWiki Forgotten Knowledge spoiler); a deviation from Canary (§12.3). |

### 12.7 Decision test (`ARCHITECTURE_DECISION_DISCIPLINE.md`)

The test covers CW2-1..4 together, because each one widens the closed v1 vocabulary of §9 rule 1. Where the
extensions differ, the differences are named. Alptramun (Q1) and Melting Frozen Horror (Q4) need no vocabulary change;
their answers are recorded in §12.6.

1. **Must decide now?**
   - **CW2-1..3: YES.** E4 admits an encounter only when its manifest has no `unresolved_semantics` row. Without these
     terms the Gorzindel and The Sandking rows cannot resolve, and the closed vocabulary rules out approximating or
     scripting them (D28). The owner scheduled this proposal on #162 (batch item 4).
   - **CW2-4: YES**, because the owner accepted the free-tile brood spawn (Q3).
   - No runtime work waits on any of this. The decision concerns content admission only.
2. **What concrete downstream work is blocked?**
   - Slice 1 of §12.5: the schema, validator and `verify_encounter_schema.py` additions (now done).
   - The Gorzindel and The Sandking transcriptions in `canary_encounters.py` and their manifests.
   - The typed Encounter profile variants in `apps/game-server/src/content/project/v2/encounter.rs`, which are slice 3
     of the admission design.
   - The E4 restage that admits these two encounters and the creatures they cover. Today those creatures wait as
     `deferred_encounter`.
3. **What becomes harder or impossible later?**
   - **Schema compatibility.** Each extension is an additive, optional branch:
     - CW2-1: `triggering` in `teleport.who` and in the `in_anchor` subject;
     - CW2-2: `corpse_of` as the alternative to `item`;
     - CW2-3: `triggering` in `map_item`;
     - CW2-4: `free`, default false.
     Every existing sample and admitted encounter stays valid with its meaning unchanged, and §12.4 makes that a
     validation obligation.
   - **Older readers.** Documents that use the new branches are rejected by older schemas and validators, and by the
     Rust profile until slice 3 of §12.5.
   - **Profile revision.** Under the E1 decision test, a change to the v2 Encounter profile shape needs a new profile
     revision and a restage. That happens once, for all four extensions.
   - **Runtime obligations**, which bind a future Encounter runtime once these encounters are admitted:
     - CW2-1: the runtime must keep a player's identity through a rule delay, and define instance membership for
       `in_anchor`. That couples the check to the activity-instance admission contract (D26).
     - CW2-2 creates the strongest coupling. The runtime must know which role and instance a corpse came from,
       across its decay stages, so it depends on the item domain's corpse and decay model.
     - CW2-3: a `stepped_on` trigger must carry the identity of the item stepped on.
     - CW2-4: the runtime must query whether a tile is free (placeable and unoccupied) when it spawns.
4. **What evidence would justify superseding it?**
   - The Encounter runtime or the item domain cannot keep corpse provenance through decay. CW2-2 would then give way
     to an item match over the decay chain, or to a marker item.
   - An interaction-domain contract that owns per-instance step-in state. The Gorzindel portal would then move to
     `map_item.interaction`, the alternative declined in §12.2.
   - An activity-instance admission contract whose rule for leaving an instance differs from the one `in_anchor` of
     `triggering` assumes. The check would follow that contract.
   - Reference-date wiki evidence (D25) that contradicts the Canary behaviour of these fights.
   - A later encounter that needs `triggering` in another subject is a new decision, not an unrecorded widening.
5. **What is deliberately not decided?**
   - `triggering` in any subject or action other than `teleport.who`, `in_anchor`, `map_item` and the existing D31
     `remove`.
   - `free` for any position kind other than `random_in`, or for any encounter other than The Sandking.
   - Ferumbras Mortal Shell and the dragon egg transcription.
   - The Encounter runtime, instancing and map binding.
   - The interaction-domain step-in exits.
   - The Rust type names.

## 13. Soul War group (SW)

- Status: **ACCEPTED** by the owner 2026-09-30 (answers in §13.6, given in the task session and posted verbatim on
  #162; the control plane assigns their D-numbers). Nothing here is implemented yet; the order is §13.5.
- Task: design only, task C of the monster-unblocking plan (KAN-16, #162).
- Scope: the 15 hunting participants of `soul_war_taint_zones`. Eight wait only for the encounter: Capricious,
  Distorted, Infernal, Mould and Vibrant Phantom, Courage Leech, Infernal Demon and Rotten Golem. Seven are blocked in
  the converter: Mirror Image, Bony Sea Devil, Brachiodemon, Branchy Crawler, Cloak of Terror, Many Faces and Turbulent
  Elemental. Under Q1 a two boss-room roles join (§13.1), so the encounter has 17 participating creatures: Spiteful
  Spitter, blocked only by its `onThink`, is admitted with the encounter (16 creatures in all); Dreadful Harvester is
  already admitted and joins as a participant.
- Companion: §10 of `OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` holds the two monster-side extensions (SW-1 fear windup,
  SW-2 magic wall removal). This section holds the encounter-side ones, the questions for all of them and the decision
  test.
- Evidence:
  - Canary `47dfd51f` (`git rev-parse` checked). Paths are relative to that checkout.
  - TibiaWiki (Fandom) at the 2026-09-27 cut (D15, D33), read through the MediaWiki API: `Goshnar's Taints` revision
    1141560, `Soul War Quest/Spoiler` 1134584, `Feared` 1149698, `Mirror Image` 1092108, `Rotten Wasteland` 1117103, and
    the creature pages of the census wiki file.
  - Tibiopedia (`tibiopedia.pl/quests/Soul_War_Quest`, read 2026-09-30) as secondary evidence, as D13 uses it for
    NPCs. It has no revision history, so it only confirms. It agrees with Fandom on the points used here: the taint
    list, the Mirror Image turning mostly into the attacker's vocation, and the Furious Crater blood that hurts when
    stepped on.
- Census check: every monster file was converted in memory with the `population_census.py` rules, and the open
  manifest rows were listed per monster. The census numbers hold (fear 4, `remove_magic_walls` 5, `mType.onThink` 15,
  `mType.onPlayerAttack` 1). Two rows are not in the task list: Cloak of Terror also has the unresolved event
  `CloakOfTerrorHealthLoss` (§13.4), and Mirror Image has `MirrorImageTransform` (§13.2).
- Rule: where the Canary behaviour is clear it is followed. A deviation or a product choice is an owner question.

| Monster | Open rows besides the encounter | Resolved by |
|---|---|---|
| Bony Sea Devil | `soulwars fear`, `destroy magic walls`, `onThink` | SW-1, SW-2, SW-3 |
| Brachiodemon | `destroy magic walls`, `onThink` | SW-2, SW-3 |
| Branchy Crawler | `onThink` | SW-3 |
| Cloak of Terror | `destroy magic walls`, `onThink`, event `CloakOfTerrorHealthLoss` | SW-2, SW-3, SW-6 |
| Many Faces | `destroy magic walls`, `onThink` | SW-2, SW-3 |
| Turbulent Elemental | `soulwars fear` | SW-1 |
| Mirror Image | `onPlayerAttack`, event `MirrorImageTransform` | SW-4 |
| the other eight | none | the anchor, SW-5 |

| Mechanic | Side | Extension |
|---|---|---|
| `soulwars fear` | monster | SW-1: `Ability.windup` (monster schema §10.1) |
| `destroy magic walls` | monster | SW-2: none; the existing `remove_items` `top_item_first_tile` and two converter fixes (§10.2) |
| taint teleport (`onThink`) | encounter | SW-3: `timer_elapsed` `each`, a picked player in `creature_present`, and a delayed `teleport` to that player |
| Mirror Image (`onPlayerAttack`) | encounter | SW-4: `damage_taken` gains `base_vocation`; `killer_is_player` gains `value: false` |
| zones with safe areas | encounter | SW-5: an area location gains `minus` boxes |
| Cloak of Terror blood | encounter | SW-6 (Q6 a): `map_item` at `subject_position`, `unless_present` |

Every encounter extension widens a parameter of an existing term. No trigger, condition or action kind is added, so
the E1 counts (17 triggers, 14 conditions, 25 actions) stay the same.

### 13.1 Taint teleport: SW-3

1. **Mechanic.** The first Goshnar's Taint: a hunting monster teleports next to a player who holds the taint.
2. **Canary behaviour.**
   - Five hunting monsters call `Monster:tryTeleportToPlayer(text)` from `mType.onThink`, each with its own text
     (`bony_sea_devil.lua:143-145`, `brachiodemon.lua:144-146`, `branchy_crawler.lua:139-141`,
     `cloak_of_terror.lua:139-141`, `many_faces.lua:140-142`). Spiteful Spitter does the same in the boss rooms
     (`spiteful_spitter.lua:102-104`).
   - `onThink` runs on every creature think, every 1,000 ms (`src/creatures/creature.hpp:47`).
   - `data-otservbr-global/lib/quests/soul_war.lua:1241-1298`:
     - Candidates are the players within 30 tiles on the creature's floor (line 1243) that pass
       `getTaintNameByNumber(1, true)` and stand in a Soul War zone (line 1249, `getSoulWarZoneMonster`, lines
       1458-1469). The farthest one is picked, by the greater of the x and y distances (line 1250).
     - With a 10% chance (line 1260) and no cooldown on that player (lines 1262-1268), the player gets a 10 s cooldown
       and a death effect shows on both creatures (lines 1271-1272).
     - 2 s later, if both still exist, the creature says its text and teleports to the tile the player stood on at the
       pick, with a teleport effect before and after (lines 1273-1285). The tile check reads `CONST_PROP_MOVEABLE`,
       which is not an engine constant (the enum has `CONST_PROP_MOVABLE`, `lua_enums.cpp:496`). Lua passes nil, which
       reads as 0, `CONST_PROP_BLOCKSOLID` (`tile_functions.cpp:563`, `items_definitions.hpp:16`). So the move needs a
       tile that blocks neither solids nor projectiles, and it ignores creatures on it (`game.cpp:3581-3588`, `queryAdd` with `FLAG_NOLIMIT`).
     - The cooldown ends after 10 s only if the player is still online (lines 1287-1295). Player ids are fixed per
       character (`player.cpp:225-233`), so a player who logs out in those 10 s is never teleported to again until the
       server restarts.
   - Defect: `getTaintNameByNumber(1, true)` skips the taint check (lines 1169-1178), so every player in a zone is a
     candidate, with or without the taint.
   - Wiki (`Goshnar's Taints`, First Taint): only players with the taint; the check runs every 2 s; the 10 s cooldown;
     the 2 s warning with the death effect in hunting grounds and a purple electricity effect in boss rooms; a magic
     wall on the tile cancels the move; the creature lands only where it could walk to. It also names Dreadful
     Harvester as a teleporter in the boss rooms, which Canary does not implement; Q1 a adds it (below). The Spoiler and
     Tibiopedia repeat the 10% chance.
3. **Extension SW-3.** Three widenings:
   - `timer_elapsed` takes an optional `each: <role>`. The rule then runs once for each creature of the role present
     when the timer fires, as a rule fired by that creature (§9 rule 2), so `triggering` and the role name that
     creature.
   - `creature_present` may take `players: true` in place of `role`, `near` may take `{"triggering": true}` in place of
     a role, and two optional fields are added:
     - `where`: conditions tested for each candidate player, whose subject is `{"candidate": true}`. Only
       `killer_progress` (which gains an optional `subject`, default the killer) and `in_anchor` may appear there.
     - `pick: "farthest"`: the candidate farthest from the centre, by the greater of the x and y distances. Ties go to
       the lowest creature id, as in the chain design (spell chain §7 Q1). The picked player is `{"picked": true}` for
       the rest of the rule. `pick` needs `present: true`.
   - `teleport` gains `to: {"picked_position": true}`: the tile the picked player stands on when the action runs. With
     it come four optional fields:
     - `after_ms`: the move happens this much later. It is dropped if the creature or the picked player is no longer in
       this encounter instance (as CW2-1), or if the tile then holds an item that blocks solids or projectiles.
       Creatures on the tile do not stop it.
     - `warning_effect`: shown on the creature and on that tile when the action runs.
     - `say` and `arrival_effect`: the creature says the text just before it moves, and the effect shows before and
       after the move.
     - `picked_cooldown_ms`: the action does nothing while a teleport of this encounter to the same player is pending
       or happened less than this long ago. The cooldown starts when the move happens and always ends on time; a move
       that is dropped (the creature or player gone, or the landing tile blocked when it fires) starts no cooldown
       (Q1 a: the wiki's "trapped before the indication", Gudii 5/6).
   - Declined:
     - a native monster behaviour (D13), because the check reads quest progress and the zones, which D9 and D18 keep in
       encounters;
     - generic `after` and `effect` actions, which would open delayed action lists in every rule, not only here;
     - a reachability check, which needs a path term (Q1 c, not chosen).
4. **Authored JSON** (one of the roles; the repeating `taint_check` timer is 2,000 ms (Q1 a); `killer_progress` under
   `where` is the first-taint test and `in_anchor` the zone test):
   ```json
   [{"key": "taint_check_runs", "trigger": {"kind": "encounter_started"}, "conditions": [],
     "actions": [{"kind": "timer", "timer": "taint_check", "operation": "start"}]},
    {"key": "bony_sea_devil_taint_teleport",
     "trigger": {"kind": "timer_elapsed", "timer": "taint_check", "each": "bony_sea_devil"},
     "conditions": [
       {"kind": "creature_present", "players": true, "near": {"triggering": true, "radius": 30},
        "where": [{"kind": "killer_progress", "subject": {"candidate": true},
                   "progress": "canary:quest-progress/soul_war_taint_1", "op": "==", "value": true},
                  {"kind": "in_anchor", "subject": {"candidate": true}, "anchor": "soul_war_taint_zones"}],
        "pick": "farthest", "present": true},
       {"kind": "chance_percent", "value": 10}],
     "actions": [{"kind": "teleport", "who": {"triggering": true}, "to": {"picked_position": true},
                  "after_ms": 2000, "picked_cooldown_ms": 10000, "say": "Get out the way!",
                  "warning_effect": "canary.appearance:effect/mortarea",
                  "arrival_effect": "canary.appearance:effect/teleport"}]}]
   ```
   The roles `bony_sea_devil`, `brachiodemon`, `branchy_crawler`, `cloak_of_terror` and `many_faces` each hold one
   creature, which also stays in `hunting_monster`: role names are unique, but a creature may be in several roles.
   Spiteful Spitter and Dreadful Harvester join as the boss-room roles in the same encounter, since the anchor includes
   the boss rooms (Q1 a). Dreadful Harvester's text is its wiki voice line "You have been chosen for a harvest!"
   (DERIVED: Canary gives that line to Spiteful Spitter's teleport, `spiteful_spitter.lua:103`, and to Dreadful
   Harvester only as a voice line, `dreadful_harvester.lua:64`); its effects are those of the other roles.
5. **Validation and tests.**
   - Schema: `each` on `timer_elapsed`; the `players`, `triggering` centre, `where` and `pick` forms of
     `creature_present`; `subject` on `killer_progress`; the `picked_position` target and the four `teleport` fields.
   - Semantic: `each` names a known role. `candidate` is valid only inside `where`, and `where` only with
     `players: true`. `picked` and `picked_position` need an earlier `creature_present` with `pick` in the same rule.
     The four `teleport` fields need `picked_position`. `near.triggering` needs a trigger fired by one creature,
     which `each` provides.
   - `verify_encounter_schema.py`: one positive check (the rule above) and negative checks for `picked_position`
     without a pick, `candidate` outside `where`, `pick` with `present: false`, and `each` with an unknown role. Every
     existing sample validates unchanged.
   - Rust (E1): the typed trigger, condition and `teleport` mirrors gain the same variants and checks, with focused
     tests.
   - Transcription: `soul_war.lua` 1241-1298 and the six `onThink` lines mapped; the skipped taint check and the
     logout cooldown are recorded as deviations under Q1 a; Dreadful Harvester's rule cites the wiki.
6. **Out of scope.** Taints 2, 3 and 5 (player-side, `eventcallback_on_combat_taint.lua`); the quest domain that
   publishes the taint progress (D27).

### 13.2 Mirror Image: SW-4, `damage_taken` with `base_vocation`

1. **Mechanic.** A Mirror Image turns into one of the five apparitions when a player first damages it, most likely the
   one of that player's vocation.
2. **Canary behaviour.**
   - `mType.onPlayerAttack` (`data-otservbr-global/monster/quests/soul_war/mirror_image.lua:113-143`) runs in
     `Combat::CombatHealthFunc` before the hit is applied (`src/creatures/combat/combat.cpp:888-899`, then
     `combatChangeHealth` at 911). With 70% the apparition of the attacker's vocation is created at full health on
     the image's tile, otherwise one of the other four at random, and the image is removed (lines 118-142). The hit
     then lands on a removed creature, so it is lost.
   - A player without a vocation leaves `apparitionType` empty: 70% of the time `createMonster("")` fails and the image
     just disappears.
   - `MirrorImageTransform` (`data-otservbr-global/scripts/creaturescripts/monster/mirror_image_transform.lua:1-22`) is an `onHealthChange`
     event. It turns the image into the attacker's apparition, keeping its health, with no Monk case. It acts only
     where `onPlayerAttack` does not run: damage over time from a player (`condition.cpp:1998-2030` calls
     `combatChangeHealth` directly). Damage from a familiar or other summon triggers neither path.
   - The file assigns `monster.events` twice. The second list (lines 109-111) replaces the first (lines 16-18), so
     Canary never registers `FourthTaintBossesPrepareDeath` on Mirror Image. `canary_encounters.py` `registrants()`
     reads the first list, so the sample lists Mirror Image as a `hunting_monster` by mistake. The transcription
     follows Canary and reads the last assignment (the same double assignment occurs in `the_ravager.lua`,
     `sir_nictros.lua` and `unpleasant_dream.lua`). There is no gameplay effect: a player's first hit turns the image
     before it can die.
   - Wiki: `Soul War Quest/Spoiler` (Mirrored Nightmare) and Tibiopedia agree with `onPlayerAttack` ("a higher chance"
     for the first attacker's vocation, "any other vocation" otherwise). The `Mirror Image` page adds that it is
     "likely to be impossible to kill one without forcing a transformation" because it never drops below 1 hit point,
     which Canary does not do.
3. **Extension SW-4.** `damage_taken` takes an optional `base_vocation`, as D34 gave `item_used`: the rule fires only
   for damage by a player of that base vocation (a promoted vocation counts as its base). The rest uses existing
   terms: one rule per base vocation with a weighted `one_of` of five `transform`s at full health. Weights 28:3:3:3:3
   give exactly 70% and 7.5% for each other vocation. After the transform the creature is no longer a Mirror Image, so
   the rules fire once. Q5 b adds a second widening: `killer_is_player` takes an optional `value` (default `true`, the
   current meaning); `value: false` holds when the damage is not a player's own. Damage from a player's summon or
   familiar is not the player's own: it neither transforms the image (`base_vocation` fires only for the player's
   own damage) nor kills it. Declined: a condition on the attacker's vocation, which would be a new condition kind.
4. **Authored JSON** (the druid rule; the other four swap the weights):
   ```json
   {"key": "mirror_image_turns_for_a_druid",
    "trigger": {"kind": "damage_taken", "role": "mirror_image", "source": "player", "base_vocation": "druid"},
    "conditions": [],
    "actions": [{"kind": "one_of", "branches": [
      {"weight": 28, "actions": [{"kind": "transform", "role": "mirror_image", "health": "full",
        "into": {"family": "Creature", "key": "canary:creature/druid_s_apparition", "revision": "canary-47dfd51f"}}]},
      {"weight": 3, "actions": [{"kind": "transform", "role": "mirror_image", "health": "full",
        "into": {"family": "Creature", "key": "canary:creature/knight_s_apparition", "revision": "canary-47dfd51f"}}]},
      {"weight": 3, "actions": [{"kind": "transform", "role": "mirror_image", "health": "full",
        "into": {"family": "Creature", "key": "canary:creature/paladin_s_apparition", "revision": "canary-47dfd51f"}}]},
      {"weight": 3, "actions": [{"kind": "transform", "role": "mirror_image", "health": "full",
        "into": {"family": "Creature", "key": "canary:creature/sorcerer_s_apparition", "revision": "canary-47dfd51f"}}]},
      {"weight": 3, "actions": [{"kind": "transform", "role": "mirror_image", "health": "full",
        "into": {"family": "Creature", "key": "canary:creature/monk_s_apparition", "revision": "canary-47dfd51f"}}]}]}]}
   ```
   The five apparitions already convert with no open row. A sixth rule (Q5 b) keeps the image at 1 hit point against
   damage that is not a player's own:
   ```json
   {"key": "mirror_image_floor", "trigger": {"kind": "lethal_damage", "role": "mirror_image"},
    "conditions": [{"kind": "killer_is_player", "value": false}],
    "actions": [{"kind": "prevent_death", "role": "mirror_image"}]}
   ```
5. **Validation and tests.**
   - Schema: optional `base_vocation` on `damage_taken`; optional boolean `value` on `killer_is_player`. Semantic:
     `base_vocation` only with `source: player`; `killer_is_player` without `value` keeps its meaning.
   - `verify_encounter_schema.py`: positive checks for the vocation rule and the floor rule; negative checks for
     `base_vocation` with `source: any` and for a non-boolean `value`. Every existing sample validates unchanged.
   - Rust (E1): the `DamageTaken` and `KillerIsPlayer` mirrors gain the fields and checks, with focused tests.
   - Transcription: `mirror_image.lua` 109-143 and `mirror_image_transform.lua` 1-22 mapped, or omitted as deviations
     under Q5 b (the over-time path, the no-vocation removal); the `registrants()` fix; Mirror Image leaves
     `hunting_monster` for its own role.
6. **Out of scope.** The apparition kill counter `MirroredNightmareBossAccess` (quest domain, already an approved
   omission); the wall mirrors that spawn apparitions (Tibiopedia; interaction domain).

### 13.3 Zones with safe areas: SW-5, `minus` boxes

1. **Mechanic.** The anchor `soul_war_taint_zones` is the union of eleven Canary zones minus five safe areas.
2. **Canary behaviour.**
   - `SoulWarQuest.areaZones.monsters` (`soul_war.lua:229-242`) lists five hunting zones and six boss zones.
   - The hunting zones are boxes over several floors (lines 862-870): Claustrophobic Inferno (33982-34051,
     30981-31110, floors 9-11), Ebb and Flow (33873-33968, 30994-31150, 8-9), Furious Crater (33814-33907,
     31819-31920, 3-7), Rotten Wasteland ((33980, 30986, 11) to (33901, 31105, 12)) and Mirrored Nightmare
     (33877-33991, 31164-31241, 9-13).
   - The safe areas (lines 872-881, "should not spawn monster, teleport, take damage from taint") are one box on one
     floor per hunting zone.
   - The boss zones are `boss.<name>` zones that `BossLever:register` fills with the lever's `specPos`
     (`data/libs/functions/boss_lever.lua:79, 270-272, 301`): Spite, Malice, Greed, Hatred and Megalomania on floor 14,
     Cruelty on floor 7 (`soul_war.lua:274-276, 310-312, 333-335, 362-364, 390-392, 418-420`).
   - Defect: `Area` keeps its corners as given (`src/game/zones/zone.hpp:32-33`), and its iterator walks x from
     `from.x` and wraps as soon as x passes `to.x` (lines 54-86). With `from.x` 33980 above `to.x` 33901, Rotten
     Wasteland is the single column x = 33980, y 30986-31105, floors 11-12. Its safe area lies outside that column.
     The wiki (`Rotten Wasteland`, `Goshnar's Taints`) treats it as a full hunting ground with Branchy Crawler as its
     teleporter (Q2).
   - The Spoiler confirms a safe zone at the entrance of every area ("arrive at the safe zone").
3. **Extension SW-5.** An area location may carry `minus`: a list of boxes in the same form. The area is every tile of
   a box that lies in no `minus` box. The existing form "boxes of whole tiles, each on one floor" already takes the
   multi-floor zones (one box per floor) and the boss rooms (more boxes), so only the subtraction is new.
   `canary_encounters.py` reads the anchor from the `addArea`, `subtractArea` and lever `specPos` lines instead of the
   prose. The anchor leaves `UNLOCATED`.
4. **Authored JSON** (abridged: 23 boxes under Q2 a, 5 `minus` boxes):
   ```json
   {"key": "soul_war_taint_zones", "kind": "area",
    "location": {"boxes": [{"x": [33982, 34051], "y": [30981, 31110], "floor": 9},
                           {"x": [33901, 33980], "y": [30986, 31105], "floor": 11},
                           {"x": [33734, 33751], "y": [31624, 31640], "floor": 14}],
                 "minus": [{"x": [34002, 34019], "y": [31008, 31019], "floor": 9},
                           {"x": [33967, 33977], "y": [31037, 31051], "floor": 11}]}}
   ```
5. **Validation and tests.**
   - Schema: optional `minus` on the area form, at least one box when present.
   - Semantic: each `minus` box must overlap a box on its floor, and at least one tile must remain. A point anchor
     cannot take `minus`.
   - `verify_encounter_schema.py`: a positive check and negative checks for a `minus` box that overlaps nothing and
     for a `minus` that removes the whole area. Every located anchor validates unchanged.
   - Rust (E2): `ProjectV2AnchorLocation::Area` gains `minus` (default empty) with the same checks.
6. **Out of scope.** Map binding; the Ebb and Flow flooding and the raid zones, which are other `Zone`s.

### 13.4 Cloak of Terror blood: SW-6 (Q6 a)

1. **Mechanic.** A hit on a Cloak of Terror leaves a blood pool that hurts players and heals Cloaks.
2. **Canary behaviour.**
   - `CloakOfTerrorHealthLoss` (`data-otservbr-global/scripts/quests/soul_war/soul_war_mechanics.lua:785-805`) creates item 33854 on the
     Cloak's tile unless it is there already. Its test `attacker:getPlayer() and primaryDamage > 0 or
     secondaryDamage > 0` never holds for damage: health-change events see damage as negative values
     (`game.cpp:8464-8471`, before `std::abs` at 8478), as found for `SandHealth` (§12.4). It holds for a heal by a
     player. So in Canary only a player healing a Cloak leaves blood.
   - Stepping in (lines 807-835, all three ids): a player takes energy damage of 20%, 15% or 10% of maximum health by
     pool size (`soul_war.lua:20-24`); a Cloak of Terror heals 1,500-2,000; the pool is removed for any creature. The
     pool decays 33854 → 34006 → 34007 → gone (`data/items/items.xml:63176-63178, 63654-63660`).
   - Wiki (`Soul War Quest/Spoiler`, Furious Crater) and Tibiopedia: the pool appears when a Cloak is hit, hurts by
     pool size up to about 20% of maximum health, and heals Cloaks by 1,500-2,000. That matches the Canary data, not
     the Canary test.
3. **Extension SW-6** (Q6 a). `map_item` `create` takes `at: "subject_position"` (the creature that fired a
   one-creature rule, as `spawn` has it) and `unless_present: true` (nothing happens if the tile already holds the
   item). The step-in is registered on the item ids for any creature, so it is interaction-domain content through the
   existing `interaction` key (D29), like the Gorzindel step-in exits (§12.2).
4. **Authored JSON** (Q6 a):
   ```json
   {"key": "cloak_of_terror_bleeds", "trigger": {"kind": "damage_taken", "role": "cloak_of_terror", "source": "player"},
    "conditions": [],
    "actions": [{"kind": "map_item", "operation": "create", "at": "subject_position", "unless_present": true,
                 "item": {"family": "Item", "key": "canary:item/33854", "revision": "canary-47dfd51f"},
                 "interaction": "canary:interaction/blood_of_cloak_of_terror"}]}
   ```
   `unless_present` is provisional: whether repeated hits enlarge one pool or stack pools on a tile is checked before
   SW-6 is implemented (Q6 answer), and the field follows that check.
5. **Validation and tests.** `subject_position` only in a one-creature rule; `unless_present` only with `create`. One
   positive and two negative checks; the Rust `map_item` mirror gains both. E4 needs item 33854 admitted.
6. **Out of scope.** The interaction content of the pool; the Pulsating Energy access drops (quest domain).

### 13.5 Other monsters the extensions free, and order

| Extension | Group monsters | Other census monsters whose only open row it clears |
|---|---|---|
| SW-2 magic walls | Bony Sea Devil, Brachiodemon, Cloak of Terror, Many Faces | The Monster |
| SW-1 fear windup | Bony Sea Devil, Turbulent Elemental | Hazardous Phantom, Goshnar's Spite (Goshnar's Megalomania green and purple once they convert) |
| SW-3 taint teleport | Bony Sea Devil, Brachiodemon, Branchy Crawler, Cloak of Terror, Many Faces | Spiteful Spitter, admitted with the encounter; Dreadful Harvester (already admitted) gains its teleport rule |
| SW-4 Mirror Image | Mirror Image | none |
| SW-5, SW-6 | all 17 participants through the anchor; Cloak of Terror | none |

- The other nine `onThink` monsters (Goshnar's Greed, the three souls, Soul Sphere, Soulsnatcher, Symbol of Hatred,
  Grand Master Oberon, The Primal Menace) run other logic, and SW-3 does not cover them.
- The `windup` is a building block of `delayed_telegraphed_nuke` (8 monsters), but each of those also speaks, removes
  itself, heals its master or filters by vocation, so none is freed by it alone.
- Order:
  1. SW-2: converter only, no schema change; frees The Monster at once.
  2. SW-1: frees Hazardous Phantom and Goshnar's Spite at once.
  3. SW-3..6 with the transcription. Under E4 the encounter is admitted together with its 16 not yet admitted
     creatures (the 15 hunting monsters and Spiteful Spitter), so the group enters only when all of them are done.

### 13.6 Owner questions (deviations and product choices only)

| # | Question | Canary behaviour | Proposed default |
|---|---|---|---|
| Q1 | Taint teleport: whom, how often, and when does the cooldown end? a) the wiki where it is explicit, Canary elsewhere; b) Canary as shipped; c) as a, plus landing only where the creature could walk and the purple effect in boss rooms. | Every player in a zone is a target, taint or not (`soul_war.lua:1173, 1249`); a check every 1 s; a logout during the 10 s cooldown makes that player immune until restart (lines 1287-1295); a direct teleport to the recorded tile; the death effect everywhere. | **a**: only holders of the first taint (`Goshnar's Taints`), a check every 2 s, and the cooldown always ends after 10 s; the direct teleport and the effects as Canary. c needs a path term not proposed here. |
| Q2 | Rotten Wasteland zone: a) the rectangle x 33901-33980 the script names; b) Canary's one column. | Reversed corners make the zone the column x = 33980 (`soul_war.lua:868`, `zone.hpp:54-86`), so taints almost never apply there. | **a**: the wiki treats Rotten Wasteland as a full taint area with Branchy Crawler as its teleporter. |
| Q3 | Fear windup: who is feared when it ends? a) the caster's target at that moment; b) the target taken at the cast. | b, even if that target has moved out of range (`soulwars_fear.lua:10-20`). | **a**: `Feared` says the new target is feared when the creature changes targets. |
| Q4 | Ability lists that differ from the wiki: a) keep Canary's; b) follow the pages. | Turbulent Elemental casts fear, which its page does not list (the Spoiler says all Ebb and Flow creatures fear); Capricious Phantom does not, although its page, `Feared` and the Spoiler say it does; Branchy Crawler lacks the wall breaker its page lists. The wall removal effect shows on the caster, not on the wall. | **a**: D18 keeps wiki ability scenes as review evidence, and b needs interval and chance values that no page gives (they would copy a sibling and be marked for verification, as in D44). The removal effect follows the `remove_items` convention (on the wall). |
| Q5 | Mirror Image: a) one rule for any player damage: 70% own vocation, else one of the other four, full health; b) a, and it never drops below 1 hit point; c) Canary's two paths. | Direct hits follow a; a player's damage over time turns it into the attacker's apparition with its health and no Monk case; a player without a vocation makes it vanish 70% of the time; non-player damage can kill it. | **a**: it matches the Spoiler and Tibiopedia and needs only SW-4. b follows the `Mirror Image` page, whose wording is hedged ("likely"). c needs a hit or over-time filter, which is not proposed. |
| Q6 | Cloak of Terror blood: a) a player's hit leaves the pool (wiki); b) only a player's heal does (Canary as shipped); c) no pool. | b, because of the sign and the operator precedence in `soul_war_mechanics.lua:792`. | **a**: the Spoiler and Tibiopedia describe it, and the Canary data (pool ids, damage shares, decay, heal) shows the same intent. a and b need SW-6. |

Owner answers (2026-09-30), after a second source check (below):

| # | Answer | What it means for the design |
|---|---|---|
| Q1 | **a** | Only holders of the first taint; a check every 2 s; the 10 s cooldown always ends. Dreadful Harvester joins Spiteful Spitter as a boss-room teleport role (wiki `Dreadful Harvester` rev 1036113, the taint page since 2023, Gudii 2/6 and 6/6). A teleport whose recorded landing tile is blocked when it fires does not happen and does not start the cooldown (wiki "trapped before the indication", Gudii 5/6: a magic wall on the marked tile stops it and the creature keeps trying). |
| Q2 | **a** | The full rectangle x 33901-33980; Canary's reversed corners are a source error (D25). |
| Q3 | **a** | The caster's target when the windup ends (D25; single-sourced on the wiki since 2023, both engines take the target at the cast). |
| Q4 | **a** | Canary's ability lists stay (D18): Turbulent Elemental keeps fear, Capricious Phantom gets none, Branchy Crawler gets no wall breaker. The earliest `Feared` revisions (812722, 2020-08-03, to 875249) name Bony Sea Devils and Turbulent Elementals, as Canary does. |
| Q5 | **b** | A player's own damage transforms it: 70% the attacker's vocation (Canary; the wiki and Tibiopedia give no number, TibiaQA #22686 measured about 60% and stays a cross-check under D15), otherwise one of the other vocations, Monk included (`monk's_apparition.lua`). Damage that is not the player's own (for example a familiar) hurts it but never takes it below 1 hit point (`Mirror Image` rev 1092108). |
| Q6 | **a** | A player's hit leaves the pool (wiki, Tibiopedia, Gudii 4/6). Before SW-6 is implemented, check whether repeated hits enlarge the pool or stack pools on one tile (Gudii 4/6 says they stack; Canary creates one only on an empty tile and has pool sizes); `unless_present` follows that check. |

Further owner answers of the same batch, outside SW-1..6:

- **Infernal Demon targeting: follow the wiki.** It retargets very often, usually to the character with the lowest
  maximum health in view (`Infernal Demon` rev 1191682; Gudii 1/6). Canary uses `nearest` 70 and `changeTarget`
  chance 0. The direction is decided; the values are not: no source gives a retarget interval or chance, and the
  wiki's "lowest maximum health" is not Canary's `health` strategy (current health). The implementation does not
  pick them: they stay UNKNOWN, an evidence item to source or an owner question in the implementation slice.
- **Many Faces critical hits: keep Canary.** The wiki and Gudii 2/6 say it can hit critically, but no source gives a
  chance or a multiplier; D18 keeps Canary until one does.
- **TibiaWiki BR is not fetched for the quest page.** The 2026-09-27 BR capture of the creature pages holds only
  placeholder ability lists for these monsters, so a quest-page capture is not expected to add anything.

Evidence status of the key claims:

- PROVEN (Canary source read at `47dfd51f`): the taint teleport code path and its two defects; the fear windup of
  2 s and 3 s fear; the magic wall ids and scan; Mirror Image's 70% and its two paths; the Cloak blood test.
- PROVEN (multiple independent sources): the five taints and their numbers (wiki, Tibiopedia, Gudii); the Cloak pool
  on hit (wiki, Tibiopedia, Gudii 4/6); Dreadful Harvester teleporting in boss rooms (wiki, Gudii 2/6 and 6/6).
- DERIVED: the 2 s check (wiki only, since 2023); fear hitting the current target (wiki only, since 2023); the
  Rotten Wasteland rectangle (the script's named corners); the familiar 1 hit point floor (wiki only).
- UNKNOWN: Mirror Image's true share (Canary 70%, TibiaQA about 60%); Infernal Demon's retarget values; Many Faces'
  critical numbers; blood pool stacking.

Second source check (2026-09-30), used for the answers above:

- Official news "In the Depths of Zarganash" (Summer Update 2020, 12.40), read through Tibiopedia's forum copy
  because tibia.com answers 403: the taint list; the fifth taint was announced as every 5 s and ships as every 10 s.
- Fandom page histories: `Goshnar's Taints` from rev 811328 (2020-07-22), `Feared` from rev 812722,
  `Mirror Image` (1 hit point floor added 2021-05-25), `The Blood of Cloak of Terror` rev 1056354.
- Tibiopedia (`tibiopedia.pl/quests/Soul_War_Quest`, read 2026-09-30 in Polish): the five taints with numbers (10%
  teleport, 0.5% summon that field, wall and bomb damage does not trigger, 15% damage taken, 10% heal to full, 10% of
  current health and mana every 10 s), the vocation priority of Mirror Image, the blood that hurts when stepped on.
- TibiaQA #22686 (2020-12-21): the measured Mirror Image vocation share.
- Gudii's Soul War guide (YouTube playlist, parts 1/6-6/6, transcripts supplied by the owner): the taint teleport to
  the farthest player, the marked landing tile, Brachiodemon breaking wild growth, Infernal Demon targeting, the fear
  windup, fear never moving a player onto a field or into the exit teleport, Many Faces critical hits, stacking blood.
- CrystalServer `00ce02a5`: descends from Canary, so it counts only where it differs (it checks the first taint).
- TibiaWiki BR: the creature infoboxes of the 2026-09-27 CI capture; the quest page is behind Cloudflare.
- Boss mechanics the videos describe (Malice, Greed, Spite, Cruelty, Hatred, Megalomania) and the Ebb and Flow tide
  are encounter and map work for later slices, not part of SW.

### 13.7 Decision test (`ARCHITECTURE_DECISION_DISCIPLINE.md`)

The test covers SW-1..6 together. SW-2 needs no schema change and is included for its converter work and the
`top_item_first_tile` scan order.

1. **Must decide now?** Yes. Under E4 the `soul_war_taint_zones` encounter and its 16 creatures wait for the seven
   blocked monsters and the unlocated anchor, and the closed vocabulary rules out approximating them (D28). SW-1 and
   SW-2 also free four monsters outside the group. No runtime work waits on this; it concerns content admission only.
2. **What concrete downstream work is blocked?**
   - The converter changes (the SW-1 template, the SW-2 constants and probe).
   - The schema, validator and `verify_*` additions for SW-1 and SW-3..6.
   - The Soul War transcription in `canary_encounters.py`: the anchor, the teleport rules, Mirror Image and the Cloak
     rule (Q6 a).
   - The typed Rust mirrors (E1, E2), and the E4 restage that admits the encounter with its creatures.
3. **What becomes harder or impossible later?**
   - Every addition is optional, so every existing sample and admitted record keeps its meaning. Older readers reject
     documents that use the new fields, and the Encounter and Ability profiles need one new revision and a restage.
   - Runtime obligations once admitted:
     - SW-1: a pending cast outlives its tick and needs the caster's identity;
     - SW-3: per-creature timer fan-out, a player pick, a per-player cooldown in encounter state, and a tile test for
       solid and projectile blocking;
     - SW-4: the attacker's base vocation on each damage event, and whether the damage is a player's own (summon and
       familiar damage is not);
     - SW-5: area membership with holes;
     - SW-6: creating an item on a creature's tile and handing its step-in to the interaction domain.
   - SW-3 is the largest widening. `picked` becomes a third rule-scoped subject beside `triggering` and `spawned`; a
     later mechanic that needs a pick among creatures instead of players is a new decision.
4. **What evidence would justify superseding it?**
   - Tibia Global evidence (D47 order) that contradicts the wiki pages cited here.
   - An Encounter runtime that cannot keep per-player state in a `channel_shared` instance. The cooldown would then
     move to the quest domain.
   - An interaction-domain contract that owns creature-dependent step-in effects differently.
   - A later need for delayed action lists in general. The `teleport` fields would then give way to a generic
     `after` action.
5. **What is deliberately not decided?**
   - `windup` over an area.
   - `pick` other than `farthest`, or over creatures.
   - `minus` for anything but area anchors.
   - Taints 2, 3 and 5.
   - The fear runtime and its party rule.
   - The Encounter runtime, instancing and map binding.
   - The Rust type names.
