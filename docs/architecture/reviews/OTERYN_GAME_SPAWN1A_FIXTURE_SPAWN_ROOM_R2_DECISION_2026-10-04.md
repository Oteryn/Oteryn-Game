# SPAWN-1A-PACKET-1: the D116 fixture spawn and entry-room revision 2

- Decision: `SPAWN-1A-PACKET-1`
- Status: **CANDIDATE; ACCEPTED WHEN THIS DECISION MERGES** for the rulings and the packet below
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D492 item 7 (owner 7a). SPAWN-1a moves here from ARCH-CORE-LOOP-PACKETS-2
  part A (#1735, `OTERYN_GAME_ARCH_BATCH_CORE_LOOP_PACKETS_2026-10-04.md` §1.4 and §2.7), with the
  three open #1735 findings: P1 4177035356, P1 4177035359 and P1 4177035362. #1735 keeps the
  SPAWN-1 split itself (1a here, 1b after MAP-LOAD-1).
- Amends, in this PR: NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS-V1 ("Amendment 2026-10-04").
- Runtime, migration and production authority: NONE. The packet needs its #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Order and shared files

SPAWN-1a stays in the runtime lane of #1735 §0.2: after CREATURE-AI-1, before CREATURE-MOVE-1.

| File | Writers, in order |
|---|---|
| `apps/game-server/src/foundation/runtime_actor_carrier.rs` | DEATH-2, ATTACK-1b, CREATURE-AI-1, SPAWN-1a (#1735 §0.3) |
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | each writer edits only its own rows; SPAWN-1a's are `CREATUREAI0-RL-12`, `-14` |
| `apps/game-server/src/gameplay_transport/qualification.rs` | SESSION-PUSH-1 (#1736, door-USE expectations), SPAWN-1a (the activation-parts consumer); whichever is allocated second starts from `main` after the first merges |

## 1. Rulings

### 1.1 Room revision 2: the rat's cell and the spawn inputs

**The rat's cell (#1735 P1 4176940109).** The committed spawn sits on `oteryn:cell/entry-east`,
which is a step-and-return proof cell. The product bindings (NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS
§1, after the joins) forbid an activated spawn there. The room fills its bounds (start, east,
north and door), so SPAWN-1a ships room revision 2, which changes only these things:

- a fifth cell `oteryn:cell/entry-den` at (2, 0, 0): Walkable, same terrain, region and area,
  `CandidateOnly`;
- bounds become (0, -1, 3, 1);
- the spawn's `cell_key`, and `accepted::SPAWN_CELL`, become `entry-den`;
- `oteryn:map/entry-r1` becomes `oteryn:map/entry-r2`, and `oteryn:content/entry-r1` becomes
  `oteryn:content/entry-r2`; the package and lock identities become r2 (§1.2);
- the spawn record, and `NativeEntrySpawn`, gain the two content inputs that first-creature §4.3
  and §4.8 require (#1735 P1 4176975932): `respawn_delay_ms` 60,000 and
  `occupancy_retry_interval_ms` 5,000, the D115 values that today live only in the test helper.
  The revision-2 qualifier refuses a spawn record that lacks either, a delay outside
  `CREATUREAI0-RL-13` (1,000 ms to 86,400,000 ms), and a retry interval of 0 or above the delay.
  The retry count stays 3 (first-creature §4.3).

Start, east, north, the door and the relocation are unchanged. The den is adjacent only to
east, and it is not a proof cell. The bindings amendment (that document, "Amendment 2026-10-04", added by
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

Definition revisions follow their bytes. No definition record changes in r2: the terrain, area,
creature, behavior, presentations, ability, effect, item, formula and the door object are all
byte-identical. Each therefore keeps `oteryn:rev/entry-r1`, and `accepted::DEFINITION_REVISION`
stays. The spawn record carries no definition revision in this format, so its change is
identified by content r2 and package r2. If SPAWN-1a finds a definition record whose bytes must
change, that record takes `oteryn:rev/entry-r2`, and the worker returns BLOCKER before any other
change. The qualifier refuses:

- a revision-1 pin;
- any mix of r1 and r2 among the package revision, the lock token and the content and map
  revisions.

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

### 1.4 SPAWN-1a scope

- **SPAWN-1a** (hard, performance review). It works from the fixture World's spawn source, read
  through one spawn-source seam that SPAWN-1b later feeds from the bundle. It builds:
  - realization at channel activation, in canonical order and windows, before the channel
    admits players (CREATURE-AI-0 §6.2);
  - the §6.3 respawn: blocking by player interest, the 4,200 ms warning, the Occupied chain and
    at most one pending occurrence per point;
  - the point link;
  - the `CREATUREAI0-RL-12` and `-14` rows.
  Creatures stand until CREATURE-MOVE-1 and fight once ATTACK-1b and CREATURE-AI-1 have merged.

## 2. Packet

### 2.1 SPAWN-1a

```yaml
task_id: OTV2-20261004-spawn-1a
decision: CREATURE-AI-0 §6.2, §6.3; this decision §1.1-§1.4; ARCH-CORE-LOOP-PACKETS-2 §1.4 (#1735)
worker: oteryn-hard-worker
review: performance and determinism review (Codex, final frozen head)
branch: claude/spawn-1a-20261004
base: main after CREATURE-AI-1 merges (the carrier is serialized, #1735 §0.3; §0 here)
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/ai/spawn*.rs                   # new: spawn-source seam, respawn chain
  - apps/game-server/src/ai/mod.rs
  - apps/game-server/src/content/project/native_entry.rs  # the activated spawn source in the content pin parts; accepted pins, package and lock identities for room revision 2 (§1.1, §1.2)
  - apps/game-server/src/content/project/native_entry_room.json  # room revision 2: the entry-den cell, the bounds, the spawn cell, map, content and package r2 (§1.1, §1.2)
  - apps/game-server/src/content/activation.rs  # NativeEntryContentPin, into_channel_parts, activate_native_entry_room (#1735 P1 4176940094)
  - apps/game-server/src/interaction/chest_use.rs  # entry_chest::CONTENT_REVISION and MAP_REVISION only, which gameplay_transport/mod.rs checks against accepted::REVISIONS (#1735 P1 4176975924)
  - apps/game-server/tests/content_native_entry.rs  # the qualification fixture: r2 revisions, bounds, entry-den, spawn inputs (#1735 P1 4176975924)
  - apps/game-server/src/gameplay_transport/qualification.rs  # the other into_channel_parts consumer and its runtime construction only (§1.3; #1735 P1 4177035359)
  - tools/monster-lab/arena_map.py  # only if it pins the room's cells or revisions
  - tools/monster-lab/test_arena_map.py  # the r2 export: five cells, the rat's slot at (2,0,0) (#1735 P1 4177035362)
  - apps/game-server/src/node/serve.rs                  # boot composition only: pass the spawn source to the runtime constructor
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # CREATUREAI0-RL-12, -14
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
  test drives that path and finds one rat before readiness.
  `world_runtime.rs` is not on this path and is not touched.
- Room revision 2 (§1.1, §1.2):
  - the rat is realized on `entry-den`, and no proof cell holds a creature;
  - the start/east step-and-return proof and the door tests pass unchanged;
  - a revision-2 source with the spawn on `entry-east` or `entry-start` is refused;
  - a revision-1 pin is refused by the revision-2 qualifier, and so is any mix of r1 and r2 among
    the package revision, the lock token and the content and map revisions;
  - every definition record keeps `oteryn:rev/entry-r1` byte-identical (§1.2);
  - `test_arena_map.py` expects five cells and the rat's slot at (2, 0, 0);
  - every committed digest and golden is regenerated with the repository tooling, never by hand,
    and the monster-lab tests pass.
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
- **Change `from_committed_assignment`'s signature.** It would touch every test call site for no
  behaviour change (§1.3).
- **Keep the rat on `entry-east`.** The bindings forbid an activated spawn on a proof cell.

## 4. Decision test

- **Must decide now:** YES. SPAWN-1a is the next runtime-lane packet after CREATURE-AI-1, and
  creatures appear on the playable path only through it.
- **Minimum sufficient:** one cell, one bounds change, two spawn inputs and the r2 identities the
  immutability rule requires. No new definition and no migration.
- **Superseding evidence:** a merged packet that already realizes the fixture spawn; a content
  decision that replaces the entry room.
- **Deliberately not decided:** SPAWN-1b and the bundle spawn family (#1735 §1.4, §2.8).
