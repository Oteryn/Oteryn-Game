# LOOP-COMBAT-MODES-1

```yaml
task_id: LOOP-COMBAT-MODES-1
title: "LOOP-COMBAT-MODES-1: fight, chase and secure modes (mechanics plan row 20)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/loop-combat-modes-1-chase-20261014
issue: 2049
governing_issue: 1622
pr: 2113
base_sha: 67cacb76
head_sha: null
final_head_sha: null
owner: claude-code-worker (CP session_01XYC239givrMonrYoBwwECV)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-13
updated_at: 2026-10-14
owned_paths:
  - apps/game-server/src/combat/attack/mod.rs
  - apps/game-server/src/combat/attack/chase.rs
  - apps/game-server/src/gameplay_transport/attack.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/foundation/channel_owner_auto_attack_tests.rs
  - docs/agents/tasks/active/LOOP-COMBAT-MODES-1.md
  - docs/agents/tasks/archive/LOOP-COMBAT-MODES-1.md
public_contracts: []
external_repositories: []
```

## Outcome

Second PR of the task (the first, #2070, pinned target retention). HARD task per CP re-scope
2026-10-14: server-driven chase (CHASE-1, RANGED-0 §8) on the existing wire; `ChaseMode` is
already on the wire, so there is no protocol or registry change. With `chase=CHASE` and a held
creature target the Channel owner steps the player towards the nearest tile next to the target,
one paced Movement-owner step at a time; `STAND` never moves the player. Secure mode is deferred
to PVP-RT-1 (see below).

## Architecture and source of truth

- PROVEN: RANGED-0 §8 (`OTERYN_GAME_RANGED0_..._CHASE_DECISION_2026-10-01.md`) and CREATURE-AI-0 §5.3 path profile: A*, neighbours N, E, S, W, 10 per step, +40 over a creature, ties by total, estimate, neighbour order, discovery order; 1,024 expansions, 128 route steps, 12-tile distance; one search per 1,000 ms, a failed one waits 2,000 ms; `RANGED0-RL-03` 64 searches per 50 ms owner window per Channel.
- DERIVED: the pure search and per-attacker route live in `combat/attack/chase.rs`; the owner decision (`ChannelAttackStates::chase_decision`) reads the held target's facts (alive, visible, same floor, no re-entry protection on the attacker) and the path tiles (`chase_path_tile`: Walkable, no closed door, no teleport, protection-zone or floor-change tile, the creature step rule's Unknown handling).
- DERIVED: a chase step is taken through the existing `paced_step` (same revalidation, speed, field ingress and pending-spell refusal as a client step) on the connection's own `StepPacer`, with at most one pending (`chase_due`); a step that does not move drops the route. A client step cancels the pending chase step and the route; chase re-paths at its next 1,000 ms check. Its frames reuse the client step's moved-step frames (`moved_step_frames`, extracted unchanged), sent without a command result.
- DERIVED: the connection arms the chase check from the 250 ms combat tick only when the domain 10 state holds a target with `chase=CHASE`, so a non-chasing session takes no extra owner locks.

## Acceptance criteria

- [x] Path search: fewest steps, deterministic ties, wall detour, +40 creature cost, 12-tile and work bounds (`combat_attack::chase::tests`).
- [x] Route state: one search per 1,000 ms, 2,000 ms after a failure, refused step drops the route, client step yields, Channel budget of 64 per window.
- [x] Owner: CHASE steps towards a distant held rat, STAND never moves, adjacent or re-entry protected holds, clearing the target ends the chase (`channel_owner_auto_attack_tests`).

## Excluded scope and recorded gaps

- Secure mode: `set_target` refuses visible players (`TargetNotACreature`) before any PvP legality, so `combat/pvp/legality.rs`'s `SecureMode` refusal cannot be called cleanly; deferred to PVP-RT-1 (accepted by CP 2026-10-14).
- The VSL-MOVE-01 §5 `CHASE` movement-occurrence kind has no runtime representation yet (no `MovementOccurrence` type); a chase step commits as an ordinary player step.
- `RANGED0-RL-03` "served after creature searches in actor-id order": chase checks run per connection, so the per-window budget is first-come; the ordering is `PARITY_PENDING`.
- Canary searches without the 12-tile chase bound: `PARITY_PENDING`.
- Client: the ATTACK-WIRE-1 client ships the chase toggle disabled until CHASE-1; enabling it is a client follow-up (CLIENT-COMBAT-INPUT-1 #1938).
- W2b seam: the auto-attack swing resolves at one site (`drain_auto_attacks`, after `record_hit`), where a fist skill try can hook.

## Validation

- `cargo +1.95.0 fmt --all --check`: pass
- `cargo +1.95.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo +1.95.0 test --locked -p oteryn-game-server --lib`: pass (3002 passed, 0 failed)
- `python tools/agents/validate_governance.py`: OK
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass
