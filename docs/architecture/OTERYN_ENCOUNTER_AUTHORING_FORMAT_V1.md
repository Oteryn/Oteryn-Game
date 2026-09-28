# Oteryn Encounter Authoring Format v1

- Date: 2026-09-27
- DecisionStatus: CANDIDATE (D20 draft; owner decisions D26-D29 recorded in §10; the vocabulary is
  implemented offline before any runtime work; §12 holds CW2 candidate extensions pending owner acceptance)
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
`killer_is_player`, `has_master(role, value)` (Canary skips summoned copies of a boss),
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

## 12. Candidate extensions for the remaining unresolved rows (CW2)

- Status: **CANDIDATE**, pending owner acceptance (owner batch item 4 on #162, 2026-09-28: the lane proposes
  extensions from CrystalServer/Canary behaviour; the owner accepts the finished design).
- Task: `OTV2-20260928-cw2-encounter-vocabulary-extensions`.
- Scope: Alptramun, Gorzindel, Melting Frozen Horror and The Sandking. Ferumbras Mortal Shell waits for a
  quest-domain contract and is not covered here.
- Evidence: CrystalServer (`zimbadev/crystalserver` at `ff7ede593c69d4c658b382c97443e8155926924a`) is the donor
  checked under D30. Its `data-global/` pack carries the Canary scripts. Its `data-crystal/` pack registers no events
  on these bosses and holds none of their fight scripts, so it adds nothing. Paths below are relative to that clone;
  the Canary transcription source stays the manifest source.
- Rule: where the Canary behaviour is clear it is followed; a deviation or a product choice is an owner question
  (§12.6), never decided here. Nothing in this section changes the schema, the samples or the tools. Until accepted,
  §4-6 stay the closed vocabulary and the four manifest rows stay `unresolved_semantics`.

| Boss | Unresolved mechanic | Proposal |
|---|---|---|
| Alptramun | the `alptramun summon` escalation | no extension: D29 `ability_cast` with D45 covers it; resolvable once Q1 is answered |
| Gorzindel | the Stolen Tome of Portals portal: each player to the next free knowledge room for 10 s | CW2-1: `teleport` of the `triggering` creature |
| Melting Frozen Horror | death actions on "the top creature" of two fixed tiles | no extension: the lever script names both roles; resolvable now |
| The Sandking | the stage counter the credit reads, and the fight that advances it | CW2-2: `stepped_on` a role's `corpse_of`; CW2-3: `map_item remove triggering` |

All three extensions widen a parameter of an existing term. No trigger, condition or action kind is added, so the
counts in E1 of `OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1.md` (17 triggers, 14 conditions, 25 actions) stay
the same.

### 12.1 Alptramun: no extension (D29 covers it)

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
3. **Extension.** None. Rechecked against D29: the script is a summon spell, which D45 already converts. The spell
   stays an Ability whose `encounter` is `alptramun`. Its `ability_cast` rules use `summon_count` (D45),
   `counter_compare`, a `{min, max}` spawn count (D29), `offset_tiles` 1 (D34, as for the Rage and the Destruction)
   and a four-branch `one_of` (D31). **The row is resolvable in the vocabulary.** What blocks it is evidence, not
   vocabulary: D45 needs a monster that owns and casts the ability, and Canary has none (Q1).
4. **Authored JSON** (one band; only if Q1 adopts the escalation):
   ```json
   {"key": "alptramun_calls_unpleasant_dreams",
    "trigger": {"kind": "ability_cast", "role": "alptramun",
                "ability": {"family": "Ability", "key": "canary:ability/spell/alptramun_summon", "revision": "canary-47dfd51f"}},
    "conditions": [{"kind": "summon_count", "role": "alptramun", "op": "<", "value": 5},
                   {"kind": "counter_compare", "counter": "dreams_killed", "op": "<=", "value": 9}],
    "actions": [{"kind": "spawn", "role": "unpleasant_dream",
                 "creature": {"family": "Creature", "key": "canary:creature/unpleasant_dream", "revision": "canary-47dfd51f"},
                 "count": {"min": 1, "max": 4}, "at": {"offset_tiles": 1}, "owner": "subject", "health": "full"}]}
   ```
   Three more band rules follow the same pattern, and an above-36 rule uses `one_of`. If Q1 follows Canary, no rule
   is added and the manifest row becomes `approved_omission`: the spell is never cast.
5. **Validation and tests.** None for the vocabulary. With the escalation, the transcription maps
   `alptramun_summon.lua` lines 1-59, `validate_encounter.py` accepts the rules, and the Rust admission checks the
   D45 ownership: Alptramun's ability list names the spell and the encounter covers Alptramun.
6. **Out of scope.** Alptramun's other spells, `facelessHealth` (already mapped) and the lever's day-of-week boss
   rotation (entry, not encounter).

### 12.2 Gorzindel: CW2-1, `teleport` of the `triggering` creature

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
3. **Extension CW2-1.** `teleport.who` also takes `{"triggering": true}`: the creature or player that fired the
   rule. This extends the D31 `remove triggering`. It is valid in a rule fired by one creature (§9 rule 2) and in
   `area_entered`/`area_left`, which also fire per creature.
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
     "delay_ms": 10000, "conditions": [],
     "actions": [{"kind": "teleport", "who": {"triggering": true}, "to": "library_middle"}]},
    {"key": "room_1_reopens", "trigger": {"kind": "timer_elapsed", "timer": "room_1_hold"},
     "conditions": [], "actions": [{"kind": "flag", "flag": "room_1_busy", "value": false}]}]
   ```
   The tome's death rule creates item 1949 `at: death_position` with `revert_after_ms` 10000, and a `delay_ms` 10000
   rule on the same death spawns the tome at `death_position`. Both use existing terms.
   - The lever admits five players (`gorzindel.lua:7-13`), and a player in a room cannot reach the portal, so every
     step finds a free room and the unconditional return equals Canary's.
   - If a larger party ever finds all rooms busy, that player only moves to the middle after 10 s.
5. **Validation and tests.**
   - Schema: `teleport.who` gains the `triggering` branch.
   - Semantic: `teleport triggering` fails in a rule whose trigger is neither fired by one creature nor
     `area_entered`/`area_left` (for example `timer_elapsed`). `remove triggering` keeps its current rule, and it
     also fails when the trigger names `who: player`, because players are never removed.
   - `verify_encounter_schema.py`: one positive check (the Gorzindel portal) and those two negative checks.
   - Rust (E1): the typed `teleport` mirror gains the variant and the same check, with a focused positive and negative
     test in `content_world_project_v2_encounter_admission.rs`.
   - Transcription: anchors `portal_tile`, `knowledge_room_1`…`5` and `library_middle` located from the lever and
     movement scripts (E2); every line of `creaturescripts_gorzindel.lua` 38-50 and `movements_gorzindel.lua` 1-38
     mapped.
6. **Out of scope.** The lever, its cooldown and player positions (entry contract); the step-in exits of
   `movements_timers.lua` (interaction domain); the defects in Q2, which this design does not reproduce.

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
4. **Authored JSON** (the arena anchor is the lever's cleanup area (32264, 31070)-(32284, 31104, 14),
   `actions_frozen_horror.lua:86`):
   ```json
   {"key": "melting_frozen_horror_hatches_the_egg",
    "trigger": {"kind": "creature_died", "role": "melting_frozen_horror"},
    "conditions": [{"kind": "creature_present", "role": "dragon_egg", "anchor": "horror_arena", "present": true}],
    "actions": [{"kind": "remove", "role": "dragon_egg"},
                {"kind": "spawn", "creature": {"family": "Creature", "key": "canary:creature/baby_dragon", "revision": "canary-47dfd51f"},
                 "count": 1, "at": {"role_position": "dragon_egg"}, "owner": "none", "health": "full"},
                {"kind": "remove", "role": "solid_frozen_horror"}]}
   ```
   `role_position` is taken when the trigger fires (§9 rule 2), so the dragon appears on the egg's tile after the egg
   is removed. If damage over time kills the parked horror, Canary removes the dying horror itself and leaves the
   solid horror in the room; this rule removes the solid horror. That edge case of a stuck fight is recorded in the
   manifest.
5. **Validation and tests.** No vocabulary change. The transcription adds the two participants, the anchor and the
   rule, maps `creaturescripts_bosses_kill.lua` 37-48 and `creaturescripts_melting_death.lua` 1-21, and adds
   `MeltingDeath` to the manifest `covers`. `validate_encounter.py` must pass with the catalog.
6. **Out of scope.** The rest of the fight is the dragon egg's `DragonEggHealthChange` and `DragonEggPrepareDeath`
   (fire heals the egg, a full egg swaps the horrors). These events block the Dragon Egg creature in the census, and
   under E4 this encounter is admitted only with it. That transcription is a later slice, and this design does not
   claim the vocabulary covers it.

### 12.4 The Sandking: CW2-2 `stepped_on corpse_of`, CW2-3 `map_item remove triggering`

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
   - Everything else uses existing terms:
     - `health_crossed` (a think check, as in the ninth slice);
     - the `stage` and `broods_left` counters, and repeating 5 s timers `brood_wave` and `brood_check`;
     - `spawn` at `random_in`, `creature_present` absent, and `remove` of a role;
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
                 "role": "sand_brood", "count": 1, "at": {"random_in": "brood_area"}, "owner": "none", "health": "full"},
                {"kind": "counter", "counter": "broods_left", "operation": "set", "value": 9},
                {"kind": "timer", "timer": "brood_wave", "operation": "start"}]}
   ```
   Vortices 2-4 are three more `spawn` actions. `stage` starts at 1, the value the lever writes. The sample's credit
   condition (`stage >= 5`) is unchanged.
5. **Validation and tests.**
   - Schema: `stepped_on` takes exactly one of `item` and `corpse_of`, and `map_item` gains `triggering`.
   - Semantic: `corpse_of` names a known role. `map_item triggering` is valid only with `operation: remove` and in a
     `stepped_on` rule, and it forbids `item`, `into`, `anchor`, `at`, `destination`, `revert_*`, `effect` and
     `interaction`.
   - `verify_encounter_schema.py`: a positive check (the corpse rule) and negative checks for both and for neither of
     `item` and `corpse_of`, an unknown `corpse_of` role, `triggering` with `create` or `transform`, and `triggering`
     outside `stepped_on`.
   - Rust (E1): the typed `stepped_on` and `map_item` mirrors gain the same variants and checks, with focused tests.
   - Transcription: every line of `creaturescripts_sandking.lua`, `movements_sandking.lua` and lever lines 480-481 is
     mapped or omitted with a reason. The anchors are located from the script coordinates (E2), and the
     `the_sandking` manifest row turns `mapped`.
6. **Out of scope.**
   - The lever, its 60-minute kick and the boss cooldown (entry and reward domains).
   - `SandHealth`, transcribed as Canary's no-op (`approved_omission`). Its intended reflection is a D25 wiki check,
     and `reflect_damage` already expresses it if the wiki confirms it.
   - The defect in Q3.

### 12.5 Implementation order after acceptance

1. Schema, `validate_encounter.py` and `verify_encounter_schema.py` for CW2-1..3.
2. Transcriptions: the Melting Frozen Horror death rule; Gorzindel and The Sandking with CW2-1..3; Alptramun after
   Q1.
3. The Rust typed profile and its tests (slice 3 of the admission design).
4. Restaging under E4. Melting Frozen Horror then stays deferred behind the dragon egg.

Each step is its own owned slice.

### 12.6 Owner questions (deviations and product choices only)

| # | Question | Canary behaviour | Proposed default |
|---|---|---|---|
| Q1 | Alptramun's escalation: follow Canary, or adopt the wiki? | The spell is never cast, and a cast would not escalate because the counter skips summons (§12.1). The reference-date wiki says killed summons are replaced by stronger ones, without numbers. | Follow Canary: record the spell as `approved_omission`. Adopting the wiki needs owner-chosen numbers: the cast interval and chance, and whether summoned dreams count. |
| Q2 | Gorzindel's portal defects: free each room after 10 s in every case, and return only players still in the instance? | The room table is server-wide. A room stays busy forever if its player is gone when the 10 s end, and a player who died and respawned is still pulled back to the middle (`movements_gorzindel.lua:24-30`). | Yes. The room state is per instance (D26), and the return acts only on a player still in the fight (§9 rule 2). |
| Q3 | The Sandking's brood calls: spawn each brood on a free tile of the area? | A brood is created on an unchecked random tile. If that fails, the call chain stops and the Sandking never returns (`creaturescripts_sandking.lua:43-44`). | Yes. `random_in` draws a free tile of `brood_area`. |
