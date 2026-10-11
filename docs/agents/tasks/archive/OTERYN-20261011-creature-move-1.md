# OTERYN-20261011-creature-move-1

```yaml
task_id: OTERYN-20261011-creature-move-1
title: "CREATURE-MOVE-1a: spawned monsters step (chase, flee, wander) on the entry room"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/creature-move-1-20261011
issue: 1622
pr: 1977
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session_01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-11
updated_at: 2026-10-11
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/monster_ai_cycle.rs
  - docs/agents/tasks/active/OTERYN-20261011-creature-move-1.md
  - docs/agents/tasks/archive/OTERYN-20261011-creature-move-1.md
public_contracts: []
depends_on: [SPAWN-1a]
blocks: []
```

## Outcome

Monsters spawned at boot on the entry room move on the Channel owner turn: the CREATURE-AI-1 think already proposes one cardinal step (chase a perceived player, flee when hurt, wander within `radius_tiles` of the spawn cell). The owner committed none (`|_| false`); it now admits a step onto an open, walkable, ordinary cell and commits it through the Movement carrier.

## Architecture and source of truth

- PROVEN: the packet `OTERYN_GAME_CREATURE_MOVE1_STEPS_PATHS_AND_GOALS_PACKET_2026-10-04.md` §1.4 walkability and R2: creatures never enter protection-zone or floor-change tiles, never open doors.
- PROVEN: the step is the think's own deterministic proposal (`AI_WANDER` draw, nearest target); nothing new is drawn.
- NARROWED: pacing is one step per think (1 s, `CREATURE_THINK_INTERVAL_MILLIS`), not the packet's speed-based step timer. Wander cadence is the profile `interval_ms`.
- NARROWED: only the qualified entry room (fixture World) runs the monster cycle; bundle Worlds have no monster think yet (`serve.rs` passes no `qualified_room` for a bundle).
- UNKNOWN: whether clients observe a creature position change without a further wire; not exercised end to end.

## Acceptance criteria

- [x] A creature step is committed only onto a Walkable cell not covered by a closed door and neither protection-zone nor floor-change.
- [x] Creature position is refreshed after the think pass so the same pass swings and perceives from the new cell.
- [ ] Speed-based step timer, A* paths, walk back, leash, dance, near-target x2 (packet §1.1-§1.5, `relocate_creature`): deferred.
- [ ] Bundle World monster cycle: deferred (SPAWN-1b / BOOT-RAT-POLICY-1).
- [ ] Real-node player-bot run: not done.

## Excluded scope

No change to `ai_think*`, `movement*`, `runtime_actor_carrier.rs`, `serve.rs`, any wire or contract.

## Implementation / findings

- `monster_ai_cycle.rs`: `creature_cell_admissible`, the `walkable` predicate passed to `run_due_thinks`, and the post-think position refresh.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-game-server --lib --tests -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib ai`: pass (623 passed, 0 failed)
- `cargo test --locked -p oteryn-game-server --lib monster_ai_cycle`: pass (6 passed, 0 failed)
- `python tools/agents/validate_governance.py`: pass (the only findings are handover paths absent from the sparse worktree used here)
- `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

- exact head: the frozen head
- method/reviewer: task writer
- material findings: none open
- verdict: ready for independent review

## PR and closeout

- changed-file review: owned paths only
- unresolved review threads: none
- merge commit/result: squash merge of #1977
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: candidate frozen
status: completed
branch: claude/creature-move-1-20261011
pr: 1977
blocker: null
next_action: independent review
```
