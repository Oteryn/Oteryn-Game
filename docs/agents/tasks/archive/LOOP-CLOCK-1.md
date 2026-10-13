# LOOP-CLOCK-1

```yaml
task_id: LOOP-CLOCK-1
title: "LOOP-CLOCK-1: World clock, day phase and light (capability 19, domain 18)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/loop-clock-1-20261013
issue: 2053
governing_issue: 1622
pr: null
base_sha: 4ac13390
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_clock.rs
  - apps/game-server/src/lib.rs
  - crates/protocol-oteryn/src/world_clock.rs
  - crates/protocol-oteryn/src/world_clock_tests.rs
  - crates/protocol-oteryn/src/lib.rs
  - docs/contracts/protocol-oteryn/v1/world_clock_v1.proto
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/agents/tasks/archive/LOOP-CLOCK-1.md
public_contracts: [PROTOCOL_OTERYN_V1_REGISTRY.json]
external_repositories: []
```

## Outcome

Registers capability 19 `WORLD_LIGHT_V1` (requires 6, no command) and state domain 18 `WORLD_CLOCK`
(snapshot 1 and delta 1, `WorldClockV1`, at most 9 bytes), and adds the pure World clock: Tibian minute
from a UTC time fact and the World epoch, the light curve, `is_day` / `world_time` predicates and the
delta pacer (WORLD-INTERACTION-0 §10.2, §11; WORLDINT0-RL-16..18). Capability stays `offered: false`.

## Scope notes

- `world_runtime.rs` is not changed: nothing in it needs the clock yet, and the epoch
  (`world_clock_epoch_utc_ms`, World ruleset) and the offer composition belong to the negotiation work.
- VIS-2 `light` on entities (§10.2) is a separate registry change and is not part of this task.
- Number 19 was only "proposed" for `LOGOUT_V1` in ARCH-KILL-REWARD-LOGOUT-PACKETS §1.6; it was
  unregistered, so LOGOUT-1 must take the next free capability number. Reported to the control plane.

## Validation

- `cargo +1.95.0 fmt --all --check`: pass
- `cargo test --locked -p oteryn-protocol-oteryn`: pass (219)
- `cargo +1.95.0 test --locked -p oteryn-game-server --lib world_clock`: pass (5)
- `oteryn-architecture-check workspace .`: pass
- `cargo clippy` on both crates: no findings in the changed files

## Self-review

Registry JSON parses and is pinned by `registry_binds_the_clock_wire_ids_and_limits`; encoder and decoder
refuse the same values; the pacer never sends before a snapshot.
