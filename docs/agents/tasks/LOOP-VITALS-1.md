# LOOP-VITALS-1

```yaml
task_id: LOOP-VITALS-1
title: "LOOP-VITALS-1: regeneration accumulator, food time, soul, stamina (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/loop-vitals-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 LOOP-VITALS-1 (rows 30, 32, 33); wave 2"
class: "hard (Character state)"
migration: yes
registry: no
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/food_regeneration.rs
  - apps/game-server/src/durability/character_vitals*.rs (new)
  - apps/game-server/tests/loop_vitals_*.rs (new)
slot_only_paths:
  - migration number
  - mod.rs (`tick_vitals` composition)
  - RESOURCE_LIMITS_REGISTRY.json
public_contracts: []
depends_on:
  - "ITEM-USE-1c: MERGED #2082"
  - "CHAR-VITALS-1: MERGED #2045"
  - "LOOP-PROGRESS-1: BLOCKING: not merged (chain #3)"
  - "OFFLINE0 (STAMINA-1): BLOCKING: CANDIDATE"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

Durable regeneration accumulator, food-time ingress, soul regen and soul orb, stamina.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| ITEM-USE-1c | MERGED #2082 |
| CHAR-VITALS-1 | MERGED #2045 |
| LOOP-PROGRESS-1 | BLOCKING: not merged (chain #3) |
| OFFLINE0 (STAMINA-1) | BLOCKING: CANDIDATE |

**BLOCKING:** LOOP-PROGRESS-1; OFFLINE0 (STAMINA-1). The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- `apps/game-server/src/spell/food_regeneration.rs`
- apps/game-server/src/durability/character_vitals*.rs (new)
- apps/game-server/tests/loop_vitals_*.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- migration number
- mod.rs (`tick_vitals` composition)
- RESOURCE_LIMITS_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `connection.rs` `tick_vitals` (l.782) / `observe_vitals` (l.727) via authority methods
- `gameplay_transport/last_position.rs` vitals projection
- `apps/game-server/migrations/` (last 0087_character_commands.sql); migration + writer + reconcile + guards land together

## Inseparable bundles

migration+writer+durability.

## Acceptance criteria

- [ ] Regen survives restart; food time caps per Reference.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

DUR-02; CHAR-VITALS-0; Reference regen tables. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 LOOP-VITALS-1 (rows 30, 32, 33).

## Builder instructions

- Work on branch `skeleton/loop-vitals-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/loop-vitals-1
blocker: LOOP-PROGRESS-1; OFFLINE0 (STAMINA-1)
next_action: CP rechecks BLOCKING rows
```
