# LOOP-COMBAT-MODES-1

```yaml
task_id: LOOP-COMBAT-MODES-1
title: "LOOP-COMBAT-MODES-1: fight, chase and secure modes (mechanics plan row 20)"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/loop-combat-modes-1-20261013
issue: 2049
governing_issue: 1622
pr: null
base_sha: 4ac13390
head_sha: null
final_head_sha: null
owner: claude-code-worker (CP session_01XYC239givrMonrYoBwwECV)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-13
updated_at: 2026-10-13
owned_paths:
  - apps/game-server/src/combat/attack/tests.rs
  - apps/game-server/src/gameplay_transport/attack.rs
  - docs/agents/tasks/active/LOOP-COMBAT-MODES-1.md
public_contracts: []
external_repositories: []
```

## Outcome

Row 20 stays PARTIAL (this PR refs #2049, it does not close it). Fight modes already act (attack
factor on the swing, defence factor after a swing); secure mode has no creature-target effect.
Server-driven chase (CHASE-1) is NOT implemented here: the RANGED-0 decision
(`OTERYN_GAME_RANGED0_..._CHASE_DECISION_2026-10-01.md`) specifies server-driven steps through the
Movement owner, and `CHASE` behaves like `STAND` today. Owner/CP decision 2026-10-13: chase stays
client-driven (#1942) for now; this PR only pins the target-retention behaviour a client-driven
chase relies on.

## Architecture and source of truth

- PROVEN: `ChaseMode` and `secure` are already on the wire (`protocol-oteryn` attack.rs); no registry change.
- PROVEN: the modes are runtime-only per (actor, session) and readable through `ChannelAttackStates::combat_state(..).modes`; no new accessor is needed.
- PROVEN: an out-of-range target is held, not cleared (`SwingWait::OutOfRange`), so a client that walks after its target needs no server change.
- PROVEN: Tibia fight modes have no targeting effect.
- DERIVED: server-driven chase would step the player through `gameplay_transport/actor_movement.rs` (speed, pacing, floor change) and can desync the client's predicted position.

## Acceptance criteria

- [x] A fleeing target is held through repeated out-of-range polls and swings once in reach; only loss of sight clears it (`a_fleeing_target_is_held_for_a_chasing_client_until_it_is_in_reach`).
- [x] The attack module doc states that server-driven chase is not implemented and CHASE behaves like STAND.

## Excluded scope

No code in `movement/`, no client work, no protocol or migration change.

## Follow-up (not now)

- CHASE-1 server-driven chase (the row-20 remainder) and secure mode against PvP targets: depends on #2041 (STEP-PACE-WIRE-1), LOOP-WORLD-1 and PVP-1 (#2032); needs `actor_movement.rs` in a future owned set.

## Validation

- `cargo +1.95.0 fmt --all --check`
- `cargo +1.95.0 test --locked -p oteryn-game-server --lib -- combat:: ai_think gameplay_transport::ordinary_combat`
