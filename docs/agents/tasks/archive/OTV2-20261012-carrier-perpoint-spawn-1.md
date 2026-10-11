# OTV2-20261012-carrier-perpoint-spawn-1

```yaml
task_id: OTV2-20261012-carrier-perpoint-spawn-1
title: "CARRIER-PERPOINT-SPAWN-1: one runtime spawn source per bundle source, with per-point creature, delay and health"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
implementation_branch: claude/carrier-perpoint-20261012
issue: 1622
pr: 2002
next_action: "SPAWN-1b (#1981) emits one ActivationSpawnSourceFacts per bundle source from table_activation_spawns and calls begin_activation_spawn_sources"
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - docs/agents/tasks/archive/OTV2-20261012-carrier-perpoint-spawn-1.md
```

## Result

- Unblocks SPAWN-1b (#1981, Codex P1 4243711197). The carrier held one creature, one respawn delay and one initial health per spawn source, so SPAWN-1b split every bundle source into runs. The pinned pre-production bundle (51,078 sources, 82,673 active points) became 70,206 carrier sources, above `CREATUREAI0-RL-02` (65,536), and the point ordinal restarted in every run.
- `SpawnDefinition` now holds the creature key, respawn delay and initial health per point (`SpawnPointDefinition`, by point ordinal, beside `placement_cells`). The occupancy retry interval and `blockable` stay per source. `SpawnDefinition::new` (uniform source) keeps its signature and builds the same value for every point.
- New activation seam: `ActivationSpawnPointFacts` (cell, respawn delay ms, creature key, initial health) and `ActivationSpawnSourceFacts` (points in authored order, occupancy retry ms), taken by `ChannelRuntimeV1::begin_activation_spawn_sources`. Each source stays one runtime source: it counts once against RL-02 and its point ordinals run `0..n` in authored order. `begin_activation_spawns` and `ActivationSpawnFacts` are unchanged for the fixture and qualification callers.
- Realization (`admit_spawn_source`, `admit_spawn_point`), the gone scan, the respawn drive's timing (`spawn_timing`), `resolve_respawn_timer` and `respawn_into_free_cell` read the point's own creature, health and delay. `resolve_respawn_timer` reads them before its snapshot, so it changes nothing when they are missing.
- Unchanged: the `CREATUREAI0-RL-01` to `-03` checks (RL-03 is now also checked before any point is converted, and RL-02 before any source is converted), the slot headroom, the windowed realization (`CREATUREAI0-RL-14`), and the deadline index and bounded gone scan of SPAWN-CEILINGS-1 (#1984).
- Tests: `spawn1b_mixed_point_source_is_one_runtime_source_with_per_point_facts` (one source of rat 60 s / bat 30 s / rat 30 s: ordinals 0..2 in one source, per-point creature and health, per-point respawn deadlines, respawn as the point's own creature), `spawn1b_mixed_point_source_keeps_the_per_source_checks` (64 points pass, 65 refuse, empty source, zero or overflowing health or delay; a refused source begins nothing) and `spawn1b_bundle_scale_sources_stay_under_the_source_ceiling` (51,078 mixed sources with 82,673 points pass RL-01/RL-02 and realize window by window as 51,078 sources; 70,206 split runs and 65,537 sources refuse `CapacityExceeded`).

## Follow-up for #1981 (out of owned paths)

- `apps/game-server/src/map/boot.rs`: `table_activation_spawns` returns one `ActivationSpawnSourceFacts` per bundle source (no grouping), each point `((x, y, -floor), respawn_ms, creature key, health)`, with `BUNDLE_OCCUPANCY_RETRY_MS`; its two tests change to that shape.
- `apps/game-server/src/node/serve.rs`: `realize_activation_paced` calls `begin_activation_spawn_sources` instead of `begin_activation_spawns`.

## Validation

- cargo fmt --all --check: pass
- git diff --check: pass
- cargo test --locked -p oteryn-game-server --lib -- spawn: pass (58 passed, 2 ignored; the three spawn1b tests pass)
- cargo clippy and the full cargo test -p oteryn-game-server: CI (disk budget)
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass (60 tests OK)
