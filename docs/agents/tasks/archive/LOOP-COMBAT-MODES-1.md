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
  - apps/game-server/src/combat/attack/tests.rs
  - apps/game-server/src/movement/path_search.rs
  - apps/game-server/src/movement.rs
  - apps/game-server/src/gameplay_transport/attack.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/foundation/channel_owner_auto_attack_tests.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
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

- PROVEN: RANGED-0 §8 (`OTERYN_GAME_RANGED0_..._CHASE_DECISION_2026-10-01.md`) and CREATURE-AI-0 §5.3 path profile: A*, neighbours N, E, S, W, 10 per step, +40 over a creature, ties by total, estimate, neighbour order, discovery order; 1,024 expansions (`AI01-PATH-SEARCH-WORK`, CREATURE-AI-0 §5.3; `RANGED0-RL-04` for a chase), at most 128 route steps (`AI01-ROUTE-STEPS`, CREATURE-AI-0 §5.3), a 12-tile search distance (Chebyshev from the start; `RANGED0-RL-05`); one search per 1,000 ms (`RANGED0-RL-06`) and a failed one waits 2,000 ms (`RANGED0-RL-07`); RANGED0-RL ids as #2111 defines them in RANGED-0 §8; `RANGED0-RL-03` 64 searches per 50 ms owner window per Channel.
- DERIVED: the bounded search is the shared CREATURE-MOVE-1 §5.3 path search in `movement/path_search.rs` (Movement-owned, creatures reuse it later; RANGED-0 row 32 amendment); the per-attacker route, its search budget and the refused-cell memory (a cell the Movement owner refused a chase step into as `Blocked` stays out of the search for 2 s, at most 8 cells (`RANGED0-RL-08`, registered in `RESOURCE_LIMITS_REGISTRY.json` by this task), cleared on reset; a transient refusal only drops the route; blocking items have no in-memory projection and the owner revalidates each step) live in `combat/attack/chase.rs`; the owner decision (`ChannelAttackStates::chase_decision`) reads the held target's facts (alive, visible, same floor, no re-entry protection on the attacker) and the path tiles (`chase_path_tile`: Walkable, no closed door, no teleport, protection-zone or floor-change tile, the creature step rule's Unknown handling) and their cost (+40 over a tile a creature, an NPC or another player stands on; the target's own tile is no path tile).
- DERIVED: neighbours are cardinal only; CREATURE-AI-0 §5.3's diagonals stay out because the Movement owner admits no diagonal step (diagonal chase steps wait for their own decision, RANGED-0 Implementation brief).
- DERIVED: a search state is a tile and its step count, with dominance pruning (a label no cheaper and no shorter than one already found is dropped), so a cheaper but longer label never hides a dearer, shorter route the step bound still admits.
- DERIVED: a chase step is taken through the existing `paced_step` (same revalidation, speed, field ingress and pending-spell refusal as a client step) on the connection's own `StepPacer`, with at most one pending (`chase_due`); a step that does not move drops the route; only a `Blocked` refusal remembers the cell, timed from when the refusal returned. A client step cancels the pending chase step and the route, and one ready client frame is read before a due chase step, which then runs before the next frame (non-step traffic never postpones it indefinitely); a target or mode change drops the route and the refused cells but keeps the per-player search cooldown; chase re-paths at its next 1,000 ms check. Its frames reuse the client step's moved-step frames (`moved_step_frames`, extracted unchanged), sent without a command result.
- DERIVED: the connection arms the chase check from the 250 ms combat tick only when the domain 10 state holds a target with `chase=CHASE`, so a non-chasing session takes no extra owner locks.

## Acceptance criteria

- [x] Path search: fewest steps, deterministic ties, wall detour, +40 creature cost, the 12-tile chase distance and 128-step route bound, a dearer shorter route under the step bound, the 1,024/1,025 expansion boundary on a corridor, an unreachable goal within the work bound (`movement::path_search::tests`).
- [x] Route state: one search per 1,000 ms, 2,000 ms after a failure, refused step drops the route, a transient refusal keeps the cell, a 12-tile chase search and a target beyond it, client step yields, Channel budget of 64 per window with oldest-waiting-then-actor-id places, a blocking item the owner refuses on the shortest route is detoured, refused cells expire after 2 s and at most 8 are kept (`combat_attack::chase::tests`).
- [x] Owner: CHASE steps towards a distant held rat, STAND never moves, adjacent or re-entry protected holds, clearing the target ends the chase, the actor-owned pacing deadline survives a replaced connection (`channel_owner_auto_attack_tests`).

## Excluded scope and recorded gaps

- Secure mode: `set_target` refuses visible players (`TargetNotACreature`) before any PvP legality, so `combat/pvp/legality.rs`'s `SecureMode` refusal cannot be called cleanly; deferred to PVP-RT-1 (accepted by CP 2026-10-14).
- The VSL-MOVE-01 §5 `CHASE` movement-occurrence kind has no runtime representation yet (no `MovementOccurrence` type); a chase step commits as an ordinary player step.
- `RANGED0-RL-03`: a due search registers in its 50 ms owner window and is admitted from the next one, so the window's whole due set is known first; searches keep their places by registration window, then actor id (`ChannelAttackStates`), owner time is read under the attack lock and a window never reopens; a search not yet admitted is retried by its connection one window later. Sharing CREATURE-AI-0 `RL-09` with its 16-search chase reserve lands with CREATURE-MOVE-1, the first creature user of the shared search; until then no creature search competes with chase.
- Pacing: the step deadline is the actor's (`ChannelAttackStates`), claimed by each client or chase step in one check-and-set and recorded when the step ends; the claim holds until its own connection records the end, however long the step takes, and is freed when that connection is gone (a liveness holder, never a timeout), so a replaced or resumed connection never steps sooner (RANGED-0 §8); the one-step buffer stays connection-local.
- Canary searches without the 12-tile search distance: `PARITY_PENDING`.
- Client: the ATTACK-WIRE-1 client ships the chase toggle disabled until CHASE-1; enabling it is a client follow-up (CLIENT-COMBAT-INPUT-1 #1938).
- W2b seam: the auto-attack swing resolves at one site (`drain_auto_attacks`, after `record_hit`), where a fist skill try can hook.

## Validation

- `cargo +1.95.0 fmt --all --check`: pass
- `cargo +1.95.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo +1.95.0 test --locked -p oteryn-game-server --lib`: pass (3011 passed, 0 failed)
- `python tools/agents/validate_governance.py`: OK
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass
