# CREATURE-MOVE-1

```yaml
task_id: CREATURE-MOVE-1
title: "CREATURE-MOVE-1: residue: speed step timer, A*, walk back, leash, dance, near-target, relocate (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/creature-move-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md App. 7 LOOP-MONSTER-1 graph, CREATURE-MOVE-1 residue; wave 2"
class: "hard (movement); movement and performance review"
migration: no
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ai/path_proposal.rs
  - apps/game-server/src/ai/resolution.rs
  - apps/game-server/src/movement/path_search.rs
  - apps/game-server/src/movement/speed.rs
  - apps/game-server/tests/creature_move_*.rs (new)
slot_only_paths:
  - RESOURCE_LIMITS_REGISTRY.json (path budget rows, if new)
  - mod.rs (only if the step timer lane changes composition)
public_contracts: []
depends_on:
  - "CREATURE-AI-1: MERGED #1813"
  - "SPEED-1: MERGED #1715"
  - "CREATURE-MOVE-1a/1b: MERGED #1977, #1987"
  - "CHASE-1 path search: MERGED #2113, #2125"
  - "CREATURE-MOVE-1 packet: ACCEPTED on merge (CREATURE-MOVE-1-PACKET-1)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Close the CREATURE-MOVE-1a deferrals: speed-based creature step timer, A* paths (reusing CHASE-1 path search), walk back, leash, dance, near-target x2, `relocate_creature` (packet §1.1-§1.5).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| CREATURE-AI-1 | MERGED #1813 |
| SPEED-1 | MERGED #1715 |
| CREATURE-MOVE-1a/1b | MERGED #1977, #1987 |
| CHASE-1 path search | MERGED #2113, #2125 |
| CREATURE-MOVE-1 packet | ACCEPTED on merge (CREATURE-MOVE-1-PACKET-1) |

All dependencies merged; the task may start.

## Owned paths (builder edits freely)

- `apps/game-server/src/ai/path_proposal.rs`
- `apps/game-server/src/ai/resolution.rs`
- `apps/game-server/src/movement/path_search.rs`
- `apps/game-server/src/movement/speed.rs`
- apps/game-server/tests/creature_move_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- RESOURCE_LIMITS_REGISTRY.json (path budget rows, if new)
- mod.rs (only if the step timer lane changes composition)
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `movement.rs` `step_cardinal`
- `foundation/owner_timer.rs` DEADLINE_STATE
- `gameplay_transport/monster_ai_cycle.rs`

## Inseparable bundles

none (no wire, no migration).

## Acceptance criteria

- [ ] Creature steps at its own speed; A* path bounded per MOVE-RL budgets; leash/walk back deterministic; replay identical.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

OTERYN_GAME_CREATURE_MOVE1_STEPS_PATHS_AND_GOALS_PACKET_2026-10-04.md §1.1-§1.5; archive OTERYN-20261011-creature-move-1.md l.44. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` App. 7 LOOP-MONSTER-1 graph, CREATURE-MOVE-1 residue.

## Builder instructions

- Work on branch `skeleton/creature-move-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/creature-move-1
blocker: null
next_action: builder confirms residue against main and starts owned paths
```
