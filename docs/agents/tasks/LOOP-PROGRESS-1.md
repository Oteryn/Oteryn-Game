# LOOP-PROGRESS-1

```yaml
task_id: LOOP-PROGRESS-1
title: "LOOP-PROGRESS-1: level-up vitals/capacity, vocations, skill gain, magic level (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/loop-progress-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 LOOP-PROGRESS-1 (rows 37, 38, 40, 41); wave 1 (migration chain #3)"
class: "hard (Character writes)"
migration: yes
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/progression.rs
  - apps/game-server/src/durability/character_progression.rs
  - apps/game-server/src/gameplay_transport/character_progression_binding.rs
  - apps/game-server/tests/loop_progress_*.rs (new)
slot_only_paths:
  - migration number (chain #3)
  - mod.rs
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "GAME-CHAR-CMD-0: MERGED (CHAR-CMD-ACCEPT-1 #1980)"
  - "CHAR-BUILD-1: MERGED"
  - "W2b: MERGED #2109"
  - "DAWNPORT-1: BLOCKING: not merged (chain #2)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Level-up updates vitals and capacity per vocation; vocations end to end; skill gain on hits; magic level. Production training stays disabled until DEATH-ML-LOSS-1 merges.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| GAME-CHAR-CMD-0 | MERGED (CHAR-CMD-ACCEPT-1 #1980) |
| CHAR-BUILD-1 | MERGED |
| W2b | MERGED #2109 |
| DAWNPORT-1 | BLOCKING: not merged (chain #2) |

**BLOCKING:** DAWNPORT-1. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/domain/progression.rs`
- `apps/game-server/src/durability/character_progression.rs`
- `apps/game-server/src/gameplay_transport/character_progression_binding.rs`
- apps/game-server/tests/loop_progress_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number (chain #3)
- mod.rs
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `gameplay_transport/character_progression_binding.rs`
- `durability/character_progression.rs`
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

migration+writer+durability.

## Acceptance criteria

- [ ] Level-up vitals/capacity match Reference per vocation.
- [ ] Training flag default off in production.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

A13; SKILLS-0; Reference vocation tables. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 LOOP-PROGRESS-1 (rows 37, 38, 40, 41).

## Builder instructions

- Work on branch `skeleton/loop-progress-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/loop-progress-1
blocker: DAWNPORT-1
next_action: CP rechecks BLOCKING rows
```
