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
- Performance review item (the governance validator defines no marker for it): the per-window
  scan is still O(spawn points) and `cell_occupied` is O(slots) per admission. Event-driven gone
  detection needs a death hook in the carrier's other modules and `ai/spawn.rs`, outside this
  task.
- At reference scale the 64-entry corpse projection FIFO may evict before a lost-response
  replay; that is an owner question, not changed here.
- The test-only cheese source uses `SpawnSourceId(u16::MAX)`; at 65,536 realized sources it
  refuses as a duplicate (fail-closed).

## Validation

- Review round on #1984 (RL-14 windows, drive coalescing), on the owner's PC (WSL Ubuntu 24.04,
  Rust 1.94.0) at the pushed head:
  - `cargo fmt --all -- --check`: pass
  - `cargo clippy -p oteryn-game-server --all-targets --locked -- -D warnings`: pass
  - `cargo test -p oteryn-game-server --lib --locked`: pass (2,740 passed, 0 failed, 28
    ignored); the first run failed one new test assertion (a removed slot keeps its
    generation, so liveness, not `validate_ref`, shows the removal), fixed in the next commit
- Earlier round: `cargo fmt --all --check`, `python tools/agents/validate_governance.py` and
  `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

- method: implementing agent read the whole diff against owned paths.
- material findings: none open; the unverified build is the open risk.

## Independent review

- required: per the bound review policy, requested by the control plane on the frozen head.

## PR and closeout

- changed-file review: within owned paths
- merge commit/result: recorded by the control plane at merge
