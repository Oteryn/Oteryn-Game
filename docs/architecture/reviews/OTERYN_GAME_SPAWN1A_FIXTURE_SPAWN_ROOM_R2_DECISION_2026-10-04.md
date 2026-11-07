# SPAWN-1A-PACKET-1: the D116 fixture spawn and entry-room revision 2

- Decision: `SPAWN-1A-PACKET-1`
- Status: **CANDIDATE; ACCEPTED WHEN THIS DECISION MERGES** for the rulings and the packet below
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D492 item 7 (owner 7a). SPAWN-1a moves here from ARCH-CORE-LOOP-PACKETS-2
  part A (#1735, `OTERYN_GAME_ARCH_BATCH_CORE_LOOP_PACKETS_2026-10-04.md` §1.4 and §2.7), with the
  three open #1735 findings: P1 4177035356, P1 4177035359 and P1 4177035362. #1735 keeps the
  SPAWN-1 split itself (1a here, 1b after MAP-LOAD-1).
- Amends, in this PR:
  - NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS-V1 ("Amendment 2026-10-04");
  - `FIRST_PRODUCTION_CONTENT_PROFILE/v1` (Amendment 04, one spawn of two placement cells);
  - `NATIVE_ENTRY_SOURCE_QUALIFICATION_V1` (Amendment 02, the revision-2 overlay and lowering).
- Runtime, migration and production authority: NONE. The packet needs its #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Order and shared files

SPAWN-1a stays in the runtime lane of #1735 §0.2: after CREATURE-AI-1, before CREATURE-MOVE-1.

| File | Writers, in order |
|---|---|
| `apps/game-server/src/foundation/runtime_actor_carrier.rs` | DEATH-2, ATTACK-1b, CREATURE-AI-1, SPAWN-1a (#1735 §0.3) |
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | each writer edits only its own rows; SPAWN-1a's are `CREATUREAI0-RL-12`, `-14` and the eight FirstProduction rows of Amendment 04 §4 |
| `apps/game-server/src/gameplay_transport/qualification.rs` | SESSION-PUSH-1 (#1736, door-USE expectations), SPAWN-1a (the activation-parts consumer); whichever is allocated second starts from `main` after the first merges |

## 1. Rulings

### 1.1 Room revision 2: the rats' cells and the spawn inputs

**The rat's cell (#1735 P1 4176940109).** The committed spawn sits on `oteryn:cell/entry-east`,
which is a step-and-return proof cell. The product bindings (NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS
§1, after the joins) forbid an activated spawn there. The room fills its bounds (start, east,
north and door), so SPAWN-1a ships room revision 2.

D116 is one spawn of 2 rats at declared placement cells (CREATURE-AI-0 §3 and §6.1; AI-2's
`d116_definition()`: two cells, population 2; #1745 P1 4177068268). The committed record's single
`cell_key` and `population_limit: 1` fall short of it, so revision 2 also realizes D116 in full.
It changes only these things:

- two new cells, both Walkable, same terrain, region and area, `CandidateOnly`, and neither a
  proof cell: `oteryn:cell/entry-den` at (2, 0, 0) and `oteryn:cell/entry-den-north` at
  (2, -1, 0);
- bounds become (0, -1, 3, 1);
- the spawn's single `cell_key` becomes an ordered `cell_keys` list,
  [`entry-den`, `entry-den-north`], with `population_limit` 2, and `accepted::SPAWN_CELL` becomes
  `accepted::SPAWN_CELLS` with the same two keys. The revision-2 qualifier refuses a list that is
  empty, longer than 2 (FirstProduction Amendment 04, §1.5), has a duplicate, names a cell that is
  not Walkable or is a proof cell (start, east, north, the door), or whose length differs from
  `population_limit`;
- `oteryn:map/entry-r1` becomes `oteryn:map/entry-r2`, and `oteryn:content/entry-r1` becomes
  `oteryn:content/entry-r2`; the package and lock identities become r2 (§1.2);
- the spawn record, and `NativeEntrySpawn`, gain the two content inputs that first-creature §4.3
  and §4.8 require (#1735 P1 4176975932): `respawn_delay_ms` 60,000 and
  `occupancy_retry_interval_ms` 5,000, the D115 values that today live only in the test helper.
  The revision-2 qualifier refuses a spawn record that lacks either, a delay outside
  `CREATUREAI0-RL-13` (1,000 ms to 86,400,000 ms), and a retry interval of 0 or above the delay.
  The retry count stays 3 (first-creature §4.3);
- the rat becomes hostile and its runtime inputs are authored in content (§1.6, #1745
  P1 4177181274).

Start, east, north, the door and the relocation are unchanged. The den is adjacent to east and
the north den to the door; neither is a proof cell, and the step-and-return proof and the door
tests never use them. The bindings amendment (that document, "Amendment 2026-10-04", added by
this PR) records revision 2. It takes effect when SPAWN-1a merges.

### 1.2 Room revision 2 is a new immutable package (#1735 P1 4177035356)

DUR-04 makes a `PackageRevision` immutable, and the bindings decision gives replacements new
revisions. Room revision 2 changes the placement set, the World bounds and the spawn record, so
it cannot be published under the r1 package identity:

- package_revision, root project_revision and manifest package_revision become
  `oteryn:package-rev/entry-r2`;
- `accepted::PACKAGE_REVISION` and `accepted::LOCK_TOKEN` become `oteryn:package-rev/entry-r2`
  and `lock:oteryn:package-rev/entry-r2`, the value the canonical writer derives;
- the digests of the source manifest, the manifest and the lock are recomputed with the repository
  tooling, never by hand.

Definition revisions follow their bytes. Two definition records change in r2 (§1.6):

- the `oteryn:behavior/passive-idle` record is replaced by `oteryn:behavior/rat-hostile` at
  `oteryn:rev/entry-r2`;
- `oteryn:creature/rat` binds the new behaviour, so it moves to `oteryn:rev/entry-r2`. Every
  reference to it moves with it: the spawn's creature reference and the overlay's creature
  definition.

The terrain, area, presentations, ability, effect, item, formula and the door object are
byte-identical, so each keeps `oteryn:rev/entry-r1`. The single `accepted::DEFINITION_REVISION`
therefore becomes one accepted revision per definition record. The accepted policy pins become
`oteryn:policy/rat-hostile-r2` for the behaviour and `oteryn:policy/creature-rat-r2` for the
creature. The spawn record carries no definition revision in this format, so its change is
identified by content r2 and package r2. If SPAWN-1a finds that any other definition record's
bytes must change, it returns BLOCKER before any other change. The qualifier refuses:

- a revision-1 pin;
- any mix of r1 and r2 among the package revision, the lock token and the content and map
  revisions;
- an r1 revision on the behaviour or the creature, or an r2 revision on any other definition.

### 1.3 The activation parts reach every consumer (#1735 P1 4177035359)

`into_channel_parts` has two consumers:

- `node/serve.rs`, the production boot;
- `gameplay_transport/qualification.rs`, the Server Seam qualification, which destructures its
  result and constructs the runtime itself.

SPAWN-1a extends the returned parts with the spawn source and updates both consumers. The spawn
source enters the runtime through one additive constructor, or a builder step, next to
`ChannelRuntimeV1::from_committed_assignment`. The existing constructor keeps its signature. Its
other call sites therefore compile unchanged and build a runtime with no spawn source, as today:
`monster_lab.rs`, `movement.rs` and the carrier's tests.

### 1.4 The owning profiles admit two placement cells (#1745 P1 4177087017, P1 4177087019)

Two accepted profiles fix one cell and one rat, and the room needs two of each:

- FirstProduction caps population at 1 per spawn and per scope (`production.rs`
  `FIRST_PRODUCTION_MAX_SPAWN_POPULATION`, the aggregate check; the registry rows), and
  `FirstProductionSpawn` has a single `cell_key`;
- the native source overlay has exactly three cells and a singular spawn `cell_key` (Amendment 01 §3).

This PR amends both owners:

- **FirstProduction Amendment 04**
  (`docs/architecture/OTERYN_GAME_FIRST_PRODUCTION_CONTENT_PROFILE_DECISION_2026-09-09_AMENDMENT_04.md`):
  - still one spawn, with a population of 1 or 2 and at most 2 per scope;
  - `cell_keys` replaces `cell_key`, whose length equals the population;
  - a new three-field spawn-cell record, because the eight-field record limit leaves no room in the
    spawn record;
  - the mechanically recomputed maxima of eight registry rows.
- **Native source Amendment 02**
  (`docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_NATIVE_ENTRY_QUALIFICATION_AMENDMENT_02.md`):
  - five room cells and six Terrain placements (five room cells and the door);
  - the envelope [0,3) × [-1,1);
  - the spawn's ordered `cell_keys` and the two spawn inputs;
  - in-order lowering to the spawn-cell records.

The room has five room cells, not four: start, east, north and the two dens. SPAWN-1a owns the
implementation paths of both (§2.1).

### 1.5 SPAWN-1a scope

- **SPAWN-1a** (hard, performance review). It works from the fixture World's spawn source, read
  through one spawn-source seam that SPAWN-1b later feeds from the bundle. It builds:
  - realization at channel activation, in canonical order and windows, before the channel
    admits players (CREATURE-AI-0 §6.2);
  - the §6.3 respawn: blocking by player interest, the 4,200 ms warning, the Occupied chain and
    at most one pending occurrence per point;
  - the point link;
  - the `CREATUREAI0-RL-12` and `-14` rows.
  Creatures stand until CREATURE-MOVE-1 and fight once ATTACK-1b and CREATURE-AI-1 have merged.

### 1.6 The hostile rat and its authored inputs (#1745 P1 4177181274)

First-creature §4.8 requires the rat to leave `passive-idle` for a hostile behaviour whose inputs
come from the monster authoring pipeline: never hardcoded, and missing means refused. Committed r1
binds passive-idle and authors no health, and `SpawnDefinition::new` needs a positive initial
health. So revision 2 authors both records through the existing v2 authoring profiles
(`ProjectV2BehaviorAuthoring`, `ProjectV2CreatureAuthoring`; monster authoring schema
`behavior.targeting`, `behavior.movement`, `behavior.attacks[]`). The values are the authored
`oteryn:behavior.creature.rat` and `oteryn:creature.rat` (Canary `mammals/rat`) and the Reference
rat fixture §11:

| Record | Authored inputs |
|---|---|
| Behavior `oteryn:behavior/rat-hostile` | targeting: `hostile` true, `can_target` true, `sense_invisible` false, `target_distance_tiles` 1, `static_attack_chance_ppm` 900,000, `flee_health` 5, `change_target` {4,000 ms, 0 ppm}, `strategy_weights` {nearest 100}; movement: `can_walk` true, `pushable` true, the other movement flags false, no `wander`; attacks: exactly one, `oteryn:ability/bite` every 2,000 ms at 1,000,000 ppm, `magnitude` {0, 8} |
| Creature `oteryn:creature/rat` | `health` 20, `initial_health` 20, `speed` 67 |

Rules:

- Revision 2 admits exactly these two authoring profiles, each targeting its record at
  `oteryn:rev/entry-r2`. Any other authoring overlay is still refused, and revision 1 still admits
  none (native source Amendment 02).
- The revision-2 qualifier refuses a missing behaviour or creature profile, and a creature profile
  without `health`, `initial_health` or `speed`. It also refuses an `initial_health` of 0 or above
  `health`, and a behaviour that is not hostile, cannot target, or has no attack. An attack must
  name the room's bite ability, with a positive interval, a nonzero chance and a magnitude. The
  existing v2 profile validation still applies.
- The qualified spawn carries the creature's `health`, `initial_health` and `speed`, and the
  behaviour profile. The pin hands them out with the spawn source (§1.3), and the carrier's
  `creature_initial_health` takes the authored `initial_health`. No literal health, speed or bite
  value exists outside the room source and the tests that read it.
- The profiles are not lowered into the FirstProduction artifact. Its definition records carry
  identities and references only, so Amendment 04 §4 is unchanged. Definitions stay at 1,042,
  because `rat-hostile` replaces `passive-idle` one for one.
- The bite's `magnitude` is authored input only. Evaluating `oteryn:formula/entry-melee-r1` stays
  behind the bindings' formula boundary and ATTACK-1b.

Three §4.8 items are no longer content, because CREATURE-AI-0, accepted after §4.8, supersedes them:

- **Perception** is the MOVE-RL-11 interest area (R1, which supersedes the 7-tile value D115).
- **Think interval** is the 1,000 ms parity value (§10).
- **Wander** is `movement.wander`, an interval and a radius (§5.4). A profile without it stands,
  and the Canary rat has none.

Replacing the remaining `D115_*` constants in the think path is CREATURE-AI-1's work (CREATURE-AI-0
§4). SPAWN-1a adds no such constant. With CREATURE-AI-1 and ATTACK-1b merged, the realized rats
target and bite.

## 2. Packet

### 2.1 SPAWN-1a

```yaml
task_id: OTV2-20261004-spawn-1a
decision: CREATURE-AI-0 §6.2, §6.3; this decision §1.1-§1.6; ARCH-CORE-LOOP-PACKETS-2 §1.4 (#1735)
worker: oteryn-hard-worker
review: performance and determinism review (Codex, final frozen head)
branch: claude/spawn-1a-20261004
base: main after CREATURE-AI-1 merges (the carrier is serialized, #1735 §0.3; §0 here)
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/ai/spawn*.rs                   # new: spawn-source seam, respawn chain
  - apps/game-server/src/ai/mod.rs
  - apps/game-server/src/content/production.rs  # FirstProduction Amendment 04: population 1..=2, aggregate 2, cell_keys, RECORD_SPAWN_CELL (kind 19) and the record-kind uniqueness test, the recomputed maxima and their max/max+1 tests
  - apps/game-server/src/content/mod.rs  # re-exports of the changed FirstProduction items only
  - apps/game-server/tests/content_first_production.rs  # Amendment 04 boundary tests and regenerated goldens
  - tools/agents/tests/test_governance_lifecycle_first_production_content_registry.py  # Amendment 04 final values
  - apps/game-server/src/content/project/native_entry.rs  # native source Amendment 02 overlay parsing and lowering (five room cells, cell_keys, spawn inputs, the two authoring profiles); the activated spawn source and the authored creature inputs in the content pin parts; accepted pins, per-record definition revisions, package and lock identities for room revision 2 (§1.1, §1.2, §1.6)
  - apps/game-server/src/content/project/native_entry_room.json  # room revision 2: the two den cells, the bounds, the spawn cells and population 2, the rat-hostile behaviour and rat creature definitions and their authoring profiles, map, content and package r2 (§1.1, §1.2, §1.6)
  - apps/game-server/src/content/activation.rs  # NativeEntryContentPin, into_channel_parts, activate_native_entry_room (#1735 P1 4176940094)
  - apps/game-server/src/interaction/chest_use.rs  # entry_chest::CONTENT_REVISION and MAP_REVISION only, which gameplay_transport/mod.rs checks against accepted::REVISIONS (#1735 P1 4176975924)
  - apps/game-server/tests/content_native_entry.rs  # the qualification fixture: r2 revisions, bounds, the two den cells, spawn cells and inputs (#1735 P1 4176975924)
  - apps/game-server/src/gameplay_transport/qualification.rs  # the other into_channel_parts consumer and its runtime construction only (§1.3; #1735 P1 4177035359)
  - tools/monster-lab/arena_map.py  # only if it pins the room's cells or revisions
  - tools/monster-lab/test_arena_map.py  # the r2 export: six cells, the rats' slots at (2,0,0) and (2,-1,0) (#1735 P1 4177035362)
  - apps/game-server/src/node/serve.rs                  # boot composition only: pass the spawn source to the runtime constructor
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # CREATUREAI0-RL-12, -14; the eight Amendment 04 §4 rows
  - docs/agents/tasks/archive/OTV2-20261004-spawn-1a.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - python -m unittest discover -s tools/monster-lab -p 'test_*.py'
  - python tools/agents/validate_governance.py
  - git diff --check
```

Acceptance:

- The production boot path realizes the fixture rat before the first admission
  (#1735 P1 4176889394):
  - `activate_native_entry_room` keeps the qualified room's spawn record in its
    `NativeEntryContentPin`;
  - `into_channel_parts` hands it out as the spawn-source input;
  - `node/serve.rs` passes it to the runtime through the additive spawn-source constructor (§1.3),
    which realizes it before the gameplay listener is bound.

  The spawn source therefore comes from the same activated generation as the content pin, never
  from a second read. `gameplay_transport/qualification.rs` takes the new spawn-source part and
  passes it the same way, and every other constructor call site compiles unchanged (§1.3). A boot
  test drives that path and finds both D116 rats, one on each den cell, before readiness.
  `world_runtime.rs` is not on this path and is not touched.
- Room revision 2 (§1.1, §1.2):
  - two rats are realized, one on `entry-den` and one on `entry-den-north`, and no proof cell
    holds a creature;
  - a revision-2 spawn whose `cell_keys` list is empty, has three entries, has a duplicate, names
    a proof cell, or differs in length from `population_limit` is refused;
  - the start/east step-and-return proof and the door tests pass unchanged;
  - a revision-2 source with the spawn on `entry-east` or `entry-start` is refused;
  - a revision-1 pin is refused by the revision-2 qualifier, and so is any mix of r1 and r2 among
    the package revision, the lock token and the content and map revisions;
  - the behaviour and the creature are at `oteryn:rev/entry-r2`, and every other definition record
    keeps `oteryn:rev/entry-r1` byte-identical (§1.2);
  - `test_arena_map.py` expects six cells and the rats' slots at (2, 0, 0) and (2, -1, 0);
  - every committed digest and golden is regenerated with the repository tooling, never by hand,
    and the monster-lab tests pass.
- FirstProduction Amendment 04 (§1.4):
  - population 2 and an aggregate of 2 are accepted;
  - population 0 and 3, and an aggregate of 3, are refused before lowering;
  - each of the eight Amendment 04 §4 rows has a max-accepted test and a max+1-refused test in
    `production.rs`, and the governance registry test pins the new values;
  - a spawn-cell record that is missing, extra, duplicate, out of order or dangling is refused;
  - a decoded artifact round-trips the two cells in order.
- Native source Amendment 02 (§1.4):
  - a revision-2 overlay with four or six room cells, or a singular `cell_key`, is refused;
  - a revision-1 overlay with `cell_keys` is refused.
- The hostile rat (§1.6):
  - the boot rats are bound to `oteryn:behavior/rat-hostile` at `oteryn:rev/entry-r2`, and no
    record names `passive-idle`;
  - each realized rat's initial health equals the source's authored `initial_health`, with no
    runtime constant;
  - a revision-2 source missing the behaviour or creature profile, missing `health`,
    `initial_health` or `speed`, with `initial_health` 0 or above `health`, with a non-hostile
    behaviour, or with no bite attack, is refused;
  - an extra authoring overlay is refused, and so is any authoring overlay in revision 1;
  - FirstProduction definitions stay 1,042 and the Amendment 04 §4 values hold;
  - with CREATURE-AI-1 and ATTACK-1b on the base, a rat targets an adjacent player and bites
    through `oteryn:ability/bite`.
- The spawn inputs (§1.1): the realized point uses the source's `respawn_delay_ms` and
  `occupancy_retry_interval_ms`, with no runtime constant. A source missing either, a delay of
  999 ms or 86,400,001 ms, and a retry interval of 0 are each refused by the qualifier.
- A dead rat respawns one full delay later.
- A player's interest blocks a blockable point, and the successor is a full delay later.
- The warning precedes admission by 4,200 ms.
- An occupied cell retries 3 times every 5,000 ms, then skips.
- There is never more than one pending occurrence per point.
- A restart realizes the same set.

## 3. Rejected options

- **Keep the r1 package identity for r2.** Two contents under one immutable revision (§1.2).
- **Bump every definition to `oteryn:rev/entry-r2`.** An unchanged definition would gain a second
  identity for the same bytes; revisions follow bytes.
- **Keep the rat passive and seed its health from the test helper's 20.** That hardcodes a content
  value and ships rats that cannot fight; first-creature §4.8 forbids both.
- **Author perception range, think interval and wander chance in the behaviour.** CREATURE-AI-0 R1,
  §10 and §5.4 replaced them with the interest area, the parity think interval and
  `movement.wander`. The authoring schema has no such fields (§1.6).
- **Lower the authoring profiles into the FirstProduction artifact.** That would widen its record
  format and every maximum, when the spawn source already comes from the qualified room (§1.6).
- **Two spawn records of one rat each.** D116 is one spawn of 2 rats, and FirstProduction admits
  exactly one spawn. Two spawn records would widen the spawn count instead of the population.
- **A ninth field in the spawn record.** That would raise the eight-field record maximum, and with it
  every byte maximum of every record; the spawn-cell record changes only the spawn rows.
- **Change `from_committed_assignment`'s signature.** It would touch every test call site for no
  behaviour change (§1.3).
- **Keep the rat on `entry-east`.** The bindings forbid an activated spawn on a proof cell.
- **One rat on one den cell.** It realizes less than D116, which is one spawn of 2 rats
  (#1745 P1 4177068268); the two-rat spawn would stay a test helper with no content behind it.

## 4. Decision test

- **Must decide now:** YES. SPAWN-1a is the next runtime-lane packet after CREATURE-AI-1, and
  creatures appear on the playable path only through it.
- **Minimum sufficient:** two cells, one bounds change, the D116 cell list and population, two
  spawn inputs, the hostile rat's two authoring profiles that first-creature §4.8 requires, and the
r2 identities the immutability rule requires. It also needs the two owning
  profile amendments without which they cannot qualify, and one record kind. No new definition and
  no migration. One behaviour definition replaces another.
- **Superseding evidence:** a merged packet that already realizes the fixture spawn; a content
  decision that replaces the entry room.
- **Deliberately not decided:** SPAWN-1b and the bundle spawn family (#1735 §1.4, §2.8).
