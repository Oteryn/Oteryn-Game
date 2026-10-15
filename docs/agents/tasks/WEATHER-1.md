# WEATHER-1

```yaml
task_id: WEATHER-1
title: "WEATHER-1: weather state and change events (skeleton)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/weather-1
issue: 1622
pr: null
base_sha: 5742f30f3
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 1 WEATHER-1 (row 17); wave 2"
class: "impl"
migration: no
registry: yes
owner: unassigned (builder)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/world_weather.rs (new)
  - apps/game-server/src/world_weather_tests.rs (new)
  - crates/protocol-oteryn/src/world_weather.rs (new)
slot_only_paths:
  - connection.rs (`world_clock_delta` l.1355 pattern)
  - mod.rs
  - capabilities.rs
  - crates/protocol-oteryn/src/lib.rs
  - PROTOCOL_OTERYN_V1_REGISTRY.json
public_contracts: []
depends_on:
  - "LOOP-CLOCK-1 slice: MERGED #2071"
  - "CLOCK-1 residue: MERGED #2114"
  - "registry order ITEM-LOOK-1, SEC-1, WEATHER-1: BLOCKING: ITEM-LOOK-1 and SEC-1 not merged; SEC-1 gated on SEC_CLIENT01 -> CP decides whether WEATHER-1 may pass SEC-1"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Skeleton task record (parallel work model, owner-approved 2026-10-15). The builder fills `owned_paths`;
`slot_only_paths` are touched only in the final commit at this PR's merge-queue turn, after merging main.
CP and architect do the final review.

## Goal

World weather state and change events (owner direction 2026-10-13: in scope).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-CLOCK-1 slice | MERGED #2071 |
| CLOCK-1 residue | MERGED #2114 |
| registry order ITEM-LOOK-1, SEC-1, WEATHER-1 | BLOCKING: ITEM-LOOK-1 and SEC-1 not merged; SEC-1 gated on SEC_CLIENT01 -> CP decides whether WEATHER-1 may pass SEC-1 |

**BLOCKING:** registry order ITEM-LOOK-1, SEC-1, WEATHER-1. The builder may prepare owned paths only for children whose gate is met; nothing in slot-only paths until every BLOCKING row is merged or waived by CP.

## Owned paths (builder edits freely)

- apps/game-server/src/world_weather.rs (new)
- apps/game-server/src/world_weather_tests.rs (new)
- crates/protocol-oteryn/src/world_weather.rs (new)

## Slot-only paths (final commit only, at the merge-queue turn, after merging main)

- connection.rs (`world_clock_delta` l.1355 pattern)
- mod.rs
- capabilities.rs
- crates/protocol-oteryn/src/lib.rs
- PROTOCOL_OTERYN_V1_REGISTRY.json
- Always slot-only, whether or not listed: regenerate_content.py outputs, `content/project.json`, `content/manifest.json`, `content/content.lock.json`, `content/interactions/index.json`, `content/world/pins/*`, Rust pins, `docs/contracts/*REGISTRY*.json`, migration numbers, wiring in `gameplay_transport/connection.rs`, `gameplay_transport/mod.rs`, `capabilities.rs`, `crates/protocol-oteryn/src/lib.rs`.

## Wiring points (verified on main@5742f30f3)

- `world_clock.rs` (pure World clock)
- `connection.rs` `world_clock_delta` (l.1355) / `world_clock_utc_ms` (l.504)
- `capabilities.rs` `WORLD_LIGHT` (l.65) pattern

## Inseparable bundles

wire+registry+capability+handler+tests.

## Acceptance criteria

- [ ] Weather deterministic from World seed and clock; no brief: builder writes packet section first, architect review.
- [ ] Builder confirms the residue against main before code (plan §4, 'each worker confirms the residue').

## Validation

- `cargo fmt --all --check`
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (and `-p protocol-oteryn` when the wire changes)
- `cargo test --locked -p oteryn-game-server --quiet`; PostgreSQL 17.6 targets for durable state (`OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test <target>`)
- content: the loading package's tests and the path validator; `python tools/content-migration/regenerate_content.py` at the slot only
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`

## Reference evidence

WORLD_INTERACTION0 §11 (clock); Reference weather. Plan: `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4 Epic 1 WEATHER-1 (row 17).

## Builder instructions

- Work on branch `skeleton/weather-1`; push rarely and batched (every push to this draft runs CI).
- Do not mark the PR ready; CP marks it ready at its merge-queue turn.
- Slot-only paths only in the final commit, after merging main at the queue turn.
- One Codex round on the final head; at most 2 repair rounds, then CP splits the task.

## Context checkpoint

```yaml
last_progress: skeleton created by CP architect
status: waiting
branch: skeleton/weather-1
blocker: registry order ITEM-LOOK-1, SEC-1, WEATHER-1
next_action: CP rechecks BLOCKING rows
```
