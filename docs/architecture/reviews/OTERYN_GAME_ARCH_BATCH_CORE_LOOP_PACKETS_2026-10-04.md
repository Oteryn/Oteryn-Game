# Architect batch: core loop packets (attack, chat, spawns)

- Batch: `ARCH-CORE-LOOP-PACKETS-2` part A (D486)
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the splits, leases and rulings below. ATTACK-0,
  CHAT-0 and CREATURE-AI-0 stay candidates for everything else they decide; the packets here
  implement their accepted semantics only.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: control plane D486 items 1 and 2: the ATTACK-1 and CHAT-1b-2 splits, the ATTACK-WIRE-1
  lease, the SPAWN-CONTENT-1 dependency and the fixture spawn.
- Amends, in this PR: ATTACK-0 implementation brief; CHAT-0 implementation brief; CREATURE-AI-0
  implementation brief.
- Runtime, migration and production authority: NONE. Each packet needs its #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Leases and order

### 0.1 Leases (control plane D486)

| Packet | Lease |
|---|---|
| ATTACK-WIRE-1 | capability 17 `ATTACK_V1` (requires 6), command type 11 `ATTACK_TARGET_INTENT`, command type 12 `FIGHT_MODES_INTENT`, state domain 10 `ACTOR_COMBAT_STATE`; resource rows `ATTACK0-RL-01`, `ATTACK0-RL-02` |
| ATTACK-1b | resource row `ATTACK0-RL-03` (in-fight deadline) |
| CHAT-1b-2b | resource row `CHAT0-RL-11` (undelivered lines per session) |

No packet here needs a migration. Capability 16, command type 22 and domain 16 stay with
QUEST-LOG-WIRE-1.

### 0.2 Order

The control plane's lanes stand (D486):

- **Wire lane:** QUEST-LOG-WIRE-1, ATTACK-WIRE-1, VIS-3, CHAT-1b-2b, ITEM-MOVE-1, ATTACK-1b.
- **Runtime lane:** DEATH-2, ATTACK-1b, CREATURE-AI-1, SPAWN-1a, CREATURE-MOVE-1.
- **Pure, in parallel now:** ATTACK-1a (`combat/attack/**`), CHAT-1b-2a (`chat/**`).
- **Content lane:** SPAWN-CONTENT-1, in parallel with everything above.
- **After MAP-LOAD-1 and SPAWN-CONTENT-1:** SPAWN-1b.

ATTACK-1b is the one packet in both lanes: it needs ATTACK-WIRE-1 and VIS-3 from the wire lane and
DEATH-2 from the runtime lane, because a swing that kills must reach the death path.

### 0.3 Shared files

| File | Writers, in order |
|---|---|
| `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` | QUEST-LOG-WIRE-1, ATTACK-WIRE-1, VIS-3, CHAT-1b-2b, ITEM-MOVE-1, ATTACK-1b |
| `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` | ATTACK-WIRE-1, CHAT-1b-2b, ATTACK-1b, SPAWN-CONTENT-1 (its `CREATUREAI0-RL-*` rows), SPAWN-1a |
| `crates/protocol-oteryn/src/lib.rs` | QUEST-LOG-WIRE-1, ATTACK-WIRE-1 |
| `apps/game-server/src/gameplay_transport/connection.rs` | VIS-3, CHAT-1b-2b, ITEM-MOVE-1, ATTACK-1b |
| `apps/game-server/src/foundation/runtime_actor_carrier.rs` | DEATH-2, ATTACK-1b, CREATURE-AI-1, SPAWN-1a |

A later writer starts from `main` after the earlier one merges. Two packets in one lane never
run at once.

## 1. Rulings

### 1.1 ATTACK-1 splits into a pure 1a and a wiring 1b

ATTACK-0 §4 and §5 hold two kinds of work. The first is pure arithmetic and state. The second
touches the owner cycle, the transport and the death path. The pure half has no shared file, so
it runs now while the wire lane is busy.

- **ATTACK-1a** (pure, `apps/game-server/src/combat/attack/**`). It builds:
  - the per-actor attack state: target, fight modes and the in-fight deadline;
  - the swing occurrence key (runtime scope, attacker actor id and generation, swing sequence);
  - the `DEADLINE_STATE` next-deadline rule, where the next deadline counts from the time the
    swing actually ran;
  - validity as a pure predicate over facts the caller supplies;
  - the §5 formulas as `player_expression` trees with the constants table, `PARITY_PENDING`.
  It builds no timer, dispatch, durability or wire. Its tests are unit tests on fixed inputs,
  including the closed RNG purposes.
- **ATTACK-1b** (wiring). It builds:
  - the owner-cycle swing timer for players and creatures, through the GAME-ABILITY-01 pipeline
    as `AutoAttack`;
  - creature melee against the GAME-AI-01 target;
  - the charm hooks;
  - the logout blocker for FND-ID-01 and the closed-client rule;
  - dispatch of command types 11 and 12, domain 10 snapshots and deltas;
  - the offer of capability 17 with a production-path admission test that selects it;
  - the `ATTACK0-RL-03` row.
  It also answers the spell `ATTACK_TARGET` resolution seam that SPELL-TARGET-1 consumes.

### 1.2 CHAT-1b-2 splits into a pure 2a and a wiring 2b

CHAT-1b-1 (merged) defined the whole wire. The remaining CHAT-1b-2 scope splits the same way.
CAP-NEG-1 and CAP-NEG-RESUME-FALLBACK-1 have merged, so CHAT-1b-2 no longer waits for
CHARM-5-COMP: it reuses the CAP-NEG-1 selection seam.

- **CHAT-1b-2a** (pure, `apps/game-server/src/chat/**`). It builds:
  - the per-session undelivered-line queue, bounded at 64 lines. When it is full, the oldest
    room line goes first, private and local lines go last, and one `DROPPED` marker stands for a
    run of drops (CHAT-0 §7);
  - the in-memory session census that `NOT_ONLINE` and the yell gate read;
  - the mute read that the spell path uses on command 3 (CHAT-0 §6).
  It builds no connection wiring. The queue bound is a constant, with a test pinning it to 64.
- **CHAT-1b-2b** (wiring). It builds:
  - dispatch of command type 13 in `connection.rs`, through the CHAT-1a speech and the 2a queue;
  - domain 12 snapshots and deltas, with a revision that stays above any the session has seen
    across a reconnect;
  - the mute read on command 3 (`SPELL_CAST_DISPOSITION_REJECTED`);
  - the `CHAT0-RL-11` row with a test that binds it to the 2a constant;
  - the offer of capability 7 with a production-path admission test that selects it.
  It updates the capability 7 `offer_gate` note to name CAP-NEG-1 instead of CHARM-5-COMP.

### 1.3 SPAWN-CONTENT-1 needs MAP-BUNDLE-1, not the overlay or the cutover

The CREATURE-AI-0 brief lists "the ADR-0021 bundle compiler (MAP-OVERLAY-1, MAP-CUTOVER-1)". That
is wrong.

- Spawn sources are a base family of the World bundle (CREATURE-AI-0 §6.1). They change only at a
  planned reset (D194).
- MAP-OVERLAY-1 owns runtime item state over the base. MAP-CUTOVER-1 boots Worlds from the
  bundle. Neither one reads or writes a spawn family.
- MAP-BUNDLE-1 (merged, `tools/world-bundle-compiler/**`) already compiles World Project families
  beside the placements (`project.rs`: teleports, houses).

**Ruling.** SPAWN-CONTENT-1 depends on MAP-BUNDLE-1 and on admitted creature definitions only. It
adds a spawn family to:

- the compiler;
- the format document `OTERYN_WORLD_BUNDLE_FORMAT_V1.md`, in a new section with the version
  rules of that document;
- the World Project source.

It also adds the `CREATUREAI0-RL-01` to `-03` and `-13` validation and the dropped-point
diagnostics. The reader side belongs to MAP-LOAD-1's shared reader in `crates/world-bundle`
(ARCH-ROOT-PACKETS-1 §1.3). SPAWN-CONTENT-1 adds the family's decode to that crate only if
MAP-LOAD-1 has merged first; otherwise SPAWN-1b adds it.

### 1.4 SPAWN-1 splits: the D116 fixture spawn first, the map after MAP-LOAD-1

AI-2 (merged) built `realize_spawn` and the respawn timer in the runtime actor carrier, and they
admit the D116 fixture spawn. In the committed fixture (`native_entry_room.json`, spawn
`oteryn:spawn/entry-rat`) that is one source with one rat (`population_limit: 1`). The two-rat
spawn in `runtime_actor_carrier.rs` is a test helper, not content, and SPAWN-1a does not use it
(#1735 P1 4176889390). No production path
calls them: a running channel has no creatures today.

**The rat's cell (#1735 P1 4176940109).** The committed spawn sits on `oteryn:cell/entry-east`,
which is a step-and-return proof cell. The product bindings (NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS
§1, after the joins) forbid an activated spawn there. The room fills its bounds (start, east,
north and door), so SPAWN-1a ships room revision 2, which changes only these things:

- a fifth cell `oteryn:cell/entry-den` at (2, 0, 0): Walkable, same terrain, region and area,
  `CandidateOnly`;
- bounds become (0, -1, 3, 1);
- the spawn's `cell_key`, and `accepted::SPAWN_CELL`, become `entry-den`;
- `oteryn:map/entry-r1` becomes `oteryn:map/entry-r2`, and `oteryn:content/entry-r1` becomes
  `oteryn:content/entry-r2`. No other revision changes;
- the spawn record, and `NativeEntrySpawn`, gain the two content inputs that first-creature §4.3
  and §4.8 require (#1735 P1 4176975932): `respawn_delay_ms` 60,000 and
  `occupancy_retry_interval_ms` 5,000, the D115 values that today live only in the test helper.
  The revision-2 qualifier refuses a spawn record that lacks either, a delay outside
  `CREATUREAI0-RL-13` (1,000 ms to 86,400,000 ms), and a retry interval of 0 or above the delay.
  The retry count stays 3 (first-creature §4.3).

Start, east, north, the door and the relocation are unchanged. The den is adjacent only to
east, and it is not a proof cell. The bindings amendment (that document, "Amendment 2026-10-04")
records revision 2. It takes effect when SPAWN-1a merges.

**Ruling.** SPAWN-1 may realize the D116 fixture spawn before MAP-LOAD-1. It splits as follows.

- **SPAWN-1a** (hard, performance review). It works from the fixture World's spawn source, read
  through one spawn-source seam that SPAWN-1b later feeds from the bundle. It builds:
  - realization at channel activation, in canonical order and windows, before the channel
    admits players (CREATURE-AI-0 §6.2);
  - the §6.3 respawn: blocking by player interest, the 4,200 ms warning, the Occupied chain and
    at most one pending occurrence per point;
  - the point link;
  - the `CREATUREAI0-RL-12` and `-14` rows.
  Creatures stand until CREATURE-MOVE-1 and fight once ATTACK-1b and CREATURE-AI-1 have merged.
- **SPAWN-1b** (hard, performance review). It feeds the seam from the active bundle's spawn
  family through MAP-LOAD-1's reader. It measures `CREATUREAI0-RL-15` and `-16` at the reference
  map. It depends on MAP-LOAD-1 and SPAWN-CONTENT-1.

The fixture World keeps D116 after SPAWN-1b (CREATURE-AI-0 §6.1). SPAWN-1a does not need
CREATURE-AI-1. It follows CREATURE-AI-1 in the runtime lane only because both write the carrier
(§0.3).

## 2. Packets

### 2.1 ATTACK-WIRE-1

```yaml
task_id: OTV2-20261004-attack-wire-1
decision: ATTACK-0 §3 (with this batch §0.1)
worker: oteryn-hard-worker
review: protocol review (Codex, final frozen head)
branch: claude/attack-wire-1-20261004
base: main after QUEST-LOG-WIRE-1 merges
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json      # ATTACK0-RL-01, -02 only
  - docs/contracts/protocol-oteryn/v1/attack_v1.proto # new
  - crates/protocol-oteryn/src/{lib,attack,attack_tests}.rs
  - docs/agents/tasks/archive/OTV2-20261004-attack-wire-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy -p oteryn-protocol-oteryn --all-targets -- -D warnings
  - cargo test --locked -p oteryn-protocol-oteryn
  - cargo test --locked -p oteryn-session
  - python tools/agents/validate_governance.py
  - git diff --check
```

Builds, the CHAT-1b-1 pattern:

- Capability 17 `ATTACK_V1`, `requires: [6]`, `offered: false`. Its `offer_gate` names
  ATTACK-1b.
- Command type 11 `ATTACK_TARGET_INTENT`: `AttackTargetIntentV1 {target: EntityRefV1}`. An absent
  target stops attacking. A present target needs a 16-byte identity and a non-zero generation;
  anything else is `Malformed`.
- Command type 12 `FIGHT_MODES_INTENT`: `FightModesIntentV1 {fight_mode, chase, secure}`. The
  enums are closed, and the zero value is `Malformed`.
- `AttackIntentResultV1`, shared by both: `OK`, `TARGET_NOT_VISIBLE`, `TARGET_NOT_A_CREATURE`,
  `PROTECTION_ZONE`, `REENTRY_PROTECTED`, `REJECTED`.
- State domain 10 `ACTOR_COMBAT_STATE`: snapshot type 1 and delta type 1 both carry
  `ActorCombatStateV1 {target: EntityRefV1 or absent, fight_mode, chase, secure, in_fight}`.
- Rows `ATTACK0-RL-01` (target changes per second) and `ATTACK0-RL-02` (fight-mode changes per
  second). The values come from Canary's client action limit, or are the CHAT-1b-1-style
  conservative bound with its evidence, stated in the record.
- A strict codec in both directions, with byte bounds, and a test that binds the registries and
  the proto to the codec constants.

Acceptance:

- Encode/decode round trips at the byte bounds.
- An empty, doubled or unknown field is refused.
- A zero generation is refused.
- The capability is not offered.

Not in scope: dispatch, the runtime and the client (ATTACK-1b, ATTACK-CLIENT-1).

Protocol review question: the record states whether `chase` carries `CHASE` now. ATTACK-0 §3
carries it, and RANGED-0 has the client ship it disabled, so the server accepts it and treats it
as `STAND` until CHASE-1.

### 2.2 ATTACK-1a

```yaml
task_id: OTV2-20261004-attack-1a
decision: ATTACK-0 §4, §5 (with this batch §1.1)
worker: oteryn-impl-worker
review: combat review (Codex, final frozen head)
branch: claude/attack-1a-20261004
base: main
owned_paths:
  - apps/game-server/src/combat/attack/**             # new
  - apps/game-server/src/combat.rs                    # `#[path = "combat/attack/mod.rs"] pub(crate) mod attack;` only (the file's explicit-path rule)
  - docs/agents/tasks/archive/OTV2-20261004-attack-1a.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server combat::attack
  - python tools/agents/validate_governance.py
  - git diff --check
```

Acceptance:

- The next deadline after a stall is the time the swing actually ran plus 2,000 ms, never a
  backlog.
- Validity refuses each condition of ATTACK-0 §4 on its own.
- The in-fight deadline is 60 s after the last hit dealt or taken.
- Fight factors 1.0, 0.75 and 0.5; fist attack 7 and fist defence 7.
- Block budget: one per 1,000 ms, at most 2.
- Armor removal for armor 1 to 3 and above 3.
- The RNG purposes are a closed enum.

Not in scope: anything outside `combat/attack/**` and the one `mod` line.

### 2.3 ATTACK-1b

```yaml
task_id: OTV2-20261004-attack-1b
decision: ATTACK-0 §3, §4 (with this batch §1.1)
worker: oteryn-hard-worker
review: combat and protocol review (Codex, final frozen head)
branch: claude/attack-1b-20261004
base: main after ATTACK-1a, ATTACK-WIRE-1, VIS-3, ITEM-MOVE-1 and DEATH-2 merge
owned_paths:
  - apps/game-server/src/combat/attack/**
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/owner_timer.rs
  - apps/game-server/src/gameplay_transport/{mod,connection,capabilities}.rs
  - apps/game-server/src/gameplay_transport/attack.rs  # new: codec glue, domain 10 emission
  - apps/game-server/src/spell/cast.rs                  # the ATTACK_TARGET resolution seam only
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json    # capability 17 offered: true
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # ATTACK0-RL-03
  - docs/agents/tasks/archive/OTV2-20261004-attack-1b.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - cargo test --locked -p oteryn-protocol-oteryn
  - python tools/agents/validate_governance.py
  - git diff --check
```

Acceptance:

- A scripted session selects capabilities 6 and 17, targets a D116 rat and kills it with fists.
  The kill reaches loot and XP through VSL-COMBAT-01 and DEATH-2.
- The rat hits back through the same pipeline and the incoming charm hooks.
- A logout during the in-fight deadline is refused. A closed client leaves the actor in the world
  until the deadline passes.
- A reconnect keeps the deadline and clears the target.
- Re-entry protection refuses a target and stops creature attacks for 4 s.
- A target in a protection zone, or an attacker standing in one, is refused.
- A player target is refused (creatures only, ATTACK-0 §3).

Until SPAWN-1a merges, the test realizes D116 through the AI-2 test path. Once SPAWN-1a merges,
SPAWN-1a's activation realizes it.

Not in scope: PvP, chase, distance weapons, spells (SPELL-TARGET-1), damage numbers
(COMBAT-PRESENT-1) and the client.

### 2.4 CHAT-1b-2a

```yaml
task_id: OTV2-20261004-chat-1b-2a
decision: CHAT-0 §6, §7 (with this batch §1.2)
worker: oteryn-impl-worker
review: security review (Codex, final frozen head)
branch: claude/chat-1b-2a-20261004
base: main
owned_paths:
  - apps/game-server/src/chat/**
  - docs/agents/tasks/archive/OTV2-20261004-chat-1b-2a.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server chat
  - python tools/agents/validate_governance.py
  - git diff --check
```

Acceptance:

- With 64 lines queued, a new room line drops the oldest room line and one marker stands for the
  run.
- Private and local lines are dropped only when no room line is left.
- The census answers online/offline by name, case-folded as CHAR-NAME requires.
- A muted actor's command-3 read answers muted.
- No chat payload reaches a log at ordinary levels (FND-02 §20); a test captures the logs.

### 2.5 CHAT-1b-2b

```yaml
task_id: OTV2-20261004-chat-1b-2b
decision: CHAT-0 §3, §6, §7 (with this batch §1.2)
worker: oteryn-hard-worker
review: protocol, security and privacy review (Codex, final frozen head)
branch: claude/chat-1b-2b-20261004
base: main after VIS-3 and CHAT-1b-2a merge
owned_paths:
  - apps/game-server/src/chat/**
  - apps/game-server/src/gameplay_transport/{mod,connection,capabilities}.rs
  - apps/game-server/src/gameplay_transport/chat.rs     # new: codec glue, domain 12 emission
  - apps/game-server/src/spell/cast.rs                  # mute read on command 3 only
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json    # capability 7 offered: true, offer_gate note
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # CHAT0-RL-11
  - docs/agents/tasks/archive/OTV2-20261004-chat-1b-2b.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy --workspace --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - cargo test --locked -p oteryn-protocol-oteryn
  - python tools/agents/validate_governance.py
  - git diff --check
```

Acceptance:

- Two scripted sessions with capability 7 selected: a say within range is heard, and one outside
  range is not.
- A whisper is obscured at distance.
- A yell is gated by level and premium (`premium_current`).
- The NPC greeting answers.
- A flood past 64 lines sends the marker and never trips slow-client termination.
- A muted cast is `SPELL_CAST_DISPOSITION_REJECTED`.
- A session without capability 7 gets no domain 12, and command type 13 is refused as
  unsupported.

Not in scope: private messages, World rooms and the durable mute (CHAT-2), and the client
(CHAT-CLIENT-1).

### 2.6 SPAWN-CONTENT-1

```yaml
task_id: OTV2-20261004-spawn-content-1
decision: CREATURE-AI-0 §6.1 (with this batch §1.3)
worker: oteryn-impl-worker
review: content and format review (Codex, final frozen head)
branch: claude/spawn-content-1-20261004
base: main
owned_paths:
  - tools/world-bundle-compiler/**
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md    # spawn family section
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # CREATUREAI0-RL-01..-03, -13
  - content/world/**                                   # spawn family source; the exact path is named at allocation
  - docs/agents/tasks/archive/OTV2-20261004-spawn-content-1.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy -p oteryn-world-bundle-compiler --all-targets -- -D warnings
  - cargo test --locked -p oteryn-world-bundle-compiler
  - python tools/agents/validate_governance.py
  - git diff --check
```

Acceptance:

- The reference map compiles with its 51,896 sources and 83,286 points, each bound to an admitted
  creature definition or listed as a diagnostic.
- Bounds are tested at max and max+1.
- A point on a cell that cannot admit its creature is dropped and listed in the parity report.
- A boss point is routed to BOSS-RAID-0 and not realized.
- The bundle stays byte-reproducible.

The allocation names the World Project source path. If the map agent's work owns that path, the
control plane serializes the two.

### 2.7 SPAWN-1a

```yaml
task_id: OTV2-20261004-spawn-1a
decision: CREATURE-AI-0 §6.2, §6.3 (with this batch §1.4)
worker: oteryn-hard-worker
review: performance and determinism review (Codex, final frozen head)
branch: claude/spawn-1a-20261004
base: main after CREATURE-AI-1 merges (the carrier is serialized, §0.3)
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/ai/spawn*.rs                   # new: spawn-source seam, respawn chain
  - apps/game-server/src/ai/mod.rs
  - apps/game-server/src/content/project/native_entry.rs  # the activated spawn source in the content pin parts; accepted pins for room revision 2 (§1.4)
  - apps/game-server/src/content/project/native_entry_room.json  # room revision 2: the entry-den cell, the bounds, the spawn cell, map and content r2 (§1.4)
  - apps/game-server/src/content/activation.rs  # NativeEntryContentPin, into_channel_parts, activate_native_entry_room (#1735 P1 4176940094)
  - apps/game-server/src/interaction/chest_use.rs  # entry_chest::CONTENT_REVISION and MAP_REVISION only, which gameplay_transport/mod.rs checks against accepted::REVISIONS (#1735 P1 4176975924)
  - apps/game-server/tests/content_native_entry.rs  # the qualification fixture: r2 revisions, bounds, entry-den, spawn inputs (#1735 P1 4176975924)
  - tools/monster-lab/arena_map.py  # only if it pins the room's cells or revisions
  - apps/game-server/src/node/serve.rs                  # boot composition only: pass the spawn source to the runtime constructor
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json       # CREATUREAI0-RL-12, -14
  - docs/agents/tasks/archive/OTV2-20261004-spawn-1a.md
validation:
  - cargo fmt --all -- --check
  - cargo clippy -p oteryn-game-server --all-targets -- -D warnings
  - cargo test --locked -p oteryn-game-server
  - python tools/agents/validate_governance.py
  - git diff --check
```

Acceptance:

- The production boot path realizes the fixture rat before the first admission
  (#1735 P1 4176889394):
  - `activate_native_entry_room` keeps the qualified room's spawn record in its
    `NativeEntryContentPin`;
  - `into_channel_parts` hands it out as the spawn-source input;
  - `node/serve.rs` passes it to `ChannelRuntimeV1::from_committed_assignment`, which realizes it
    before the gameplay listener is bound.

  The spawn source therefore comes from the same activated generation as the content pin, never
  from a second read. A boot test drives that path and finds one rat before readiness.
  `world_runtime.rs` is not on this path and is not touched.
- Room revision 2 (§1.4):
  - the rat is realized on `entry-den`, and no proof cell holds a creature;
  - the start/east step-and-return proof and the door tests pass unchanged;
  - a revision-2 source with the spawn on `entry-east` or `entry-start` is refused;
  - a revision-1 pin is refused by the revision-2 qualifier;
  - every committed digest and golden is regenerated with the repository tooling, never by hand,
    and the monster-lab tests pass.
- The spawn inputs (§1.4): the realized point uses the source's `respawn_delay_ms` and
  `occupancy_retry_interval_ms`, with no runtime constant. A source missing either, a delay of
  999 ms or 86,400,001 ms, and a retry interval of 0 are each refused by the qualifier.
- A dead rat respawns one full delay later.
- A player's interest blocks a blockable point, and the successor is a full delay later.
- The warning precedes admission by 4,200 ms.
- An occupied cell retries 3 times every 5,000 ms, then skips.
- There is never more than one pending occurrence per point.
- A restart realizes the same set.

### 2.8 SPAWN-1b

Packeted when MAP-LOAD-1 and SPAWN-CONTENT-1 have merged, with the measurement node named by
ADR-0021 §4.8.

## 3. Brief amendments made in this PR

- **ATTACK-0 brief:** ATTACK-1 becomes ATTACK-1a and ATTACK-1b (§1.1); ATTACK-WIRE-1's numbers are
  leased (§0.1); client target selection and the fight-mode buttons move to ATTACK-CLIENT-1
  (part B §2.5; #1735 P2 4176889397).
- **CHAT-0 brief:** CHAT-1 is built as CHAT-1a, CHAT-1b-1, CHAT-1b-2a and CHAT-1b-2b (§1.2).
- **CREATURE-AI-0 brief:** SPAWN-CONTENT-1 depends on MAP-BUNDLE-1 (§1.3); SPAWN-1 becomes
  SPAWN-1a and SPAWN-1b (§1.4).

## 4. Rejected options

- **One ATTACK-1.** It would wait for the whole wire lane before any combat code exists.
- **SPAWN-CONTENT-1 after MAP-CUTOVER-1.** Spawns are base content; the overlay and the boot path
  never touch them.
- **SPAWN-1 waiting for MAP-LOAD-1.** The fixture spawn is enough to make creatures appear, fight
  and respawn on the playable path now.
- **CHAT-1b-2 waiting for CHARM-5-COMP.** CAP-NEG-1 already built the seam CHARM-5-COMP was to
  build.

## 5. Decision test

- **Must decide now:** YES. The wire and runtime lanes are idle without these packets.
- **Minimum sufficient:** splits along existing file boundaries, and no new decision content.
- **Superseding evidence:** a merged packet that already covers a split half.
