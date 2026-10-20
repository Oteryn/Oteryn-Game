# CLOCK-1

```yaml
task_id: CLOCK-1
title: "CLOCK-1: offer capability 19 (domain 18 world clock) and the VIS-2 light"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/clock-1-light-20261014
issue: 2053
governing_issue: 1622
pr: 2114
base_sha: 67cacb76
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_clock.rs
  - crates/protocol-oteryn/src/world_clock_tests.rs
  - crates/protocol-oteryn/src/world_spatial_entities.rs
  - docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - apps/game-server/src/gameplay_transport/world_spatial.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - docs/architecture/reviews/OTERYN_GAME_MOVE_RL11_VISIBILITY_DECISION_2026-09-28.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/archive/CLOCK-1.md
public_contracts: [PROTOCOL_OTERYN_V1_REGISTRY.json, world_spatial_v1.proto]
external_repositories: []
```

## Outcome

Mechanics #15 (world clock) and #16 (server-side light) are closed from PARTIAL.

- **Capability 19:** `WORLD_LIGHT_V1` is offered in the production, Item view and bundle World
  sets, and requires 6.
- **Domain 18:**
  - A snapshot is sent at admission, reconnect and transfer, at the session's carried revision
    (`SessionContinuity::world_clock_revision`).
  - Paced deltas are sent from the 250 ms visibility tick, at most one per 10,000 ms.
  - The clock is read at the node's UTC time fact against `DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS` = 0.
  - A wall clock that cannot be read fails the session closed.
- **VIS-2 light:**
  - Fields 11-12 on player and creature entries, at most 6 B.
  - Sent only under capability 19, as a sidecar `EntityLights` map, so the entity types stay
    unchanged.
  - Paced to one change per entity per 1,000 ms.
  - A player's light is its current LIGHT condition.

## Scope notes

- The CP granted the A+B paths in one PR. The connection.rs edits are limited to the capability
  offer, domain 18 and threading the lights into the visibility encoders.
- `world_clock_tests.rs` changed one line: capability 19 is now offered.
- Known gaps:
  - Equipped items give no light: no item LIGHT content rows exist.
  - Monster content light has no runtime path.
  - The epoch comes from no World ruleset field (default 0, CP-approved).
- `RESOURCE_LIMITS_REGISTRY.json` gains only the `WORLDINT0-RL-18` and `WORLDINT0-RL-20` rows
  (CP grant).
- Follow-up (SIM-DETERMINISM, accepted by the CP, Codex 4274232194): a recorded normalized UTC
  tick/admission fact and a replay envelope for domain 18. Until then the UTC fact is read from
  the wall clock once per process and advanced on the monotonic clock.

## Validation

- `cargo +1.95.0 fmt --all --check`
- `cargo test --locked -p oteryn-protocol-oteryn`: 230 passed.
- `cargo test --locked -p oteryn-game-server --lib`: 2992 passed.
- clippy, governance, `git diff --check` and the architecture check: see the PR.

## Self-review

- Fail closed:
  - Light on an NPC or object, level 0, a color without a level, a repeated field or a value over
    255 is refused.
  - Lights without capability 19 are refused.
- A session without 19 gets unchanged bytes (tested).
- The domain 18 revision is advanced before the write, so a later snapshot never winds back.
