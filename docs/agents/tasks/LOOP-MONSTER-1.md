# LOOP-MONSTER-1

```yaml
task_id: LOOP-MONSTER-1
title: "LOOP-MONSTER-1: world respawn scheduler; monster ranged, spell, summon AI (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/loop-monster-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 LOOP-MONSTER-1 (rows 94, 96); wave 2"
class: "hard (child graph)"
migration: maybe
registry: maybe
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ai/
  - apps/game-server/src/ai_think/
  - apps/game-server/src/monster_owner_cycle.rs
  - apps/game-server/tests/loop_monster_*.rs (new)
slot_only_paths:
  - mod.rs (scheduler composition)
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "MAGIC-CAST-1: MERGED"
  - "LOOP-RANGED-1: BLOCKING: not merged (active/held: RANGED-CONTENT-1 #2096 then PR2)"
  - "CREATURE_AI0 ACCEPTED: BLOCKING: CANDIDATE"
  - "MAP-OVERLAY-1, MAP-CUTOVER-1b: MERGED #1800/#1809, #1916"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

World-owned respawn scheduler independent of sessions; monster ranged, spell and summon AI.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| MAGIC-CAST-1 | MERGED |
| LOOP-RANGED-1 | BLOCKING: not merged (active/held: RANGED-CONTENT-1 #2096 then PR2) |
| CREATURE_AI0 ACCEPTED | BLOCKING: CANDIDATE |
| MAP-OVERLAY-1, MAP-CUTOVER-1b | MERGED #1800/#1809, #1916 |

**BLOCKING:** LOOP-RANGED-1; CREATURE_AI0 ACCEPTED. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/ai/`
- `apps/game-server/src/ai_think/`
- `apps/game-server/src/monster_owner_cycle.rs`
- apps/game-server/tests/loop_monster_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- mod.rs (scheduler composition)
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/monster_ai_cycle.rs`
- `ai/spawn.rs` respawn

## Inseparable bundles

none beyond registry rows.

## Acceptance criteria

- [ ] Respawn continues with no session; deterministic schedule.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

CREATURE_AI0 §; BUNDLE-MONSTER-AI-1 record. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 LOOP-MONSTER-1 (rows 94, 96).

## Builder instructions

- Work on branch `skeleton/loop-monster-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/loop-monster-1
blocker: LOOP-RANGED-1; CREATURE_AI0 ACCEPTED
next_action: CP rechecks BLOCKING rows
```
