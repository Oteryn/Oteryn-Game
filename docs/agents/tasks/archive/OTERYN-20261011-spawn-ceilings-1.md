# OTERYN-20261011-spawn-ceilings-1

```yaml
task_id: SPAWN-CEILINGS-1
title: "SPAWN-CEILINGS-1: the Game carrier realizes spawns up to the registered CREATURE-AI-0 ceilings"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spawn-ceilings-1-20261011
pr: 1984
base_sha: 407b0f2b80034925fc2070eae308850e49ede264
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
coordination: "#1622; origin Codex P1 on #1981 (SPAWN-1b)"
created_at: 2026-10-11
updated_at: 2026-10-11
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/OTERYN-20261011-spawn-ceilings-1.md
public_contracts:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
external_repositories: []
```

## Outcome

`ChannelRuntimeV1::realize_activation_spawns` no longer refuses readiness above 16 sources or 4
cells. Spawn realization reads the registered CREATURE-AI-0 §10 rows: `CREATUREAI0-RL-02`
65,536 sources per channel, `CREATUREAI0-RL-03` 64 points per source and `CREATUREAI0-RL-01`
131,072 points per channel. Above any of them activation refuses with zero mutation.

## Architecture and source of truth

- CREATURE-AI-0 decision §10 (`OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md`):
  RL-02 replaces `AI01-SPAWN-SOURCES-PER-SCOPE` 16, RL-03 replaces `AI01-SPAWN-POPULATION` 4 and
  `AI01-SPAWN-PLACEMENT-CELLS` 4.
- The carrier's spawn state and spawned population are bounded by RL-01 (a sum of points), not
  by the RL-02 × RL-03 product (4,194,304).
- `RESOURCE_LIMITS_REGISTRY.json` names the carrier constants as consumers of RL-01/02/03; the
  three AI01 rows note that they are superseded for spawn realization.

## Excluded scope

- `map/boot.rs` and `node/serve.rs` (#1981), the world pin and bundle.
- `ai/spawn.rs` (`RespawnSchedule`), death paths in other carrier modules, `smelly_cheese.rs`.
- RL-15/RL-16 activation time and memory measurement (owed by SPAWN-1).

## Acceptance criteria

- 64 points per source pass, 65 refuse (`InvalidSpawnDefinition`).
- 65,536 one-point sources realize at the runtime; 65,537 refuse with nothing admitted.
- 131,072 points pass the channel check; 131,073 refuse at the runtime with nothing admitted, and
  the single-source path refuses one point past the ceiling.
- No quadratic activation work and no per-removal scan of the spawn cells.
- `CREATUREAI0-RL-14` (Codex 4242094682 on #1984): no owner window realizes more than 4,096
  points. One owner turn (`realize_activation_spawns`) refuses a larger activation with nothing
  admitted; `begin_activation_spawns` keeps it as a persisted cursor and
  `realize_activation_spawn_window(now)` realizes the next `spawn::realization_windows` window,
  at most one per 50 ms owner window, in canonical order.
- The full respawn scan runs once per Channel owner window, not once per session drive
  (Codex 4242094686 on #1984).
- Activation leaves `SPAWN_ACTIVATION_PLAYER_HEADROOM` (500) carrier slots free for players
  (Codex 4242245064 on #1984): on the 131,072-actor pre-production carrier at most 130,572
  points are accepted and 500 players are then admitted.
- Respawn work per owner window is bounded (Codex 4242270322 on #1984): one window runs at most
  `SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX` (32) respawn steps however many occurrences fall due
  together; the rest stay pending, run first in the next windows, and every point respawns.
  A refused activation window removes its creatures with one pass over the occupied index.

## Implementation / findings

- Activation checks every ceiling and the free-slot count once, then admits all sources under
  one snapshot (`admit_spawn_source`); the old per-source `slots.clone()` made activation
  O(sources × slots).
- `spawns` stays in ascending source order; every source lookup is a binary search.
- Removal no longer scans every spawn cell; `creature_alive` now checks the exact actor-local
  generation, so a stale cell reference never reads as the slot's later occupant. The summon
  move rollback therefore no longer snapshots `spawns`.
- The 16/4 constants remain only as the fixed D57 64-entry ledger envelope
  (`MAX_RETAINED_CORPSE_PROJECTIONS`, creature bite ledger, smelly cheese ledger).
- RL-14 (review round on #1984): the activation is split into owner windows by
  `spawn::realization_windows` at point granularity (`admit_spawn_point`; a source is recorded
  at its point 0 and may span two windows). `SpawnActivation` persists the plan, the next-window
  cursor and the last owner window used; a second call in one owner window realizes nothing.
  The Channel must not admit players until `remaining` is 0; the respawn drive and the
  qualification spawn do nothing or refuse while an activation is pending. A refusal in a later
  window removes every creature the activation admitted (by removal, which relinks the free
  list, so owner work between windows is never undone) and a new activation may begin. The
  one-turn path keeps its exact snapshot restore. No current caller passes more than 4,096
  points (bundle worlds pass none until SPAWN-1b, #1981, which consumes the windowed API).
- Spawn drive coalescing (review round on #1984): the owner calls `drive_spawn_respawns` once
  per connected session (`tick_vitals` -> `drain_monster_melee` -> `drive_owner_spawns`, before
  the monster-AI pass gate). The gone scan over every realized point and `RespawnSchedule::due`
  now run once per owner window (the first drive with `now / 50 ms` past the last one), which
  keeps the occurrences due by the window's end in a deadline-ordered queue; later drives of
  the window pop only the occurrences whose deadline they reach, each still run at its exact
  microsecond, and successors due inside the window are inserted in order. A death is now
  observed at the first drive of the next owner window (up to 50 ms later than before; the
  occurrence is still due one full delay after its observation).
- Player headroom (review round on #1984): the pre-production carrier capacity
  (`PREPRODUCTION_FIRST_SLICE_ACTOR_CAPACITY`, 131,072, `node/config.rs`) equals
  `CREATUREAI0-RL-01`, so a maximum activation filled every slot and `reserve_player`
  (`admit_inner`) refused the first player. `check_spawn_activation` now requires the slots left
  after the activation to be at least `SPAWN_ACTIVATION_PLAYER_HEADROOM`, the D128 design target
  of 500 players per Channel (half the carrier for carriers under 1,000 slots, for tests and
  qualification). Sizing the carrier to `CREATUREAI0-RL-04` instead needs `node/config.rs` and
  an owner decision on the capacity; it is outside this task. The headroom does not cover
  monster or player summons or NPCs (owner question in the report).
- Respawn work bound (review round on #1984): each due step cost O(slots) (the census of every
  positioned actor for the Blocked check; `cell_occupied` twice, a deep `slots.clone()` and an
  `occupied.clone()` per admission), and every occurrence due in a window ran in it, so a mass
  death of up to 131,072 creatures cost one window O(points × slots). The drive now runs at most
  `SPAWN_RESPAWN_STEPS_PER_WINDOW_MAX` = 32 steps per owner window (stale queue entries are free
  and bounded by the queue built once per window); the occurrences left over are collected
  again, overdue first, by the next window's `RespawnSchedule::due`. Their successors are timed
  from the step that runs, so a backlog delays them by the windows they wait. A step now reads
  the present players once per drive from the occupied index (`present_player_positions`,
  the census's player rule, no census of every creature), checks the cell once and admits
  through `respawn_into_free_cell`: every foreseeable refusal precedes the first mutation and
  no slot snapshot is taken; after the dead creature's best-effort removal only the protected
  generation exhaustion of the freed slot (#541) can refuse. `resolve_respawn_timer` keeps its
  snapshot restore and is no longer called by the drive. A position index would make each
  step O(1) but positions are written in carrier modules outside this task; the budget is a
  work bound, not a registered limit (noted on `CREATUREAI0-RL-12`; owner question in the
  report).
- Audit at the 131,072 ceiling (review round on #1984), all under the Channel runtime mutex:
  - activation (`check_spawn_activation`, `realize_activation_window`, `admit_spawn_point`):
    one free-slot count per activation, then O(1) per point (free-list admission, binary-search
    source lookup, in-order source insert), ≤ 4,096 points per window; policy binding searches
    the content policy table (content-bounded, `runtime_actor_companion.rs`);
  - activation refusal: was one O(occupied) `unindex_occupied` per removed creature (quadratic,
    up to 130,572 creatures); `remove_all` now removes them and drops their slots from the
    occupied index and the attacker leases in one pass each;
  - gone scan and `RespawnSchedule::due`: O(points) once per owner window (coalesced);
  - respawn steps: bounded as above; a removal is one pass over the occupied index;
  - player admission (`reserve_player`, `commit_reserved_player`): O(1) free-list admission,
    no scan; the headroom keeps 500 slots for it;
  - other `cell_occupied`-style scans: `native_summon_cell_free`, `current_cell_occupied_except`,
    `position_occupied_by_other`, `committed_player_positions`, the census and
    `npc_actor.rs` placement remain O(slots) per call, bounded by their callers' own budgets
    (summons, movement, RL-05 thinks); a position index for them needs paths outside this task.
- At reference scale the 64-entry corpse projection FIFO may evict before a lost-response
  replay; that is an owner question, not changed here.
- The test-only cheese source uses `SpawnSourceId(u16::MAX)`; at 65,536 realized sources it
  refuses as a duplicate (fail-closed).

## Validation

- Review round on #1984 (RL-14 windows, drive coalescing), on the owner's PC (WSL Ubuntu 24.04,
  Rust 1.94.0) at the pushed head:
  - `cargo fmt --all -- --check`: pass
  - `cargo clippy -p oteryn-game-server --all-targets --locked -- -D warnings`: pass
  - `cargo test -p oteryn-game-server --lib --locked`: pass at a2a1a060 (2,740 passed, 0
    failed, 28 ignored); the first run failed one new test assertion (a removed slot keeps its
    generation, so liveness, not `validate_ref`, shows the removal), fixed in a2a1a060
  - Player headroom round (Codex 4242245064, 3b393bcc): pushed before its PC run finished;
    CI found an unresolved `crate::node::config` path in the test (this file is also built
    into crates without the node configuration) and a clippy `manual_is_multiple_of`, both
    fixed in the respawn-bound round
  - Respawn-bound and audit round (Codex 4242270322), on the PC before the push, in a
    private checkout of 3b393bcc plus this round's diff (code and registry identical to the
    pushed commit): `cargo fmt --all -- --check` pass; `cargo clippy -p
    oteryn-game-server --all-targets --locked -- -D warnings` pass; `cargo test -p
    oteryn-game-server --no-run --locked` pass (every test target, including the integration
    tests CI refused); `cargo test -p oteryn-game-server --lib --locked` pass (2,742 passed,
    0 failed, 28 ignored)
- Earlier round: `cargo fmt --all --check`, `python tools/agents/validate_governance.py` and
  `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

- method: implementing agent read the whole diff against owned paths.
- material findings: none open.

## Independent review

- required: per the bound review policy, requested by the control plane on the frozen head.

## PR and closeout

- changed-file review: within owned paths
- merge commit/result: recorded by the control plane at merge
