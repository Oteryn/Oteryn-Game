# OTV2-20261010-map-floor-1

```yaml
task_id: OTV2-20261010-map-floor-1
title: "MAP-FLOOR-1: floor changes from static map cells (stairs, ladders, holes)"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/map-floor-1-20261010
issue: 1622
pr: 1967
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session_01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-10
updated_at: 2026-10-10
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/map/floor.rs
  - apps/game-server/src/map/floor_catalogue.rs
  - apps/game-server/src/map/mod.rs
  - apps/game-server/src/map/boot.rs
  - apps/game-server/src/gameplay_transport/actor_movement.rs
  - apps/game-server/src/node/serve.rs
  - tools/player-bots/src/live_tests.rs
  - docs/agents/tasks/archive/OTV2-20261010-map-floor-1.md
public_contracts: []
depends_on: []
blocks: []
```

## Outcome

A player moves between z-levels on bundle worlds using the catalogue `floor_change` fact of the placed objects, server-authoritatively, activated at the preprod boot.

## Architecture and source of truth

- PROVEN: ADR-0021 and the world bundle format: floor changes come from the catalogue fact, not a bundle family.
- PROVEN: destination rules ported from `source_floor_change.rs` (Canary queryDestination); sealed `SourceStepCommitProof` reused.
- UNKNOWN: the source height-3 climb rule (the bundle carries no height facts); not modelled.

## Acceptance criteria

- [x] Resolver, boot collection, refusal of unbounded chains, count>0 activation in serve.rs.
- [x] Step commit through the proof on both step paths; stale proof refused.
- [x] Catalogue drift test against content/world.
- [x] Scripted-peer player-bot test.
- [ ] Real-node player-bot and PostgreSQL integration: not done (see Excluded scope).

## Excluded scope

No PostgreSQL integration test: no durable position or floor state exists. No real-node player-bot run. No change to static_cell_engine.rs, monster_ai_cycle.rs, runtime_actor_carrier.rs, connection.rs.

## Implementation / findings

- New `map/floor.rs` and generated `map/floor_catalogue.rs`; boot and serve wiring; bundle step in `actor_movement.rs`.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib`: pass (2671 passed, 0 failed)
- `cargo clippy --locked -p oteryn-player-bots --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-player-bots`: pass (12 passed)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

## Self-review

- exact head: the frozen head
- method/reviewer: task writer
- material findings: none open
- verdict: ready for independent review

## PR and closeout

- changed-file review: owned paths only
- unresolved review threads: none
- merge commit/result: squash merge of #1967
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: candidate frozen
status: completed
branch: claude/map-floor-1-20261010
pr: 1967
blocker: null
next_action: independent review
```
